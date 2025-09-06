use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::Generics;

/// Builder for generating impl blocks
pub struct ImplBuilder {
    target_type: Ident,
    generics: Generics,
    trait_name: Option<TokenStream2>,
    methods: Vec<TokenStream2>,
    assoc_types: Vec<TokenStream2>,
    assoc_consts: Vec<TokenStream2>,
    impl_attrs: Vec<TokenStream2>,
}

impl ImplBuilder {
    pub fn new(target_type: Ident, generics: Generics) -> Self {
        Self {
            target_type,
            generics,
            trait_name: None,
            methods: Vec::new(),
            assoc_types: Vec::new(),
            assoc_consts: Vec::new(),
            impl_attrs: Vec::new(),
        }
    }

    /// Add a trait implementation
    pub fn implement_trait(mut self, trait_name: TokenStream2) -> Self {
        self.trait_name = Some(trait_name);
        self
    }

    /// Add a method to the impl block
    pub fn add_method(mut self, method: TokenStream2) -> Self {
        self.methods.push(method);
        self
    }

    /// Add an associated type declaration: `type Name = Ty;`
    pub fn add_assoc_type(mut self, name: Ident, ty: TokenStream2) -> Self {
        let item = quote! { type #name = #ty; };
        self.assoc_types.push(item);
        self
    }

    /// Add an associated const declaration: `const NAME: Ty = value;`
    pub fn add_assoc_const(mut self, name: Ident, ty: TokenStream2, value: TokenStream2) -> Self {
        let item = quote! { const #name: #ty = #value; };
        self.assoc_consts.push(item);
        self
    }

    /// Attach a doc comment to the impl block
    pub fn with_docs(mut self, docs: &str) -> Self {
        let d = docs.to_string();
        let attr = quote! { #[doc = #d] };
        self.impl_attrs.push(attr);
        self
    }

    /// Attach arbitrary attributes to the impl block (e.g., cfg, allow, etc.)
    pub fn with_attrs(mut self, attrs: TokenStream2) -> Self {
        self.impl_attrs.push(attrs);
        self
    }

    /// Build the final impl block
    pub fn build(self) -> TokenStream2 {
        let target_type = &self.target_type;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let methods = &self.methods;
        let assoc_types = &self.assoc_types;
        let assoc_consts = &self.assoc_consts;
        let impl_attrs = &self.impl_attrs;

        if let Some(trait_name) = &self.trait_name {
            quote! {
                #( #impl_attrs )*
                impl #impl_generics #trait_name for #target_type #ty_generics #where_clause {
                    #( #assoc_types )*
                    #( #assoc_consts )*
                    #( #methods )*
                }
            }
        } else {
            quote! {
                #( #impl_attrs )*
                impl #impl_generics #target_type #ty_generics #where_clause {
                    #( #assoc_types )*
                    #( #assoc_consts )*
                    #( #methods )*
                }
            }
        }
    }
}

/// Builder for generating match arms
pub struct MatchArmBuilder {
    arms: Vec<TokenStream2>,
}

impl MatchArmBuilder {
    pub fn new() -> Self {
        Self { arms: Vec::new() }
    }

    /// Add a match arm
    pub fn add_arm(mut self, pattern: TokenStream2, body: TokenStream2) -> Self {
        let arm = quote! { #pattern => #body };
        self.arms.push(arm);
        self
    }

    /// Add a wildcard (_) arm
    pub fn add_wildcard(self, body: TokenStream2) -> Self {
        self.add_arm(quote! { _ }, body)
    }

    /// Add an arm with a guard: `<pattern> if <guard> => <body>`
    pub fn add_guarded_arm(mut self, pattern: TokenStream2, guard: TokenStream2, body: TokenStream2) -> Self {
        let arm = quote! { #pattern if #guard => #body };
        self.arms.push(arm);
        self
    }

    /// Add an arm matching multiple patterns combined with `|`: `A | B | C => body`
    pub fn add_multi_pattern<I>(mut self, patterns: I, body: TokenStream2) -> Self
    where
        I: IntoIterator<Item = TokenStream2>,
    {
        let patterns: Vec<TokenStream2> = patterns.into_iter().collect();
        let arm = quote! { #( #patterns )|* => #body };
        self.arms.push(arm);
        self
    }

    /// Build the match expression
    pub fn build_match(self, scrutinee: TokenStream2) -> TokenStream2 {
        let arms = &self.arms;
        quote! {
            match #scrutinee {
                #( #arms ),*
            }
        }
    }

    /// Build just the arms (for use in existing match expressions)
    pub fn build_arms(self) -> Vec<TokenStream2> {
        self.arms
    }

    /// Current number of arms (for heuristics)
    pub fn len(&self) -> usize { self.arms.len() }
    /// Whether there are no arms
    pub fn is_empty(&self) -> bool { self.arms.is_empty() }
}

impl Default for MatchArmBuilder {
    fn default() -> Self {
        Self::new()
    }
}
