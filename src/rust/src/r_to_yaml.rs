use crate::r_ext;
use crate::{api_other, Fallible};
use crate::{
    timestamp::{
        core_timestamp_tag, format_posix_precise, format_r_time, offset_minutes_from_tzone,
        yaml_from_formatted_timestamp, yaml_from_formatted_timestamp_with_tag,
    },
    TIMESTAMP_SUPPORT_ENABLED,
};
use saphyr::{Mapping, Scalar, Tag, Yaml, YamlEmitter};
use savvy::{
    IntegerSexp, ListSexp, LogicalSexp, NotAvailableValue, RealSexp, Sexp, StringSexp, TypedSexp,
};
use savvy_ffi as ffi;
use std::{borrow::Cow, fs, os::raw::c_char};

const PRINTF_NO_FMT_CSTRING: &[c_char] = &[37, 115, 0]; // "%s\0"

pub(crate) fn yaml_body(yaml: &str, multi: bool) -> &str {
    if multi || !yaml.starts_with("---\n") {
        yaml
    } else {
        &yaml[4..]
    }
}

fn emit_yaml_documents(docs: &[Yaml<'static>], multi: bool) -> Fallible<String> {
    if docs.is_empty() {
        return if multi {
            Ok(String::from("---\n"))
        } else {
            Ok(String::new())
        };
    }
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.multiline_strings(true);
    if multi {
        emitter
            .dump_docs(docs)
            .map_err(|err| api_other(err.to_string()))?;
    } else {
        emitter
            .dump(&docs[0])
            .map_err(|err| api_other(err.to_string()))?;
    }
    Ok(output)
}

fn write_to_r_stdout(mut content: String) -> Fallible<()> {
    // R character vectors cannot contain embedded NUL bytes, so it is safe to
    // emit the YAML buffer without scanning for interior terminators.
    debug_assert!(
        !content.as_bytes().contains(&0),
        "R character data cannot contain embedded NULs",
    );
    content.push('\0');
    unsafe {
        savvy::unwind_protect(|| {
            ffi::Rprintf(
                PRINTF_NO_FMT_CSTRING.as_ptr(),
                content.as_ptr() as *const c_char,
            );
            ffi::R_NilValue
        })?;
    }
    Ok(())
}

fn robj_to_yaml(robj: &Sexp) -> Fallible<Yaml<'static>> {
    if TIMESTAMP_SUPPORT_ENABLED && r_ext::get_attrib_sym(robj, r_ext::sym_yaml_tag()?)?.is_none() {
        if r_ext::inherits(robj, "POSIXt") || r_ext::inherits(robj, "POSIXct") {
            return posix_to_yaml(robj);
        }
        if r_ext::inherits(robj, "Date") {
            return date_to_yaml(robj);
        }
    }

    let node = match Sexp(robj.0).into_typed() {
        TypedSexp::Null(_) => Ok(Yaml::Value(Scalar::Null)),
        TypedSexp::Logical(value) => logical_to_yaml(value),
        TypedSexp::Integer(value) => integer_to_yaml(value),
        TypedSexp::Real(value) => real_to_yaml(value),
        TypedSexp::String(value) => character_to_yaml(value),
        TypedSexp::List(value) => list_to_yaml(robj, value),
        other => Err(api_other(format!(
            "Unsupported R type {rtype} for YAML conversion",
            rtype = typed_name(&other)
        ))),
    }?;
    apply_tag_if_present(robj, node)
}

fn typed_name(value: &TypedSexp) -> &'static str {
    match value {
        TypedSexp::Null(_) => "Null",
        TypedSexp::Logical(_) => "Logicals",
        TypedSexp::Integer(_) => "Integers",
        TypedSexp::Real(_) => "Doubles",
        TypedSexp::String(_) => "Strings",
        TypedSexp::List(_) => "List",
        TypedSexp::Function(_) => "Function",
        _ => "Other",
    }
}

fn logical_to_yaml(robj: LogicalSexp) -> Fallible<Yaml<'static>> {
    let slice = robj.as_slice_raw();
    if let [value] = slice {
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::Boolean(*value != 0))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::Boolean(*value != 0)));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn integer_to_yaml(robj: IntegerSexp) -> Fallible<Yaml<'static>> {
    let slice = robj.as_slice();
    if let [value] = slice {
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::Integer(*value as i64))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::Integer(*value as i64)));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn real_to_yaml(robj: RealSexp) -> Fallible<Yaml<'static>> {
    let slice = robj.as_slice();
    if let [value] = slice {
        return Ok(if value.is_nan() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::FloatingPoint((*value).into()))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if value.is_nan() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::FloatingPoint((*value).into())));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn character_to_yaml(robj: StringSexp) -> Fallible<Yaml<'static>> {
    if robj.len() == 1 {
        let value = r_ext::string_elt(&robj, 0);
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::String(Cow::Borrowed(value)))
        });
    }
    let mut values = Vec::with_capacity(robj.len());
    for value in robj.iter() {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::String(Cow::Borrowed(value))));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn posix_to_yaml(robj: &Sexp) -> Fallible<Yaml<'static>> {
    let tzone_attr = r_ext::get_attrib_str(robj, "tzone")?;
    let tz_name = tzone_attr
        .as_ref()
        .and_then(r_ext::as_string_scalar)
        .filter(|s| !s.is_empty());

    enum PosixTz<'a> {
        NaiveLocal,
        Utc,
        Fixed { offset_minutes: i32 },
        Named(Cow<'a, str>),
    }

    let tz_kind = match tz_name {
        None => PosixTz::NaiveLocal,
        Some(tz) => match offset_minutes_from_tzone(tz) {
            Some(0) => PosixTz::Utc,
            Some(offset_minutes) => PosixTz::Fixed { offset_minutes },
            None => PosixTz::Named(Cow::Borrowed(tz)),
        },
    };

    let formatted = match tz_kind {
        PosixTz::NaiveLocal => {
            let offset_minutes = local_offset_minutes(robj)?;
            format_posix_precise(robj, offset_minutes, true, false)?
        }
        PosixTz::Utc => format_posix_precise(robj, 0, false, true)?,
        PosixTz::Fixed { offset_minutes, .. } => {
            format_posix_precise(robj, offset_minutes, false, false)?
        }
        PosixTz::Named(tz) => format_r_time(robj, "%Y-%m-%dT%H:%M:%OS9%z", Some(&tz))?,
    };

    Ok(yaml_from_formatted_timestamp_with_tag(
        formatted,
        core_timestamp_tag(),
    ))
}

fn date_to_yaml(robj: &Sexp) -> Fallible<Yaml<'static>> {
    let formatted = format_r_time(robj, "%Y-%m-%d", None)?;
    Ok(yaml_from_formatted_timestamp(formatted))
}

fn local_offset_minutes(robj: &Sexp) -> Fallible<i32> {
    let formatted = format_r_time(robj, "%z", None)?;
    let minutes = formatted
        .into_iter()
        .flatten()
        .find_map(|s| offset_minutes_from_tzone(&s))
        .unwrap_or(0);
    Ok(minutes)
}

fn list_to_yaml(robj: &Sexp, list: ListSexp) -> Fallible<Yaml<'static>> {
    if let Some(keys_attr) = r_ext::get_attrib_sym(robj, r_ext::sym_yaml_keys()?)? {
        if !keys_attr.is_null() {
            let keys = ListSexp::try_from(keys_attr)
                .map_err(|_| api_other("`yaml_keys` attribute must be a list"))?;
            if keys.len() != list.len() {
                return Err(api_other(
                    "`yaml_keys` attribute must have the same length as the list",
                ));
            }
            let mut mapping = Mapping::with_capacity(list.len());
            for i in 0..list.len() {
                let value = unsafe { list.get_by_index_unchecked(i) };
                let key = unsafe { keys.get_by_index_unchecked(i) };
                mapping.insert(robj_to_yaml(&key)?, robj_to_yaml(&value)?);
            }
            return Ok(Yaml::Mapping(mapping));
        }
    }

    match r_ext::names(robj)? {
        Some(names) => {
            let mut mapping = Mapping::with_capacity(list.len());
            for i in 0..list.len() {
                let value = unsafe { list.get_by_index_unchecked(i) };
                let name = r_ext::string_elt(&names, i);
                let key = if name.is_na() {
                    Yaml::Value(Scalar::Null)
                } else {
                    Yaml::Value(Scalar::String(Cow::Borrowed(name)))
                };
                if mapping.insert(key, robj_to_yaml(&value)?).is_some() {
                    let duplicate = if name.is_na() {
                        String::from("null")
                    } else if name.is_empty() {
                        String::from("(empty string)")
                    } else {
                        name.to_string()
                    };
                    return Err(api_other(format!(
                        "Duplicate mapping key `{duplicate}`; list names must be unique"
                    )));
                }
            }
            Ok(Yaml::Mapping(mapping))
        }
        None => {
            let mut seq = Vec::with_capacity(list.len());
            for i in 0..list.len() {
                let value = unsafe { list.get_by_index_unchecked(i) };
                seq.push(robj_to_yaml(&value)?);
            }
            Ok(Yaml::Sequence(seq))
        }
    }
}

fn apply_tag_if_present(robj: &Sexp, node: Yaml<'static>) -> Fallible<Yaml<'static>> {
    if let Some(tag) = extract_yaml_tag(robj)? {
        // saphyr represents a bare `!` tag as handle="" / suffix="!" when parsing,
        // but the emitter cannot round-trip that shape. Normalize to handle="!" /
        // suffix="" so the output is a single `!` tag that still round-trips.
        let tag = if tag.handle.is_empty() && tag.suffix.as_str() == "!" {
            Tag {
                handle: "!".to_string(),
                suffix: String::new(),
            }
        } else {
            tag
        };
        Ok(Yaml::Tagged(Cow::Owned(tag), Box::new(node)))
    } else {
        Ok(node)
    }
}

fn extract_yaml_tag(robj: &Sexp) -> Fallible<Option<Tag>> {
    let attr = match r_ext::get_attrib_sym(robj, r_ext::sym_yaml_tag()?)? {
        Some(value) => value,
        None => return Ok(None),
    };
    let tag_str = r_ext::as_string_scalar(&attr).ok_or_else(|| {
        api_other(
            "Invalid `yaml_tag` attribute: expected a single, non-missing string. Must not be NA",
        )
    })?;
    let tag_str = tag_str.trim();
    if tag_str.is_empty() {
        return Ok(None);
    }

    let invalid_tag_error = || api_other(format!("Invalid YAML tag `{tag_str}`"));

    let tag = if tag_str == "!" {
        Tag {
            handle: String::new(),
            suffix: "!".to_string(),
        }
    } else if let Some(rest) = tag_str.strip_prefix("!!") {
        if rest.is_empty() {
            return Err(invalid_tag_error());
        }
        let mut suffix = String::with_capacity(rest.len() + 1);
        suffix.push('!');
        suffix.push_str(rest);
        Tag {
            handle: "!".to_string(),
            suffix,
        }
    } else if let Some(rest) = tag_str.strip_prefix('!') {
        if rest.is_empty() {
            return Err(invalid_tag_error());
        }
        Tag {
            handle: "!".to_string(),
            suffix: rest.to_string(),
        }
    } else if let Some((handle, suffix)) = tag_str.rsplit_once('!') {
        if suffix.is_empty() {
            return Err(invalid_tag_error());
        }
        Tag {
            handle: handle.to_string(),
            suffix: suffix.to_string(),
        }
    } else {
        Tag {
            handle: String::new(),
            suffix: tag_str.to_string(),
        }
    };

    Ok(Some(tag))
}

pub(crate) fn format_yaml_impl(value: &Sexp, multi: bool) -> Fallible<String> {
    if multi {
        let list = ListSexp::try_from(Sexp(value.0))
            .map_err(|_| api_other("`value` must be a list when `multi = TRUE`"))?;
        if r_ext::names(value)?.is_some() {
            return Err(api_other(
                "`value` must be an unnamed list when `multi = TRUE` (names must be NULL)",
            ));
        }
        let mut docs = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            let doc = unsafe { list.get_by_index_unchecked(i) };
            docs.push(robj_to_yaml(&doc)?);
        }
        emit_yaml_documents(&docs, true)
    } else {
        robj_to_yaml(value).and_then(|yaml| emit_yaml_documents(&[yaml], false))
    }
}

pub(crate) fn write_yaml_impl(value: &Sexp, path: Option<&str>, multi: bool) -> Fallible<()> {
    let mut output = format_yaml_impl(value, multi)?;
    // `dump_docs()` ends multi-doc streams with a trailing newline; `dump()` does not.
    // Both always emit the `---\n` document start.
    if multi {
        output.push_str("...\n");
    } else {
        output.push_str("\n...\n");
    }
    if let Some(path) = path {
        fs::write(path, &output)
            .map_err(|err| api_other(format!("Failed to write `{path}`: {err}")))?;
    } else {
        write_to_r_stdout(output)?;
    }
    Ok(())
}
