use crate::{api_other, Fallible};
use savvy::{FunctionSexp, NotAvailableValue, OwnedListSexp, Sexp, StringSexp};
use savvy_ffi as ffi;
use std::marker::PhantomData;
use std::os::raw::c_char;
use std::ptr;
use std::{slice, str};

// Prefer savvy's public wrappers when they cover the operation. This module
// holds the remaining symbol-based attribute and raw preservation helpers.

// Initialized during package load before any exported .Call wrapper runs.
static mut YAML_KEYS_SYMBOL: ffi::SEXP = ptr::null_mut();
static mut YAML_TAG_SYMBOL: ffi::SEXP = ptr::null_mut();

#[allow(improper_ctypes)]
extern "C" {
    fn yaml12_charsxp_encoding(x: ffi::SEXP) -> i32;
    fn yaml12_has_attributes(x: ffi::SEXP) -> i32;
    fn yaml12_translate_char_utf8(
        x: ffi::SEXP,
        out: *mut *const c_char,
        out_len: *mut usize,
    ) -> ffi::SEXP;
    fn yaml12_scalar_logical(value: i32) -> ffi::SEXP;
    fn yaml12_scalar_integer(value: i32) -> ffi::SEXP;
    fn yaml12_scalar_real(value: f64) -> ffi::SEXP;
    fn yaml12_scalar_string(value: *const c_char, value_len: i32, is_na: i32) -> ffi::SEXP;
    fn yaml12_materialize_list(
        target: ffi::SEXP,
        elements: *const ListElement<'_>,
        names: *const StringData<'_>,
        length: ffi::R_xlen_t,
    ) -> ffi::SEXP;
    fn yaml12_materialize_string_vector(
        values: *const StringData<'_>,
        length: ffi::R_xlen_t,
    ) -> ffi::SEXP;
    fn yaml12_call1(function: ffi::SEXP, argument: ffi::SEXP) -> ffi::SEXP;
}

const CHARSXP_MUST_TRANSLATE: i32 = 0;
const CHARSXP_UTF8: i32 = 1;
const CHARSXP_NATIVE: i32 = 2;
const CHARSXP_ASCII: i32 = 3;

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

// These scalar leaves return raw SEXPs without a Rust ownership guard. A caller
// must return the result, insert it into an already-rooted container, or
// preserve any allocation-backed result before the next allocation-capable R
// operation.
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

// String pointers borrow storage kept alive by the synchronous Rust caller.
#[repr(C)]
pub(crate) struct ListElement<'a> {
    kind: i32,
    int_value: i32,
    real_value: f64,
    string_value: *const c_char,
    string_len: i32,
    string_is_na: i32,
    string_borrow: PhantomData<&'a str>,
}

#[repr(C)]
pub(crate) struct StringData<'a> {
    value: *const c_char,
    value_len: i32,
    is_na: i32,
    borrow: PhantomData<&'a str>,
}

impl<'a> ListElement<'a> {
    fn new(kind: i32) -> Self {
        Self {
            kind,
            int_value: 0,
            real_value: 0.0,
            string_value: ptr::null(),
            string_len: 0,
            string_is_na: 0,
            string_borrow: PhantomData,
        }
    }

    pub(crate) fn null() -> Self {
        Self::new(0)
    }

    pub(crate) fn logical(value: bool) -> Self {
        let mut element = Self::new(1);
        element.int_value = value.into();
        element
    }

    pub(crate) fn integer(value: i32) -> Self {
        let mut element = Self::new(2);
        element.int_value = value;
        element
    }

    pub(crate) fn real(value: f64) -> Self {
        let mut element = Self::new(3);
        element.real_value = value;
        element
    }

    pub(crate) fn string(value: &'a str) -> Fallible<Self> {
        let string = string_data(value)?;
        let mut element = Self::new(4);
        element.string_value = string.value;
        element.string_len = string.value_len;
        element.string_is_na = string.is_na;
        Ok(element)
    }

    pub(crate) fn skip() -> Self {
        Self::new(5)
    }

    fn is_skip(&self) -> bool {
        self.kind == 5
    }
}

pub(crate) fn string_data(value: &str) -> Fallible<StringData<'_>> {
    let (value, value_len, is_na) = string_parts(value)?;
    Ok(StringData {
        value,
        value_len,
        is_na,
        borrow: PhantomData,
    })
}

pub(crate) fn materialize_list(
    target: Option<&OwnedListSexp>,
    elements: &[ListElement<'_>],
    names: Option<&[StringData<'_>]>,
) -> Fallible<Sexp> {
    if let Some(target) = target {
        debug_assert_eq!(target.len(), elements.len());
    }
    debug_assert!(target.is_some() || elements.iter().all(|element| !element.is_skip()));
    if let Some(names) = names {
        debug_assert_eq!(names.len(), elements.len());
    }
    if let Some(target) = target {
        if names.is_none() && elements.iter().all(ListElement::is_skip) {
            return Ok(Sexp(target.inner()));
        }
    }

    let length = ffi::R_xlen_t::try_from(elements.len())
        .map_err(|_| api_other("R list length exceeds R_xlen_t::MAX"))?;
    let names_ptr = names.map_or(ptr::null(), <[StringData<'_>]>::as_ptr);

    unsafe {
        check_unwind(yaml12_materialize_list(
            target.map_or(ffi::R_NilValue, OwnedListSexp::inner),
            elements.as_ptr(),
            names_ptr,
            length,
        ))
        .map(Sexp)
    }
}

pub(crate) fn materialize_string_vector(values: &[StringData<'_>]) -> Fallible<Sexp> {
    let length = ffi::R_xlen_t::try_from(values.len())
        .map_err(|_| api_other("R vector length exceeds R_xlen_t::MAX"))?;
    unsafe { check_unwind(yaml12_materialize_string_vector(values.as_ptr(), length)).map(Sexp) }
}

fn string_parts(value: &str) -> Fallible<(*const c_char, i32, i32)> {
    if value.is_na() {
        return Ok((ptr::null(), 0, 1));
    }
    let value_len = i32::try_from(value.len())
        .map_err(|_| api_other("R strings cannot exceed i32::MAX bytes"))?;
    Ok((value.as_ptr().cast::<c_char>(), value_len, 0))
}

pub(crate) fn call1(handler: &FunctionSexp, arg: Sexp) -> Fallible<Sexp> {
    unsafe { check_unwind(yaml12_call1(handler.inner(), arg.0)).map(Sexp) }
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

pub(crate) fn has_attributes(value: &Sexp) -> bool {
    unsafe { yaml12_has_attributes(value.0) != 0 }
}

fn charsxp_to_str(charsxp: ffi::SEXP) -> Fallible<&'static str> {
    unsafe {
        let len = usize::try_from(ffi::Rf_xlength(charsxp))
            .map_err(|_| api_other("R string has an invalid length"))?;
        let bytes = slice::from_raw_parts(ffi::R_CHAR(charsxp).cast::<u8>(), len);

        match yaml12_charsxp_encoding(charsxp) {
            CHARSXP_ASCII => {
                // R records this flag only after checking every byte.
                return Ok(str::from_utf8_unchecked(bytes));
            }
            CHARSXP_UTF8 => {
                // R encoding marks are declarations, not validity guarantees.
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
