mod emitter;
mod handlers;
mod r_ext;
mod r_to_yaml;
mod warning;
mod yaml_to_r;

use crate::r_ext::null;
use crate::r_to_yaml::yaml_body;
use saphyr::{LoadableYamlNode, Yaml};
use savvy::{savvy, savvy_init, NotAvailableValue, NumericScalar, Sexp, StringSexp};
use savvy_ffi::DllInfo;

pub(crate) type Fallible<T> = savvy::Result<T>;

pub(crate) const R_STRING_MAX_BYTES: usize = i32::MAX as usize;

fn api_other(msg: impl Into<String>) -> savvy::Error {
    savvy::Error::new(msg.into())
}

#[savvy_init]
fn init_yaml12(_dll_info: *mut DllInfo) -> savvy::Result<()> {
    r_ext::init_symbols()
}

fn width_arg(width: Sexp, name: &str) -> savvy::Result<Option<usize>> {
    let invalid = || {
        api_other(format!(
            "`{name}` must be NULL, Inf, or a single number >= 1"
        ))
    };
    if width.is_null() {
        return Ok(None);
    }

    let width = NumericScalar::try_from(width)
        .map_err(|_| invalid())?
        .as_f64();
    if !width.is_finite() {
        return Ok(None);
    }
    if width < 1.0 {
        return Err(invalid());
    }
    Ok(Some(width.floor() as usize))
}

#[savvy]
fn format_yaml_native(value: Sexp, multi: bool, width: Sexp) -> savvy::Result<Sexp> {
    let yaml = r_to_yaml::format_yaml_impl(&value, multi, width_arg(width, "width")?)?;
    let body = yaml_body(&yaml, multi);
    if body.len() > R_STRING_MAX_BYTES {
        return Err(api_other(
            "Formatted YAML exceeds R's 2^31-1 byte string limit",
        ));
    }
    r_ext::string_scalar(body)
}

#[savvy]
fn parse_yaml_native(
    text: StringSexp,
    multi: bool,
    simplify: bool,
    handlers: Sexp,
) -> savvy::Result<Sexp> {
    yaml_to_r::parse_yaml_impl(text, multi, simplify, handlers)
}

#[savvy]
fn dbg_yaml_native(text: StringSexp) -> savvy::Result<Sexp> {
    if text.is_empty() {
        return Ok(null());
    }

    let mut joined = String::new();
    for idx in 0..text.len() {
        let part = r_ext::string_elt(&text, idx)?;
        if part.is_na() {
            Err(api_other("`text` must not contain NA strings"))?;
        }
        if idx > 0 {
            joined.push('\n');
        }
        joined.push_str(part);
    }

    let docs = Yaml::load_from_str(&joined)
        .map_err(|err| api_other(format!("YAML parse error: {err}")))?;
    savvy::io::r_print(&format!("{:#?}", docs), true);
    Ok(null())
}

fn path_arg(path: &StringSexp, name: &str) -> savvy::Result<String> {
    if path.len() != 1 {
        return Err(api_other(format!(
            "`{name}` must be a single, non-missing string"
        )));
    }
    let path = r_ext::string_elt(path, 0)?;
    if path.is_na() {
        return Err(api_other(format!(
            "`{name}` must be a single, non-missing string"
        )));
    }
    Ok(path.to_string())
}

fn optional_path_arg(path: Sexp) -> savvy::Result<Option<String>> {
    if path.is_null() {
        return Ok(None);
    }

    let path = StringSexp::try_from(path)
        .map_err(|_| api_other("`path` must be NULL or a single, non-missing string"))?;
    path_arg(&path, "path").map(Some)
}

#[savvy]
fn read_yaml_native(
    path: StringSexp,
    multi: bool,
    simplify: bool,
    handlers: Sexp,
) -> savvy::Result<Sexp> {
    let path = path_arg(&path, "path")?;
    yaml_to_r::read_yaml_impl(&path, multi, simplify, handlers)
}

#[savvy]
fn write_yaml_native(
    value: Sexp,
    path: Sexp,
    multi: bool,
    width: Sexp,
    append: bool,
) -> savvy::Result<Sexp> {
    let path = optional_path_arg(path)?;
    r_to_yaml::write_yaml_impl(
        &value,
        path.as_deref(),
        multi,
        width_arg(width, "width")?,
        append,
    )?;
    Ok(value)
}
