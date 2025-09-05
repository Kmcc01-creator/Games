use proc_macro2::TokenStream as TokenStream2;
// use quote::quote; // not currently needed directly here

use crate::common::builders::MatchArmBuilder;
use quote::quote;
use crate::ir::{EnumSpec, FieldKind, FieldSpec, VariantSpec};

/// Build a MatchArmBuilder with one arm per enum variant.
/// The mapper returns a (pattern, body) pair per variant.
pub fn match_variants<F>(en: &EnumSpec, mut mapper: F) -> MatchArmBuilder
where
    F: FnMut(&VariantSpec) -> (TokenStream2, TokenStream2),
{
    let mut builder = MatchArmBuilder::new();
    for v in &en.variants {
        let (pat, body) = mapper(v);
        builder = builder.add_arm(pat, body);
    }
    builder
}

/// Build a MatchArmBuilder with arms derived from struct/tuple fields.
/// The mapper returns an optional (pattern, body) per field; `None` skips the field.
pub fn match_fields<F>(fields: &FieldKind, mut mapper: F) -> MatchArmBuilder
where
    F: FnMut(&FieldSpec) -> Option<(TokenStream2, TokenStream2)>,
{
    let mut builder = MatchArmBuilder::new();
    match fields {
        FieldKind::Named(named) | FieldKind::Unnamed(named) => {
            for f in named {
                if let Some((pat, body)) = mapper(f) {
                    builder = builder.add_arm(pat, body);
                }
            }
        }
        FieldKind::Unit => {
            // nothing to match
        }
    }
    builder
}

/// If the builder appears to have fewer arms than the expected number of variants,
/// append a wildcard arm that calls `unreachable!(note)` to ensure exhaustiveness
/// without constraining the expression type.
pub fn suggest_wildcard_if_non_exhaustive(
    mut builder: MatchArmBuilder,
    expected_variants: usize,
    note: &str,
) -> MatchArmBuilder {
    if builder.len() < expected_variants {
        builder = builder.add_wildcard(quote! { unreachable!(#note) });
    }
    builder
}
