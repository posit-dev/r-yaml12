use crate::r_ext;
use crate::{api_other, Fallible};
use saphyr::Tag;
use savvy::{FunctionSexp, ListSexp, NotAvailableValue, Sexp};
use std::collections::HashMap;
use std::mem;

const HASHMAP_MIN_LEN: usize = 8;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct HandlerKey<'a> {
    handle: &'a str,
    suffix: &'a str,
}

impl HandlerKey<'_> {
    fn matches(&self, key: TagKeyRef<'_>) -> bool {
        self.handle == key.handle && self.suffix == key.suffix
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct TagKeyRef<'a> {
    handle: &'a str,
    suffix: &'a str,
}

impl<'a> From<&'a Tag> for TagKeyRef<'a> {
    fn from(tag: &'a Tag) -> Self {
        Self {
            handle: tag.handle.as_str(),
            suffix: tag.suffix.as_str(),
        }
    }
}

struct HandlerEntry<'a> {
    key: HandlerKey<'a>,
    handler: FunctionSexp,
}

enum HandlerStore<'a> {
    Small(Vec<HandlerEntry<'a>>),
    Large(HashMap<HandlerKey<'a>, FunctionSexp>),
}

pub(crate) struct HandlerRegistry<'a> {
    store: HandlerStore<'a>,
}

impl<'a> HandlerRegistry<'a> {
    pub(crate) fn from_robj(handlers: &'a Sexp) -> Fallible<Option<Self>> {
        if handlers.is_null() {
            return Ok(None);
        }

        let list = ListSexp::try_from(Sexp(handlers.0))
            .map_err(|_| api_other("`handlers` must be a named list of functions"))?;

        if list.is_empty() {
            return Ok(None);
        }

        let Some(names_attr) = r_ext::names(handlers)? else {
            return Err(api_other("`handlers` must be a named list of functions"));
        };

        let len = list.len();
        let use_hash_map = len >= HASHMAP_MIN_LEN;

        if use_hash_map {
            let mut handlers_map = HashMap::with_capacity(len);
            for i in 0..len {
                let name = r_ext::string_elt(&names_attr, i)?;
                let value = unsafe { list.get_by_index_unchecked(i) };
                let entry = handler_entry_from_parts(name, &value)?;
                if handlers_map.insert(entry.key, entry.handler).is_some() {
                    return Err(api_other(format!(
                        "Duplicate handler `{name}`; handler names must be unique"
                    )));
                }
            }
            return Ok(Some(Self {
                store: HandlerStore::Large(handlers_map),
            }));
        }

        let mut entries: Vec<HandlerEntry<'a>> = Vec::with_capacity(len);
        for i in 0..len {
            let name = r_ext::string_elt(&names_attr, i)?;
            let value = unsafe { list.get_by_index_unchecked(i) };
            let entry = handler_entry_from_parts(name, &value)?;
            if entries.iter().any(|existing| existing.key == entry.key) {
                return Err(api_other(format!(
                    "Duplicate handler `{name}`; handler names must be unique"
                )));
            }
            entries.push(entry);
        }

        Ok(Some(Self {
            store: HandlerStore::Small(entries),
        }))
    }

    pub(crate) fn get_for_tag(&self, tag: &Tag) -> Option<&FunctionSexp> {
        let key_ref = TagKeyRef::from(tag);
        match &self.store {
            HandlerStore::Small(entries) => entries
                .iter()
                .find(|entry| entry.key.matches(key_ref))
                .map(|entry| &entry.handler),
            HandlerStore::Large(map) => {
                let lookup_key = HandlerKey {
                    handle: key_ref.handle,
                    suffix: key_ref.suffix,
                };
                // HashMap::get does not store the borrowed lookup key.
                let lookup_key: &HandlerKey<'a> = unsafe { mem::transmute(&lookup_key) };
                map.get(lookup_key)
            }
        }
    }

    pub(crate) fn apply(&self, handler: &FunctionSexp, arg: Sexp) -> Fallible<Sexp> {
        r_ext::call1(handler, arg)
    }
}

fn handler_entry_from_parts<'a>(name: &'a str, value: &Sexp) -> Fallible<HandlerEntry<'a>> {
    if name.is_na() || name.is_empty() {
        return Err(api_other("`handlers` must be a named list of functions"));
    }
    let key = parse_handler_name(name)?;
    let handler = FunctionSexp::try_from(Sexp(value.0)).map_err(|_| {
        api_other(format!(
            "Handler `{name}` must be a function (closure or primitive)"
        ))
    })?;
    Ok(HandlerEntry { key, handler })
}

fn parse_handler_name<'a>(name: &'a str) -> Fallible<HandlerKey<'a>> {
    if let Some((handle, suffix)) = split_tag_name(name) {
        Ok(HandlerKey { handle, suffix })
    } else {
        Err(api_other("`handlers` names must be valid YAML tag strings"))
    }
}

fn split_tag_name(name: &str) -> Option<(&str, &str)> {
    if let Some(pos) = name.rfind('!') {
        if pos + 1 < name.len() {
            let (handle, suffix) = name.split_at(pos + 1);
            return Some((handle, suffix));
        }
    }
    if let Some(pos) = name.rfind(':') {
        if pos + 1 < name.len() {
            let (handle, suffix) = name.split_at(pos + 1);
            return Some((handle, suffix));
        }
    }
    None
}
