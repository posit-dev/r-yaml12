use crate::handlers::HandlerRegistry;
use crate::r_ext::{self, PreservedSexp};
use crate::timestamp::{is_timestamp_tag, parse_timestamp_node, simplify_timestamp_sequence};
use crate::warning::emit_warning;
use crate::{api_other, Fallible, TIMESTAMP_SUPPORT_ENABLED};
use saphyr::{Mapping, Scalar, Tag, Yaml, YamlLoader};
use saphyr_parser::{Parser, ScalarStyle};
use savvy::{
    NotAvailableValue, OwnedIntegerSexp, OwnedListSexp, OwnedLogicalSexp, OwnedRealSexp,
    OwnedStringSexp, Sexp, StringSexp,
};
use std::{fs, mem};

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
                    // _ if is_timestamp_tag(tag.as_ref()) => {
                    //     Yaml::Tagged(tag, Box::new(Yaml::Value(Scalar::String(value))))
                    // }
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
        Scalar::Boolean(value) => OwnedLogicalSexp::try_from_scalar(*value).map(Into::into),
        Scalar::Integer(value) => {
            if let Ok(v) = i32::try_from(*value) {
                OwnedIntegerSexp::try_from_scalar(v).map(Into::into)
            } else {
                OwnedRealSexp::try_from_scalar(*value as f64).map(Into::into)
            }
        }
        Scalar::FloatingPoint(value) => {
            OwnedRealSexp::try_from_scalar(value.into_inner()).map(Into::into)
        }
        Scalar::String(value) => r_ext::string_scalar(value.as_ref()),
    }
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
        let mut list = OwnedListSexp::new(seq.len(), false)?;
        for (i, node) in seq.iter_mut().enumerate() {
            resolve_representation(node, simplify_seqs);
            let value = yaml_to_robj(node, simplify_seqs, handlers)?;
            list.set_value(i, value)?;
        }
        return Ok(list.into());
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
                let mut logicals = unsafe { OwnedLogicalSexp::new_without_init(seq.len())? };
                for (i, node) in seq.iter().enumerate() {
                    match node {
                        Yaml::Value(Scalar::Boolean(b)) => logicals.set_elt(i, *b)?,
                        Yaml::Value(Scalar::Null) => logicals.set_na(i)?,
                        _ => unreachable!("expected only booleans or nulls"),
                    }
                }
                return Ok(logicals.into());
            }
            RVectorType::Integer => {
                let mut integers = unsafe { OwnedIntegerSexp::new_without_init(seq.len())? };
                for (out, node) in integers.as_mut_slice().iter_mut().zip(seq.iter()) {
                    *out = match node {
                        Yaml::Value(Scalar::Integer(value)) => *value as i32,
                        Yaml::Value(Scalar::Null) => i32::na(),
                        _ => unreachable!("expected only integers or nulls"),
                    };
                }
                return Ok(integers.into());
            }
            RVectorType::Double => {
                let mut doubles = unsafe { OwnedRealSexp::new_without_init(seq.len())? };
                for (out, node) in doubles.as_mut_slice().iter_mut().zip(seq.iter()) {
                    *out = match node {
                        Yaml::Value(Scalar::FloatingPoint(value)) => value.into_inner(),
                        Yaml::Value(Scalar::Integer(value)) => *value as f64,
                        Yaml::Value(Scalar::Null) => f64::na(),
                        _ => unreachable!("expected only doubles, integers, or nulls"),
                    };
                }
                return Ok(doubles.into());
            }
            RVectorType::Character => {
                let mut strings = OwnedStringSexp::new(seq.len())?;
                for (i, node) in seq.iter().enumerate() {
                    match node {
                        Yaml::Value(Scalar::String(value)) => strings.set_elt(i, value.as_ref())?,
                        Yaml::Value(Scalar::Null) => strings.set_na(i)?,
                        _ => unreachable!("expected only strings or nulls"),
                    }
                }
                return Ok(strings.into());
            }
            RVectorType::List => {}
        }
    }

    // can't simplify via scalar types; try timestamp-aware simplification
    if TIMESTAMP_SUPPORT_ENABLED {
        if let Some(out) =
            simplify_timestamp_sequence(seq, |node| resolve_representation(node, true))?
        {
            return Ok(out);
        }
    }

    // can't simplify, return a list
    let mut list = OwnedListSexp::new(seq.len(), false)?;
    for (i, node) in seq.iter_mut().enumerate() {
        let value = yaml_to_robj(node, simplify_seqs, handlers)?;
        list.set_value(i, value)?;
    }

    Ok(list.into())
}

fn mapping_to_robj(
    map: &mut Mapping,
    simplify: bool,
    handlers: Option<&HandlerRegistry<'_>>,
) -> Fallible<Sexp> {
    let len = map.len();

    if handlers.is_none() {
        let all_plain_string_keys = map
            .iter()
            .all(|(key, _)| matches!(key, Yaml::Value(Scalar::String(_))));

        if all_plain_string_keys {
            let mut list = OwnedListSexp::new(len, true)?;
            for (i, (key, mut value)) in mem::take(map).into_iter().enumerate() {
                let name = match key {
                    Yaml::Value(Scalar::String(name)) => name,
                    _ => unreachable!("checked for only plain string keys"),
                };
                let value = yaml_to_robj(&mut value, simplify, handlers)?;
                list.set_value(i, value)?;
                list.set_name(i, name.as_ref())?;
            }

            return Ok(list.into());
        }
    }

    let mut keys: Vec<Yaml> = Vec::with_capacity(len);
    let mut key_handler_results: Vec<Option<PreservedSexp>> = Vec::with_capacity(len);
    let mut list = OwnedListSexp::new(len, true)?;

    // 1st pass: resolve keys/values while consuming the mapping to avoid cloning keys.
    for (i, (mut key, mut value)) in mem::take(map).into_iter().enumerate() {
        resolve_representation(&mut key, simplify);

        // If the key is tagged and a handler exists, apply it to the key itself.
        // Keep the handled value alive so we can borrow its string data when
        // constructing R names without allocating.
        let key_handler_result = if let (Some(registry), Yaml::Tagged(tag, _)) = (handlers, &key) {
            if let Some(handler) = registry.get_for_tag(tag.as_ref()) {
                let key_obj = yaml_to_robj(&mut key, simplify, handlers)?;
                Some(registry.apply_preserved(handler, key_obj)?)
            } else {
                None
            }
        } else {
            None
        };

        keys.push(key);
        key_handler_results.push(key_handler_result);
        let value = yaml_to_robj(&mut value, simplify, handlers)?;
        list.set_value(i, value)?;
    }

    // 2nd pass: build names as &str from keys.
    // String mapping keys should contribute regular R names. `needs_yaml_keys_attr`
    // tracks whether we must attach the `yaml_keys` attribute because at least
    // one key cannot be represented purely by R names: either a non-string key,
    // or a string key carrying a non-canonical (informative) tag. Canonical
    // core string tags are treated as "no information" for this purpose.
    let mut needs_yaml_keys_attr = false;
    for (i, (key, key_handler_result)) in keys.iter().zip(key_handler_results.iter()).enumerate() {
        if let Some(handled) = key_handler_result {
            if let Some(name_from_handler) = name_if_bare_string(handled.value()) {
                list.set_name(i, name_from_handler)?;
            } else {
                needs_yaml_keys_attr = true;
                list.set_name(i, "")?;
            }
        } else {
            match key {
                Yaml::Value(Scalar::String(string_key)) => {
                    // Plain string key: representable as an R name with no extra metadata.
                    list.set_name(i, string_key.as_ref())?;
                }
                _ => {
                    // Tagged or non-string keys get tracked in `yaml_keys`. Core string tags are
                    // normalized to plain strings by `resolve_representation`, so any tagged key
                    // reaching here carries extra information.
                    needs_yaml_keys_attr = true;
                    list.set_name(i, "")?;
                }
            }
        }
    }

    if needs_yaml_keys_attr {
        let mut yaml_keys = OwnedListSexp::new(keys.len(), false)?;
        for (i, (mut key, handled_value)) in keys.into_iter().zip(key_handler_results).enumerate() {
            if let Some(val) = handled_value {
                yaml_keys.set_value(i, val.value())?;
            } else {
                let key = yaml_to_robj(&mut key, simplify, handlers)?;
                yaml_keys.set_value(i, key)?;
            }
        }
        let keys_attr = Sexp(yaml_keys.inner());
        let mut list_sexp = Sexp(list.inner());
        r_ext::set_attrib_sym(&mut list_sexp, r_ext::sym_yaml_keys()?, keys_attr)?;
    }

    Ok(list.into())
}

fn name_if_bare_string(robj: Sexp) -> Option<&'static str> {
    let name = r_ext::as_string_scalar(&robj)?;
    (!r_ext::has_attributes(&robj)).then_some(name)
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

    if TIMESTAMP_SUPPORT_ENABLED && is_timestamp_tag(tag) {
        let keep_empty_tzone = tag.handle.as_str() == "!";
        let preserve_tzone = true;
        if let Some(timestamp) = parse_timestamp_node(node, preserve_tzone, keep_empty_tzone)? {
            return Ok(timestamp);
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

fn set_yaml_tag_attr(mut value: Sexp, tag: &Tag) -> Fallible<Sexp> {
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

    let tag_value = OwnedStringSexp::try_from_scalar(rendered_tag.as_str())?;
    r_ext::set_attrib_sym(&mut value, r_ext::sym_yaml_tag()?, Sexp(tag_value.inner()))?;
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
            let first = r_ext::string_elt(&text, 0);
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
        let mut list = OwnedListSexp::new(docs.len(), false)?;
        for (i, doc) in docs.iter_mut().enumerate() {
            let value = yaml_to_robj(doc, simplify, handlers).map_err(wrap_unsupported)?;
            list.set_value(i, value)?;
        }
        Ok(list.into())
    } else {
        match docs.first_mut() {
            Some(doc) => yaml_to_robj(doc, simplify, handlers).map_err(wrap_unsupported),
            None => Ok(r_ext::null()),
        }
    }
}

fn joined_lines_iter<'a>(text: &'a StringSexp) -> Fallible<JoinedLinesIter<'a>> {
    for line in text.iter() {
        if line.is_na() {
            return Err(api_other("`text` must not contain NA strings"));
        }
    }
    Ok(JoinedLinesIter::new(text))
}

struct JoinedLinesIter<'a> {
    text: &'a StringSexp,
    index: usize,
    current: std::str::Chars<'a>,
}

impl<'a> JoinedLinesIter<'a> {
    fn new(text: &'a StringSexp) -> Self {
        let current = if text.is_empty() {
            "".chars()
        } else {
            r_ext::string_elt(text, 0).chars()
        };
        Self {
            text,
            index: 1,
            current,
        }
    }
}

impl<'a> Iterator for JoinedLinesIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ch) = self.current.next() {
            return Some(ch);
        }
        if self.index < self.text.len() {
            self.current = r_ext::string_elt(self.text, self.index).chars();
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
            ("!<tag:yaml.org,2002:str> true", ParsedValueKind::Boolean),
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
