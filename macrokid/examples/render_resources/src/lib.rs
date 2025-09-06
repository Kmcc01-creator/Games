use proc_macro::TokenStream;
use proc_macro2::Span;
use macrokid_core::{
    ir::{TypeSpec, FieldKind},
    builders::ImplBuilder,
    common::attrs::{validate_attrs, AttrSpec, AttrType, AttrValue, has_attr},
    diag::{err_at_span},
};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

// Note: types/traits live in render_resources_support

#[proc_macro_derive(ResourceBinding, attributes(uniform, texture, sampler))]
pub fn derive_resource_binding(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    expand(input).into()
}

fn expand(input: DeriveInput) -> proc_macro2::TokenStream {
    match expand_inner(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn expand_inner(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();
    let fields = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => match st.fields() {
            FieldKind::Named(v) => v.clone(),
            _ => return Err(syn::Error::new(spec.span, "ResourceBinding expects a struct with named fields")),
        },
        _ => return Err(syn::Error::new(spec.span, "ResourceBinding expects a struct")),
    };

    let schema = [
        AttrSpec { key: "set", required: true, ty: AttrType::Int },
        AttrSpec { key: "binding", required: true, ty: AttrType::Int },
    ];

    // Collect and validate
    #[derive(Clone, Debug)]
    struct Rec { field: String, set: u32, binding: u32, kind: &'static str, _span: proc_macro2::Span }
    let mut out: Vec<Rec> = Vec::new();
    use std::collections::HashSet;
    let mut seen: HashSet<(u32, u32)> = HashSet::new();

    for f in &fields {
        let span = f.span;
        let fname = f.ident.as_ref().unwrap().to_string();
        let mut kinds = Vec::new();
        if has_attr(&f.attrs, "uniform") { kinds.push("uniform"); }
        if has_attr(&f.attrs, "texture") { kinds.push("texture"); }
        if has_attr(&f.attrs, "sampler") { kinds.push("sampler"); }
        if kinds.len() > 1 {
            return Err(err_at_span(span, &format!("field '{}' has multiple resource kinds: {:?}", fname, kinds)));
        }
        if let Some(kind) = kinds.first() {
            let map = validate_attrs(&f.attrs, kind, &schema)?;
            let set = match map.get("set").unwrap() { AttrValue::Int(i) => *i as u32, _ => unreachable!() };
            let binding = match map.get("binding").unwrap() { AttrValue::Int(i) => *i as u32, _ => unreachable!() };
            if !seen.insert((set, binding)) {
                return Err(err_at_span(span, &format!("duplicate (set={}, binding={})", set, binding)));
            }
            out.push(Rec { field: fname, set, binding, kind, _span: span });
        }
    }

    // Generate module + method
    let mod_ident = syn::Ident::new(&format!("__rb_{}", ident), Span::call_site());
    let descs = out.iter().map(|r| {
        let field = &r.field;
        let set = r.set;
        let binding = r.binding;
        let kind = match r.kind { "uniform" => quote! { render_resources_support::ResourceKind::Uniform }, "texture" => quote! { render_resources_support::ResourceKind::Texture }, _ => quote! { render_resources_support::ResourceKind::Sampler } };
        quote! { render_resources_support::BindingDesc { field: #field, set: #set, binding: #binding, kind: #kind } }
    });

    let module = quote! {
        #[allow(non_snake_case)]
        mod #mod_ident {
            pub static DESCS: &[render_resources_support::BindingDesc] = &[ #( #descs ),* ];
        }
    };

    let method = ImplBuilder::new(ident.clone(), spec.generics)
        .add_method(quote! {
            pub fn describe_bindings() -> &'static [render_resources_support::BindingDesc] { #mod_ident::DESCS }
        })
        .build();
    let trait_impl = quote! { impl render_resources_support::ResourceBindings for #ident { fn bindings() -> &'static [render_resources_support::BindingDesc] { #mod_ident::DESCS } } };
    Ok(quote! { #module #method #trait_impl })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn duplicate_binding_fails() {
        let di: DeriveInput = parse_quote! {
            #[derive(ResourceBinding)]
            struct R {
                #[uniform(set = 0, binding = 0)] a: u32,
                #[texture(set = 0, binding = 0)] b: u32,
            }
        };
        let res = expand_inner(di);
        assert!(res.is_err());
    }
}

// ================= BufferLayout derive =================

#[proc_macro_derive(BufferLayout, attributes(vertex, buffer))]
pub fn derive_buffer_layout(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    expand_buffer_layout(input).into()
}

fn expand_buffer_layout(input: DeriveInput) -> proc_macro2::TokenStream {
    match expand_buffer_layout_inner(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn expand_buffer_layout_inner(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();
    let fields = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => match st.fields() {
            FieldKind::Named(v) => v.clone(),
            FieldKind::Unnamed(v) => v.clone(),
            FieldKind::Unit => Vec::new(),
        },
        _ => return Err(syn::Error::new(spec.span, "BufferLayout expects a struct")),
    };

    let schema = [
        AttrSpec { key: "location", required: true, ty: AttrType::Int },
        AttrSpec { key: "format", required: false, ty: AttrType::Str },
    ];

    #[derive(Clone, Debug)]
    struct AttrRec { field: String, location: u32, format: Option<String>, span: proc_macro2::Span }

    let mut recs: Vec<AttrRec> = Vec::new();
    use std::collections::HashSet;
    let mut seen_loc: HashSet<u32> = HashSet::new();

    for f in &fields {
        if !macrokid_core::common::attrs::has_attr(&f.attrs, "vertex") { continue; }
        let map = validate_attrs(&f.attrs, "vertex", &schema)?;
        let location = match map.get("location").unwrap() { AttrValue::Int(i) => *i as u32, _ => unreachable!() };
        if !seen_loc.insert(location) {
            return Err(err_at_span(f.span, &format!("duplicate vertex location {}", location)));
        }
        let format = match map.get("format") { Some(AttrValue::Str(s)) => Some(s.clone()), _ => None };
        let fname = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_else(|| format!("_{}", f.index));
        recs.push(AttrRec { field: fname, location, format, span: f.span });
    }

    let mod_ident = syn::Ident::new(&format!("__vl_{}", ident), Span::call_site());
    // Sort by location, compute offsets and sizes
    let mut recs_sorted = recs.clone();
    recs_sorted.sort_by_key(|r| r.location);

    fn infer_size_from_format(fmt: &str) -> Option<usize> {
        match fmt {
            "f32" | "u32" | "i32" => Some(4),
            "vec2" => Some(8),
            "vec3" => Some(12),
            "vec4" => Some(16),
            "mat4" => Some(64),
            _ => None,
        }
    }

    fn infer_size_from_type(ty: &syn::Type) -> Option<usize> {
        match ty {
            syn::Type::Path(p) => {
                if let Some(seg) = p.path.segments.last() {
                    match seg.ident.to_string().as_str() {
                        "f32" | "u32" | "i32" => Some(4),
                        _ => None,
                    }
                } else { None }
            }
            syn::Type::Array(a) => {
                let elem = &*a.elem;
                let elem_size = infer_size_from_type(elem)?;
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(n), .. }) = &a.len {
                    let count = n.base10_parse::<usize>().ok()?;
                    Some(elem_size * count)
                } else { None }
            }
            _ => None,
        }
    }

    // Compute per-attr offsets and sizes
    let mut offset_acc = 0usize;
    let mut items_vec: Vec<proc_macro2::TokenStream> = Vec::new();
    for r in &recs_sorted {
        let field = &r.field;
        let loc = r.location;
        let fmt_str = r.format.clone().unwrap_or_else(|| "auto".to_string());
        let size = if fmt_str != "auto" {
            infer_size_from_format(&fmt_str).ok_or_else(|| syn::Error::new(r.span, format!("unknown format '{}' for field '{}'", fmt_str, field)))?
        } else {
            // find the original field to use its type
            let f = fields.iter().find(|f| f.ident.as_ref().map(|i| i.to_string()) == Some(field.clone()));
            if let Some(f) = f { infer_size_from_type(&f.ty).ok_or_else(|| syn::Error::new(r.span, format!("cannot infer size for field '{}'", field)))? } else { 0 }
        };
        let off = offset_acc;
        offset_acc += size;
        let fmt = fmt_str;
        items_vec.push(quote! { render_resources_support::VertexAttr { field: #field, location: #loc, format: #fmt, offset: #off as u32, size: #size as u32 } });
    }

    let items = items_vec.into_iter();

    let module = quote! {
        #[allow(non_snake_case)]
        mod #mod_ident { pub static ATTRS: &[render_resources_support::VertexAttr] = &[ #( #items ),* ]; }
    };

    // Parse optional buffer-level attributes: #[buffer(stride = N, step = "vertex"|"instance")]
    let buf_schema = [
        AttrSpec { key: "stride", required: false, ty: AttrType::Int },
        AttrSpec { key: "step", required: false, ty: AttrType::Str },
    ];
    let buf_map = validate_attrs(&spec.attrs, "buffer", &buf_schema).unwrap_or_default();
    let step = match buf_map.get("step").and_then(|v| if let AttrValue::Str(s) = v { Some(s.as_str()) } else { None }) {
        Some("instance") => quote! { render_resources_support::StepMode::Instance },
        _ => quote! { render_resources_support::StepMode::Vertex },
    };
    let stride_val: Option<i64> = match buf_map.get("stride") { Some(AttrValue::Int(i)) => Some(*i), _ => None };
    let stride = if let Some(v) = stride_val { v as u32 } else { offset_acc as u32 };

    let method = ImplBuilder::new(ident.clone(), spec.generics)
        .add_method(quote! { pub fn describe_vertex_layout() -> &'static [render_resources_support::VertexAttr] { #mod_ident::ATTRS } })
        .add_method(quote! { pub fn describe_vertex_buffer() -> render_resources_support::VertexBufferDesc { render_resources_support::VertexBufferDesc { stride: #stride, step: #step } } })
        .build();
    let trait_impl = quote! { impl render_resources_support::VertexLayout for #ident { fn vertex_attrs() -> &'static [render_resources_support::VertexAttr] { #mod_ident::ATTRS } fn vertex_buffer() -> render_resources_support::VertexBufferDesc { render_resources_support::VertexBufferDesc { stride: #stride, step: #step } } } };
    Ok(quote! { #module #method #trait_impl })
}

#[cfg(test)]
mod buffer_tests {
    use super::*;
    use syn::parse_quote;
    #[test]
    fn duplicate_location_fails() {
        let di: DeriveInput = parse_quote! {
            #[derive(BufferLayout)]
            struct V {
                #[vertex(location = 0)] a: [f32; 3],
                #[vertex(location = 0)] b: [f32; 3],
            }
        };
        let res = expand_buffer_layout_inner(di);
        assert!(res.is_err());
    }
}
