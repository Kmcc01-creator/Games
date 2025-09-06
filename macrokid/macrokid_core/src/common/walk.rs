use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;

use crate::ir::{EnumSpec, FieldKind, FieldSpec, StructSpec, VariantSpec};

#[derive(Debug, Clone)]
pub struct Bindings {
    pub pattern: TokenStream2,
    pub idents: Vec<Ident>,
    pub fields: Vec<FieldSpec>,
}

fn make_ident(prefix: &str, idx: usize, span: Span) -> Ident {
    syn::Ident::new(&format!("__{prefix}{idx}"), span)
}

/// Create a pattern and binding idents for a variant. Pattern is suitable
/// for use in a match arm within an impl (uses `Self::Variant`).
pub fn bind_variant(variant: &VariantSpec) -> Bindings {
    let vname = &variant.ident;
    match &variant.fields {
        FieldKind::Named(named) => {
            let mut pats = Vec::with_capacity(named.len());
            let mut idents = Vec::with_capacity(named.len());
            for (i, f) in named.iter().enumerate() {
                let fname = f.ident.as_ref().expect("named field");
                let b = make_ident("v", i, f.span);
                pats.push(quote! { #fname: #b });
                idents.push(b);
            }
            let pattern = quote! { Self::#vname { #( #pats ),* } };
            Bindings { pattern, idents, fields: named.clone() }
        }
        FieldKind::Unnamed(unnamed) => {
            let mut binders = Vec::with_capacity(unnamed.len());
            let mut idents = Vec::with_capacity(unnamed.len());
            for (i, f) in unnamed.iter().enumerate() {
                let b = make_ident("v", i, f.span);
                binders.push(quote! { #b });
                idents.push(b);
            }
            let pattern = quote! { Self::#vname( #( #binders ),* ) };
            Bindings { pattern, idents, fields: unnamed.clone() }
        }
        FieldKind::Unit => {
            let pattern = quote! { Self::#vname };
            Bindings { pattern, idents: Vec::new(), fields: Vec::new() }
        }
    }
}

/// Create a pattern and binding idents for a struct. Pattern is suitable
/// for use in a match arm within an impl (uses `Self { .. }` or `Self(..)`).
pub fn bind_struct(st: &StructSpec, ty_span: Span) -> Bindings {
    match &st.fields {
        FieldKind::Named(named) => {
            let mut pats = Vec::with_capacity(named.len());
            let mut idents = Vec::with_capacity(named.len());
            for (i, f) in named.iter().enumerate() {
                let fname = f.ident.as_ref().expect("named field");
                let b = make_ident("s", i, f.span);
                pats.push(quote! { #fname: #b });
                idents.push(b);
            }
            let pattern = quote! { Self { #( #pats ),* } };
            Bindings { pattern, idents, fields: named.clone() }
        }
        FieldKind::Unnamed(unnamed) => {
            let mut binders = Vec::with_capacity(unnamed.len());
            let mut idents = Vec::with_capacity(unnamed.len());
            for (i, f) in unnamed.iter().enumerate() {
                let b = make_ident("s", i, f.span);
                binders.push(quote! { #b });
                idents.push(b);
            }
            let pattern = quote! { Self( #( #binders ),* ) };
            Bindings { pattern, idents, fields: unnamed.clone() }
        }
        FieldKind::Unit => {
            let _ = ty_span; // no-op; pattern does not need it
            let pattern = quote! { Self };
            Bindings { pattern, idents: Vec::new(), fields: Vec::new() }
        }
    }
}

/// Lightweight context wrapper for fields.
pub struct FieldCtx<'a> {
    pub field: &'a FieldSpec,
}

impl<'a> FieldCtx<'a> {
    pub fn ident(&self) -> Option<&'a syn::Ident> { self.field.ident.as_ref() }
    pub fn index(&self) -> usize { self.field.index }
    pub fn attrs(&self) -> &'a [syn::Attribute] { &self.field.attrs }
    pub fn span(&self) -> Span { self.field.span }
    pub fn ty(&self) -> &'a syn::Type { &self.field.ty }
    pub fn name_string(&self) -> String {
        self.field.ident.as_ref().map(|i| i.to_string()).unwrap_or_else(|| format!("_{}", self.field.index))
    }
}

/// Lightweight context wrapper for variants.
pub struct VariantCtx<'a> { pub variant: &'a VariantSpec }
impl<'a> VariantCtx<'a> {
    pub fn ident(&self) -> &'a syn::Ident { &self.variant.ident }
    pub fn attrs(&self) -> &'a [syn::Attribute] { &self.variant.attrs }
    pub fn span(&self) -> Span { self.variant.span }
    pub fn fields(&self) -> &'a FieldKind { &self.variant.fields }
}

/// Process only named fields and map to items.
pub fn process_named_fields<T, F>(st: &StructSpec, mut f: F) -> syn::Result<Vec<T>>
where
    F: FnMut(FieldCtx) -> syn::Result<T>,
{
    match st.fields() {
        FieldKind::Named(fields) => {
            let mut out = Vec::new();
            for field in fields {
                out.push(f(FieldCtx { field })?);
            }
            Ok(out)
        }
        _ => Err(syn::Error::new(proc_macro2::Span::call_site(), "expected a struct with named fields")),
    }
}

/// Map enum variants to items.
pub fn process_variants<T, F>(en: &EnumSpec, mut f: F) -> syn::Result<Vec<T>>
where
    F: FnMut(VariantCtx) -> syn::Result<T>,
{
    let mut out = Vec::new();
    for v in &en.variants {
        out.push(f(VariantCtx { variant: v })?);
    }
    Ok(out)
}

