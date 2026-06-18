use crate::{api_other, Fallible, R_STRING_MAX_BYTES};
use savvy::{Error, NotAvailableValue, Sexp, StringSexp};
use savvy_ffi as ffi;
use std::ffi::CString;
use std::os::raw::c_char;

pub(crate) fn null() -> Sexp {
    unsafe { Sexp(ffi::R_NilValue) }
}

pub(crate) struct PreservedSexp {
    sexp: Sexp,
    token: ffi::SEXP,
}

impl PreservedSexp {
    pub(crate) fn new(sexp: Sexp) -> Self {
        let raw = sexp.0;
        let token = savvy::protect::insert_to_preserved_list(raw);
        Self {
            sexp: Sexp(raw),
            token,
        }
    }

    pub(crate) fn value(&self) -> Sexp {
        Sexp(self.sexp.0)
    }
}

impl Drop for PreservedSexp {
    fn drop(&mut self) {
        savvy::protect::release_from_preserved_list(self.token);
    }
}

pub(crate) fn install(name: &str) -> Fallible<ffi::SEXP> {
    let name =
        CString::new(name).map_err(|_| api_other("Attribute name contains interior nul byte"))?;
    unsafe { savvy::unwind_protect(|| ffi::Rf_install(name.as_ptr())) }
}

pub(crate) fn sym_yaml_keys() -> Fallible<ffi::SEXP> {
    unsafe { savvy::unwind_protect(|| ffi::Rf_install(b"yaml_keys\0".as_ptr() as *const c_char)) }
}

pub(crate) fn sym_yaml_tag() -> Fallible<ffi::SEXP> {
    unsafe { savvy::unwind_protect(|| ffi::Rf_install(b"yaml_tag\0".as_ptr() as *const c_char)) }
}

pub(crate) fn get_attrib_sym(value: &Sexp, attr: ffi::SEXP) -> Fallible<Option<Sexp>> {
    let attr_value = unsafe { savvy::unwind_protect(|| ffi::Rf_getAttrib(value.0, attr))? };
    if attr_value == unsafe { ffi::R_NilValue } {
        Ok(None)
    } else {
        Ok(Some(Sexp(attr_value)))
    }
}

pub(crate) fn get_attrib_str(value: &Sexp, attr: &str) -> Fallible<Option<Sexp>> {
    let attr = install(attr)?;
    get_attrib_sym(value, attr)
}

pub(crate) fn set_attrib_sym(value: &mut Sexp, attr: ffi::SEXP, attr_value: Sexp) -> Fallible<()> {
    let _value_guard = PreservedSexp::new(Sexp(value.0));
    let _attr_value_guard = PreservedSexp::new(Sexp(attr_value.0));
    unsafe {
        savvy::unwind_protect(|| {
            ffi::Rf_setAttrib(value.0, attr, attr_value.0);
            ffi::R_NilValue
        })?;
    }
    Ok(())
}

pub(crate) fn set_attrib_str(value: &mut Sexp, attr: &str, attr_value: Sexp) -> Fallible<()> {
    let _value_guard = PreservedSexp::new(Sexp(value.0));
    let _attr_value_guard = PreservedSexp::new(Sexp(attr_value.0));
    let attr = install(attr)?;
    unsafe {
        savvy::unwind_protect(|| {
            ffi::Rf_setAttrib(value.0, attr, attr_value.0);
            ffi::R_NilValue
        })?;
    }
    Ok(())
}

pub(crate) fn set_class<T, U>(value: &mut Sexp, classes: T) -> Fallible<()>
where
    T: AsRef<[U]>,
    U: AsRef<str>,
{
    let _value_guard = PreservedSexp::new(Sexp(value.0));
    value.set_class(classes)
}

pub(crate) fn string_scalar(value: &str) -> Fallible<Sexp> {
    if value.len() > R_STRING_MAX_BYTES {
        return Err(api_other(
            "R character value exceeds R's 2^31-1 byte string limit",
        ));
    }
    savvy::OwnedStringSexp::try_from_scalar(value).map(Into::into)
}

pub(crate) fn as_string_scalar(value: &Sexp) -> Option<&'static str> {
    let strings = StringSexp::try_from(Sexp(value.0)).ok()?;
    if strings.len() != 1 {
        return None;
    }
    let value = strings.iter().next()?;
    (!value.is_na()).then_some(value)
}

pub(crate) fn as_bool_scalar(value: &Sexp) -> Option<bool> {
    bool::try_from(Sexp(value.0)).ok()
}

pub(crate) fn string_sexp(value: &Sexp) -> Option<StringSexp> {
    StringSexp::try_from(Sexp(value.0)).ok()
}

pub(crate) fn string_elt(strings: &StringSexp, i: usize) -> &'static str {
    debug_assert!(i < strings.len());
    unsafe {
        let charsxp = ffi::STRING_ELT(strings.inner(), i as _);
        if charsxp == ffi::R_NaString {
            <&str>::na()
        } else {
            charsxp_to_str(charsxp)
        }
    }
}

pub(crate) fn names(value: &Sexp) -> Fallible<Option<StringSexp>> {
    let Some(names) = get_attrib_sym(value, unsafe { ffi::R_NamesSymbol })? else {
        return Ok(None);
    };
    Ok(Some(StringSexp::try_from(names)?))
}

pub(crate) fn class(value: &Sexp) -> Fallible<Option<StringSexp>> {
    let Some(class) = get_attrib_sym(value, unsafe { ffi::R_ClassSymbol })? else {
        return Ok(None);
    };
    Ok(Some(StringSexp::try_from(class)?))
}

pub(crate) fn inherits(value: &Sexp, class_name: &str) -> bool {
    class(value)
        .ok()
        .flatten()
        .map(|class_attr| {
            class_attr
                .iter()
                .any(|value| !value.is_na() && value == class_name)
        })
        .unwrap_or(false)
}

pub(crate) fn has_attributes(value: &Sexp) -> bool {
    unsafe { ffi::ATTRIB(value.0) != ffi::R_NilValue }
}

pub(crate) fn rtype_name(value: &Sexp) -> &'static str {
    match unsafe { ffi::TYPEOF(value.0) } {
        ffi::NILSXP => "Null",
        ffi::LGLSXP => "Logicals",
        ffi::INTSXP => "Integers",
        ffi::REALSXP => "Doubles",
        ffi::CPLXSXP => "Complexes",
        ffi::STRSXP => "Strings",
        ffi::VECSXP => "List",
        ffi::CLOSXP | ffi::BUILTINSXP | ffi::SPECIALSXP => "Function",
        _ => "Other",
    }
}

pub(crate) fn expected_strings_error(value: &Sexp) -> Error {
    api_other(format!("Expected Strings got {}", rtype_name(value)))
}

fn charsxp_to_str(charsxp: ffi::SEXP) -> &'static str {
    unsafe {
        let ptr = ffi::R_CHAR(charsxp) as *const u8;
        let len = ffi::Rf_xlength(charsxp) as usize;
        let bytes = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8_unchecked(bytes)
    }
}
