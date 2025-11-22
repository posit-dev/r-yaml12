use crate::{api_other, sym_yaml_keys, sym_yaml_tag, Fallible};
use extendr_api::prelude::*;
use saphyr::{Mapping, Scalar, Tag, Yaml, YamlEmitter};
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
        return Ok(String::new());
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
        extendr_ffi::Rprintf(
            PRINTF_NO_FMT_CSTRING.as_ptr(),
            content.as_ptr() as *const c_char,
        );
    }
    Ok(())
}

fn robj_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    if robj.get_attrib(sym_yaml_tag()).is_none() {
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
        .ok_or_else(|| Error::Other("Expected a logical vector".to_string()))?;
    if let [value] = slice {
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::Boolean(value.to_bool()))
        });
    }
    let mut values = Vec::with_capacity(slice.len());
    for value in slice {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::Boolean(value.to_bool())));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn integer_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice = robj
        .as_integer_slice()
        .ok_or_else(|| Error::Other("Expected an integer vector".to_string()))?;
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
        .ok_or_else(|| Error::Other("Expected a numeric vector".to_string()))?;
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
        .ok_or_else(|| Error::Other("Expected a character vector".to_string()))?;
    if robj.len() == 1 {
        let value = strings
            .next()
            .expect("character vector length of 1 should yield 1 element");
        return Ok(if value.is_na() {
            Yaml::Value(Scalar::Null)
        } else {
            Yaml::Value(Scalar::String(Cow::Borrowed(value)))
        });
    }
    let mut values = Vec::with_capacity(robj.len());
    for value in strings {
        if value.is_na() {
            values.push(Yaml::Value(Scalar::Null));
        } else {
            values.push(Yaml::Value(Scalar::String(Cow::Borrowed(value))));
        }
    }
    Ok(Yaml::Sequence(values))
}

fn has_class(robj: &Robj, class: &str) -> bool {
    if robj.inherits(class) {
        return true;
    }
    robj.class()
        .map(|iter| iter.into_iter().any(|c| c == class))
        .unwrap_or(false)
}

fn posix_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice: Vec<f64> = if let Some(real) = robj.as_real_slice() {
        real.to_vec()
    } else if let Some(ints) = robj.as_integer_slice() {
        ints.iter().map(|v| *v as f64).collect()
    } else {
        return Err(api_other("Expected a numeric POSIXct vector"));
    };

    let mut values = Vec::with_capacity(slice.len());
    for value in slice.iter() {
        if value.is_nan() {
            values.push(Yaml::Value(Scalar::Null));
            continue;
        }
        let rendered =
            render_posix_timestamp(*value).ok_or_else(|| api_other("Invalid POSIXct value"))?;
        values.push(tagged_timestamp(rendered));
    }
    if values.len() == 1 {
        Ok(values
            .into_iter()
            .next()
            .expect("vector length of 1 should yield one element"))
    } else {
        Ok(Yaml::Sequence(values))
    }
}

fn date_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let slice: Vec<f64> = if let Some(real) = robj.as_real_slice() {
        real.to_vec()
    } else if let Some(ints) = robj.as_integer_slice() {
        ints.iter().map(|v| *v as f64).collect()
    } else {
        return Err(api_other("Expected a numeric Date vector"));
    };

    let mut values = Vec::with_capacity(slice.len());
    for value in slice.iter() {
        if value.is_nan() {
            values.push(Yaml::Value(Scalar::Null));
            continue;
        }
        let rendered =
            render_date_timestamp(*value).ok_or_else(|| api_other("Invalid Date value"))?;
        values.push(tagged_timestamp(rendered));
    }
    if values.len() == 1 {
        Ok(values
            .into_iter()
            .next()
            .expect("vector length of 1 should yield one element"))
    } else {
        Ok(Yaml::Sequence(values))
    }
}

fn tagged_timestamp(value: String) -> Yaml<'static> {
    let tag = timestamp_tag();
    Yaml::Tagged(
        Cow::Owned(tag),
        Box::new(Yaml::Value(Scalar::String(Cow::Owned(value)))),
    )
}

fn timestamp_tag() -> Tag {
    Tag {
        handle: String::new(),
        suffix: "!timestamp".to_string(),
    }
}

fn render_date_timestamp(days: f64) -> Option<String> {
    if !days.is_finite() {
        return None;
    }
    let rounded = days.round();
    let day_count = rounded as i64;
    if (days - days.round()).abs() > 1e-9 {
        return None;
    }
    let (year, month, day) = civil_from_days(day_count)?;
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn render_posix_timestamp(secs: f64) -> Option<String> {
    if !secs.is_finite() {
        return None;
    }
    let mut days = (secs / 86_400.0).floor() as i64;
    let mut seconds = secs - (days as f64) * 86_400.0;
    if seconds < 0.0 {
        days -= 1;
        seconds += 86_400.0;
    }

    let (year, month, day) = civil_from_days(days)?;

    let hour = (seconds / 3_600.0).floor() as u32;
    seconds -= hour as f64 * 3_600.0;
    let minute = (seconds / 60.0).floor() as u32;
    seconds -= minute as f64 * 60.0;
    let second = seconds.floor() as u32;
    let mut fraction = seconds - second as f64;

    // Normalize rounding errors that might push us over a second.
    if fraction >= 0.999_999_5 {
        fraction = 0.0;
    }

    let fraction_str = if fraction > 0.0 {
        let mut frac = fraction;
        let mut buf = String::from(".");
        for _ in 0..9 {
            frac *= 10.0;
            let digit = frac.floor() as u8;
            buf.push((b'0' + digit) as char);
            frac -= digit as f64;
            if frac < 1e-9 {
                break;
            }
        }
        while buf.ends_with('0') {
            buf.pop();
        }
        buf
    } else {
        String::new()
    };

    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{fraction_str}Z"
    ))
}

fn civil_from_days(days: i64) -> Option<(i32, u32, u32)> {
    let z = days.checked_add(719_468)?;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    let year = y + (month <= 2) as i32;
    Some((year, month as u32, day as u32))
}

fn list_to_yaml(robj: &Robj) -> Fallible<Yaml<'static>> {
    let list = robj
        .as_list()
        .ok_or_else(|| Error::Other("Expected a list".to_string()))?;
    if let Some(keys_attr) = robj.get_attrib(sym_yaml_keys()) {
        if !keys_attr.is_null() {
            let keys: List = keys_attr
                .try_into()
                .map_err(|_| Error::Other("`yaml_keys` attribute must be a list".to_string()))?;
            if keys.len() != list.len() {
                return Err(api_other(
                    "`yaml_keys` attribute must have the same length as the list",
                ));
            }
            let mut mapping = Mapping::with_capacity(list.len());
            for ((_, value), (_, key)) in list.iter().zip(keys.iter()) {
                mapping.insert(robj_to_yaml(&key)?, robj_to_yaml(&value)?);
            }
            return Ok(Yaml::Mapping(mapping));
        }
    }

    match robj.names() {
        Some(names) => {
            let mut mapping = Mapping::with_capacity(list.len());
            for (value, name) in list.as_slice().iter().zip(names) {
                let key = if name.is_na() {
                    Yaml::Value(Scalar::Null)
                } else {
                    Yaml::Value(Scalar::String(name.into()))
                };
                mapping.insert(key, robj_to_yaml(value)?);
            }
            Ok(Yaml::Mapping(mapping))
        }
        None => {
            let seq = list
                .as_slice()
                .iter()
                .map(robj_to_yaml)
                .collect::<Fallible<Vec<_>>>()?;
            Ok(Yaml::Sequence(seq))
        }
    }
}

fn apply_tag_if_present(robj: &Robj, node: Yaml<'static>) -> Fallible<Yaml<'static>> {
    if let Some(tag) = extract_yaml_tag(robj)? {
        Ok(Yaml::Tagged(Cow::Owned(tag), Box::new(node)))
    } else {
        Ok(node)
    }
}

fn extract_yaml_tag(robj: &Robj) -> Fallible<Option<Tag>> {
    let attr = match robj.get_attrib(sym_yaml_tag()) {
        Some(value) => value,
        None => return Ok(None),
    };
    let tag_str: &str = (&attr).try_into().map_err(|err: Error| {
        Error::Other(format!(
            "Invalid `yaml_tag` attribute: expected a single, non-missing string ({err})"
        ))
    })?;
    let tag_str = tag_str.trim();
    if tag_str.is_empty() || is_core_schema_tag(tag_str) {
        return Ok(None);
    }
    parse_tag_string(tag_str).map(Some)
}

fn is_core_schema_tag(tag: &str) -> bool {
    let tag = tag.trim();
    tag.starts_with("!!")
        || tag.starts_with("!<tag:yaml.org,2002:")
        || tag.starts_with("!tag:yaml.org,2002:")
        || tag.starts_with("<tag:yaml.org,2002:")
        || tag.starts_with("tag:yaml.org,2002:")
}

fn parse_tag_string(tag: &str) -> Fallible<Tag> {
    if tag.is_empty() {
        return Err(api_other(
            "`yaml_tag` attribute must not be the empty string",
        ));
    }
    if let Some(pos) = tag.rfind('!') {
        if pos + 1 >= tag.len() {
            return Err(api_other(format!("Invalid YAML tag `{tag}`")));
        }
        let handle = &tag[..pos];
        let suffix = &tag[pos + 1..];
        if handle.is_empty() {
            Ok(Tag {
                handle: "!".to_string(),
                suffix: suffix.to_string(),
            })
        } else {
            Ok(Tag {
                handle: handle.to_string(),
                suffix: suffix.to_string(),
            })
        }
    } else {
        Err(api_other(format!("Invalid YAML tag `{tag}`")))
    }
}

pub(crate) fn format_yaml_impl(value: &Robj, multi: bool) -> Fallible<String> {
    if multi {
        let list = value.as_list().ok_or_else(|| {
            Error::Other("`value` must be a list when `multi = TRUE`".to_string())
        })?;
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
