use crate::r::{self, List, Robj, Rtype};
use crate::{api_other, sym_yaml_keys, sym_yaml_tag, Fallible};
use crate::{
    timestamp::{
        core_timestamp_tag, format_posix_precise, format_r_time, offset_minutes_from_tzone,
        yaml_from_formatted_timestamp, yaml_from_formatted_timestamp_with_tag,
    },
    unwind::{run_with_unwind_protect, EvalError},
    TIMESTAMP_SUPPORT_ENABLED,
};
use saphyr::{Mapping, Scalar, Tag, Yaml, YamlEmitter};
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
    run_with_unwind_protect(|| unsafe {
        ffi::Rprintf(
            PRINTF_NO_FMT_CSTRING.as_ptr(),
            content.as_ptr() as *const c_char,
        );
    })
    .map_err(EvalError::Jump)
}

fn robj_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    if TIMESTAMP_SUPPORT_ENABLED && robj.get_attrib_sym(sym_yaml_tag()).is_none() {
        if has_class(robj, "POSIXt") || has_class(robj, "POSIXct") {
            return posix_to_yaml(robj);
        }
        if has_class(robj, "Date") {
            return date_to_yaml(robj);
        }
    }

    let node = match robj.rtype() {
        Rtype::Null => Ok(Yaml::Value(Scalar::Null)),
        Rtype::Logicals => logical_to_yaml(robj),
        Rtype::Integers => integer_to_yaml(robj),
        Rtype::Doubles => real_to_yaml(robj),
        Rtype::Strings => character_to_yaml(robj),
        Rtype::List => list_to_yaml(robj),
        _ => Err(api_other(format!(
            "Unsupported R type {rtype:?} for YAML conversion",
            rtype = robj.rtype()
        ))),
    }?;
    apply_tag_if_present(robj, node)
}

fn logical_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice = robj
        .as_logical_slice()
        .ok_or_else(|| api_other("Expected a logical vector"))?;
    if let [value] = slice {
        return Ok(if *value == r::r_na_int() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::Boolean(*value != 0))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if *value == r::r_na_int() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::Boolean(*value != 0)));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn integer_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice = robj
        .as_integer_slice()
        .ok_or_else(|| api_other("Expected an integer vector"))?;
    if let [value] = slice {
        return Ok(if *value == i32::MIN {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::Integer(*value as i64))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if *value == i32::MIN {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::Integer(*value as i64)));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn real_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice = robj
        .as_real_slice()
        .ok_or_else(|| api_other("Expected a numeric vector"))?;
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

fn character_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let mut strings = robj
        .as_str_iter()
        .ok_or_else(|| api_other("Expected a character vector"))?;
    if robj.len() == 1 {
        let value = strings
            .next()
            .expect("character vector length of 1 should yield 1 element");
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            let value: &'static str = value.as_str();
            Yaml::Value(Scalar::String(Cow::Borrowed(value)))
        });
    }
    let mut values = Vec::with_capacity(robj.len());
    for value in strings {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            let value: &'static str = value.as_str();
            values.push(Yaml::Value(Scalar::String(Cow::Borrowed(value))));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn has_class(robj: &Robj, class: &str) -> bool {
    robj.inherits(class)
}

fn posix_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let tzone_attr = robj.get_attrib_str("tzone")?;
    let tz_name = tzone_attr
        .as_ref()
        .and_then(|tz| tz.as_str())
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

fn date_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let formatted = format_r_time(robj, "%Y-%m-%d", None)?;
    Ok(yaml_from_formatted_timestamp(formatted))
}

fn local_offset_minutes(robj: &Robj) -> Fallible<i32> {
    let formatted = format_r_time(robj, "%z", None)?;
    let minutes = formatted
        .into_iter()
        .flatten()
        .find_map(|s| offset_minutes_from_tzone(&s))
        .unwrap_or(0);
    Ok(minutes)
}

fn list_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let list = robj.as_list().ok_or_else(|| api_other("Expected a list"))?;
    if let Some(keys_attr) = robj.get_attrib_sym(sym_yaml_keys()) {
        if !keys_attr.is_null() {
            let keys: List = keys_attr
                .try_into()
                .map_err(|_| api_other("`yaml_keys` attribute must be a list"))?;
            if keys.len() != list.len() {
                return Err(api_other(
                    "`yaml_keys` attribute must have the same length as the list",
                ));
            }
            let mut mapping = Mapping::with_capacity(list.len());
            for (value, key) in list.values().zip(keys.values()) {
                mapping.insert(robj_to_yaml(&key)?, robj_to_yaml(&value)?);
            }
            return Ok(Yaml::Mapping(mapping));
        }
    }

    match list.names() {
        Some(names) => {
            let mut mapping = Mapping::with_capacity(list.len());
            for (value, name) in list.values().zip(names) {
                let key = if name.is_na() {
                    Yaml::Value(Scalar::Null)
                } else {
                    let name: &'static str = name.as_str();
                    Yaml::Value(Scalar::String(Cow::Borrowed(name)))
                };
                if mapping.insert(key, robj_to_yaml(&value)?).is_some() {
                    let duplicate = if name.is_na() {
                        String::from("null")
                    } else {
                        let key_str = name.as_str();
                        if key_str.is_empty() {
                            String::from("(empty string)")
                        } else {
                            key_str.to_string()
                        }
                    };
                    return Err(api_other(format!(
                        "Duplicate mapping key `{duplicate}`; list names must be unique"
                    )));
                }
            }
            Ok(Yaml::Mapping(mapping))
        }
        None => {
            let seq = list
                .values()
                .map(|value| robj_to_yaml(&value))
                .collect::<Fallible<Vec<_>>>()?;
            Ok(Yaml::Sequence(seq))
        }
    }
}

fn apply_tag_if_present(robj: &Robj, node: Yaml<'static>) -> Fallible<Yaml<'static>> {
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

fn extract_yaml_tag(robj: &Robj) -> Fallible<Option<Tag>> {
    let attr = match robj.get_attrib_sym(sym_yaml_tag()) {
        Some(value) => value,
        None => return Ok(None),
    };
    let tag_str = attr.as_str().ok_or_else(|| {
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

pub(crate) fn format_yaml_impl(value: &Robj, multi: bool) -> Fallible<String> {
    if multi {
        let list = value
            .as_list()
            .ok_or_else(|| api_other("`value` must be a list when `multi = TRUE`"))?;
        if list.names().is_some() {
            return Err(api_other(
                "`value` must be an unnamed list when `multi = TRUE` (names must be NULL)",
            ));
        }
        let mut docs = Vec::with_capacity(list.len());
        for doc in list.values() {
            docs.push(robj_to_yaml(&doc)?);
        }
        emit_yaml_documents(&docs, true)
    } else {
        robj_to_yaml(value).and_then(|yaml| emit_yaml_documents(&[yaml], false))
    }
}

pub(crate) fn write_yaml_impl(value: &Robj, path: Option<&str>, multi: bool) -> Fallible<()> {
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
