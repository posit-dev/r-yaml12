use extendr_api::prelude::*;
use saphyr::{LoadableYamlNode, Mapping, Scalar, Tag, Yaml, YamlEmitter};

fn yaml_to_robj(node: &Yaml) -> std::result::Result<Robj, String> {
    match node {
        Yaml::Value(scalar) => Ok(scalar_to_robj(scalar)),
        Yaml::Tagged(tag, inner) => convert_tagged(tag, inner),
        Yaml::Sequence(seq) => sequence_to_robj(seq),
        Yaml::Mapping(map) => mapping_to_robj(map),
        Yaml::Representation(raw, _, maybe_tag) => {
            let value = r!(raw.as_ref());
            if let Some(tag) = maybe_tag {
                Ok(set_yaml_tag_attr(value, &format!("{}", tag)))
            } else {
                Ok(value)
            }
        }
        Yaml::Alias(_) => Err("YAML aliases are not supported by yaml12".to_string()),
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
        names.push(render_key(key)?);
        values.push(yaml_to_robj(value)?);
    }
    let list = List::from_names_and_values(names.iter().map(String::as_str), values.into_iter())
        .map_err(|err| err.to_string())?;
    Ok(list.into())
}

fn convert_tagged(tag: &Tag, node: &Yaml) -> std::result::Result<Robj, String> {
    let value = yaml_to_robj(node)?;
    Ok(set_yaml_tag_attr(value, &format!("{}", tag)))
}

fn set_yaml_tag_attr(mut value: Robj, tag: &str) -> Robj {
    if !tag.is_empty()
        && value
            .set_attrib("yaml_tag", Robj::from(tag.to_string()))
            .is_err()
    {
        // ignore types that cannot carry attributes
    }
    value
}

fn render_key(key: &Yaml) -> std::result::Result<String, String> {
    match key {
        Yaml::Value(Scalar::String(value)) => Ok(value.as_ref().to_string()),
        Yaml::Value(Scalar::Integer(value)) => Ok(value.to_string()),
        Yaml::Value(Scalar::FloatingPoint(value)) => Ok(value.into_inner().to_string()),
        Yaml::Value(Scalar::Boolean(value)) => Ok(value.to_string()),
        Yaml::Value(Scalar::Null) => Ok("null".to_string()),
        _ => emit_yaml_borrowed(key),
    }
}

fn emit_yaml_borrowed(doc: &Yaml) -> std::result::Result<String, String> {
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.dump(doc).map_err(|err| err.to_string())?;
    Ok(strip_document_marker(&output).to_string())
}

fn strip_document_marker(text: &str) -> &str {
    text.strip_prefix("---\n").unwrap_or(text)
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
/// Supports scalars, sequences, and mappings; YAML tags are preserved in a
/// `yaml_tag` attribute when possible. YAML aliases are rejected.
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
