use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse::{Parse, ParseStream}, punctuated::Punctuated, Token};
use crate::common::builders::ImplBuilder;

/// Input structure for make_enum[Name: Foo, Bar, Baz] syntax
pub struct BracketEnumInput {
    pub name: Ident,
    pub variants: Vec<BracketEnumVariant>,
}

/// Represents a single enum variant with optional attributes in bracket syntax
pub struct BracketEnumVariant {
    pub name: Ident,
    pub display_name: Option<String>,
}

impl Parse for BracketEnumInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        
        let variant_list: Punctuated<Ident, Token![,]> = 
            Punctuated::parse_separated_nonempty(input)?;
            
        let variants = variant_list.into_iter()
            .map(|ident| BracketEnumVariant {
                name: ident,
                display_name: None,
            })
            .collect();

        Ok(Self { name, variants })
    }
}

/// Generate a complete enum with derived traits (bracket syntax version)
pub fn expand_bracket_enum(input: BracketEnumInput) -> TokenStream2 {
    let enum_name = &input.name;
    let variant_names: Vec<_> = input.variants.iter().map(|v| &v.name).collect();
    let variant_strings: Vec<String> = input.variants.iter()
        .map(|v| v.display_name.as_ref().unwrap_or(&v.name.to_string()).clone())
        .collect();

    // Generate the enum definition with additional derive traits for bracket syntax
    let enum_def = quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum #enum_name {
            #( #variant_names ),*
        }
    };

    // Generate Display impl
    let display_impl = ImplBuilder::new(enum_name.clone(), syn::Generics::default())
        .implement_trait(quote! { ::core::fmt::Display })
        .add_method(quote! {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #( Self::#variant_names => f.write_str(#variant_strings) ),*
                }
            }
        })
        .build();

    // Generate FromStr impl
    let from_str_impl = ImplBuilder::new(enum_name.clone(), syn::Generics::default())
        .implement_trait(quote! { ::core::str::FromStr })
        .add_method(quote! {
            type Err = &'static str;
        })
        .add_method(quote! {
            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                match s {
                    #( #variant_strings => ::core::result::Result::Ok(Self::#variant_names) ),*,
                    _ => ::core::result::Result::Err("invalid variant"),
                }
            }
        })
        .build();

    // Add additional methods for bracket-style enums
    let variant_count = variant_names.len();
    let extra_methods = ImplBuilder::new(enum_name.clone(), syn::Generics::default())
        .add_method(quote! {
            /// Get all variant names as a slice
            pub const fn all_variants() -> &'static [&'static str] {
                &[#( #variant_strings ),*]
            }
        })
        .add_method(quote! {
            /// Get the variant count
            pub const fn variant_count() -> usize {
                #variant_count
            }
        })
        .build();

    quote! {
        #enum_def
        #display_impl
        #from_str_impl
        #extra_methods
    }
}
