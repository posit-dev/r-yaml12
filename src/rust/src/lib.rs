mod handlers;
mod r_ext;
mod r_to_yaml;
mod timestamp;
mod warning;
mod yaml_to_r;

use crate::r_ext::null;
use crate::r_to_yaml::yaml_body;
use saphyr::{LoadableYamlNode, Yaml};
use savvy::{savvy, savvy_init, NotAvailableValue, OwnedStringSexp, Sexp, StringSexp};
use savvy_ffi::DllInfo;

pub(crate) type Fallible<T> = savvy::Result<T>;

pub(crate) const R_STRING_MAX_BYTES: usize = i32::MAX as usize;
/// Toggle timestamp parsing/formatting. Set to `true` to re-enable.
pub(crate) const TIMESTAMP_SUPPORT_ENABLED: bool = false;

fn api_other(msg: impl Into<String>) -> savvy::Error {
    savvy::Error::new(msg.into())
}

#[savvy_init]
fn init_yaml12(_dll_info: *mut DllInfo) -> savvy::Result<()> {
    r_ext::init_symbols()
}

#[savvy]
fn format_yaml_native(value: Sexp, multi: bool) -> savvy::Result<Sexp> {
    let yaml = r_to_yaml::format_yaml_impl(&value, multi)?;
    let body = yaml_body(&yaml, multi);
    if body.len() > R_STRING_MAX_BYTES {
        return Err(api_other(
            "Formatted YAML exceeds R's 2^31-1 byte string limit",
        ));
    }
    OwnedStringSexp::try_from_scalar(body).map(Into::into)
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
    for (idx, part) in text.iter().enumerate() {
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

#[savvy]
fn read_yaml_native(
    path: &str,
    multi: bool,
    simplify: bool,
    handlers: Sexp,
) -> savvy::Result<Sexp> {
    yaml_to_r::read_yaml_impl(path, multi, simplify, handlers)
}

#[savvy]
fn write_yaml_native(value: Sexp, multi: bool, path: Option<&str>) -> savvy::Result<Sexp> {
    r_to_yaml::write_yaml_impl(&value, path, multi)?;
    Ok(value)
}
