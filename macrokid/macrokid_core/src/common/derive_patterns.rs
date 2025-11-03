//! Common patterns for derive macro implementations.
//!
//! This module provides reusable trait-based patterns that extract common boilerplate
//! from derive macros, particularly those that generate static metadata.
//!
//! # Patterns
//!
//! ## StaticSliceDerive
//!
//! For derives that collect descriptors from fields and emit them as a static slice.
//! Common in resource binding derives where each field contributes a descriptor.
//!
//! **Example: ResourceBinding pattern**
//!
//! ```ignore
//! struct RBDescriptor {
//!     field: String,
//!     set: u32,
//!     binding: u32,
//!     kind: ResourceKind,
//! }
//!
//! impl quote::ToTokens for RBDescriptor { /* ... */ }
//!
//! struct ResourceBindingDerive;
//!
//! impl StaticSliceDerive for ResourceBindingDerive {
//!     type Descriptor = RBDescriptor;
//!
//!     fn descriptor_type() -> TokenStream2 {
//!         quote! { macrokid_graphics::resources::BindingDesc }
//!     }
//!
//!     fn collect_descriptors(spec: &TypeSpec) -> syn::Result<Vec<Self::Descriptor>> {
//!         // Parse field attributes, validate, build descriptors
//!         // ...domain-specific logic only...
//!     }
//!
//!     fn trait_path() -> TokenStream2 {
//!         quote! { macrokid_graphics::resources::ResourceBindings }
//!     }
//!
//!     fn method_name() -> Ident {
//!         Ident::new("bindings", Span::call_site())
//!     }
//!
//!     fn module_hint() -> &'static str {
//!         "rb"
//!     }
//! }
//!
//! // Usage in proc macro:
//! let tokens = ResourceBindingDerive::generate(&spec)?;
//! ```
//!
//! ## StaticItemDerive
//!
//! For derives that generate a single descriptor item (not a slice) with associated static data.
//! Common in pipeline derives where the entire type maps to one descriptor struct.
//!
//! **Example: GraphicsPipeline pattern**
//!
//! ```ignore
//! struct PipelineDescriptor {
//!     name: String,
//!     vs: String,
//!     fs: String,
//!     // ...
//! }
//!
//! impl quote::ToTokens for PipelineDescriptor { /* ... */ }
//!
//! struct GraphicsPipelineDerive;
//!
//! impl StaticItemDerive for GraphicsPipelineDerive {
//!     type Descriptor = PipelineDescriptor;
//!
//!     fn descriptor_type() -> TokenStream2 {
//!         quote! { macrokid_graphics::pipeline::PipelineDesc }
//!     }
//!
//!     fn build_descriptor(spec: &TypeSpec) -> syn::Result<Self::Descriptor> {
//!         // Parse type-level attributes, build descriptor
//!         // ...domain-specific logic only...
//!     }
//!
//!     fn trait_path() -> TokenStream2 {
//!         quote! { macrokid_graphics::pipeline::PipelineInfo }
//!     }
//!
//!     fn method_name() -> Ident {
//!         Ident::new("pipeline_desc", Span::call_site())
//!     }
//!
//!     fn module_hint() -> &'static str {
//!         "gp"
//!     }
//! }
//!
//! // Usage in proc macro:
//! let tokens = GraphicsPipelineDerive::generate(&spec)?;
//! ```

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};

use crate::common::builders::ImplBuilder;
use crate::common::codegen;
use crate::ir::TypeSpec;

// ============================================================================
// StaticSliceDerive Pattern
// ============================================================================

/// Pattern for derives that collect descriptors from fields and emit a static slice.
///
/// This trait encapsulates the common pattern:
/// 1. Parse field attributes
/// 2. Collect descriptor records
/// 3. Generate static module: `mod __mk_hint { pub static DATA: &[Ty] = &[...]; }`
/// 4. Generate trait impl: `fn method() -> &'static [Ty] { __mk_hint::DATA }`
/// 5. Generate inherent method for convenience
///
/// Implementors only need to provide domain-specific descriptor collection logic.
pub trait StaticSliceDerive {
    /// The descriptor type (intermediate representation).
    ///
    /// Must implement `ToTokens` to convert to code.
    type Descriptor: ToTokens;

    /// The runtime type path for the descriptor.
    ///
    /// Example: `quote! { macrokid_graphics::resources::BindingDesc }`
    fn descriptor_type() -> TokenStream2;

    /// Collect descriptors from the input type.
    ///
    /// This is where domain-specific logic lives:
    /// - Parse attributes (using `AttrSchema`, `exclusive_schemas!`, etc.)
    /// - Validate (uniqueness, ranges, etc.)
    /// - Build descriptor records
    ///
    /// Return `Err` for any validation failures.
    fn collect_descriptors(spec: &TypeSpec) -> syn::Result<Vec<Self::Descriptor>>;

    /// The trait to implement.
    ///
    /// Example: `quote! { macrokid_graphics::resources::ResourceBindings }`
    fn trait_path() -> TokenStream2;

    /// The trait method name that returns the static slice.
    ///
    /// Example: `Ident::new("bindings", Span::call_site())`
    fn method_name() -> Ident;

    /// Module name hint for the generated static module.
    ///
    /// Will be prefixed with `__mk_`. Example: `"rb"` becomes `__mk_rb`.
    fn module_hint() -> &'static str;

    /// Optional: inherent method name.
    ///
    /// Defaults to `describe_{module_hint}`.
    fn inherent_method_name() -> String {
        format!("describe_{}", Self::module_hint())
    }

    /// Generate the complete derive output.
    ///
    /// Provided implementation that composes all pieces using the CodeGen framework.
    fn generate(spec: &TypeSpec) -> syn::Result<TokenStream2> {
        // Collect descriptors (domain logic)
        let descriptors = Self::collect_descriptors(spec)?;

        // Convert to token streams
        let descriptor_tokens: Vec<TokenStream2> = descriptors
            .iter()
            .map(|d| d.to_token_stream())
            .collect();

        // Generate static module
        let ty = Self::descriptor_type();
        let (mod_ident, module) = codegen::static_slice_mod(
            Self::module_hint(),
            ty.clone(),
            descriptor_tokens,
        );

        // Generate trait impl
        let trait_impl = codegen::impl_trait_method_static_slice(
            spec,
            Self::trait_path(),
            Self::method_name(),
            ty.clone(),
            mod_ident.clone(),
        );

        // Generate inherent method
        let inherent_method_ident = Ident::new(&Self::inherent_method_name(), Span::call_site());
        let inherent = codegen::impl_inherent_methods(
            spec,
            &[quote! {
                pub fn #inherent_method_ident() -> &'static [#ty] {
                    #mod_ident::DATA
                }
            }],
        );

        Ok(quote! {
            #module
            #trait_impl
            #inherent
        })
    }
}

// ============================================================================
// StaticItemDerive Pattern
// ============================================================================

/// Pattern for derives that generate a single static descriptor item.
///
/// Similar to `StaticSliceDerive` but for single items instead of slices.
/// Common in pipeline/config derives where the type itself maps to one descriptor.
///
/// Generated code pattern:
/// ```ignore
/// mod __mk_hint {
///     pub static DESC: DescriptorType = DescriptorType { /* ... */ };
/// }
///
/// impl TraitName for Type {
///     fn method() -> &'static DescriptorType {
///         &__mk_hint::DESC
///     }
/// }
/// ```
pub trait StaticItemDerive {
    /// The descriptor type (intermediate representation).
    type Descriptor: ToTokens;

    /// The runtime type path for the descriptor.
    fn descriptor_type() -> TokenStream2;

    /// Build the descriptor from the input type.
    ///
    /// Domain-specific logic for parsing type-level attributes and constructing
    /// the descriptor.
    fn build_descriptor(spec: &TypeSpec) -> syn::Result<Self::Descriptor>;

    /// The trait to implement.
    fn trait_path() -> TokenStream2;

    /// The trait method name that returns a reference to the static item.
    fn method_name() -> Ident;

    /// Module name hint for the generated static module.
    fn module_hint() -> &'static str;

    /// Static item name within the module.
    ///
    /// Defaults to `"DESC"`.
    fn static_name() -> &'static str {
        "DESC"
    }

    /// Optional: inherent method name.
    fn inherent_method_name() -> String {
        format!("describe_{}", Self::module_hint())
    }

    /// Generate the complete derive output.
    fn generate(spec: &TypeSpec) -> syn::Result<TokenStream2> {
        // Build descriptor (domain logic)
        let descriptor = Self::build_descriptor(spec)?;
        let descriptor_tokens = descriptor.to_token_stream();

        // Generate static module
        let ty = Self::descriptor_type();
        let mod_ident = Ident::new(
            &format!("__mk_{}", Self::module_hint()),
            Span::call_site(),
        );
        let static_ident = Ident::new(Self::static_name(), Span::call_site());

        let module = quote! {
            #[allow(non_snake_case, non_upper_case_globals)]
            mod #mod_ident {
                pub static #static_ident: #ty = #descriptor_tokens;
            }
        };

        // Generate trait impl
        let method_ident = Self::method_name();
        let trait_impl = ImplBuilder::new(spec.ident.clone(), spec.generics.clone())
            .implement_trait(Self::trait_path())
            .add_method(quote! {
                fn #method_ident() -> &'static #ty {
                    &#mod_ident::#static_ident
                }
            })
            .build();

        // Generate inherent method
        let inherent_method_ident = Ident::new(&Self::inherent_method_name(), Span::call_site());
        let inherent = codegen::impl_inherent_methods(
            spec,
            &[quote! {
                pub fn #inherent_method_ident() -> &'static #ty {
                    <Self as #(Self::trait_path())>::#method_ident()
                }
            }],
        );

        Ok(quote! {
            #module
            #trait_impl
            #inherent
        })
    }
}

// ============================================================================
// Builder API for Derive Patterns
// ============================================================================

/// Builder for constructing a StaticSliceDerive implementation dynamically.
///
/// This provides an ergonomic alternative to manually implementing the trait.
/// Useful for quick prototyping or when you want to configure a derive at runtime.
///
/// # Example
///
/// ```ignore
/// let derive = StaticSliceBuilder::new()
///     .descriptor_type(quote! { MyDescriptor })
///     .trait_path(quote! { MyTrait })
///     .method_name("my_method")
///     .module_hint("my_mod")
///     .collector(|spec| {
///         // Collection logic
///         Ok(vec![/* descriptors */])
///     })
///     .build();
///
/// let tokens = derive.generate(&spec)?;
/// ```
pub struct StaticSliceBuilder<D> {
    descriptor_type: Option<TokenStream2>,
    trait_path: Option<TokenStream2>,
    method_name: Option<String>,
    module_hint: Option<String>,
    inherent_method: Option<String>,
    collector: Option<Box<dyn Fn(&TypeSpec) -> syn::Result<Vec<D>>>>,
}

impl<D: ToTokens> StaticSliceBuilder<D> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            descriptor_type: None,
            trait_path: None,
            method_name: None,
            module_hint: None,
            inherent_method: None,
            collector: None,
        }
    }

    /// Set the runtime descriptor type.
    pub fn descriptor_type(mut self, ty: TokenStream2) -> Self {
        self.descriptor_type = Some(ty);
        self
    }

    /// Set the trait to implement.
    pub fn trait_path(mut self, path: TokenStream2) -> Self {
        self.trait_path = Some(path);
        self
    }

    /// Set the trait method name.
    pub fn method_name(mut self, name: impl Into<String>) -> Self {
        self.method_name = Some(name.into());
        self
    }

    /// Set the module hint for generated code.
    pub fn module_hint(mut self, hint: impl Into<String>) -> Self {
        self.module_hint = Some(hint.into());
        self
    }

    /// Set custom inherent method name (optional).
    pub fn inherent_method_name(mut self, name: impl Into<String>) -> Self {
        self.inherent_method = Some(name.into());
        self
    }

    /// Set the descriptor collection function.
    pub fn collector<F>(mut self, f: F) -> Self
    where
        F: Fn(&TypeSpec) -> syn::Result<Vec<D>> + 'static,
    {
        self.collector = Some(Box::new(f));
        self
    }

    /// Build the derive and generate tokens.
    ///
    /// Returns an error if required fields are missing.
    pub fn generate(&self, spec: &TypeSpec) -> syn::Result<TokenStream2> {
        let descriptor_type = self
            .descriptor_type
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "descriptor_type is required"))?;

        let trait_path = self
            .trait_path
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "trait_path is required"))?;

        let method_name = self
            .method_name
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "method_name is required"))?;

        let module_hint = self
            .module_hint
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "module_hint is required"))?;

        let collector = self
            .collector
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "collector is required"))?;

        // Collect descriptors
        let descriptors = collector(spec)?;

        // Convert to tokens
        let descriptor_tokens: Vec<TokenStream2> =
            descriptors.iter().map(|d| d.to_token_stream()).collect();

        // Generate static module
        let (mod_ident, module) = codegen::static_slice_mod(
            module_hint,
            descriptor_type.clone(),
            descriptor_tokens,
        );

        // Generate trait impl
        let method_ident = Ident::new(method_name, Span::call_site());
        let trait_impl = codegen::impl_trait_method_static_slice(
            spec,
            trait_path.clone(),
            method_ident.clone(),
            descriptor_type.clone(),
            mod_ident.clone(),
        );

        // Generate inherent method
        let inherent_method_name = self
            .inherent_method
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| method_name.as_str());
        let inherent_method_ident = Ident::new(inherent_method_name, Span::call_site());

        let inherent = codegen::impl_inherent_methods(
            spec,
            &[quote! {
                pub fn #inherent_method_ident() -> &'static [#descriptor_type] {
                    #mod_ident::DATA
                }
            }],
        );

        Ok(quote! {
            #module
            #trait_impl
            #inherent
        })
    }
}

impl<D: ToTokens> Default for StaticSliceBuilder<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a StaticItemDerive implementation dynamically.
///
/// Similar to `StaticSliceBuilder` but for single-item descriptors.
pub struct StaticItemBuilder<D> {
    descriptor_type: Option<TokenStream2>,
    trait_path: Option<TokenStream2>,
    method_name: Option<String>,
    module_hint: Option<String>,
    static_name: Option<String>,
    inherent_method: Option<String>,
    builder: Option<Box<dyn Fn(&TypeSpec) -> syn::Result<D>>>,
}

impl<D: ToTokens> StaticItemBuilder<D> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            descriptor_type: None,
            trait_path: None,
            method_name: None,
            module_hint: None,
            static_name: Some("DESC".to_string()),
            inherent_method: None,
            builder: None,
        }
    }

    /// Set the runtime descriptor type.
    pub fn descriptor_type(mut self, ty: TokenStream2) -> Self {
        self.descriptor_type = Some(ty);
        self
    }

    /// Set the trait to implement.
    pub fn trait_path(mut self, path: TokenStream2) -> Self {
        self.trait_path = Some(path);
        self
    }

    /// Set the trait method name.
    pub fn method_name(mut self, name: impl Into<String>) -> Self {
        self.method_name = Some(name.into());
        self
    }

    /// Set the module hint for generated code.
    pub fn module_hint(mut self, hint: impl Into<String>) -> Self {
        self.module_hint = Some(hint.into());
        self
    }

    /// Set the static item name (defaults to "DESC").
    pub fn static_name(mut self, name: impl Into<String>) -> Self {
        self.static_name = Some(name.into());
        self
    }

    /// Set custom inherent method name (optional).
    pub fn inherent_method_name(mut self, name: impl Into<String>) -> Self {
        self.inherent_method = Some(name.into());
        self
    }

    /// Set the descriptor building function.
    pub fn builder<F>(mut self, f: F) -> Self
    where
        F: Fn(&TypeSpec) -> syn::Result<D> + 'static,
    {
        self.builder = Some(Box::new(f));
        self
    }

    /// Build the derive and generate tokens.
    pub fn generate(&self, spec: &TypeSpec) -> syn::Result<TokenStream2> {
        let descriptor_type = self
            .descriptor_type
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "descriptor_type is required"))?;

        let trait_path = self
            .trait_path
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "trait_path is required"))?;

        let method_name = self
            .method_name
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "method_name is required"))?;

        let module_hint = self
            .module_hint
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "module_hint is required"))?;

        let static_name = self
            .static_name
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "static_name is required"))?;

        let builder = self
            .builder
            .as_ref()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "builder is required"))?;

        // Build descriptor
        let descriptor = builder(spec)?;
        let descriptor_tokens = descriptor.to_token_stream();

        // Generate static module
        let mod_ident = Ident::new(&format!("__mk_{}", module_hint), Span::call_site());
        let static_ident = Ident::new(static_name, Span::call_site());

        let module = quote! {
            #[allow(non_snake_case, non_upper_case_globals)]
            mod #mod_ident {
                pub static #static_ident: #descriptor_type = #descriptor_tokens;
            }
        };

        // Generate trait impl
        let method_ident = Ident::new(method_name, Span::call_site());
        let trait_impl = ImplBuilder::new(spec.ident.clone(), spec.generics.clone())
            .implement_trait(trait_path.clone())
            .add_method(quote! {
                fn #method_ident() -> &'static #descriptor_type {
                    &#mod_ident::#static_ident
                }
            })
            .build();

        // Generate inherent method
        let inherent_method_name = self
            .inherent_method
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| method_name.as_str());
        let inherent_method_ident = Ident::new(inherent_method_name, Span::call_site());

        let inherent = codegen::impl_inherent_methods(
            spec,
            &[quote! {
                pub fn #inherent_method_ident() -> &'static #descriptor_type {
                    &#mod_ident::#static_ident
                }
            }],
        );

        Ok(quote! {
            #module
            #trait_impl
            #inherent
        })
    }
}

impl<D: ToTokens> Default for StaticItemBuilder<D> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Common validation utilities for use in `collect_descriptors` implementations.
pub mod validation {
    use proc_macro2::Span;
    use std::collections::HashSet;
    use std::hash::Hash;

    /// Validate uniqueness of a key extracted from items.
    ///
    /// Returns `Err` if any duplicates are found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// validate_unique(&records, |r| (r.set, r.binding), "duplicate (set, binding)")?;
    /// ```
    pub fn validate_unique<T, K, F>(
        items: &[T],
        key_fn: F,
        error_msg: &str,
    ) -> syn::Result<()>
    where
        K: Eq + Hash,
        F: Fn(&T) -> K,
    {
        let mut seen = HashSet::new();
        for item in items {
            let key = key_fn(item);
            if !seen.insert(key) {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("{}: found duplicate", error_msg),
                ));
            }
        }
        Ok(())
    }

    /// Validate that a value is within a range.
    pub fn validate_range<T: PartialOrd + std::fmt::Display>(
        value: T,
        min: T,
        max: T,
        name: &str,
    ) -> syn::Result<()> {
        if value < min || value > max {
            Err(syn::Error::new(
                Span::call_site(),
                format!("{} must be in range [{}..{}], got {}", name, min, max, value),
            ))
        } else {
            Ok(())
        }
    }
}
