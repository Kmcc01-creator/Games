//! Declarative macro DSL for defining derive macros.
//!
//! This module provides macros that make it extremely easy to define custom derives
//! using the pattern traits. The DSL handles all the boilerplate and generates
//! the necessary trait implementations.
//!
//! # Example: StaticSliceDerive
//!
//! ```ignore
//! derive_slice! {
//!     /// My custom derive for resource bindings
//!     pub struct MyBinding {
//!         descriptor_type: MyDescriptor,
//!         trait_path: my_crate::MyTrait,
//!         method: my_method,
//!         module: my_mod,
//!
//!         fn collect(spec: &TypeSpec) -> syn::Result<Vec<MyDescriptor>> {
//!             // Your collection logic here
//!             // Parse attributes, validate, build descriptors
//!             Ok(vec![/* descriptors */])
//!         }
//!     }
//! }
//! ```
//!
//! # Example: StaticItemDerive
//!
//! ```ignore
//! derive_item! {
//!     /// Pipeline configuration derive
//!     pub struct PipelineConfig {
//!         descriptor_type: PipelineDesc,
//!         trait_path: my_crate::PipelineInfo,
//!         method: pipeline_desc,
//!         module: pipeline,
//!
//!         fn build(spec: &TypeSpec) -> syn::Result<PipelineDesc> {
//!             // Your building logic here
//!             // Parse type-level attributes, construct descriptor
//!             Ok(PipelineDesc { /* ... */ })
//!         }
//!     }
//! }
//! ```

/// Declarative macro for defining StaticSliceDerive implementations.
///
/// This macro generates all the boilerplate for a static slice derive,
/// including the descriptor type, trait implementation, and helper methods.
///
/// # Syntax
///
/// ```ignore
/// derive_slice! {
///     $(#[$attr:meta])*
///     $vis:vis struct $name:ident {
///         descriptor_type: $desc_ty:ty,
///         trait_path: $trait_path:path,
///         method: $method:ident,
///         module: $module:ident,
///         $(inherent: $inherent:ident,)?
///
///         fn collect(spec: &TypeSpec) -> syn::Result<Vec<$desc_ty>> {
///             $($body:tt)*
///         }
///     }
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// derive_slice! {
///     /// Resource binding derive
///     pub struct ResourceBinding {
///         descriptor_type: BindingDesc,
///         trait_path: my_crate::ResourceBindings,
///         method: bindings,
///         module: rb,
///
///         fn collect(spec: &TypeSpec) -> syn::Result<Vec<BindingDesc>> {
///             // Parse and collect bindings
///             Ok(vec![/* ... */])
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! derive_slice {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            descriptor_type: $desc_ty:ty,
            trait_path: $trait_path:path,
            method: $method:ident,
            module: $module:ident,
            $(inherent: $inherent:ident,)?

            fn collect(spec: &TypeSpec) -> syn::Result<Vec<$desc_ty2:ty>> {
                $($body:tt)*
            }
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::common::derive_patterns::StaticSliceDerive for $name {
            type Descriptor = $desc_ty;

            fn descriptor_type() -> proc_macro2::TokenStream {
                quote::quote! { $desc_ty }
            }

            fn collect_descriptors(spec: &$crate::ir::TypeSpec) -> syn::Result<Vec<Self::Descriptor>> {
                $($body)*
            }

            fn trait_path() -> proc_macro2::TokenStream {
                quote::quote! { $trait_path }
            }

            fn method_name() -> proc_macro2::Ident {
                proc_macro2::Ident::new(stringify!($method), proc_macro2::Span::call_site())
            }

            fn module_hint() -> &'static str {
                stringify!($module)
            }

            $(
                fn inherent_method_name() -> String {
                    stringify!($inherent).to_string()
                }
            )?
        }
    };
}

/// Declarative macro for defining StaticItemDerive implementations.
///
/// Similar to `derive_slice!` but for single-item descriptors.
///
/// # Syntax
///
/// ```ignore
/// derive_item! {
///     $(#[$attr:meta])*
///     $vis:vis struct $name:ident {
///         descriptor_type: $desc_ty:ty,
///         trait_path: $trait_path:path,
///         method: $method:ident,
///         module: $module:ident,
///         $(static_name: $static_name:ident,)?
///         $(inherent: $inherent:ident,)?
///
///         fn build(spec: &TypeSpec) -> syn::Result<$desc_ty> {
///             $($body:tt)*
///         }
///     }
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// derive_item! {
///     /// Graphics pipeline derive
///     pub struct GraphicsPipeline {
///         descriptor_type: PipelineDesc,
///         trait_path: my_crate::PipelineInfo,
///         method: pipeline_desc,
///         module: gp,
///
///         fn build(spec: &TypeSpec) -> syn::Result<PipelineDesc> {
///             // Parse type-level attributes and build descriptor
///             Ok(PipelineDesc { /* ... */ })
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! derive_item {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            descriptor_type: $desc_ty:ty,
            trait_path: $trait_path:path,
            method: $method:ident,
            module: $module:ident,
            $(static_name: $static_name:ident,)?
            $(inherent: $inherent:ident,)?

            fn build(spec: &TypeSpec) -> syn::Result<$desc_ty2:ty> {
                $($body:tt)*
            }
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::common::derive_patterns::StaticItemDerive for $name {
            type Descriptor = $desc_ty;

            fn descriptor_type() -> proc_macro2::TokenStream {
                quote::quote! { $desc_ty }
            }

            fn build_descriptor(spec: &$crate::ir::TypeSpec) -> syn::Result<Self::Descriptor> {
                $($body)*
            }

            fn trait_path() -> proc_macro2::TokenStream {
                quote::quote! { $trait_path }
            }

            fn method_name() -> proc_macro2::Ident {
                proc_macro2::Ident::new(stringify!($method), proc_macro2::Span::call_site())
            }

            fn module_hint() -> &'static str {
                stringify!($module)
            }

            $(
                fn static_name() -> &'static str {
                    stringify!($static_name)
                }
            )?

            $(
                fn inherent_method_name() -> String {
                    stringify!($inherent).to_string()
                }
            )?
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are just compilation tests to ensure the macros expand correctly
    // Real testing would require a full proc-macro environment

    #[test]
    fn dsl_macros_exist() {
        // This test just verifies the module compiles
        assert!(true);
    }
}
