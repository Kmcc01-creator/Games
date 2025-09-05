use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, Ident, Lit, Path};

/// A minimal, typed pattern DSL for building match patterns in a structured way.
#[derive(Clone, Debug)]
pub enum PatternSpec {
    Literal(Lit),
    Wildcard,
    Ident(Ident),
    Path(Path),
    Struct { path: Path, fields: StructFields },
    Tuple { path: Path, elements: Vec<PatternSpec> },
    Or(Vec<PatternSpec>),
    Guarded { base: Box<PatternSpec>, guard: Expr },
}

#[derive(Clone, Debug, Default)]
pub struct StructFields {
    pub named: Vec<(Ident, PatternSpec)>,
    pub rest: bool,
}

impl PatternSpec {
    pub fn or(self, other: PatternSpec) -> PatternSpec {
        match self {
            PatternSpec::Or(mut v) => { v.push(other); PatternSpec::Or(v) }
            x => PatternSpec::Or(vec![x, other])
        }
    }

    pub fn with_guard(self, guard: Expr) -> PatternSpec {
        PatternSpec::Guarded { base: Box::new(self), guard }
    }

    pub fn into_tokens(self) -> TokenStream2 {
        match self {
            PatternSpec::Literal(l) => quote! { #l },
            PatternSpec::Wildcard => quote! { _ },
            PatternSpec::Ident(i) => quote! { #i },
            PatternSpec::Path(p) => quote! { #p },
            PatternSpec::Struct { path, fields } => {
                let has_named = !fields.named.is_empty();
                let named_items: Vec<_> = fields.named.into_iter().map(|(id, pat)| {
                    let ts = pat.into_tokens();
                    quote! { #id: #ts }
                }).collect();
                let named_ts = quote! { #( #named_items ),* };
                let rest_ts = if fields.rest { Some(quote! { .. }) } else { None };

                match (has_named, rest_ts) {
                    (true, Some(r)) => quote! { #path { #named_ts, #r } },
                    (true, None)    => quote! { #path { #named_ts } },
                    (false, Some(r))=> quote! { #path { #r } },
                    (false, None)   => quote! { #path { } },
                }
            }
            PatternSpec::Tuple { path, elements } => {
                let elems = elements.into_iter().map(|p| p.into_tokens());
                quote! { #path ( #( #elems ),* ) }
            }
            PatternSpec::Or(parts) => {
                let parts = parts.into_iter().map(|p| p.into_tokens());
                quote! { #( #parts )|* }
            }
            PatternSpec::Guarded { base, guard } => {
                let base_ts = base.into_tokens();
                quote! { #base_ts if #guard }
            }
        }
    }
}

#[cfg(all(test, feature = "pattern_dsl"))]
mod tests {
    use super::*;
    use quote::quote;

    fn parse_via_match_arm(ts: proc_macro2::TokenStream) -> syn::Pat {
        let wrapped = quote! { match __x { #ts => (), _ => () } };
        let expr: syn::Expr = syn::parse2(wrapped).expect("parse match expr");
        if let syn::Expr::Match(m) = expr {
            m.arms.into_iter().next().expect("one arm").pat
        } else {
            panic!("expected match expr");
        }
    }

    #[test]
    fn struct_named_only() {
        let path: syn::Path = syn::parse_quote!(Foo);
        let pat = PatternSpec::Struct {
            path,
            fields: StructFields {
                named: vec![
                    (syn::Ident::new("x", proc_macro2::Span::call_site()), PatternSpec::Ident(syn::Ident::new("a", proc_macro2::Span::call_site()))),
                    (syn::Ident::new("y", proc_macro2::Span::call_site()), PatternSpec::Wildcard),
                ],
                rest: false,
            },
        };
        let ts = pat.into_tokens();
        let parsed: syn::Pat = parse_via_match_arm(ts);
        match parsed {
            syn::Pat::Struct(ps) => {
                assert!(ps.fields.len() == 2);
                assert!(ps.rest.is_none());
            }
            _ => panic!("expected Pat::Struct"),
        }
    }

    #[test]
    fn struct_named_with_rest() {
        let path: syn::Path = syn::parse_quote!(Bar);
        let pat = PatternSpec::Struct {
            path,
            fields: StructFields { named: vec![], rest: true },
        };
        let ts = pat.into_tokens();
        let parsed: syn::Pat = parse_via_match_arm(ts);
        match parsed {
            syn::Pat::Struct(ps) => {
                assert!(ps.fields.is_empty());
                assert!(ps.rest.is_some());
            }
            _ => panic!("expected Pat::Struct"),
        }
    }

    #[test]
    fn tuple_and_or_and_guard() {
        let path: syn::Path = syn::parse_quote!(Baz);
        let base = PatternSpec::Tuple {
            path,
            elements: vec![PatternSpec::Wildcard, PatternSpec::Literal(syn::Lit::Int(syn::LitInt::new("1", proc_macro2::Span::call_site())))],
        };
        let ord = base.clone().or(PatternSpec::Path(syn::parse_quote!(BazDefault)));
        let guarded = ord.with_guard(syn::parse_quote!(true));
        let ts = guarded.into_tokens();
        // Ensure parses as a pattern via match arm context.
        let _parsed: syn::Pat = parse_via_match_arm(ts);
    }
}
