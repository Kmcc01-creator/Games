use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;

/// Debug implementation helper for struct-like output with named fields.
/// `ty_name` is the type identifier, `fields` is an iterator of (field_name, expr) pairs.
pub fn debug_struct(
    ty_name: &Ident,
    fields: impl IntoIterator<Item = (String, TokenStream2)>,
) -> TokenStream2 {
    let name = ty_name.to_string();
    let mut parts: Vec<TokenStream2> = Vec::new();
    for (fname, expr) in fields.into_iter() {
        parts.push(quote! { .field(#fname, &#expr) });
    }
    quote! {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            let mut ds = f.debug_struct(#name);
            ds #( #parts )* .finish()
        }
    }
}

/// Debug implementation helper for tuple-like output with positional fields.
pub fn debug_tuple(
    ty_name: &Ident,
    elems: impl IntoIterator<Item = TokenStream2>,
) -> TokenStream2 {
    let name = ty_name.to_string();
    let parts: Vec<TokenStream2> = elems.into_iter().map(|e| quote! { .field(&#e) }).collect();
    quote! {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            let mut dt = f.debug_tuple(#name);
            dt #( #parts )* .finish()
        }
    }
}

