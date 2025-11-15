use extendr_api::prelude::*;
use saphyr::{LoadableYamlNode, Mapping, Scalar, Yaml};

fn yaml_to_robj(node: &Yaml) -> std::result::Result<Robj, String> {
    match node {
        Yaml::Value(scalar) => Ok(scalar_to_robj(scalar)),
        Yaml::Sequence(seq) => sequence_to_robj(seq),
        Yaml::Mapping(map) => mapping_to_robj(map),
        Yaml::Tagged(_, _) | Yaml::Representation(_, _, _) => {
            Err("YAML tags and raw representations are not supported yet".to_string())
        }
        Yaml::Alias(_) => Err("YAML aliases are not supported yet".to_string()),
        Yaml::BadValue => Err("Encountered an invalid YAML scalar value".to_string()),
    }
}

fn scalar_to_robj(scalar: &Scalar) -> Robj {
    match scalar {
        Scalar::Null => NULL.into(),
        Scalar::Boolean(value) => r!(*value),
        Scalar::Integer(value) => {
            if let Ok(v) = i32::try_from(*value) {
                r!(v)
            } else {
                r!(*value as f64)
            }
        }
        Scalar::FloatingPoint(value) => r!(value.into_inner()),
        Scalar::String(value) => r!(value.as_ref()),
    }
}

fn sequence_to_robj(seq: &[Yaml]) -> std::result::Result<Robj, String> {
    let mut values = Vec::with_capacity(seq.len());
    for node in seq {
        values.push(yaml_to_robj(node)?);
    }
    Ok(List::from_values(values).into())
}

fn mapping_to_robj(map: &Mapping) -> std::result::Result<Robj, String> {
    let mut names = Vec::with_capacity(map.len());
    let mut values = Vec::with_capacity(map.len());
    for (key, value) in map.iter() {
        let name = match key {
            Yaml::Value(Scalar::String(value)) => value.as_ref().to_string(),
            _ => {
                return Err("parse_yaml currently supports only string keys in mappings".to_string())
            }
        };
        names.push(name);
        values.push(yaml_to_robj(value)?);
    }
    let list = List::from_names_and_values(names.iter().map(String::as_str), values.into_iter())
        .map_err(|err| err.to_string())?;
    Ok(list.into())
}

fn collapse_lines(text: &Strings) -> Result<String> {
    let mut out = String::new();
    for line in text.iter() {
        if line.is_na() {
            return Err(Error::Other(
                "`text` must not contain NA strings".to_string(),
            ));
        }
        out.push_str(line.as_ref());
        out.push('\n');
    }
    out.pop();
    Ok(out)
}

/// Parse a single YAML 1.2 document into base R structures.
///
/// Supports the common scalar types plus sequences and mappings with string
/// keys; YAML tags, aliases, and other advanced nodes are rejected.
/// @param text Character vector; elements are concatenated with `"\n"`.
/// @export
#[extendr]
fn parse_yaml(text: Strings) -> Result<Robj> {
    let joined = collapse_lines(&text)?;
    let docs = Yaml::load_from_str(&joined)
        .map_err(|err| Error::Other(format!("YAML parse error: {}", err)))?;
    match docs.first() {
        Some(doc) => {
            yaml_to_robj(doc).map_err(|msg| Error::Other(format!("Unsupported YAML: {}", msg)))
        }
        None => Ok(NULL.into()),
    }
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod yaml12;
    fn parse_yaml;
}
