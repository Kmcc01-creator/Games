use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Example custom Display derive macro built using macrokid_core
/// 
/// This demonstrates how to use the macrokid framework to build
/// your own derive macros without dealing with syn/quote directly.
/// 
/// Supports:
/// - `#[display("CustomName")]` on types and variants  
/// - Automatic variant name display for enums
/// - Automatic type name display for structs
#[proc_macro_derive(Display, attributes(display))]
pub fn derive_display(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    display_impl::expand(input).into()
}

mod display_impl {
    use macrokid_core::{
        ir::{FieldKind, TypeKind, TypeSpec}, 
        attrs::attr_string_value,
        diag::{err_on, err_at_span},
        builders::ImplBuilder,
    };
    use proc_macro2::TokenStream as TokenStream2;
    use quote::{quote, quote_spanned};
    use syn::DeriveInput;

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        match expand_inner(input) {
            Ok(ts) => ts,
            Err(e) => e.to_compile_error(),
        }
    }

    fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
        // Step 1: Parse the input into our normalized IR
        let spec = TypeSpec::from_derive_input(input)?;
        let ident = &spec.ident;

        // Step 2: Generate the display logic based on the type
        let body = match &spec.kind {
            TypeKind::Enum(en) => {
                // For enums: match each variant and display its name or custom attribute
                let arms = en.variants.iter().map(|v| {
                    let v_ident = &v.ident;
                    // Use macrokid_core's attribute parsing to get custom display name
                    let name = match attr_string_value(&v.attrs, "display") {
                        Some(s) if s.is_empty() => {
                            return err_at_span(v.span, "#[display(\"\")] is not allowed: empty string").to_compile_error()
                        }
                        Some(s) => s,
                        None => v_ident.to_string(),
                    };
                    // Match with .. to ignore fields regardless of kind
                    quote_spanned! { v.ident.span() => Self::#v_ident { .. } => f.write_str(#name) }
                });
                quote! {
                    match self {
                        #( #arms ),*
                    }
                }
            }
            TypeKind::Struct(st) => {
                // For structs: display the type name or custom attribute
                let default_name = ident.to_string();
                let name = match attr_string_value(&spec.attrs, "display") {
                    Some(s) if s.is_empty() => return Err(err_at_span(spec.span, "#[display(\"\")] is not allowed: empty string")),
                    Some(s) => s,
                    None => default_name,
                };
                match st.fields {
                    FieldKind::Named(_) | FieldKind::Unnamed(_) | FieldKind::Unit => {
                        quote! { f.write_str(#name) }
                    }
                }
            }
        };

        // Step 3: Use the builder pattern to generate clean implementation
        let impl_block = ImplBuilder::new(ident.clone(), spec.generics)
            .implement_trait(quote! { ::core::fmt::Display })
            .add_method(quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #body
                }
            })
            .build();

        Ok(impl_block)
    }
}

/// Example of a more complex derive macro showing advanced framework usage
#[proc_macro_derive(DebugVerbose, attributes(debug_verbose, skip))]
pub fn derive_debug_verbose(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    debug_verbose_impl::expand(input).into()
}

mod debug_verbose_impl {
    use macrokid_core::{
        ir::{FieldKind, TypeKind, TypeSpec},
        attrs::{attr_string_value, has_attr},
        diag::{suggest_with_note, err_at_span},
        builders::ImplBuilder,
    };
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::DeriveInput;

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        match expand_inner(input) {
            Ok(ts) => ts,
            Err(e) => e.to_compile_error(),
        }
    }

    fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
        let spec = TypeSpec::from_derive_input(input)?;
        let ident = &spec.ident;

        let body = match &spec.kind {
            TypeKind::Enum(en) => {
                let arms = en.variants.iter().map(|v| {
                    let v_ident = &v.ident;
                    let variant_name = v_ident.to_string();
                    
                    match &v.fields {
                        FieldKind::Unit => {
                            quote! {
                                Self::#v_ident => f.debug_struct(#variant_name).finish()
                            }
                        }
                        FieldKind::Named(fields) => {
                            let field_names: Vec<_> = fields.iter().map(|field| {
                                field.ident.as_ref().unwrap()
                            }).collect();
                            let field_debug = fields.iter().map(|field| {
                                let field_ident = field.ident.as_ref().unwrap();
                                let field_name = field_ident.to_string();
                                quote! { .field(#field_name, #field_ident) }
                            });
                            quote! {
                                Self::#v_ident { #(#field_names),* } => {
                                    f.debug_struct(#variant_name)#(#field_debug)*.finish()
                                }
                            }
                        }
                        FieldKind::Unnamed(fields) => {
                            let field_patterns: Vec<_> = (0..fields.len()).map(|i| {
                                syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site())
                            }).collect();
                            let field_debug = field_patterns.iter().enumerate().map(|(i, field_var)| {
                                let field_name = format!("field_{}", i);
                                quote! { .field(#field_name, #field_var) }
                            });
                            quote! {
                                Self::#v_ident(#(#field_patterns),*) => {
                                    f.debug_struct(#variant_name)#(#field_debug)*.finish()
                                }
                            }
                        }
                    }
                });

                quote! {
                    match self {
                        #( #arms ),*
                    }
                }
            }
            TypeKind::Struct(st) => {
                let type_name = ident.to_string();
                let custom_name = match attr_string_value(&spec.attrs, "debug_verbose") {
                    Some(s) if s.is_empty() => return Err(err_at_span(spec.span, "#[debug_verbose(\"\")] is not allowed: empty string")),
                    Some(s) => s,
                    None => type_name,
                };
                    
                match &st.fields {
                    FieldKind::Unit => {
                        quote! { f.debug_struct(#custom_name).finish() }
                    }
                    FieldKind::Named(fields) => {
                        let field_debug = fields.iter().map(|field| {
                            let field_ident = field.ident.as_ref().unwrap();
                            let field_name = field_ident.to_string();
                            if has_attr(&field.attrs, "skip") {
                                // Showcase diagnostics helper for a benign hint
                                let _ = suggest_with_note(&field.ident, "field is skipped", "remove #[skip] to include in DebugVerbose");
                                quote! {} // Skip this field
                            } else {
                                quote! { .field(#field_name, &self.#field_ident) }
                            }
                        });
                        quote! {
                            f.debug_struct(#custom_name)#(#field_debug)*.finish()
                        }
                    }
                    FieldKind::Unnamed(fields) => {
                        let field_debug = fields.iter().enumerate().map(|(i, _)| {
                            let index = syn::Index::from(i);
                            let field_name = format!("field_{}", i);
                            quote! { .field(#field_name, &self.#index) }
                        });
                        quote! {
                            f.debug_struct(#custom_name)#(#field_debug)*.finish()
                        }
                    }
                }
            }
        };

        let impl_block = ImplBuilder::new(ident.clone(), spec.generics)
            .implement_trait(quote! { ::core::fmt::Debug })
            .add_method(quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #body
                }
            })
            .build();

        Ok(impl_block)
    }
}

/// Example derive using semantic helper: match_variants
#[proc_macro_derive(Display2)]
pub fn derive_display2(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    display2_impl::expand(input).into()
}

mod display2_impl {
    use macrokid_core::{
        ir::{TypeKind, TypeSpec},
        builders::ImplBuilder,
        patterns::match_variants,
    };
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::DeriveInput;

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        match expand_inner(input) {
            Ok(ts) => ts,
            Err(e) => e.to_compile_error(),
        }
    }

    fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
        let spec = TypeSpec::from_derive_input(input)?;
        let ident = &spec.ident;

        let body = match &spec.kind {
            TypeKind::Enum(en) => {
                // Build arms per variant using semantic helper
                match_variants(en, |v| {
                    let vi = &v.ident;
                    let name = vi.to_string();
                    (quote! { Self::#vi { .. } }, quote! { f.write_str(#name) })
                })
                .build_match(quote! { self })
            }
            TypeKind::Struct(_) => {
                // Fallback: type name for structs
                let name = ident.to_string();
                quote! { f.write_str(#name) }
            }
        };

        let impl_block = ImplBuilder::new(ident.clone(), spec.generics)
            .implement_trait(quote! { ::core::fmt::Display })
            .add_method(quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #body
                }
            })
            .build();

        Ok(impl_block)
    }
}

/// Derive that generates a `first_exposed(&self) -> Option<(&'static str, &dyn Debug)>`
/// for named-field structs using `#[expose]` markers on fields.
#[proc_macro_derive(FirstExposed, attributes(expose))]
pub fn derive_first_exposed(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    first_exposed_impl::expand(input).into()
}

mod first_exposed_impl {
    use macrokid_core::{
        ir::{FieldKind, TypeKind, TypeSpec},
        attrs::has_attr,
        builders::ImplBuilder,
        patterns::match_fields,
    };
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::DeriveInput;

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        match expand_inner(input) {
            Ok(ts) => ts,
            Err(e) => e.to_compile_error(),
        }
    }

    fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
        let spec = TypeSpec::from_derive_input(input)?;
        let ident = &spec.ident;

        // Only meaningful for named-field structs; otherwise always return None
        let method_body = match &spec.kind {
            TypeKind::Struct(st) => match &st.fields {
                FieldKind::Named(fields) => {
                    let arms = match_fields(&st.fields, |f| {
                        if let Some(fid) = &f.ident {
                            if has_attr(&f.attrs, "expose") {
                                let name = fid.to_string();
                                return Some((
                                    quote! { Self { #fid, .. } },
                                    quote! { return Some((#name, #fid as &dyn ::core::fmt::Debug)) },
                                ));
                            }
                        }
                        None
                    });
                    arms
                        .add_wildcard(quote! { None })
                        .build_match(quote! { self })
                }
                _ => quote! { None },
            },
            _ => quote! { None },
        };

        let impl_block = ImplBuilder::new(ident.clone(), spec.generics)
            .add_method(quote! {
                pub fn first_exposed(&self) -> Option<(&'static str, &dyn ::core::fmt::Debug)> {
                    #method_body
                }
            })
            .build();

        Ok(impl_block)
    }
}

/// Example derive using pattern DSL to implement Display for enums
#[proc_macro_derive(DisplayDSL)]
pub fn derive_display_dsl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    display_dsl_impl::expand(input).into()
}

mod display_dsl_impl {
    use macrokid_core::{
        ir::{FieldKind, TypeKind, TypeSpec},
        builders::ImplBuilder,
        // Use the public builder directly; the patterns-reexported one is private
        common::builders::MatchArmBuilder,
        pattern_dsl as dsl,
    };
    use proc_macro2::TokenStream as TokenStream2;
    use quote::{quote, ToTokens};
    use syn::{parse_quote, DeriveInput};

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        match expand_inner(input) {
            Ok(ts) => ts,
            Err(e) => e.to_compile_error(),
        }
    }

    fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
        let spec = TypeSpec::from_derive_input(input)?;
        let ident = &spec.ident;

        let body = match &spec.kind {
            TypeKind::Enum(en) => {
                let mut b = MatchArmBuilder::new();
                for v in &en.variants {
                    let vi = &v.ident;
                    let name = vi.to_string();
                    let path: syn::Path = parse_quote!(Self::#vi);
                    let pat = match &v.fields {
                        FieldKind::Named(_) => dsl::PatternSpec::Struct { path, fields: dsl::StructFields { named: vec![], rest: true } },
                        FieldKind::Unnamed(fields) => {
                            let elems = (0..fields.len()).map(|_| dsl::PatternSpec::Wildcard).collect();
                            dsl::PatternSpec::Tuple { path, elements: elems }
                        }
                        FieldKind::Unit => dsl::PatternSpec::Path(path),
                    };
                    b = b.add_arm(pat.into_tokens(), quote! { f.write_str(#name) });
                }
                b.build_match(quote! { self })
            }
            TypeKind::Struct(_) => {
                let name = ident.to_string();
                quote! { f.write_str(#name) }
            }
        };

        let impl_block = ImplBuilder::new(ident.clone(), spec.generics)
            .implement_trait(quote! { ::core::fmt::Display })
            .add_method(quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #body
                }
            })
            .build();

        Ok(impl_block)
    }
}

// --- Associated items example ---
// The trait lives in a normal lib crate to conform to proc-macro crate rules.

/// Derive that implements `AssocDemo` using ImplBuilder's associated items features
#[proc_macro_derive(AssocImpl)]
pub fn derive_assoc_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    assoc_impl::expand(input).into()
}

mod assoc_impl {
    use macrokid_core::builders::ImplBuilder;
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::DeriveInput;

    pub fn expand(input: DeriveInput) -> TokenStream2 {
        let ident = input.ident;
        let generics = input.generics;

        ImplBuilder::new(ident.clone(), generics)
            .implement_trait(quote! { custom_derive_support::AssocDemo })
            .add_assoc_type(syn::Ident::new("Output", ident.span()), quote! { u32 })
            .add_assoc_const(syn::Ident::new("COUNT", ident.span()), quote! { usize }, quote! { 7 })
            .add_method(quote! { fn get(&self) -> Self::Output { 0 } })
            .build()
    }
}
