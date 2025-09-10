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

derive_entry!(ResourceBinding, attrs = [uniform, texture, sampler, combined], handler = expand_resource_binding);

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
        uniform(set: int, binding: int, stages: str),
        texture(set: int, binding: int, stages: str),
        sampler(set: int, binding: int, stages: str),
        combined(set: int, binding: int, stages: str),
    ];

    #[derive(Clone, Debug)]
    struct RBRec { field: String, set: u32, binding: u32, kind: String, stages: Option<String>, span: proc_macro2::Span }

    let items: Vec<RBRec> = collect::from_named_fields(st, |f| {
        if let Some((kind, parsed)) = kind_set.parse(&f.attrs)? {
            let field = f.ident.as_ref().unwrap().to_string();
            let set = parsed.try_get_int("set")? as u32;
            let binding = parsed.try_get_int("binding")? as u32;
            let stages = parsed.get_str("stages").map(|s| s.to_string());
            Ok(Some(RBRec { field, set, binding, kind, stages, span: f.span }))
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
            "sampler" => quote! { macrokid_graphics::resources::ResourceKind::Sampler },
            _ => quote! { macrokid_graphics::resources::ResourceKind::CombinedImageSampler },
        };
        let stages = if let Some(s) = &r.stages {
            let mut vs = false; let mut fs = false; let mut cs = false;
            for part in s.split(|c| c == '|' || c == ',' || c == ' ') { match part.trim().to_lowercase().as_str() {
                "vs" | "vert" | "vertex" => vs = true,
                "fs" | "frag" | "fragment" => fs = true,
                "cs" | "comp" | "compute" => cs = true,
                "" => {},
                other => {
                    // Unknown tokens are ignored to keep derive tolerant; consider erroring in future
                    let _ = other;
                }
            }}
            let vs_b = vs; let fs_b = fs; let cs_b = cs;
            quote! { Some(macrokid_graphics::resources::BindingStages { vs: #vs_b, fs: #fs_b, cs: #cs_b }) }
        } else { quote! { None } };
        quote! { macrokid_graphics::resources::BindingDesc { field: #field, set: #set, binding: #binding, kind: #kind, stages: #stages } }
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
        .opt_int("binding")
        .opt_str("format");
    let buffer_schema = macrokid_core::attr_schema::AttrSchema::new("buffer")
        .opt_int("binding")
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
    struct VRec { field: String, binding: u32, location: u32, format: Option<String>, offset: u32, size: u32, span: proc_macro2::Span }

    // Helper: infer size from format string
    fn size_from_format(fmt: &str) -> Option<usize> {
        match fmt {
            "f32" | "u32" | "i32" => Some(4),
            "vec2" => Some(8),
            "vec3" => Some(12),
            "vec4" => Some(16),
            "rgba8_unorm" | "u8x4_norm" => Some(4),
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
    let mut recs_raw: Vec<((u32, u32), VRec)> = Vec::new();
    match st.fields() {
        FieldKind::Named(fields) | FieldKind::Unnamed(fields) => {
            for f in fields {
                // Only fields with #[vertex(..)] are included
                if let Ok(v) = vertex_schema.parse(&f.attrs) {
                    if v.map.is_empty() { continue; }
                    let location = v.try_get_int("location")? as u32;
                    let binding = v.get_int("binding").unwrap_or(0) as u32;
                    let format_str = v.get_str("format").map(|s| s.to_string());
                    let field_name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_else(|| format!("_{}", f.index));

                    // Determine size
                    let size = if let Some(ref fmt) = format_str {
                        size_from_format(fmt).ok_or_else(|| syn::Error::new(f.span, format!("unknown format '{}' for field '{}'", fmt, field_name)))?
                    } else {
                        size_from_type(&f.ty).ok_or_else(|| syn::Error::new(f.span, format!("cannot infer size for field '{}'", field_name)))?
                    } as u32;

                    recs_raw.push(((binding, location), VRec { field: field_name, binding, location, format: format_str, offset: 0, size, span: f.span }));
                }
            }
        }
        FieldKind::Unit => {}
    }

    // Sort by location and detect duplicates
    recs_raw.sort_by_key(|(key, _)| *key);
    for w in recs_raw.windows(2) {
        if w[0].0 == w[1].0 {
            let span = w[1].1.span;
            return Err(syn::Error::new(span, format!("duplicate (binding, location) {:?}", w[1].0)));
        }
    }

    // Compute offsets and final records
    use std::collections::BTreeMap;
    let mut by_binding: BTreeMap<u32, Vec<VRec>> = BTreeMap::new();
    for (_, r) in recs_raw.into_iter() { by_binding.entry(r.binding).or_default().push(r); }
    for list in by_binding.values_mut() {
        list.sort_by_key(|r| r.location);
        let mut acc = 0u32;
        for r in list.iter_mut() { r.offset = acc; acc = acc.saturating_add(r.size); }
    }
    let mut recs: Vec<VRec> = by_binding.values().flat_map(|v| v.clone()).collect();

    // Stride from buffer attr or sum of sizes
    // Strides: compute per binding (sum sizes), allow override via type-level stride (applies to all bindings)
    let override_stride = buf.get_int("stride").map(|v| v as u32);
    let mut strides: BTreeMap<u32, u32> = BTreeMap::new();
    for (b, list) in by_binding.iter() {
        let total: u32 = list.iter().map(|r| r.size).sum();
        strides.insert(*b, override_stride.unwrap_or(total));
    }

    // Emit static attrs
    let ty_attr = quote! { macrokid_graphics::resources::VertexAttr };
    let entries = recs.iter().map(|r| {
        let field = &r.field;
        let binding = r.binding;
        let location = r.location;
        let format_s = r.format.as_deref().unwrap_or("auto");
        let offset = r.offset;
        let size = r.size;
        quote! { macrokid_graphics::resources::VertexAttr { field: #field, binding: #binding, location: #location, format: #format_s, offset: #offset, size: #size } }
    });
    let (mod_ident, module) = codegen::static_slice_mod("vl", ty_attr.clone(), entries);

    // Trait impl for VertexLayout
    // Trait impl: both required methods
    // Emit buffer descriptors for each binding
    let bufd_ty = quote! { macrokid_graphics::resources::VertexBufferDesc };
    let bufd_entries = strides.iter().map(|(b, s)| {
        quote! { macrokid_graphics::resources::VertexBufferDesc { binding: #b, stride: #s, step: #step_mode } }
    });
    let (buf_mod_ident, buf_mod) = codegen::static_slice_mod("vb", bufd_ty.clone(), bufd_entries);

    let trait_impl = quote! {
        impl macrokid_graphics::resources::VertexLayout for #ident {
            fn vertex_attrs() -> &'static [#ty_attr] { #mod_ident::DATA }
            fn vertex_buffers() -> &'static [#bufd_ty] { #buf_mod_ident::DATA }
        }
    };
    // Inherent methods for convenience
    let inherent = codegen::impl_inherent_methods(&spec, &[quote! {
        pub fn describe_vertex_layout() -> &'static [#ty_attr] { #mod_ident::DATA }
    }, quote! {
        pub fn describe_vertex_buffers() -> &'static [macrokid_graphics::resources::VertexBufferDesc] {
            <Self as macrokid_graphics::resources::VertexLayout>::vertex_buffers()
        }
    }]);

    Ok(quote! { #module #buf_mod #trait_impl #inherent })
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
        .opt_bool("depth")
        .opt_str("polygon")
        .opt_str("cull")
        .opt_str("front_face")
        .opt_bool("blend")
        .opt_int("samples")
        // depth/stencil extensions
        .opt_bool("depth_test")
        .opt_bool("depth_write")
        .opt_str("depth_compare")
        // dynamic states and push constants
        .opt_str("dynamic")
        .opt_int("push_constants_size")
        .opt_str("push_constants_stages");
    let attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &schema)?;

    let vs = attrs.try_get_str("vs")?.to_string();
    let fs = attrs.try_get_str("fs")?.to_string();
    let topology_s = attrs.get_str("topology").unwrap_or("TriangleList");
    let depth = attrs.get_bool("depth").unwrap_or(true);
    let polygon_s = attrs.get_str("polygon");
    let cull_s = attrs.get_str("cull");
    let front_s = attrs.get_str("front_face");
    let blend_b = attrs.get_bool("blend");
    let samples_i = attrs.get_int("samples");

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
        raster: proc_macro2::TokenStream,
        blend: proc_macro2::TokenStream,
        samples: proc_macro2::TokenStream,
        depth_stencil: proc_macro2::TokenStream,
        dynamic: proc_macro2::TokenStream,
        push_constants: proc_macro2::TokenStream,
    }
    // Build optional state tokens
    let polygon_tokens = match polygon_s.unwrap_or("Fill") {
        "Fill" => quote! { macrokid_graphics::pipeline::PolygonMode::Fill },
        "Line" => quote! { macrokid_graphics::pipeline::PolygonMode::Line },
        other => return Err(syn::Error::new(spec.span, format!("unknown polygon mode '{}': expected Fill|Line", other))),
    };
    let cull_tokens = match cull_s.unwrap_or("Back") {
        "None" => quote! { macrokid_graphics::pipeline::CullMode::None },
        "Front" => quote! { macrokid_graphics::pipeline::CullMode::Front },
        "Back" => quote! { macrokid_graphics::pipeline::CullMode::Back },
        other => return Err(syn::Error::new(spec.span, format!("unknown cull mode '{}': expected None|Front|Back", other))),
    };
    let front_tokens = match front_s.unwrap_or("Ccw") {
        "Cw" | "CW" => quote! { macrokid_graphics::pipeline::FrontFace::Cw },
        "Ccw" | "CCW" => quote! { macrokid_graphics::pipeline::FrontFace::Ccw },
        other => return Err(syn::Error::new(spec.span, format!("unknown front_face '{}': expected Cw|Ccw", other))),
    };
    let raster_tokens = quote! { Some(macrokid_graphics::pipeline::RasterState { polygon: #polygon_tokens, cull: #cull_tokens, front_face: #front_tokens }) };
    let blend_tokens = if blend_b.unwrap_or(false) { quote! { Some(macrokid_graphics::pipeline::ColorBlendState { enable: true }) } } else { quote! { None } };
    let samples_tokens = if let Some(s) = samples_i { let s = s as u32; quote! { Some(#s) } } else { quote! { None } };

    // Depth state tokens
    let compare_tokens = match attrs.get_str("depth_compare").unwrap_or("Less") {
        "Never" => quote! { macrokid_graphics::pipeline::CompareOp::Never },
        "Less" => quote! { macrokid_graphics::pipeline::CompareOp::Less },
        "Equal" => quote! { macrokid_graphics::pipeline::CompareOp::Equal },
        "LEqual" | "LessOrEqual" => quote! { macrokid_graphics::pipeline::CompareOp::LessOrEqual },
        "Greater" => quote! { macrokid_graphics::pipeline::CompareOp::Greater },
        "NotEqual" => quote! { macrokid_graphics::pipeline::CompareOp::NotEqual },
        "GEqual" | "GreaterOrEqual" => quote! { macrokid_graphics::pipeline::CompareOp::GreaterOrEqual },
        "Always" => quote! { macrokid_graphics::pipeline::CompareOp::Always },
        other => return Err(syn::Error::new(spec.span, format!("unknown depth_compare '{}': expected Never|Less|Equal|LessOrEqual|Greater|NotEqual|GreaterOrEqual|Always", other))),
    };
    let dt = attrs.get_bool("depth_test").unwrap_or(false);
    let dw = attrs.get_bool("depth_write").unwrap_or(false);
    let depth_tokens = if dt || dw { quote! { Some(macrokid_graphics::pipeline::DepthState { test: #dt, write: #dw, compare: #compare_tokens }) } } else { quote! { None } };

    // Dynamic states tokens
    let dynamic_tokens = if let Some(d) = attrs.get_str("dynamic") {
        let mut vp = false; let mut sc = false;
        for part in d.split(|c| c=='|'||c==','||c==' ') { match part.trim().to_lowercase().as_str() { "viewport" => vp = true, "scissor" => sc = true, _ => {} } }
        let vp_b = vp; let sc_b = sc;
        quote! { Some(macrokid_graphics::pipeline::DynamicStateDesc { viewport: #vp_b, scissor: #sc_b }) }
    } else { quote! { None } };

    // Push constants tokens
    let pc_tokens = if let Some(sz) = attrs.get_int("push_constants_size") { 
        let stages = if let Some(s) = attrs.get_str("push_constants_stages") { 
            let mut vs = false; let mut fs = false; let mut cs = false;
            for part in s.split(|c| c=='|'||c==','||c==' ') { match part.trim().to_lowercase().as_str() { "vs"|"vert"|"vertex"=>vs=true, "fs"|"frag"|"fragment"=>fs=true, "cs"|"comp"|"compute"=>cs=true, _=>{} } }
            let vsb=vs; let fsb=fs; let csb=cs;
            quote! { Some(macrokid_graphics::pipeline::StageMask { vs: #vsb, fs: #fsb, cs: #csb }) }
        } else { quote! { None } };
        let sz = sz as u32;
        quote! { Some(macrokid_graphics::pipeline::PushConstantRange { size: #sz, stages: #stages }) }
    } else { quote! { None } };

    let gp_input = GPInput {
        mod_ident: mod_ident.clone(),
        name: name.to_string(),
        vs,
        fs,
        topology: topology_tokens.clone(),
        depth,
        ident: ident.clone(),
        raster: raster_tokens,
        blend: blend_tokens,
        samples: samples_tokens,
        depth_stencil: depth_tokens,
        dynamic: dynamic_tokens,
        push_constants: pc_tokens,
    };

    struct ModGen;
    impl crate::gen::CodeGen<GPInput> for ModGen {
        type Output = proc_macro2::TokenStream;
        fn generate(i: &GPInput) -> Self::Output {
            let GPInput { mod_ident, name, vs, fs, topology, depth, raster, blend, samples, depth_stencil, dynamic, push_constants, .. } = i;
            quote! {
                #[allow(non_snake_case)]
                mod #mod_ident {
                    pub static DESC: macrokid_graphics::pipeline::PipelineDesc = macrokid_graphics::pipeline::PipelineDesc {
                        name: #name,
                        shaders: macrokid_graphics::pipeline::ShaderPaths { vs: #vs, fs: #fs },
                        topology: #topology,
                        depth: #depth,
                        raster: #raster,
                        blend: #blend,
                        samples: #samples,
                        depth_stencil: #depth_stencil,
                        dynamic: #dynamic,
                        push_constants: #push_constants,
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

// ================= RenderEngine derive =================

derive_entry!(RenderEngine, attrs = [app, window, use_pipeline], handler = expand_render_engine);

fn expand_render_engine(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    use macrokid_core::ir::TypeKind;
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    // Type-level attributes: app(name), window(width,height,vsync)
    let app_schema = macrokid_core::attr_schema::AttrSchema::new("app").opt_str("name");
    let win_schema = macrokid_core::attr_schema::AttrSchema::new("window")
        .opt_int("width")
        .opt_int("height")
        .opt_bool("vsync");
    let app_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &app_schema)?;
    let win_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &win_schema)?;

    let app_name = app_attrs.get_str("name").unwrap_or("Untitled");
    let width = win_attrs.get_int("width").unwrap_or(1280) as u32;
    let height = win_attrs.get_int("height").unwrap_or(720) as u32;
    let vsync = win_attrs.get_bool("vsync").unwrap_or(true);

    // Fields: any field marked with #[use_pipeline] will be treated as a pipeline type
    // that implements macrokid_graphics::pipeline::PipelineInfo. We collect their descs.
    let use_schema = macrokid_core::attr_schema::AttrSchema::new("use_pipeline");

    let mut pipeline_ty_tokens: Vec<proc_macro2::TokenStream> = Vec::new();
    match &spec.kind {
        TypeKind::Struct(st) => {
            match st.fields() {
                FieldKind::Named(fields) | FieldKind::Unnamed(fields) => {
                    for f in fields {
                        if let Ok(v) = use_schema.parse(&f.attrs) {
                            if v.map.is_empty() { continue; }
                            // Use the field type from syn metadata
                            let ty = &f.ty;
                            let ts = quote! { <#ty as macrokid_graphics::pipeline::PipelineInfo>::pipeline_desc() };
                            pipeline_ty_tokens.push(ts);
                        }
                    }
                }
                FieldKind::Unit => {}
            }
        }
        _ => return Err(syn::Error::new(spec.span, "RenderEngine expects a struct")),
    }

    // If no fields marked, allow empty pipelines (user can add later), but likely a mistake.
    // Build EngineConfig at call-site by cloning PipelineDesc values from PipelineInfo types.
    let app_s = app_name.to_string();
    let gen = quote! {
        impl macrokid_graphics::engine::RenderEngineInfo for #ident {
            fn engine_config() -> macrokid_graphics::engine::EngineConfig {
                let mut pipelines: ::std::vec::Vec<macrokid_graphics::pipeline::PipelineDesc> = ::std::vec::Vec::new();
                #( pipelines.push((#pipeline_ty_tokens).clone()); )*
                macrokid_graphics::engine::EngineConfig {
                    app: #app_s,
                    window: macrokid_graphics::engine::WindowCfg { width: #width, height: #height, vsync: #vsync },
                    pipelines,
                }
            }
        }
        impl #ident {
            pub fn engine_config() -> macrokid_graphics::engine::EngineConfig { <Self as macrokid_graphics::engine::RenderEngineInfo>::engine_config() }
        }
    };
    Ok(gen)
}
