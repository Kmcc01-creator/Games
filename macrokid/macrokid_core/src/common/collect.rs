use std::collections::HashSet;
use std::hash::Hash;

use crate::ir::{FieldKind, FieldSpec, StructSpec};

/// Process only named fields of a struct, applying the mapper.
/// - mapper returns Ok(Some(T)) to include an item, Ok(None) to skip the field.
pub fn from_named_fields<T, F>(st: &StructSpec, mut mapper: F) -> syn::Result<Vec<T>>
where
    F: FnMut(&FieldSpec) -> syn::Result<Option<T>>,
{
    match st.fields() {
        FieldKind::Named(fields) => {
            let mut out = Vec::new();
            for f in fields {
                if let Some(t) = mapper(f)? { out.push(t); }
            }
            Ok(out)
        }
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected a struct with named fields",
        )),
    }
}

/// Ensure uniqueness by key; returns error with provided message if duplicate is found.
pub fn unique_by<T, K, KF>(items: Vec<T>, mut keyf: KF, msg: &str) -> syn::Result<Vec<T>>
where
    K: Eq + Hash,
    KF: FnMut(&T) -> (K, proc_macro2::Span),
{
    let mut seen: HashSet<K> = HashSet::new();
    for item in &items {
        let (k, span) = keyf(item);
        if !seen.insert(k) {
            return Err(syn::Error::new(span, msg));
        }
    }
    Ok(items)
}

/// Require the collection to be non-empty; returns provided message otherwise.
pub fn require_non_empty<T>(items: Vec<T>, msg: &str) -> syn::Result<Vec<T>> {
    if items.is_empty() {
        return Err(syn::Error::new(proc_macro2::Span::call_site(), msg));
    }
    Ok(items)
}
