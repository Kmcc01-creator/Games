use proc_macro2::Span;
use macrokid_core::{
    ir::{TypeSpec, FieldKind},
    collect,
    codegen,
    derive_entry,
};
use macrokid_core::exclusive_schemas;
use quote::quote;
use crate::gen::CodeGen;
use syn::DeriveInput;

mod gen;

derive_entry!(ResourceBinding, attrs = [uniform, texture, sampler], handler = expand_resource_binding);

fn expand_resource_binding(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;

    // Ensure struct with named fields
    let st = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => st,
        _ => return Err(syn::Error::new(spec.span, "ResourceBinding expects a struct")),
    };
    if !matches!(st.fields(), FieldKind::Named(_)) {
        return Err(syn::Error::new(spec.span, "ResourceBinding expects a struct with named fields"));
    }

    // Define mutually exclusive resource kind schemas
    let kind_set = macrokid_core::exclusive_schemas![
        uniform(set: int, binding: int),
        texture(set: int, binding: int),
        sampler(set: int, binding: int),
    ];

    #[derive(Clone, Debug)]
    struct RBRec { field: String, set: u32, binding: u32, kind: String, span: proc_macro2::Span }

    let items: Vec<RBRec> = collect::from_named_fields(st, |f| {
        if let Some((kind, parsed)) = kind_set.parse(&f.attrs)? {
            let field = f.ident.as_ref().unwrap().to_string();
            let set = parsed.try_get_int("set")? as u32;
            let binding = parsed.try_get_int("binding")? as u32;
            Ok(Some(RBRec { field, set, binding, kind, span: f.span }))
        } else {
            Ok(None)
        }
    })?;

    // Enforce uniqueness of (set, binding)
    let items = collect::unique_by(items, |r| ((r.set, r.binding), r.span), "duplicate (set,binding)")?;

    // Build static module + trait impls using CodeGen composition
    let ty = quote! { macrokid_graphics::resources::BindingDesc };
    let entry_tokens: Vec<proc_macro2::TokenStream> = items.iter().map(|r| {
        let field = &r.field;
        let set = r.set;
        let binding = r.binding;
        let kind = match r.kind.as_str() {
            "uniform" => quote! { macrokid_graphics::resources::ResourceKind::Uniform },
            "texture" => quote! { macrokid_graphics::resources::ResourceKind::Texture },
            _ => quote! { macrokid_graphics::resources::ResourceKind::Sampler },
        };
        quote! { macrokid_graphics::resources::BindingDesc { field: #field, set: #set, binding: #binding, kind: #kind } }
    }).collect();

    struct RBInput { mod_ident: syn::Ident, ty: proc_macro2::TokenStream, entries: Vec<proc_macro2::TokenStream>, spec: TypeSpec }
    let rb_input = RBInput {
        mod_ident: syn::Ident::new("__mk_rb", Span::call_site()),
        ty: ty.clone(),
        entries: entry_tokens,
        spec: spec.clone(),
    };

    struct RBModuleGen;
    impl crate::gen::CodeGen<RBInput> for RBModuleGen {
        type Output = proc_macro2::TokenStream;
        fn generate(i: &RBInput) -> Self::Output {
            let mod_ident = &i.mod_ident;
            let ty = &i.ty;
            let entries = &i.entries;
            quote! {
                #[allow(non_snake_case, non_upper_case_globals)]
                mod #mod_ident { pub static DATA: &[#ty] = &[ #( #entries ),* ]; }
            }
        }
    }

    struct RBImplGen;
    impl crate::gen::CodeGen<RBInput> for RBImplGen {
        type Output = proc_macro2::TokenStream;
        fn generate(i: &RBInput) -> Self::Output {
            let method_ident = syn::Ident::new("bindings", Span::call_site());
            let trait_impl = codegen::impl_trait_method_static_slice(
                &i.spec,
                quote! { macrokid_graphics::resources::ResourceBindings },
                method_ident,
                i.ty.clone(),
                i.mod_ident.clone(),
            );
            let ty = &i.ty;
            let mod_ident = &i.mod_ident;
            let inherent = codegen::impl_inherent_methods(&i.spec, &[quote! {
                pub fn describe_bindings() -> &'static [#ty] { #mod_ident::DATA }
            }]);
            quote! { #trait_impl #inherent }
        }
    }

    type RBFull = crate::gen::Chain<RBModuleGen, RBImplGen>;
    let ts = RBFull::generate(&rb_input);
    Ok(ts)
}

// ================= BufferLayout derive =================

derive_entry!(BufferLayout, attrs = [vertex, buffer], handler = expand_buffer_layout);

fn expand_buffer_layout(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    let st = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => st,
        _ => return Err(syn::Error::new(spec.span, "BufferLayout expects a struct")),
    };

    // Schemas
    let vertex_schema = macrokid_core::attr_schema::AttrSchema::new("vertex")
        .req_int("location")
        .opt_str("format");
    let buffer_schema = macrokid_core::attr_schema::AttrSchema::new("buffer")
        .opt_int("stride")
        .opt_str("step");

    // Parse type-level buffer attrs
    let buf = macrokid_core::common::attr_schema::scope::on_type(&spec, &buffer_schema)?;
    let step_mode = match buf.get_str("step").unwrap_or("vertex") {
        "vertex" => quote! { macrokid_graphics::resources::StepMode::Vertex },
        "instance" => quote! { macrokid_graphics::resources::StepMode::Instance },
        other => return Err(syn::Error::new(spec.span, format!("unknown step mode '{}': expected 'vertex' or 'instance'", other))),
    };

    // Collect per-field vertex attributes where present
    #[derive(Clone, Debug)]
    struct VRec { field: String, location: u32, format: Option<String>, offset: u32, size: u32, span: proc_macro2::Span }

    // Helper: infer size from format string
    fn size_from_format(fmt: &str) -> Option<usize> {
        match fmt {
            "f32" | "u32" | "i32" => Some(4),
            "vec2" => Some(8),
            "vec3" => Some(12),
            "vec4" => Some(16),
            "mat4" => Some(64),
            _ => None,
        }
    }

    // Helper: infer size from syn::Type (supports paths and arrays)
    fn size_from_type(ty: &syn::Type) -> Option<usize> {
        match ty {
            syn::Type::Path(p) => p.path.segments.last().and_then(|seg| match seg.ident.to_string().as_str() {
                "f32" | "u32" | "i32" => Some(4),
                _ => None,
            }),
            syn::Type::Array(a) => {
                let elem = &*a.elem;
                let elem_size = size_from_type(elem)?;
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(n), .. }) = &a.len {
                    let count = n.base10_parse::<usize>().ok()?;
                    Some(elem_size * count)
                } else { None }
            }
            _ => None,
        }
    }

    // Build records
    let mut recs_raw: Vec<(u32, VRec)> = Vec::new();
    match st.fields() {
        FieldKind::Named(fields) | FieldKind::Unnamed(fields) => {
            for f in fields {
                // Only fields with #[vertex(..)] are included
                if let Ok(v) = vertex_schema.parse(&f.attrs) {
                    if v.map.is_empty() { continue; }
                    let location = v.try_get_int("location")? as u32;
                    let format_str = v.get_str("format").map(|s| s.to_string());
                    let field_name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_else(|| format!("_{}", f.index));

                    // Determine size
                    let size = if let Some(ref fmt) = format_str {
                        size_from_format(fmt).ok_or_else(|| syn::Error::new(f.span, format!("unknown format '{}' for field '{}'", fmt, field_name)))?
                    } else {
                        size_from_type(&f.ty).ok_or_else(|| syn::Error::new(f.span, format!("cannot infer size for field '{}'", field_name)))?
                    } as u32;

                    recs_raw.push((location, VRec { field: field_name, location, format: format_str, offset: 0, size, span: f.span }));
                }
            }
        }
        FieldKind::Unit => {}
    }

    // Sort by location and detect duplicates
    recs_raw.sort_by_key(|(loc, _)| *loc);
    for w in recs_raw.windows(2) {
        if w[0].0 == w[1].0 {
            let span = w[1].1.span;
            return Err(syn::Error::new(span, format!("duplicate vertex location {}", w[1].0)));
        }
    }

    // Compute offsets and final records
    let mut offset_acc: u32 = 0;
    let mut recs: Vec<VRec> = Vec::new();
    for (_, mut r) in recs_raw.into_iter() {
        r.offset = offset_acc;
        offset_acc = offset_acc.saturating_add(r.size);
        recs.push(r);
    }

    // Stride from buffer attr or sum of sizes
    let stride = buf.get_int("stride").map(|v| v as u32).unwrap_or(offset_acc);

    // Emit static attrs
    let ty_attr = quote! { macrokid_graphics::resources::VertexAttr };
    let entries = recs.iter().map(|r| {
        let field = &r.field;
        let location = r.location;
        let format_s = r.format.as_deref().unwrap_or("auto");
        let offset = r.offset;
        let size = r.size;
        quote! { macrokid_graphics::resources::VertexAttr { field: #field, location: #location, format: #format_s, offset: #offset, size: #size } }
    });
    let (mod_ident, module) = codegen::static_slice_mod("vl", ty_attr.clone(), entries);

    // Trait impl for VertexLayout
    // Trait impl: both required methods
    let trait_impl = quote! {
        impl macrokid_graphics::resources::VertexLayout for #ident {
            fn vertex_attrs() -> &'static [#ty_attr] { #mod_ident::DATA }
            fn vertex_buffer() -> macrokid_graphics::resources::VertexBufferDesc {
                macrokid_graphics::resources::VertexBufferDesc { stride: #stride, step: #step_mode }
            }
        }
    };
    // Inherent methods for convenience
    let inherent = codegen::impl_inherent_methods(&spec, &[quote! {
        pub fn describe_vertex_layout() -> &'static [#ty_attr] { #mod_ident::DATA }
    }, quote! {
        pub fn describe_vertex_buffer() -> macrokid_graphics::resources::VertexBufferDesc {
            <Self as macrokid_graphics::resources::VertexLayout>::vertex_buffer()
        }
    }]);

    Ok(quote! { #module #trait_impl #inherent })
}

// ================= GraphicsPipeline derive =================

derive_entry!(GraphicsPipeline, attrs = [pipeline], handler = expand_graphics_pipeline);

fn expand_graphics_pipeline(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    // Parse type-level pipeline attributes
    let schema = macrokid_core::attr_schema::AttrSchema::new("pipeline")
        .req_str("vs")
        .req_str("fs")
        .opt_str("topology")
        .opt_bool("depth");
    let attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &schema)?;

    let vs = attrs.try_get_str("vs")?.to_string();
    let fs = attrs.try_get_str("fs")?.to_string();
    let topology_s = attrs.get_str("topology").unwrap_or("TriangleList");
    let depth = attrs.get_bool("depth").unwrap_or(true);

    let topology_tokens = match topology_s {
        "TriangleList" => quote! { macrokid_graphics::pipeline::Topology::TriangleList },
        "LineList" => quote! { macrokid_graphics::pipeline::Topology::LineList },
        "PointList" => quote! { macrokid_graphics::pipeline::Topology::PointList },
        other => return Err(syn::Error::new(spec.span, format!("unknown topology '{}': expected TriangleList|LineList|PointList", other))),
    };

    let name = ident.to_string();
    let mod_ident = syn::Ident::new(&format!("__mk_gp_{}", name), Span::call_site());
    // Prototype CodeGen usage: split module and inherent impl and chain them.
    struct GPInput {
        mod_ident: syn::Ident,
        name: String,
        vs: String,
        fs: String,
        topology: proc_macro2::TokenStream,
        depth: bool,
        ident: syn::Ident,
    }
    let gp_input = GPInput { mod_ident: mod_ident.clone(), name: name.to_string(), vs, fs, topology: topology_tokens.clone(), depth, ident: ident.clone() };

    struct ModGen;
    impl crate::gen::CodeGen<GPInput> for ModGen {
        type Output = proc_macro2::TokenStream;
        fn generate(i: &GPInput) -> Self::Output {
            let GPInput { mod_ident, name, vs, fs, topology, depth, .. } = i;
            quote! {
                #[allow(non_snake_case)]
                mod #mod_ident {
                    pub static DESC: macrokid_graphics::pipeline::PipelineDesc = macrokid_graphics::pipeline::PipelineDesc {
                        name: #name,
                        shaders: macrokid_graphics::pipeline::ShaderPaths { vs: #vs, fs: #fs },
                        topology: #topology,
                        depth: #depth,
                    };
                }
            }
        }
    }

    let trait_impl = quote! {
        impl macrokid_graphics::pipeline::PipelineInfo for #ident {
            fn pipeline_desc() -> &'static macrokid_graphics::pipeline::PipelineDesc { &#mod_ident::DESC }
        }
    };
    struct InherentGen;
    impl crate::gen::CodeGen<GPInput> for InherentGen {
        type Output = proc_macro2::TokenStream;
        fn generate(i: &GPInput) -> Self::Output {
            let ident = &i.ident;
            quote! {
                impl #ident {
                    pub fn describe_pipeline() -> &'static macrokid_graphics::pipeline::PipelineDesc { <Self as macrokid_graphics::pipeline::PipelineInfo>::pipeline_desc() }
                }
            }
        }
    }

    type Both = crate::gen::Chain<ModGen, InherentGen>;
    let chained = Both::generate(&gp_input);
    Ok(quote! { #chained #trait_impl })
}
