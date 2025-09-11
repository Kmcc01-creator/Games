use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(LightingModel, attributes(model))]
pub fn derive_lighting_model(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let ident = &ast.ident;
    // Parse #[model = "phong" | "pbr" | "blinn"] (defaults to phong)
    let mut model = String::from("phong");
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("model") {
            if let Ok(meta) = attr.meta.clone().require_name_value() {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &meta.value {
                    model = s.value();
                }
            }
        }
    }
    let (vs_src, fs_src) = match model.to_ascii_lowercase().as_str() {
        // For MVP, all models share the same minimalist shaders
        _ => (
            quote! { macrokid_graphics_lighting::default_shaders::VS_POS_UV },
            quote! { macrokid_graphics_lighting::default_shaders::FS_PHONG_MIN },
        ),
    };
    let rb_ident = syn::Ident::new(&format!("{}Bindings", ident), ident.span());
    let gen = quote! {
        // Generated ResourceBindings type for this lighting model
        pub struct #rb_ident;
        impl macrokid_graphics::resources::ResourceBindings for #rb_ident {
            fn bindings() -> &'static [macrokid_graphics::resources::BindingDesc] {
                use macrokid_graphics::resources::{BindingDesc, ResourceKind, BindingStages};
                static B: [BindingDesc; 2] = [
                    BindingDesc { field: "scene", set: 0, binding: 0, kind: ResourceKind::Uniform, stages: Some(BindingStages { vs: true, fs: true, cs: false }) },
                    BindingDesc { field: "albedo", set: 0, binding: 1, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages { vs: false, fs: true, cs: false }) },
                ];
                &B
            }
        }

        impl macrokid_graphics_lighting::LightingModel for #ident {
            fn shader_sources() -> macrokid_graphics_lighting::ShaderSources {
                macrokid_graphics_lighting::ShaderSources { vs: #vs_src, fs: #fs_src }
            }
        }
        impl macrokid_graphics_lighting::HasBindings for #ident { type RB = #rb_ident; }
        impl #ident {
            pub fn shader_sources() -> macrokid_graphics_lighting::ShaderSources { <Self as macrokid_graphics_lighting::LightingModel>::shader_sources() }
            pub fn bindings() -> &'static [macrokid_graphics::resources::BindingDesc] { <#rb_ident as macrokid_graphics::resources::ResourceBindings>::bindings() }
        }
    };
    gen.into()
}

#[proc_macro_derive(LightSetup, attributes(light_setup))]
pub fn derive_light_setup(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let ident = &ast.ident;
    // Parse optional #[light_setup(shadow_size = "WxH")]
    let mut shadow_w: u32 = 2048;
    let mut shadow_h: u32 = 2048;
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("light_setup") {
            if let Ok(list) = attr.meta.clone().require_list() {
                let tokens = list.tokens.to_string();
                // extremely simple parse: look for shadow_size = "WxH"
                if let Some(eq) = tokens.find("shadow_size") {
                    if let Some(start) = tokens[eq..].find('"') {
                        let a = eq + start + 1;
                        if let Some(end_rel) = tokens[a..].find('"') {
                            let b = a + end_rel;
                            let val = &tokens[a..b];
                            if let Some(xpos) = val.find('x') { if let (Ok(w), Ok(h)) = (val[..xpos].parse::<u32>(), val[xpos+1..].parse::<u32>()) { shadow_w = w; shadow_h = h; } }
                        }
                    }
                }
            }
        }
    }

    // Generate a SceneBindings type (set=1, binding=0 uniform)
    let scene_bind_ident = syn::Ident::new(&format!("{}SceneBindings", ident), ident.span());
    let mod_ident = syn::Ident::new(&format!("__mk_ls_{}_shadow", ident), ident.span());
    let name_str = "shadow_depth".to_string();
    let gen = quote! {
        impl macrokid_graphics_lighting::LightSetup for #ident {}

        pub struct #scene_bind_ident;
        impl macrokid_graphics::resources::ResourceBindings for #scene_bind_ident {
            fn bindings() -> &'static [macrokid_graphics::resources::BindingDesc] {
                use macrokid_graphics::resources::{BindingDesc, ResourceKind, BindingStages};
                static B: [BindingDesc; 1] = [
                    BindingDesc { field: "scene_lights", set: 1, binding: 0, kind: ResourceKind::Uniform, stages: Some(BindingStages { vs: true, fs: true, cs: false }) },
                ];
                &B
            }
        }

        #[allow(non_snake_case)]
        mod #mod_ident {
            pub static __OUTS: &[macrokid_graphics::render_graph::OutputDesc] = &[
                macrokid_graphics::render_graph::OutputDesc { name: "shadow_depth", format: "D32_SFLOAT", size: macrokid_graphics::render_graph::SizeSpec::Abs { width: #shadow_w, height: #shadow_h }, usage: macrokid_graphics::render_graph::UsageMask::DEPTH | macrokid_graphics::render_graph::UsageMask::SAMPLED, samples: 1, is_depth: true },
            ];
            pub static DESC: macrokid_graphics::render_graph::PassDesc = macrokid_graphics::render_graph::PassDesc {
                name: "shadow_depth",
                kind: macrokid_graphics::render_graph::PassKind::Graphics,
                color: None,
                depth: Some(macrokid_graphics::pipeline::DepthTargetDesc { format: "D32_SFLOAT" }),
                inputs: None,
                outputs: Some(__OUTS),
            };
        }
        impl #ident {
            pub fn shadow_pass() -> &'static macrokid_graphics::render_graph::PassDesc { &#mod_ident::DESC }
            pub fn scene_bindings() -> &'static [macrokid_graphics::resources::BindingDesc] { <#scene_bind_ident as macrokid_graphics::resources::ResourceBindings>::bindings() }
        }
    };
    gen.into()
}
