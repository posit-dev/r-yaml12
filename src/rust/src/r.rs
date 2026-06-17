use crate::unwind::{run_with_unwind_protect, run_with_unwind_value, EvalError};
use crate::{api_other, Fallible, R_STRING_MAX_BYTES};
use savvy_ffi as ffi;
use std::ffi::CString;
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::c_char;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Rtype {
    Null,
    Logicals,
    Integers,
    Doubles,
    Complexes,
    Strings,
    List,
    Function,
    Other(ffi::SEXPTYPE),
}

#[derive(Copy, Clone)]
pub(crate) struct Robj {
    sexp: ffi::SEXP,
}

impl Robj {
    #[inline]
    pub(crate) unsafe fn from_sexp(sexp: ffi::SEXP) -> Self {
        Self { sexp }
    }

    #[inline]
    pub(crate) fn get(&self) -> ffi::SEXP {
        self.sexp
    }

    #[inline]
    pub(crate) fn is_null(&self) -> bool {
        self.sexp == unsafe { ffi::R_NilValue }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        unsafe { ffi::Rf_xlength(self.sexp) as usize }
    }

    pub(crate) fn rtype(&self) -> Rtype {
        match unsafe { ffi::TYPEOF(self.sexp) } {
            ffi::NILSXP => Rtype::Null,
            ffi::LGLSXP => Rtype::Logicals,
            ffi::INTSXP => Rtype::Integers,
            ffi::REALSXP => Rtype::Doubles,
            ffi::CPLXSXP => Rtype::Complexes,
            ffi::STRSXP => Rtype::Strings,
            ffi::VECSXP => Rtype::List,
            ffi::CLOSXP | ffi::BUILTINSXP | ffi::SPECIALSXP => Rtype::Function,
            other => Rtype::Other(other),
        }
    }

    pub(crate) fn as_bool_scalar(&self) -> Option<bool> {
        if self.rtype() != Rtype::Logicals || self.len() != 1 {
            return None;
        }
        let value = unsafe { ffi::LOGICAL_ELT(self.sexp, 0) };
        if value == r_na_int() {
            None
        } else {
            Some(value != 0)
        }
    }

    pub(crate) fn as_str(&self) -> Option<&'static str> {
        if self.rtype() != Rtype::Strings || self.len() != 1 {
            return None;
        }
        let value = unsafe { ffi::STRING_ELT(self.sexp, 0) };
        if value == unsafe { ffi::R_NaString } {
            None
        } else {
            Some(charsxp_to_str(value))
        }
    }

    pub(crate) fn as_str_iter(&self) -> Option<StringIter<'_>> {
        (self.rtype() == Rtype::Strings).then_some(StringIter::new(self.sexp))
    }

    pub(crate) fn as_logical_slice(&self) -> Option<&[i32]> {
        if self.rtype() != Rtype::Logicals {
            return None;
        }
        let len = self.len();
        if len == 0 {
            return Some(&[]);
        }
        Some(unsafe { std::slice::from_raw_parts(ffi::LOGICAL(self.sexp), len) })
    }

    pub(crate) fn as_integer_slice(&self) -> Option<&[i32]> {
        if self.rtype() != Rtype::Integers {
            return None;
        }
        let len = self.len();
        if len == 0 {
            return Some(&[]);
        }
        Some(unsafe { std::slice::from_raw_parts(ffi::INTEGER(self.sexp), len) })
    }

    pub(crate) fn as_real_slice(&self) -> Option<&[f64]> {
        if self.rtype() != Rtype::Doubles {
            return None;
        }
        let len = self.len();
        if len == 0 {
            return Some(&[]);
        }
        Some(unsafe { std::slice::from_raw_parts(ffi::REAL(self.sexp), len) })
    }

    pub(crate) fn as_list(&self) -> Option<List> {
        (self.rtype() == Rtype::List).then_some(List::borrowed(self.sexp))
    }

    pub(crate) fn as_function(&self) -> Option<Function> {
        (self.rtype() == Rtype::Function).then_some(Function { sexp: self.sexp })
    }

    pub(crate) fn get_attrib_sym(&self, attr: ffi::SEXP) -> Option<Robj> {
        let value = unsafe { ffi::Rf_getAttrib(self.sexp, attr) };
        (value != unsafe { ffi::R_NilValue }).then_some(unsafe { Robj::from_sexp(value) })
    }

    pub(crate) fn get_attrib_str(&self, attr: &str) -> Fallible<Option<Robj>> {
        let attr = install(attr)?;
        Ok(self.get_attrib_sym(attr))
    }

    pub(crate) fn set_attrib_sym(&mut self, attr: ffi::SEXP, value: Robj) -> Fallible<()> {
        let _self_guard = Protect::new(self.sexp);
        let _value_guard = Protect::new(value.sexp);
        run_with_unwind_value(|| unsafe { ffi::Rf_setAttrib(self.sexp, attr, value.sexp) })
            .map_err(EvalError::Jump)?;
        Ok(())
    }

    pub(crate) fn set_attrib_str(&mut self, attr: &str, value: Robj) -> Fallible<()> {
        let _self_guard = Protect::new(self.sexp);
        let _value_guard = Protect::new(value.sexp);
        let attr = install(attr)?;
        run_with_unwind_value(|| unsafe { ffi::Rf_setAttrib(self.sexp, attr, value.sexp) })
            .map_err(EvalError::Jump)?;
        Ok(())
    }

    pub(crate) fn set_class(&mut self, classes: &[&str]) -> Fallible<()> {
        let class = string_vector_from_iter(classes.iter().copied())?;
        self.set_attrib_sym(unsafe { ffi::R_ClassSymbol }, class)
    }

    pub(crate) fn class_iter(&self) -> Option<StringIter<'_>> {
        let class = self.get_attrib_sym(unsafe { ffi::R_ClassSymbol })?;
        (class.rtype() == Rtype::Strings).then_some(StringIter::new(class.sexp))
    }

    pub(crate) fn inherits(&self, class: &str) -> bool {
        self.class_iter()
            .map(|mut iter| iter.any(|value| !value.is_na() && value.as_ref() == class))
            .unwrap_or(false)
    }

    pub(crate) fn has_attributes(&self) -> bool {
        unsafe { ffi::ATTRIB(self.sexp) != ffi::R_NilValue }
    }
}

impl fmt::Display for Rtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rtype::Null => f.write_str("Null"),
            Rtype::Logicals => f.write_str("Logicals"),
            Rtype::Integers => f.write_str("Integers"),
            Rtype::Doubles => f.write_str("Doubles"),
            Rtype::Complexes => f.write_str("Complexes"),
            Rtype::Strings => f.write_str("Strings"),
            Rtype::List => f.write_str("List"),
            Rtype::Function => f.write_str("Function"),
            Rtype::Other(_) => f.write_str("Other"),
        }
    }
}

pub(crate) struct Protect;

impl Protect {
    #[inline]
    pub(crate) fn new(sexp: ffi::SEXP) -> Self {
        unsafe {
            ffi::Rf_protect(sexp);
        }
        Self
    }
}

impl Drop for Protect {
    fn drop(&mut self) {
        unsafe {
            ffi::Rf_unprotect(1);
        }
    }
}

pub(crate) struct ProtectedRobj {
    value: Robj,
    _guard: Protect,
}

impl ProtectedRobj {
    pub(crate) fn new(value: Robj) -> Self {
        let _guard = Protect::new(value.get());
        Self { value, _guard }
    }

    #[inline]
    pub(crate) fn value(&self) -> Robj {
        self.value
    }
}

pub(crate) struct RNull;
pub(crate) const NULL: RNull = RNull;

impl From<RNull> for Robj {
    fn from(_: RNull) -> Self {
        unsafe { Robj::from_sexp(ffi::R_NilValue) }
    }
}

pub(crate) fn r_na_int() -> i32 {
    unsafe { ffi::R_NaInt }
}

pub(crate) fn r_na_real() -> f64 {
    unsafe { ffi::R_NaReal }
}

pub(crate) fn alloc_vector(kind: ffi::SEXPTYPE, len: usize) -> Fallible<Robj> {
    let sexp = run_with_unwind_value(|| unsafe { ffi::Rf_allocVector(kind, len as _) })
        .map_err(EvalError::Jump)?;
    Ok(unsafe { Robj::from_sexp(sexp) })
}

pub(crate) fn logical_scalar(value: bool) -> Fallible<Robj> {
    let out = alloc_vector(ffi::LGLSXP, 1)?;
    unsafe {
        *ffi::LOGICAL(out.get()) = i32::from(value);
    }
    Ok(out)
}

pub(crate) fn integer_scalar(value: i32) -> Fallible<Robj> {
    let out = alloc_vector(ffi::INTSXP, 1)?;
    unsafe {
        *ffi::INTEGER(out.get()) = value;
    }
    Ok(out)
}

pub(crate) fn real_scalar(value: f64) -> Fallible<Robj> {
    let out = alloc_vector(ffi::REALSXP, 1)?;
    unsafe {
        *ffi::REAL(out.get()) = value;
    }
    Ok(out)
}

pub(crate) fn string_scalar(value: &str) -> Fallible<Robj> {
    let out = alloc_vector(ffi::STRSXP, 1)?;
    let _guard = Protect::new(out.get());
    let charsxp = mk_char(value)?;
    unsafe {
        ffi::SET_STRING_ELT(out.get(), 0, charsxp);
    }
    Ok(out)
}

pub(crate) fn logical_vector_from_iter<I>(len: usize, iter: I) -> Fallible<Robj>
where
    I: IntoIterator<Item = i32>,
{
    let out = alloc_vector(ffi::LGLSXP, len)?;
    let dst = unsafe { std::slice::from_raw_parts_mut(ffi::LOGICAL(out.get()), len) };
    for (slot, value) in dst.iter_mut().zip(iter) {
        *slot = value;
    }
    Ok(out)
}

pub(crate) fn integer_vector_from_iter<I>(len: usize, iter: I) -> Fallible<Robj>
where
    I: IntoIterator<Item = i32>,
{
    let out = alloc_vector(ffi::INTSXP, len)?;
    let dst = unsafe { std::slice::from_raw_parts_mut(ffi::INTEGER(out.get()), len) };
    for (slot, value) in dst.iter_mut().zip(iter) {
        *slot = value;
    }
    Ok(out)
}

pub(crate) fn real_vector_from_iter<I>(len: usize, iter: I) -> Fallible<Robj>
where
    I: IntoIterator<Item = f64>,
{
    let out = alloc_vector(ffi::REALSXP, len)?;
    let dst = unsafe { std::slice::from_raw_parts_mut(ffi::REAL(out.get()), len) };
    for (slot, value) in dst.iter_mut().zip(iter) {
        *slot = value;
    }
    Ok(out)
}

pub(crate) fn string_vector_from_iter<'a, I>(iter: I) -> Fallible<Robj>
where
    I: IntoIterator<Item = &'a str>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = iter.into_iter();
    let out = alloc_vector(ffi::STRSXP, iter.len())?;
    let _guard = Protect::new(out.get());
    for (i, value) in iter.enumerate() {
        let charsxp = mk_char(value)?;
        unsafe {
            ffi::SET_STRING_ELT(out.get(), i as _, charsxp);
        }
    }
    Ok(out)
}

pub(crate) fn string_vector_from_options<'a, I>(len: usize, iter: I) -> Fallible<Robj>
where
    I: IntoIterator<Item = Option<&'a str>>,
{
    let out = alloc_vector(ffi::STRSXP, len)?;
    let _guard = Protect::new(out.get());
    for (i, value) in iter.into_iter().enumerate() {
        let charsxp = if let Some(value) = value {
            mk_char(value)?
        } else {
            unsafe { ffi::R_NaString }
        };
        unsafe {
            ffi::SET_STRING_ELT(out.get(), i as _, charsxp);
        }
    }
    Ok(out)
}

pub(crate) struct List {
    sexp: ffi::SEXP,
    names: Option<ffi::SEXP>,
    _guard: Option<Protect>,
}

impl List {
    fn borrowed(sexp: ffi::SEXP) -> Self {
        Self {
            sexp,
            names: None,
            _guard: None,
        }
    }

    pub(crate) fn new(len: usize, named: bool) -> Fallible<Self> {
        let obj = alloc_vector(ffi::VECSXP, len)?;
        let guard = Protect::new(obj.get());
        let names = if named {
            let names = alloc_vector(ffi::STRSXP, len)?;
            let _names_guard = Protect::new(names.get());
            run_with_unwind_value(|| unsafe {
                ffi::Rf_setAttrib(obj.get(), ffi::R_NamesSymbol, names.get())
            })
            .map_err(EvalError::Jump)?;
            Some(names.get())
        } else {
            None
        };

        Ok(Self {
            sexp: obj.get(),
            names,
            _guard: Some(guard),
        })
    }

    pub(crate) fn len(&self) -> usize {
        unsafe { ffi::Rf_xlength(self.sexp) as usize }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn values(&self) -> ListValueIter<'_> {
        ListValueIter {
            sexp: self.sexp,
            i: 0,
            len: self.len(),
            _owner: PhantomData,
        }
    }

    pub(crate) fn names(&self) -> Option<StringIter<'_>> {
        let names = unsafe { ffi::Rf_getAttrib(self.sexp, ffi::R_NamesSymbol) };
        (names != unsafe { ffi::R_NilValue }).then_some(StringIter::new(names))
    }

    pub(crate) fn set_value(&mut self, i: usize, value: Robj) -> Fallible<()> {
        if i >= self.len() {
            return Err(api_other("Internal error: list index out of bounds"));
        }
        unsafe {
            ffi::SET_VECTOR_ELT(self.sexp, i as _, value.get());
        }
        Ok(())
    }

    pub(crate) fn set_name(&mut self, i: usize, name: &str) -> Fallible<()> {
        let Some(names) = self.names else {
            return Err(api_other("Internal error: list has no names vector"));
        };
        if i >= self.len() {
            return Err(api_other("Internal error: list name index out of bounds"));
        }
        let charsxp = mk_char(name)?;
        unsafe {
            ffi::SET_STRING_ELT(names, i as _, charsxp);
        }
        Ok(())
    }

    pub(crate) fn set_attrib_sym(&mut self, attr: ffi::SEXP, value: Robj) -> Fallible<()> {
        let mut obj = unsafe { Robj::from_sexp(self.sexp) };
        obj.set_attrib_sym(attr, value)
    }

    pub(crate) fn into_robj(self) -> Robj {
        unsafe { Robj::from_sexp(self.sexp) }
    }
}

pub(crate) struct ListValueIter<'a> {
    sexp: ffi::SEXP,
    i: usize,
    len: usize,
    _owner: PhantomData<&'a List>,
}

impl Iterator for ListValueIter<'_> {
    type Item = Robj;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.len {
            return None;
        }
        let i = self.i;
        self.i += 1;
        Some(unsafe { Robj::from_sexp(ffi::VECTOR_ELT(self.sexp, i as _)) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.i;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ListValueIter<'_> {}

impl TryFrom<Robj> for List {
    type Error = EvalError;

    fn try_from(value: Robj) -> Fallible<Self> {
        if value.rtype() == Rtype::List {
            Ok(Self::borrowed(value.get()))
        } else {
            Err(api_other("Expected a list"))
        }
    }
}

pub(crate) struct Strings {
    sexp: ffi::SEXP,
}

impl Strings {
    pub(crate) fn len(&self) -> usize {
        unsafe { ffi::Rf_xlength(self.sexp) as usize }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn elt(&self, i: usize) -> Rstr {
        debug_assert!(i < self.len());
        unsafe { Rstr::from_charsxp(ffi::STRING_ELT(self.sexp, i as _)) }
    }

    pub(crate) fn iter(&self) -> StringIter<'_> {
        StringIter::new(self.sexp)
    }
}

impl TryFrom<Robj> for Strings {
    type Error = EvalError;

    fn try_from(value: Robj) -> Fallible<Self> {
        if value.rtype() == Rtype::Strings {
            Ok(Self { sexp: value.get() })
        } else {
            Err(api_other(format!("Expected Strings got {}", value.rtype())))
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Rstr {
    charsxp: ffi::SEXP,
}

impl Rstr {
    #[inline]
    unsafe fn from_charsxp(charsxp: ffi::SEXP) -> Self {
        Self { charsxp }
    }

    #[inline]
    pub(crate) fn is_na(&self) -> bool {
        self.charsxp == unsafe { ffi::R_NaString }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        !self.is_na() && self.as_str().is_empty()
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        if self.is_na() {
            "NA"
        } else {
            charsxp_to_str(self.charsxp)
        }
    }
}

impl AsRef<str> for Rstr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

pub(crate) struct StringIter<'a> {
    sexp: ffi::SEXP,
    i: usize,
    len: usize,
    _owner: PhantomData<&'a ()>,
}

impl<'a> StringIter<'a> {
    fn new(sexp: ffi::SEXP) -> Self {
        Self {
            sexp,
            i: 0,
            len: unsafe { ffi::Rf_xlength(sexp) as usize },
            _owner: PhantomData,
        }
    }
}

impl Iterator for StringIter<'_> {
    type Item = Rstr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.len {
            return None;
        }
        let i = self.i;
        self.i += 1;
        Some(unsafe { Rstr::from_charsxp(ffi::STRING_ELT(self.sexp, i as _)) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.i;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StringIter<'_> {}

#[derive(Copy, Clone)]
pub(crate) struct Function {
    sexp: ffi::SEXP,
}

impl Function {
    #[inline]
    pub(crate) fn get(&self) -> ffi::SEXP {
        self.sexp
    }

    pub(crate) fn environment(&self) -> Option<Robj> {
        if unsafe { ffi::TYPEOF(self.sexp) } == ffi::CLOSXP {
            Some(unsafe { Robj::from_sexp(yaml12_closure_env(self.sexp)) })
        } else {
            None
        }
    }
}

pub(crate) fn global_env() -> Robj {
    unsafe { Robj::from_sexp(ffi::R_GlobalEnv) }
}

pub(crate) fn sym_yaml_keys() -> ffi::SEXP {
    unsafe { ffi::Rf_install(b"yaml_keys\0".as_ptr() as *const c_char) }
}

pub(crate) fn sym_yaml_tag() -> ffi::SEXP {
    unsafe { ffi::Rf_install(b"yaml_tag\0".as_ptr() as *const c_char) }
}

pub(crate) fn rprintln(message: &str) -> Fallible<()> {
    let line = CString::new(format!("{message}\n"))
        .map_err(|_| api_other("R console output contains interior nul byte"))?;
    run_with_unwind_protect(|| unsafe {
        ffi::Rprintf(b"%s\0".as_ptr() as *const c_char, line.as_ptr());
    })
    .map_err(EvalError::Jump)
}

fn install(name: &str) -> Fallible<ffi::SEXP> {
    let name =
        CString::new(name).map_err(|_| api_other("Attribute name contains interior nul byte"))?;
    run_with_unwind_value(|| unsafe { ffi::Rf_install(name.as_ptr()) }).map_err(EvalError::Jump)
}

fn mk_char(value: &str) -> Fallible<ffi::SEXP> {
    if value.len() > R_STRING_MAX_BYTES {
        return Err(api_other(
            "R character value exceeds R's 2^31-1 byte string limit",
        ));
    }
    run_with_unwind_value(|| unsafe {
        ffi::Rf_mkCharLenCE(
            value.as_ptr() as *const c_char,
            value.len() as i32,
            ffi::cetype_t_CE_UTF8,
        )
    })
    .map_err(EvalError::Jump)
}

fn charsxp_to_str(charsxp: ffi::SEXP) -> &'static str {
    unsafe {
        let ptr = ffi::R_CHAR(charsxp) as *const u8;
        let len = ffi::Rf_xlength(charsxp) as usize;
        let bytes = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8_unchecked(bytes)
    }
}

extern "C" {
    fn yaml12_closure_env(fun: ffi::SEXP) -> ffi::SEXP;
}
