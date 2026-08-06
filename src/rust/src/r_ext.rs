use crate::{api_other, Fallible};
use savvy::{
    FunctionArgs, FunctionSexp, NotAvailableValue, OwnedListSexp, OwnedStringSexp, Sexp, StringSexp,
};
use savvy_ffi as ffi;
use std::os::raw::c_char;
use std::ptr;
use std::{slice, str};

// Prefer savvy's public wrappers when they cover the operation. This module
// holds the remaining symbol-based attribute and raw preservation helpers.

// Initialized during package load before any exported .Call wrapper runs.
static mut YAML_KEYS_SYMBOL: ffi::SEXP = ptr::null_mut();
static mut YAML_TAG_SYMBOL: ffi::SEXP = ptr::null_mut();
static mut TZONE_SYMBOL: ffi::SEXP = ptr::null_mut();

#[allow(improper_ctypes)]
extern "C" {
    fn yaml12_charsxp_encoding(x: ffi::SEXP) -> i32;
    fn yaml12_translate_char_utf8(
        x: ffi::SEXP,
        out: *mut *const c_char,
        out_len: *mut usize,
    ) -> ffi::SEXP;
    fn yaml12_scalar_logical(value: i32) -> ffi::SEXP;
    fn yaml12_scalar_integer(value: i32) -> ffi::SEXP;
    fn yaml12_scalar_real(value: f64) -> ffi::SEXP;
    fn yaml12_scalar_string(value: *const c_char, value_len: i32, is_na: i32) -> ffi::SEXP;
    fn yaml12_set_string_elt(
        strings: ffi::SEXP,
        index: ffi::R_xlen_t,
        value: *const c_char,
        value_len: i32,
        is_na: i32,
    ) -> ffi::SEXP;
    fn yaml12_set_name(
        list: ffi::SEXP,
        index: ffi::R_xlen_t,
        value: *const c_char,
        value_len: i32,
        is_na: i32,
    ) -> ffi::SEXP;
}

const CHARSXP_MUST_TRANSLATE: i32 = 0;
const CHARSXP_UTF8: i32 = 1;
const CHARSXP_NATIVE: i32 = 2;

fn check_unwind(result: ffi::SEXP) -> Fallible<ffi::SEXP> {
    if result as usize & 1 == 1 {
        Err(savvy::Error::Aborted(result))
    } else {
        Ok(result)
    }
}

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

fn install_symbol(name: &'static [u8]) -> ffi::SEXP {
    debug_assert_eq!(name.last(), Some(&0));
    unsafe { ffi::Rf_install(name.as_ptr() as *const c_char) }
}

pub(crate) fn init_symbols() -> Fallible<()> {
    unsafe {
        // `Rf_install()` allocates when the symbol is not already interned.
        savvy::unwind_protect(|| {
            YAML_KEYS_SYMBOL = install_symbol(b"yaml_keys\0");
            YAML_TAG_SYMBOL = install_symbol(b"yaml_tag\0");
            TZONE_SYMBOL = install_symbol(b"tzone\0");
            ffi::R_NilValue
        })?;
    }
    Ok(())
}

pub(crate) fn sym_yaml_keys() -> ffi::SEXP {
    unsafe { YAML_KEYS_SYMBOL }
}

pub(crate) fn sym_yaml_tag() -> ffi::SEXP {
    unsafe { YAML_TAG_SYMBOL }
}

pub(crate) fn sym_tzone() -> ffi::SEXP {
    unsafe { TZONE_SYMBOL }
}

pub(crate) fn get_attrib_sym(value: &Sexp, attr: ffi::SEXP) -> Option<Sexp> {
    let attr_value = unsafe { ffi::Rf_getAttrib(value.0, attr) };
    if attr_value == unsafe { ffi::R_NilValue } {
        None
    } else {
        Some(Sexp(attr_value))
    }
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

pub(crate) fn set_class<T, U>(value: &mut Sexp, classes: T) -> Fallible<()>
where
    T: AsRef<[U]>,
    U: AsRef<str>,
{
    let _value_guard = PreservedSexp::new(Sexp(value.0));
    value.set_class(classes)
}

// These scalar leaves return fresh, unprotected SEXPs. A caller must return the
// result, insert it into an already-rooted container, or preserve it before the
// next allocation-capable R operation.
pub(crate) fn string_scalar(value: &str) -> Fallible<Sexp> {
    let (value, value_len, is_na) = string_parts(value)?;
    unsafe { check_unwind(yaml12_scalar_string(value, value_len, is_na)).map(Sexp) }
}

pub(crate) fn logical_scalar(value: bool) -> Fallible<Sexp> {
    unsafe { check_unwind(yaml12_scalar_logical(value.into())).map(Sexp) }
}

pub(crate) fn integer_scalar(value: i32) -> Fallible<Sexp> {
    unsafe { check_unwind(yaml12_scalar_integer(value)).map(Sexp) }
}

pub(crate) fn real_scalar(value: f64) -> Fallible<Sexp> {
    unsafe { check_unwind(yaml12_scalar_real(value)).map(Sexp) }
}

fn string_parts(value: &str) -> Fallible<(*const c_char, i32, i32)> {
    if value.is_na() {
        return Ok((ptr::null(), 0, 1));
    }
    let value_len = i32::try_from(value.len())
        .map_err(|_| api_other("R strings cannot exceed i32::MAX bytes"))?;
    Ok((value.as_ptr().cast::<c_char>(), value_len, 0))
}

pub(crate) fn set_string_elt(strings: &mut OwnedStringSexp, i: usize, value: &str) -> Fallible<()> {
    if i >= strings.len() {
        return Err(api_other("string index out of bounds"));
    }
    let (value, value_len, is_na) = string_parts(value)?;
    unsafe {
        check_unwind(yaml12_set_string_elt(
            strings.inner(),
            i as ffi::R_xlen_t,
            value,
            value_len,
            is_na,
        ))?;
    }
    Ok(())
}

pub(crate) fn set_name(list: &mut OwnedListSexp, i: usize, value: &str) -> Fallible<()> {
    if i >= list.len() {
        return Err(api_other("list index out of bounds"));
    }
    let (value, value_len, is_na) = string_parts(value)?;
    unsafe {
        check_unwind(yaml12_set_name(
            list.inner(),
            i as ffi::R_xlen_t,
            value,
            value_len,
            is_na,
        ))?;
    }
    Ok(())
}

pub(crate) fn call1(handler: &FunctionSexp, arg: Sexp) -> Fallible<Sexp> {
    let arg_guard = PreservedSexp::new(arg);
    let mut args = FunctionArgs::new();
    args.add("", arg_guard.value())?;
    handler.call(args).map(Into::into)
}

pub(crate) fn as_string_scalar(value: &Sexp) -> Fallible<Option<&'static str>> {
    let strings = match StringSexp::try_from(Sexp(value.0)) {
        Ok(strings) => strings,
        Err(_) => return Ok(None),
    };
    if strings.len() != 1 {
        return Ok(None);
    }
    let value = string_elt(&strings, 0)?;
    Ok((!value.is_na()).then_some(value))
}

pub(crate) fn string_sexp(value: &Sexp) -> Option<StringSexp> {
    StringSexp::try_from(Sexp(value.0)).ok()
}

pub(crate) fn string_elt(strings: &StringSexp, i: usize) -> Fallible<&'static str> {
    debug_assert!(i < strings.len());
    unsafe {
        let charsxp = ffi::STRING_ELT(strings.inner(), i as _);
        if charsxp == ffi::R_NaString {
            Ok(<&str>::na())
        } else {
            charsxp_to_str(charsxp)
        }
    }
}

pub(crate) fn names(value: &Sexp) -> Fallible<Option<StringSexp>> {
    let Some(names) = get_attrib_sym(value, unsafe { ffi::R_NamesSymbol }) else {
        return Ok(None);
    };
    Ok(Some(StringSexp::try_from(names)?))
}

pub(crate) fn class(value: &Sexp) -> Fallible<Option<StringSexp>> {
    let Some(class) = get_attrib_sym(value, unsafe { ffi::R_ClassSymbol }) else {
        return Ok(None);
    };
    Ok(Some(StringSexp::try_from(class)?))
}

pub(crate) fn inherits(value: &Sexp, class_name: &str) -> Fallible<bool> {
    let Some(class_attr) = class(value)? else {
        return Ok(false);
    };
    for i in 0..class_attr.len() {
        let value = string_elt(&class_attr, i)?;
        if !value.is_na() && value == class_name {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn has_attributes(value: &Sexp) -> bool {
    unsafe { ffi::ATTRIB(value.0) != ffi::R_NilValue }
}

fn charsxp_to_str(charsxp: ffi::SEXP) -> Fallible<&'static str> {
    unsafe {
        let len = usize::try_from(ffi::Rf_xlength(charsxp))
            .map_err(|_| api_other("R string has an invalid length"))?;
        let bytes = slice::from_raw_parts(ffi::R_CHAR(charsxp).cast::<u8>(), len);

        match yaml12_charsxp_encoding(charsxp) {
            CHARSXP_UTF8 => {
                return str::from_utf8(bytes)
                    .map_err(|_| api_other("R UTF-8 string contains invalid UTF-8"));
            }
            CHARSXP_NATIVE if bytes.is_ascii() => {
                // ASCII is valid UTF-8 under every supported native encoding.
                return Ok(str::from_utf8_unchecked(bytes));
            }
            CHARSXP_MUST_TRANSLATE | CHARSXP_NATIVE => {}
            _ => return Err(api_other("R string has an unknown encoding")),
        }

        let mut translated = ptr::null();
        let mut translated_len = 0;
        let result = yaml12_translate_char_utf8(charsxp, &mut translated, &mut translated_len);
        check_unwind(result)?;

        str::from_utf8(slice::from_raw_parts(
            translated.cast::<u8>(),
            translated_len,
        ))
        .map_err(|_| api_other("Rf_translateCharUTF8 returned invalid UTF-8"))
    }
}
