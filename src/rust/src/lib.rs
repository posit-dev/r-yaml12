mod handlers;
mod r_to_yaml;
mod timestamp;
mod unwind;
mod warning;
mod yaml_to_r;

use crate::r_to_yaml::yaml_body;
use extendr_api::prelude::*;
use extendr_ffi as ffi;
use saphyr::{LoadableYamlNode, Yaml};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::result::Result as StdResult;
use std::{cell::OnceCell, thread_local};
use unwind::EvalError;

type Fallible<T> = StdResult<T, EvalError>;

pub(crate) const R_STRING_MAX_BYTES: usize = i32::MAX as usize;
/// Toggle timestamp parsing/formatting. Set to `true` to re-enable.
pub(crate) const TIMESTAMP_SUPPORT_ENABLED: bool = false;

fn api_other(msg: impl Into<String>) -> EvalError {
    EvalError::Api(Error::Other(msg.into()))
}

const TAGGED_POINTER_MASK: usize = 1;

fn tagged_error_message(message: &str) -> ffi::SEXP {
    unsafe {
        let string = ffi::Rf_mkCharLenCE(
            message.as_ptr() as *const std::os::raw::c_char,
            message.len() as i32,
            ffi::cetype_t::CE_UTF8,
        );
        (string as usize | TAGGED_POINTER_MASK) as ffi::SEXP
    }
}

fn ffi_result(result: Fallible<Robj>) -> ffi::SEXP {
    match result {
        Ok(value) => unsafe { value.get() },
        Err(EvalError::Jump(token)) => token.into_tagged_sexp(),
        Err(EvalError::Api(err)) => {
            let message = err.to_string();
            drop(err);
            tagged_error_message(&message)
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Rust panic".to_string()
    }
}

fn ffi_catch(f: impl FnOnce() -> Fallible<Robj>) -> ffi::SEXP {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => ffi_result(result),
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            drop(payload);
            tagged_error_message(&message)
        }
    }
}

fn robj_from_sexp(sexp: ffi::SEXP) -> Robj {
    unsafe { Robj::from_sexp(sexp) }
}

fn bool_arg(value: &Robj, name: &str) -> Fallible<bool> {
    bool::try_from(value).map_err(|_| api_other(format!("`{name}` must be TRUE or FALSE")))
}

macro_rules! cached_sym {
    ($cell:ident, $name:ident, $getter:ident) => {
        thread_local! {
            static $cell: OnceCell<Robj> = OnceCell::new();
        }

        #[inline]
        pub(crate) fn $getter() -> Robj {
            $cell.with(|cell| cell.get_or_init(|| sym!($name)).clone())
        }
    };
}

cached_sym!(YAML_KEYS_SYM, yaml_keys, sym_yaml_keys);
cached_sym!(YAML_TAG_SYM, yaml_tag, sym_yaml_tag);

/// Format or write R objects as YAML 1.2.
///
/// `format_yaml()` returns YAML as a character string. `write_yaml()` writes a
/// YAML stream to a file or stdout and always emits document start (`---`)
/// markers and a final end (`...`) marker. Both functions honor a `yaml_tag`
/// attribute on values (see examples).
///
/// @param value Any R object composed of lists, atomic vectors, and scalars.
/// @param path Scalar string file path to write YAML to when using `write_yaml()`.
///   When `NULL` (the default), write to R's standard output connection.
/// @param multi When `TRUE`, treat `value` as a list of YAML documents and encode a stream.
/// @return `format_yaml()` returns a scalar character string containing YAML.
///   `write_yaml()` invisibly returns `value`.
/// @rdname format_yaml
/// @export
/// @examples
/// cat(format_yaml(list(foo = 1, bar = list(TRUE, NA))))
///
/// docs <- list("first", "second")
/// cat(format_yaml(docs, multi = TRUE))
///
/// tagged <- structure("1 + 1", yaml_tag = "!expr")
/// cat(tagged_yaml <- format_yaml(tagged), "\n")
///
/// dput(parse_yaml(tagged_yaml))
#[no_mangle]
pub extern "C" fn yaml12_format_yaml_ffi(value: ffi::SEXP, multi: ffi::SEXP) -> ffi::SEXP {
    ffi_catch(|| {
        let value = robj_from_sexp(value);
        let multi = robj_from_sexp(multi);
        let multi = bool_arg(&multi, "multi")?;

        let yaml = r_to_yaml::format_yaml_impl(&value, multi)?;
        let body = yaml_body(&yaml, multi);
        if body.len() > R_STRING_MAX_BYTES {
            return Err(api_other(
                "Formatted YAML exceeds R's 2^31-1 byte string limit",
            ));
        }
        Ok(Robj::from(body))
    })
}

/// Parse YAML 1.2 document(s) into base R structures.
///
/// `parse_yaml()` takes strings of YAML; `read_yaml()` reads from a file path.
///
/// YAML tags without a corresponding `handler` are preserved in a `yaml_tag` attribute.
/// Mappings with keys that are not all simple scalar strings are returned as a named list with a `yaml_keys` attribute.
///
/// @param text Character vector; elements are concatenated with `"\n"`.
/// @param path Scalar string path to a YAML file`.
/// @param multi When `TRUE`, return a list containing all documents in the stream.
/// @param simplify When `FALSE`, keep YAML sequences as R lists instead of simplifying to atomic vectors.
/// @param handlers Named list of R functions with names corresponding to YAML tags; matching handlers transform tagged values.
/// @return When `multi = FALSE`, returns a parsed R object for the first document.
///   When `multi = TRUE`, returns a list of parsed documents.
/// @rdname parse_yaml
/// @examples
/// dput(parse_yaml("foo: [1, 2, 3]"))
///
/// # homogeneous sequences simplify by default.
/// # YAML null maps to NA in otherwise homogeneous sequences.
/// dput(parse_yaml("foo: [1, 2, 3, null]"))
///
/// # mixed type sequence never simplify
/// dput(parse_yaml("[1, true, cat]"))
///
/// # use `simplify=FALSE` to always return sequences as lists.
/// str(parse_yaml("foo: [1, 2, 3, null]", simplify = FALSE))
///
/// # Parse multiple documents when requested.
/// stream <- "
/// ---
/// first: 1
/// ---
/// second: 2
/// "
/// str(parse_yaml(stream, multi = TRUE))
///
/// # Read from a file; keep sequences as lists.
/// path <- tempfile(fileext = ".yaml")
/// writeLines("alpha: [true, null]\nbeta: 3.5", path)
/// str(read_yaml(path, simplify = FALSE))
/// @export
#[no_mangle]
pub extern "C" fn yaml12_parse_yaml_ffi(
    text: ffi::SEXP,
    multi: ffi::SEXP,
    simplify: ffi::SEXP,
    handlers: ffi::SEXP,
) -> ffi::SEXP {
    ffi_catch(|| {
        let text = robj_from_sexp(text);
        let text: Strings = text.try_into()?;
        let multi = robj_from_sexp(multi);
        let multi = bool_arg(&multi, "multi")?;
        let simplify = robj_from_sexp(simplify);
        let simplify = bool_arg(&simplify, "simplify")?;
        let handlers = robj_from_sexp(handlers);

        yaml_to_r::parse_yaml_impl(text, multi, simplify, handlers)
    })
}

/// Debug helper: print saphyr `Yaml` nodes without converting to R objects.
///
/// @noRd
#[no_mangle]
pub extern "C" fn yaml12_dbg_yaml_ffi(text: ffi::SEXP) -> ffi::SEXP {
    ffi_catch(|| {
        let text = robj_from_sexp(text);
        let text: Strings = text.try_into()?;

        if text.is_empty() {
            return Ok(NULL.into());
        }

        let mut joined = String::new();
        for (idx, part) in text.iter().enumerate() {
            if part.is_na() {
                Err(api_other("`text` must not contain NA strings"))?;
            }
            if idx > 0 {
                joined.push('\n');
            }
            joined.push_str(part.as_ref());
        }

        let docs = Yaml::load_from_str(&joined)
            .map_err(|err| api_other(format!("YAML parse error: {err}")))?;
        rprintln!("{:#?}", docs);
        Ok(NULL.into())
    })
}

/// Read YAML 1.2 document(s) from a file path.
///
/// @rdname parse_yaml
/// @export
#[no_mangle]
pub extern "C" fn yaml12_read_yaml_ffi(
    path: ffi::SEXP,
    multi: ffi::SEXP,
    simplify: ffi::SEXP,
    handlers: ffi::SEXP,
) -> ffi::SEXP {
    ffi_catch(|| {
        let path = robj_from_sexp(path);
        let path: &str = (&path)
            .try_into()
            .map_err(|_| api_other("`path` must be a single, non-missing string"))?;
        let multi = robj_from_sexp(multi);
        let multi = bool_arg(&multi, "multi")?;
        let simplify = robj_from_sexp(simplify);
        let simplify = bool_arg(&simplify, "simplify")?;
        let handlers = robj_from_sexp(handlers);

        yaml_to_r::read_yaml_impl(path, multi, simplify, handlers)
    })
}

/// Write an R object as YAML 1.2 to a file.
///
/// @rdname format_yaml
/// @examples
///
///
/// write_yaml(list(foo = 1, bar = list(2, "baz")))
///
/// write_yaml(list("foo", "bar"), multi = TRUE)
///
/// tagged <- structure("1 + 1", yaml_tag = "!expr")
/// write_yaml(tagged)
/// @export
#[no_mangle]
pub extern "C" fn yaml12_write_yaml_ffi(
    value: ffi::SEXP,
    path: ffi::SEXP,
    multi: ffi::SEXP,
) -> ffi::SEXP {
    ffi_catch(|| {
        let value = robj_from_sexp(value);
        let path = robj_from_sexp(path);
        let path_opt =
            if path.is_null() {
                None
            } else {
                Some(path.as_str().ok_or_else(|| {
                    api_other("`path` must be NULL or a single, non-missing string")
                })?)
            };
        let multi = robj_from_sexp(multi);
        let multi = bool_arg(&multi, "multi")?;

        r_to_yaml::write_yaml_impl(&value, path_opt, multi)?;
        Ok(value)
    })
}
