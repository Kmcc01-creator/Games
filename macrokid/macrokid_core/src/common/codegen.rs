use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;

use crate::common::builders::ImplBuilder;
use crate::ir::TypeSpec;

/// Generate a private module with a single `DATA` static slice.
/// Returns (module_ident, module_tokens).
///
/// This is the canonical helper for emitting compile-time static slices from derives
/// and macro expansions. Use it to publish constant metadata (e.g., resource bindings)
/// without polluting the caller's namespace.
///
/// Example (emitting `&[Item]` and an inherent getter):
/// ```ignore
/// let ty = quote! { Item };
/// let (mod_ident, module) = codegen::static_slice_mod("items", ty.clone(), entries);
/// let inherent = codegen::impl_inherent_methods(&spec, &[quote! {
///     pub fn items() -> &'static [#ty] { #mod_ident::DATA }
/// }]);
/// ```
pub fn static_slice_mod(
    hint: &str,
    item_ty: TokenStream2,
    items: impl IntoIterator<Item = TokenStream2>,
) -> (Ident, TokenStream2) {
    let mod_ident = Ident::new(&format!("__mk_{hint}"), Span::call_site());
    let data_items: Vec<TokenStream2> = items.into_iter().collect();
    let module = quote! {
        #[allow(non_snake_case, non_upper_case_globals)]
        mod #mod_ident {
            pub static DATA: &[#item_ty] = &[ #( #data_items ),* ];
        }
    };
    (mod_ident, module)
}

/// Implement a trait method that returns the static slice in `mod_ident::DATA`.
///
/// This stitches the generated module from `static_slice_mod` into a trait impl,
/// keeping the caller surface small and consistent.
pub fn impl_trait_method_static_slice(
    spec: &TypeSpec,
    trait_path: TokenStream2,
    method_ident: Ident,
    item_ty: TokenStream2,
    mod_ident: Ident,
) -> TokenStream2 {
    ImplBuilder::new(spec.ident.clone(), spec.generics.clone())
        .implement_trait(trait_path)
        .add_method(quote! { fn #method_ident() -> &'static [#item_ty] { #mod_ident::DATA } })
        .build()
}

/// Implement inherent methods for a type.
///
/// Useful for adding small convenience accessors that expose generated data.
pub fn impl_inherent_methods(spec: &TypeSpec, methods: &[TokenStream2]) -> TokenStream2 {
    let mut b = ImplBuilder::new(spec.ident.clone(), spec.generics.clone());
    for m in methods { b = b.add_method(m.clone()); }
    b.build()
}
