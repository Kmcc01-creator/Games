use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

use crate::{common::builders::ImplBuilder, TypeSpec};

/// Convert DeriveInput to TypeSpec and run the provided closure.
pub fn with_type_spec<F>(input: DeriveInput, f: F) -> syn::Result<TokenStream2>
where
    F: FnOnce(TypeSpec) -> syn::Result<TokenStream2>,
{
    let spec = TypeSpec::from_derive_input(input)?;
    f(spec)
}

/// Create an ImplBuilder for a given trait path on the target represented by `spec`.
pub fn impl_for_trait(spec: &TypeSpec, trait_path: TokenStream2) -> ImplBuilder {
    ImplBuilder::new(spec.ident.clone(), spec.generics.clone()).implement_trait(trait_path)
}

/// Macro to generate a proc_macro_derive entrypoint with minimal boilerplate.
/// Must be invoked from a proc-macro crate.
#[macro_export]
macro_rules! derive_entry {
    (
        $name:ident,
        attrs = [ $( $attr:ident ),* $(,)? ],
        handler = $handler:path
    ) => {
        #[allow(non_snake_case)]
        #[proc_macro_derive($name, attributes( $( $attr ),* ))]
        pub fn $name(input: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
            let di: ::syn::DeriveInput = ::syn::parse_macro_input!(input as ::syn::DeriveInput);
            match $handler(di) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
    };
    (
        $name:ident,
        handler = $handler:path
    ) => {
        #[allow(non_snake_case)]
        #[proc_macro_derive($name)]
        pub fn $name(input: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
            let di: ::syn::DeriveInput = ::syn::parse_macro_input!(input as ::syn::DeriveInput);
            match $handler(di) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
    };
}
