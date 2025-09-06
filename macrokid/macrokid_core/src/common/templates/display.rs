use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// Generate a `fmt` method body for Display using the provided body tokens.
/// Body should evaluate to `::core::fmt::Result`.
pub fn write_str(body: TokenStream2) -> TokenStream2 {
    quote! {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            #body
        }
    }
}

/// Convenience: write a literal string.
pub fn write_literal<S: AsRef<str>>(s: S) -> TokenStream2 {
    let s = s.as_ref();
    quote! { f.write_str(#s) }
}

