use proc_macro::TokenStream;
use macrokid_core::{ir::TypeSpec, builders::ImplBuilder};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Ident, LitStr};

#[proc_macro_derive(FluentBuilder, attributes(builder, builder_transition))]
pub fn derive_fluent_builder(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    expand(input).into()
}

fn expand(input: DeriveInput) -> TokenStream2 {
    match expand_inner(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn expand_inner(input: DeriveInput) -> syn::Result<TokenStream2> {
    let spec = TypeSpec::from_derive_input(input.clone())?;
    let ident = spec.ident.clone();
    let mut methods: Vec<TokenStream2> = Vec::new();

    // Expect a struct with named fields
    let field_specs: Vec<macrokid_core::ir::FieldSpec> = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => match st.fields() {
            macrokid_core::FieldKind::Named(v) => v.clone(),
            _ => return Err(syn::Error::new(spec.span, "FluentBuilder expects a struct with named fields")),
        },
        _ => return Err(syn::Error::new(spec.span, "FluentBuilder expects a struct type")),
    };

    for f in field_specs.iter() {
        let fid = if let Some(id) = &f.ident { id } else { continue };
        let ty = &f.ty;
        // Parse #[builder(...)] options
        let mut method_name: Option<Ident> = None;
        let mut tuple_spec: Option<TupleSpec> = None;
        for attr in &f.attrs {
            if !attr.path().is_ident("builder") { continue; }
            match &attr.meta {
                syn::Meta::Path(_) => {
                    // plain #[builder] -> default single-arg setter
                }
                syn::Meta::List(list) => {
                    for nested in &list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)? {
                        match nested {
                            syn::Meta::NameValue(kv) if kv.path.is_ident("name") => {
                                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                                    method_name = Some(Ident::new(&s.value(), s.span()));
                                } else {
                                    return Err(syn::Error::new(kv.value.span(), "builder(name = \"...\") expects string"));
                                }
                            }
                            syn::Meta::NameValue(kv) if kv.path.is_ident("tuple") => {
                                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                                    let spec = parse_tuple_spec(&s)?;
                                    tuple_spec = Some(spec);
                                } else {
                                    return Err(syn::Error::new(kv.value.span(), "builder(tuple = \"...\") expects string"));
                                }
                            }
                            other => return Err(syn::Error::new(other.span(), "unknown builder option; expected name/tuple")),
                        }
                    }
                }
                syn::Meta::NameValue(_) => {}
            }
        }

        let m_ident = method_name.unwrap_or_else(|| fid.clone());

        let method = if let Some(tspec) = tuple_spec {
            let params = tspec.params.iter().map(|(id, ty)| quote! { #id: #ty });
            let expr = &tspec.expr;
            quote_spanned! { fid.span() =>
                pub fn #m_ident(mut self, #( #params ),* ) -> Self { self.#fid = #expr; self }
            }
        } else {
            quote_spanned! { fid.span() =>
                pub fn #m_ident(mut self, value: #ty) -> Self { self.#fid = value; self }
            }
        };
        methods.push(method);
    }

    // Parse struct-level transitions: #[builder_transition(method = "finish", to = "Type", receiver = "self|&mut self", body = "{ ... }")]
    let transitions = parse_transitions(&spec)?;
    for tr in transitions {
        let Transition { method, receiver, ret_ty, body } = tr;
        let sig = match receiver {
            ReceiverKind::ByValue => quote! { pub fn #method(self) -> #ret_ty },
            ReceiverKind::ByMutRef => quote! { pub fn #method(&mut self) -> #ret_ty },
        };
        methods.push(quote! { #sig #body });
    }

    let impl_block = ImplBuilder::new(ident.clone(), spec.generics).add_method(quote! { #( #methods )* }).build();
    Ok(impl_block)
}

struct Transition { method: Ident, receiver: ReceiverKind, ret_ty: syn::Type, body: syn::Block }
enum ReceiverKind { ByValue, ByMutRef }

fn parse_transitions(spec: &TypeSpec) -> syn::Result<Vec<Transition>> {
    let mut out = Vec::new();
    for attr in &spec.attrs {
        if !attr.path().is_ident("builder_transition") { continue; }
        let mut method_ident: Option<Ident> = None;
        let mut ret_ty: Option<syn::Type> = None;
        let mut receiver = ReceiverKind::ByValue;
        let mut body: Option<syn::Block> = None;

        if let syn::Meta::List(list) = &attr.meta {
            for nested in &list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)? {
                match nested {
                    syn::Meta::NameValue(kv) if kv.path.is_ident("method") => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                            method_ident = Some(Ident::new(&s.value(), s.span()));
                        } else {
                            return Err(syn::Error::new(kv.value.span(), "builder_transition(method = \"...\") expects string"));
                        }
                    }
                    syn::Meta::NameValue(kv) if kv.path.is_ident("to") => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                            ret_ty = Some(syn::parse_str::<syn::Type>(&s.value()).map_err(|e| syn::Error::new(s.span(), format!("invalid type: {}", e)))?);
                        } else {
                            return Err(syn::Error::new(kv.value.span(), "builder_transition(to = \"Type\") expects string"));
                        }
                    }
                    syn::Meta::NameValue(kv) if kv.path.is_ident("receiver") => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                            let v = s.value();
                            receiver = match v.as_str() { "self" => ReceiverKind::ByValue, "&mut self" => ReceiverKind::ByMutRef, _ => return Err(syn::Error::new(s.span(), "receiver must be 'self' or '&mut self'")) };
                        } else {
                            return Err(syn::Error::new(kv.value.span(), "builder_transition(receiver = \"...\") expects string"));
                        }
                    }
                    syn::Meta::NameValue(kv) if kv.path.is_ident("body") => {
                        if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &kv.value {
                            body = Some(syn::parse_str::<syn::Block>(&s.value()).map_err(|e| syn::Error::new(s.span(), format!("invalid block: {}", e)))?);
                        } else {
                            return Err(syn::Error::new(kv.value.span(), "builder_transition(body = \"{ ... }\") expects string"));
                        }
                    }
                    other => return Err(syn::Error::new(other.span(), "unknown builder_transition option; expected method/to/receiver/body")),
                }
            }
        } else {
            return Err(syn::Error::new(attr.span(), "expected #[builder_transition(method = ..., to = ..., body = ...)]"));
        }

        let method = method_ident.ok_or_else(|| syn::Error::new(attr.span(), "missing method"))?;
        let ret_ty = ret_ty.ok_or_else(|| syn::Error::new(attr.span(), "missing to type"))?;
        let body = body.ok_or_else(|| syn::Error::new(attr.span(), "missing body"))?;
        out.push(Transition { method, receiver, ret_ty, body });
    }
    Ok(out)
}

struct TupleSpec { params: Vec<(Ident, syn::Type)>, expr: syn::Expr }

fn parse_tuple_spec(s: &LitStr) -> syn::Result<TupleSpec> {
    // Expect format: "(a: Ty, b: Ty2) => EXPR"
    let src = s.value();
    // Build a dummy function signature to reuse syn parsing: fn f(a: Ty, b: Ty2) {}
    // And parse expr separately after '=>'
    let parts: Vec<&str> = src.split("=>").collect();
    if parts.len() != 2 { return Err(syn::Error::new(s.span(), "tuple spec must be '(a: Ty, ...) => EXPR'")); }
    let params_src = parts[0].trim();
    let expr_src = parts[1].trim();
    let sig_src = format!("fn __f{} {{}}", params_src);
    let item: syn::ItemFn = syn::parse_str(&sig_src).map_err(|e| syn::Error::new(s.span(), format!("invalid params: {}", e)))?;
    let mut params = Vec::new();
    for input in item.sig.inputs.iter() {
        match input {
            syn::FnArg::Typed(pt) => {
                if let syn::Pat::Ident(pi) = &*pt.pat {
                    params.push((pi.ident.clone(), (*pt.ty).clone()));
                } else {
                    return Err(syn::Error::new(pt.pat.span(), "unsupported pattern; use simple identifiers"));
                }
            }
            _ => return Err(syn::Error::new(input.span(), "unsupported receiver")),
        }
    }
    let expr: syn::Expr = syn::parse_str(expr_src).map_err(|e| syn::Error::new(s.span(), format!("invalid expr: {}", e)))?;
    Ok(TupleSpec { params, expr })
}
