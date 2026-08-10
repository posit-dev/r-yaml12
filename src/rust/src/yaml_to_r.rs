use crate::handlers::HandlerRegistry;
use crate::r_ext::{self, PreservedSexp};
use crate::warning::emit_warning;
use crate::{api_other, Fallible};
use saphyr::{Mapping, Scalar, Tag, Yaml, YamlLoader};
use saphyr_parser::{Parser, ScalarStyle};
use savvy::{
    NotAvailableValue, OwnedIntegerSexp, OwnedListSexp, OwnedLogicalSexp, OwnedRealSexp, Sexp,
    StringSexp,
};
use savvy_ffi as ffi;
use std::{
    fs,
    mem::{self, MaybeUninit},
};

fn resolve_representation(node: &mut Yaml, _simplify: bool) {
    let (value, style, tag) = match mem::replace(node, Yaml::BadValue) {
        Yaml::Representation(value, style, tag) => (value, style, tag),
        other => {
            *node = other;
            return;
        }
    };

    let is_plain_empty = style == ScalarStyle::Plain && value.trim().is_empty();

    let parsed = match tag {
        Some(tag) => {
            if tag.is_yaml_core_schema() {
                match tag.suffix.as_str() {
                    "str" => Yaml::value_from_cow_and_metadata(value, style, Some(&tag)),
                    "null" => {
                        if is_plain_empty {
                            Yaml::Value(Scalar::Null)
                        } else {
                            Yaml::value_from_cow_and_metadata(value, style, Some(&tag))
                        }
                    }
                    "binary" | "set" | "omap" | "pairs" | "timestamp" => {
                        Yaml::Tagged(tag, Box::new(Yaml::Value(Scalar::String(value))))
                    }
                    _ => {
                        let parsed =
                            Yaml::value_from_cow_and_metadata(value.clone(), style, Some(&tag));
                        if matches!(parsed, Yaml::BadValue)
                            && !matches!(
                                tag.suffix.as_str(),
                                "bool" | "int" | "float" | "null" | "str"
                            )
                        {
                            Yaml::Tagged(tag, Box::new(Yaml::Value(Scalar::String(value))))
                        } else {
                            parsed
                        }
                    }
                }
            } else {
                Yaml::Tagged(tag, Box::new(Yaml::Value(Scalar::String(value))))
            }
        }
        None if is_plain_empty => Yaml::Value(Scalar::Null),
        None => Yaml::value_from_cow_and_metadata(value, style, None),
    };

    *node = parsed;
}

fn yaml_to_robj(
    node: &mut Yaml,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    match node {
        Yaml::Value(scalar) => scalar_to_robj(scalar),
        Yaml::Tagged(tag, inner) => convert_tagged(tag, inner.as_mut(), simplify, handlers),
        Yaml::Sequence(seq) => sequence_to_robj(seq, simplify, handlers),
        Yaml::Mapping(map) => mapping_to_robj(map, simplify, handlers),
        Yaml::Alias(_) => Err(api_other(
            "Internal error: encountered unresolved YAML alias node",
        )),
        Yaml::BadValue => Err(api_other("Encountered an invalid YAML scalar value")),
        Yaml::Representation(_, _, _) => {
            resolve_representation(node, simplify);
            yaml_to_robj(node, simplify, handlers)
        }
    }
}

fn scalar_to_robj(scalar: &Scalar) -> Fallible<Sexp> {
    match scalar {
        Scalar::Null => Ok(r_ext::null()),
        Scalar::Boolean(value) => r_ext::logical_scalar(*value),
        Scalar::Integer(value) => {
            if let Ok(v) = i32::try_from(*value) {
                r_ext::integer_scalar(v)
            } else {
                r_ext::real_scalar(*value as f64)
            }
        }
        Scalar::FloatingPoint(value) => r_ext::real_scalar(value.into_inner()),
        Scalar::String(value) => r_ext::string_scalar(value.as_ref()),
    }
}

fn scalar_to_list_element<'a>(scalar: &'a Scalar<'_>) -> Fallible<r_ext::ListElement<'a>> {
    Ok(match scalar {
        Scalar::Null => r_ext::ListElement::null(),
        Scalar::Boolean(value) => r_ext::ListElement::logical(*value),
        Scalar::Integer(value) => match i32::try_from(*value) {
            Ok(value) => r_ext::ListElement::integer(value),
            Err(_) => r_ext::ListElement::real(*value as f64),
        },
        Scalar::FloatingPoint(value) => r_ext::ListElement::real(value.into_inner()),
        Scalar::String(value) => return r_ext::ListElement::string(value.as_ref()),
    })
}

fn prepare_list_element(
    node: &mut Yaml,
    index: usize,
    length: usize,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
    target: &mut Option<OwnedListSexp>,
) -> Fallible<bool> {
    resolve_representation(node, simplify);
    if matches!(node, Yaml::Value(_)) {
        return Ok(true);
    }

    if target.is_none() {
        *target = Some(OwnedListSexp::new(length, false)?);
    }
    let value = yaml_to_robj(node, simplify, handlers)?;
    target.as_mut().unwrap().set_value(index, value)?;
    // The R value is rooted now; retain only a marker for the final batch.
    *node = Yaml::BadValue;
    Ok(false)
}

fn prepared_list_element<'a>(node: &'a Yaml<'_>) -> Fallible<r_ext::ListElement<'a>> {
    match node {
        Yaml::Value(scalar) => scalar_to_list_element(scalar),
        _ => Ok(r_ext::ListElement::skip()),
    }
}

fn materialize_node_list(
    nodes: &mut [Yaml],
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    let length = nodes.len();
    let mut target = None;
    let mut has_immediate = false;
    for (index, node) in nodes.iter_mut().enumerate() {
        has_immediate |=
            prepare_list_element(node, index, length, simplify, handlers, &mut target)?;
    }
    if !has_immediate {
        if let Some(target) = target.take() {
            return Ok(target.into());
        }
    }
    let elements = nodes
        .iter()
        .map(prepared_list_element)
        .collect::<Fallible<Vec<_>>>()?;
    r_ext::materialize_list(target.as_ref(), &elements, None)
}

fn sequence_to_robj(
    seq: &mut [Yaml],
    simplify_seqs: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    #[derive(Copy, Clone, PartialEq, Eq)]
    enum RVectorType {
        List,
        Logical,
        Integer,
        Double,
        Character,
    }

    let mut out_type = RVectorType::List;
    let mut simplify = simplify_seqs;

    if !simplify_seqs {
        return materialize_node_list(seq, simplify_seqs, handlers);
    }

    // iterate over the vec once to see if we can simplify, fail early/fast if not
    for node in seq.iter_mut() {
        resolve_representation(node, simplify_seqs);
        match node {
            Yaml::Tagged(_, _) => {
                simplify = false;
                break;
            }
            Yaml::Value(scalar_type) => {
                let this_kind = match scalar_type {
                    Scalar::Null => RVectorType::List,
                    Scalar::Boolean(_) => RVectorType::Logical,
                    Scalar::Integer(_) => RVectorType::Integer,
                    Scalar::FloatingPoint(_) => RVectorType::Double,
                    Scalar::String(_) => RVectorType::Character,
                };

                if this_kind == out_type || matches!(scalar_type, Scalar::Null) {
                    continue;
                }
                if this_kind == RVectorType::Double && out_type == RVectorType::Integer {
                    out_type = RVectorType::Double;
                    continue;
                }
                if let Scalar::Integer(i) = scalar_type {
                    if out_type == RVectorType::Double {
                        continue;
                    }
                    if i32::try_from(*i).is_err() {
                        out_type = RVectorType::Double;
                        continue;
                    }
                }
                if out_type == RVectorType::List {
                    out_type = this_kind;
                    continue;
                }
                simplify = false;
                break;
            }
            _ => {
                simplify = false;
                break;
            }
        }
    }

    if simplify {
        match out_type {
            RVectorType::Logical => {
                return simplified_logical_sequence_to_robj(seq);
            }
            RVectorType::Integer => {
                return simplified_integer_sequence_to_robj(seq);
            }
            RVectorType::Double => {
                return simplified_double_sequence_to_robj(seq);
            }
            RVectorType::Character => {
                let values: Vec<_> = seq
                    .iter()
                    .map(|node| match node {
                        Yaml::Value(Scalar::String(value)) => r_ext::string_data(value.as_ref()),
                        Yaml::Value(Scalar::Null) => r_ext::string_data(<&str>::na()),
                        _ => unreachable!("expected only strings or nulls"),
                    })
                    .collect::<Fallible<Vec<_>>>()?;
                return r_ext::materialize_string_vector(&values);
            }
            RVectorType::List => {}
        }
    }

    // can't simplify, return a list
    materialize_node_list(seq, simplify_seqs, handlers)
}

fn simplified_logical_sequence_to_robj(seq: &[Yaml]) -> Fallible<Sexp> {
    // SAFETY: `sequence_to_robj()` calls this only after checking this same
    // sequence contains only booleans and nulls. After allocation succeeds, the
    // loop writes exactly one initialized value to every slot before return.
    let out = unsafe {
        let out = OwnedLogicalSexp::new_without_init(seq.len())?;
        let slots = std::slice::from_raw_parts_mut(
            ffi::LOGICAL(out.inner()).cast::<MaybeUninit<i32>>(),
            seq.len(),
        );
        for (slot, node) in slots.iter_mut().zip(seq) {
            let value = match node {
                Yaml::Value(Scalar::Boolean(b)) => *b as i32,
                Yaml::Value(Scalar::Null) => i32::na(),
                _ => unreachable!("expected only booleans or nulls"),
            };
            MaybeUninit::write(slot, value);
        }
        out
    };
    Ok(out.into())
}

fn simplified_integer_sequence_to_robj(seq: &[Yaml]) -> Fallible<Sexp> {
    // SAFETY: `sequence_to_robj()` calls this only after checking this same
    // sequence contains only i32-representable integers and nulls. After
    // allocation succeeds, the loop writes every slot before return.
    let out = unsafe {
        let out = OwnedIntegerSexp::new_without_init(seq.len())?;
        let slots = std::slice::from_raw_parts_mut(
            ffi::INTEGER(out.inner()).cast::<MaybeUninit<i32>>(),
            seq.len(),
        );
        for (slot, node) in slots.iter_mut().zip(seq) {
            let value = match node {
                Yaml::Value(Scalar::Integer(value)) => *value as i32,
                Yaml::Value(Scalar::Null) => i32::na(),
                _ => unreachable!("expected only integers or nulls"),
            };
            MaybeUninit::write(slot, value);
        }
        out
    };
    Ok(out.into())
}

fn simplified_double_sequence_to_robj(seq: &[Yaml]) -> Fallible<Sexp> {
    // SAFETY: `sequence_to_robj()` calls this only after checking this same
    // sequence contains only doubles, integers, and nulls. After allocation
    // succeeds, the loop writes every slot before return.
    let out = unsafe {
        let out = OwnedRealSexp::new_without_init(seq.len())?;
        let slots = std::slice::from_raw_parts_mut(
            ffi::REAL(out.inner()).cast::<MaybeUninit<f64>>(),
            seq.len(),
        );
        for (slot, node) in slots.iter_mut().zip(seq) {
            let value = match node {
                Yaml::Value(Scalar::FloatingPoint(value)) => value.into_inner(),
                Yaml::Value(Scalar::Integer(value)) => *value as f64,
                Yaml::Value(Scalar::Null) => f64::na(),
                _ => unreachable!("expected only doubles, integers, or nulls"),
            };
            MaybeUninit::write(slot, value);
        }
        out
    };
    Ok(out.into())
}

enum KeyHandlerResult {
    BareString {
        name: &'static str,
        _guard: PreservedSexp,
    },
    Preserved(PreservedSexp),
}

fn mapping_to_robj(
    map: &mut Mapping,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    let len = map.len();

    let all_plain_string_keys = map
        .iter()
        .all(|(key, _)| matches!(key, Yaml::Value(Scalar::String(_))));

    if all_plain_string_keys {
        let mut entries: Vec<_> = mem::take(map).into_iter().collect();
        let mut target = None;
        for (index, (_, value)) in entries.iter_mut().enumerate() {
            prepare_list_element(value, index, len, simplify, handlers, &mut target)?;
        }
        let elements = entries
            .iter()
            .map(|(_, value)| prepared_list_element(value))
            .collect::<Fallible<Vec<_>>>()?;
        let names = entries
            .iter()
            .map(|(key, _)| match key {
                Yaml::Value(Scalar::String(name)) => r_ext::string_data(name.as_ref()),
                _ => unreachable!("checked for only plain string keys"),
            })
            .collect::<Fallible<Vec<_>>>()?;

        return r_ext::materialize_list(target.as_ref(), &elements, Some(&names));
    }

    let mut keys: Vec<Yaml> = Vec::with_capacity(len);
    let mut values: Vec<Yaml> = Vec::with_capacity(len);
    let mut key_handler_results: Vec<Option<KeyHandlerResult>> = Vec::with_capacity(len);
    let mut value_target = None;

    // 1st pass: resolve keys/values while consuming the mapping to avoid cloning keys.
    for (i, (mut key, mut value)) in mem::take(map).into_iter().enumerate() {
        resolve_representation(&mut key, simplify);

        // If the key is tagged and a handler exists, apply it to the key itself.
        // Keep the handled value alive so we can borrow its string data when
        // constructing R names without allocating.
        let key_handler_result = if let (Some(registry), Yaml::Tagged(tag, _)) = (handlers, &key) {
            if let Some(handler) = registry.get_for_tag(tag.as_ref()) {
                let key_obj = yaml_to_robj(&mut key, simplify, handlers)?;
                let handled = PreservedSexp::new(registry.apply(handler, key_obj)?);
                Some(if let Some(name) = name_if_bare_string(&handled.value())? {
                    KeyHandlerResult::BareString {
                        name,
                        _guard: handled,
                    }
                } else {
                    KeyHandlerResult::Preserved(handled)
                })
            } else {
                None
            }
        } else {
            None
        };

        prepare_list_element(&mut value, i, len, simplify, handlers, &mut value_target)?;
        keys.push(key);
        values.push(value);
        key_handler_results.push(key_handler_result);
    }

    // 2nd pass: build names as &str from keys.
    // String mapping keys should contribute regular R names. `needs_yaml_keys_attr`
    // tracks whether we must attach the `yaml_keys` attribute because at least
    // one key cannot be represented purely by R names: either a non-string key,
    // or a string key carrying a non-canonical (informative) tag. Canonical
    // core string tags are treated as "no information" for this purpose.
    let mut needs_yaml_keys_attr = false;
    let mut names = Vec::with_capacity(len);
    for (key, key_handler_result) in keys.iter().zip(key_handler_results.iter()) {
        if let Some(handled) = key_handler_result {
            match handled {
                KeyHandlerResult::BareString { name, .. } => names.push(r_ext::string_data(name)?),
                KeyHandlerResult::Preserved(_) => {
                    names.push(r_ext::string_data("")?);
                    needs_yaml_keys_attr = true;
                }
            }
        } else {
            match key {
                Yaml::Value(Scalar::String(string_key)) => {
                    // Plain string key: representable as an R name with no extra metadata.
                    names.push(r_ext::string_data(string_key.as_ref())?);
                }
                _ => {
                    // Tagged or non-string keys get tracked in `yaml_keys`. Core string tags are
                    // normalized to plain strings by `resolve_representation`, so any tagged key
                    // reaching here carries extra information.
                    names.push(r_ext::string_data("")?);
                    needs_yaml_keys_attr = true;
                }
            }
        }
    }
    let elements = values
        .iter()
        .map(prepared_list_element)
        .collect::<Fallible<Vec<_>>>()?;
    if needs_yaml_keys_attr && value_target.is_none() {
        value_target = Some(OwnedListSexp::new(len, false)?);
    }
    let list = r_ext::materialize_list(value_target.as_ref(), &elements, Some(&names))?;

    if !needs_yaml_keys_attr {
        return Ok(list);
    }

    drop(elements);
    drop(names);

    let mut keys_target = Some(OwnedListSexp::new(len, false)?);
    for (i, (key, handled_value)) in keys.iter_mut().zip(key_handler_results.iter()).enumerate() {
        match handled_value {
            Some(KeyHandlerResult::BareString { .. }) => {}
            Some(KeyHandlerResult::Preserved(value)) => {
                keys_target.as_mut().unwrap().set_value(i, value.value())?;
            }
            None => {
                prepare_list_element(key, i, len, simplify, handlers, &mut keys_target)?;
            }
        }
    }

    let key_elements = keys
        .iter()
        .zip(key_handler_results.iter())
        .map(|(key, handled_value)| match handled_value {
            Some(KeyHandlerResult::BareString { name, .. }) => r_ext::ListElement::string(name),
            Some(KeyHandlerResult::Preserved(_)) => Ok(r_ext::ListElement::skip()),
            None => prepared_list_element(key),
        })
        .collect::<Fallible<Vec<_>>>()?;
    let yaml_keys = r_ext::materialize_list(keys_target.as_ref(), &key_elements, None)?;
    let mut list = list;
    r_ext::set_attrib_sym(&mut list, r_ext::sym_yaml_keys(), yaml_keys)?;
    Ok(list)
}

fn name_if_bare_string(robj: &Sexp) -> Fallible<Option<&'static str>> {
    let Some(name) = r_ext::as_string_scalar(robj)? else {
        return Ok(None);
    };
    Ok((!r_ext::has_attributes(robj)).then_some(name))
}

fn convert_tagged(
    tag: &Tag,
    node: &mut Yaml,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    if let Some(registry) = handlers {
        if let Some(handler) = registry.get_for_tag(tag) {
            let value = yaml_to_robj(node, simplify, handlers)?;
            return registry.apply(handler, value);
        }
    }

    let value = yaml_to_robj(node, simplify, handlers)?;
    if tag.is_yaml_core_schema() {
        return match tag.suffix.as_str() {
            "str" | "null" | "bool" | "int" | "float" | "seq" | "map" => Ok(value),
            "timestamp" | "set" | "omap" | "pairs" | "binary" => set_yaml_tag_attr(value, tag),
            other => Err(api_other(format!(
                "Unsupported core-schema tag `{handle}{other}`",
                handle = tag.handle
            ))),
        };
    }

    set_yaml_tag_attr(value, tag)
}

#[cfg_attr(not(test), allow(dead_code))]
fn is_core_string_tag(tag: &Tag) -> bool {
    tag.is_yaml_core_schema() && tag.suffix.as_str() == "str"
}

fn is_core_null_tag(tag: &Tag) -> bool {
    tag.is_yaml_core_schema() && tag.suffix.as_str() == "null"
}

fn set_yaml_tag_attr(value: Sexp, tag: &Tag) -> Fallible<Sexp> {
    let mut rendered_tag = String::with_capacity(tag.handle.len() + tag.suffix.len());
    rendered_tag.push_str(tag.handle.as_str());
    rendered_tag.push_str(tag.suffix.as_str());

    if rendered_tag.is_empty() {
        return Ok(value);
    }

    if value.is_null() {
        if !is_core_null_tag(tag) {
            let warn_msg = format!(
                "yaml12: discarding tag `{rendered_tag}` on null scalar; R NULL cannot carry attributes"
            );
            emit_warning(&warn_msg)?;
        }
        return Ok(value);
    }

    let value_guard = PreservedSexp::new(value);
    let tag_value = PreservedSexp::new(r_ext::string_scalar(rendered_tag.as_str())?);
    let mut value = value_guard.value();
    r_ext::set_attrib_sym(&mut value, r_ext::sym_yaml_tag(), tag_value.value())?;
    Ok(value)
}

fn wrap_unsupported(err: savvy::Error) -> savvy::Error {
    match err {
        savvy::Error::Aborted(token) => savvy::Error::Aborted(token),
        other => api_other(format!("Unsupported YAML: {other}")),
    }
}

fn load_yaml_documents<'input>(text: &'input str, multi: bool) -> Fallible<Vec<Yaml<'input>>> {
    let mut parser = Parser::new_from_str(text);
    let mut loader = YamlLoader::default();
    loader.early_parse(false);
    parser
        .load(&mut loader, multi)
        .map_err(|err| api_other(format!("YAML parse error: {err}")))?;
    Ok(loader.into_documents())
}

pub(crate) fn parse_yaml_impl(
    text: StringSexp,
    multi: bool,
    simplify: bool,
    handlers: Sexp,
) -> Fallible<Sexp> {
    let handler_registry = HandlerRegistry::from_robj(&handlers)?;
    let handlers = handler_registry.as_ref();

    match text.len() {
        0 => Ok(r_ext::null()),
        1 => {
            let first = r_ext::string_elt(&text, 0)?;
            if first.is_na() {
                return Err(api_other("`text` must not contain NA strings"));
            }
            let docs = load_yaml_documents(first, multi)?;
            docs_to_robj(docs, multi, simplify, handlers)
        }
        _ => {
            let joined_iter = joined_lines_iter(&text)?;
            let docs = load_yaml_documents_iter(joined_iter, multi)?;
            docs_to_robj(docs, multi, simplify, handlers)
        }
    }
}

fn docs_to_robj(
    mut docs: Vec<Yaml<'_>>,
    multi: bool,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    if multi {
        materialize_node_list(&mut docs, simplify, handlers).map_err(wrap_unsupported)
    } else {
        match docs.first_mut() {
            Some(doc) => yaml_to_robj(doc, simplify, handlers).map_err(wrap_unsupported),
            None => Ok(r_ext::null()),
        }
    }
}

fn joined_lines_iter(text: &StringSexp) -> Fallible<JoinedLinesIter> {
    let mut lines = Vec::with_capacity(text.len());
    for i in 0..text.len() {
        let line = r_ext::string_elt(text, i)?;
        if line.is_na() {
            return Err(api_other("`text` must not contain NA strings"));
        }
        lines.push(line);
    }
    Ok(JoinedLinesIter::new(lines))
}

struct JoinedLinesIter {
    lines: Vec<&'static str>,
    index: usize,
    current: std::str::Chars<'static>,
}

impl JoinedLinesIter {
    fn new(lines: Vec<&'static str>) -> Self {
        let current = lines.first().copied().unwrap_or("").chars();
        Self {
            lines,
            index: 1,
            current,
        }
    }
}

impl Iterator for JoinedLinesIter {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ch) = self.current.next() {
            return Some(ch);
        }
        if self.index < self.lines.len() {
            self.current = self.lines[self.index].chars();
            self.index += 1;
            return Some('\n');
        }
        None
    }
}

fn load_yaml_documents_iter<'input, I>(iter: I, multi: bool) -> Fallible<Vec<Yaml<'input>>>
where
    I: Iterator<Item = char> + 'input,
{
    let mut parser = Parser::new_from_iter(iter);
    let mut loader = YamlLoader::default();
    loader.early_parse(false);
    parser
        .load(&mut loader, multi)
        .map_err(|err| api_other(format!("YAML parse error: {err}")))?;
    Ok(loader.into_documents())
}

pub(crate) fn read_yaml_impl(
    path: &str,
    multi: bool,
    simplify: bool,
    handlers: Sexp,
) -> Fallible<Sexp> {
    let handler_registry = HandlerRegistry::from_robj(&handlers)?;
    let handlers = handler_registry.as_ref();

    let contents = fs::read_to_string(path)
        .map_err(|err| api_other(format!("Failed to read `{path}`: {err}")))?;
    let docs = load_yaml_documents(&contents, multi)?;
    docs_to_robj(docs, multi, simplify, handlers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::{LoadableYamlNode, Scalar as YamlScalar};

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum ParsedValueKind {
        String,
        Boolean,
    }

    fn load_scalar(input: &str) -> Yaml<'_> {
        let mut docs = Yaml::load_from_str(input).expect("parser should load tagged scalar");
        docs.pop().expect("expected one document")
    }

    fn normalized_suffix(suffix: &str) -> &str {
        let suffix = suffix.trim_start_matches('!');
        suffix.strip_prefix("tag:yaml.org,2002:").unwrap_or(suffix)
    }

    #[test]
    fn canonical_string_tags_cover_all_forms() {
        let canonical_string = Tag {
            handle: "tag:yaml.org,2002:".to_string(),
            suffix: "str".to_string(),
        };
        assert!(is_core_string_tag(&canonical_string));

        let cases = [
            ("!!str true", ParsedValueKind::String),
            ("!str true", ParsedValueKind::Boolean),
            ("!<str> true", ParsedValueKind::Boolean),
            ("!<!str> true", ParsedValueKind::Boolean),
            ("!<!!str> true", ParsedValueKind::Boolean),
            ("!<tag:yaml.org,2002:str> true", ParsedValueKind::String),
        ];

        for (input, expected_value) in cases {
            let parsed = load_scalar(input);
            match parsed {
                Yaml::Value(YamlScalar::String(value)) => {
                    assert_eq!(
                        expected_value,
                        ParsedValueKind::String,
                        "input `{input}` should resolve to string value"
                    );
                    assert_eq!(value.as_ref(), "true");
                }
                Yaml::Tagged(tag, inner) => {
                    assert_eq!(
                        is_core_string_tag(&tag),
                        tag.is_yaml_core_schema()
                            && normalized_suffix(tag.suffix.as_str()) == "str",
                        "input `{input}` canonical detection should match core `str` suffix",
                    );
                    match (expected_value, inner.as_ref()) {
                        (ParsedValueKind::Boolean, Yaml::Value(YamlScalar::Boolean(value))) => {
                            assert!(
                                *value,
                                "input `{input}` should parse to boolean `true` when not core"
                            );
                        }
                        (expected, other) => {
                            panic!(
                                "input `{input}` expected value kind {expected:?}, got {other:?}"
                            )
                        }
                    }
                }
                other => panic!("input `{input}` expected tagged or string value, got {other:?}"),
            }
        }
    }

    #[test]
    fn canonical_null_tags_cover_all_forms() {
        let canonical_null = Tag {
            handle: "tag:yaml.org,2002:".to_string(),
            suffix: "null".to_string(),
        };
        assert!(is_core_null_tag(&canonical_null));

        let cases = [
            "!!null null",
            "!<null> null",
            "!<!null> null",
            "!<!!null> null",
            "!<tag:yaml.org,2002:null> null",
        ];

        for input in cases {
            let parsed = load_scalar(input);
            match parsed {
                Yaml::Value(YamlScalar::Null) => {
                    // Canonical null scalars should not carry tags.
                }
                Yaml::Tagged(tag, inner) => {
                    assert_eq!(
                        is_core_null_tag(&tag),
                        tag.is_yaml_core_schema()
                            && normalized_suffix(tag.suffix.as_str()) == "null",
                        "input `{input}` canonical detection should match core `null` suffix",
                    );
                    assert!(
                        matches!(inner.as_ref(), Yaml::Value(YamlScalar::Null)),
                        "input `{input}` should parse to tagged null scalar"
                    );
                }
                other => panic!("input `{input}` expected null scalar, got {other:?}"),
            }
        }
    }
}
