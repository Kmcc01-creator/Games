use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitBool, LitInt, LitStr, Result, Token,
};

// Minimal Vulkan-first graphics DSL as a token-based macro.
// Goal: generate boilerplate-light engine scaffolding and a backend trait impl.

// ===================== AST =====================
#[derive(Debug, Clone)]
struct EngineCfgAst {
    app: Option<LitStr>,
    window: Option<WindowCfgAst>,
    graph: Option<GraphCfgAst>,
}

#[derive(Debug, Clone)]
struct WindowCfgAst {
    width: Option<LitInt>,
    height: Option<LitInt>,
    vsync: Option<LitBool>,
}

#[derive(Debug, Clone)]
struct GraphCfgAst {
    passes: Vec<PassCfgAst>,
}

#[derive(Debug, Clone)]
struct PassCfgAst {
    name: Ident,
    pipelines: Vec<PipelineCfgAst>,
}

#[derive(Debug, Clone)]
struct PipelineCfgAst {
    name: Ident,
    vs: Option<LitStr>,
    fs: Option<LitStr>,
    topology: Option<Ident>,
    depth: Option<LitBool>,
}

impl Parse for EngineCfgAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);
        let mut app = None;
        let mut window = None;
        let mut graph = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "app" => {
                    app = Some(content.parse()?);
                }
                "window" => {
                    window = Some(content.parse()?);
                }
                "graph" => {
                    graph = Some(content.parse()?);
                }
                _ => return Err(syn::Error::new_spanned(key, "unknown key in engine config")),
            }
            let _ = content.parse::<Token![,]>();
        }

        Ok(Self { app, window, graph })
    }
}

impl Parse for WindowCfgAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);
        let mut width = None;
        let mut height = None;
        let mut vsync = None;
        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "width" => width = Some(content.parse()?),
                "height" => height = Some(content.parse()?),
                "vsync" => vsync = Some(content.parse()?),
                _ => return Err(syn::Error::new_spanned(key, "unknown key in window config")),
            }
            let _ = content.parse::<Token![,]>();
        }
        Ok(Self { width, height, vsync })
    }
}

impl Parse for GraphCfgAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);
        let mut passes = Vec::new();
        while !content.is_empty() {
            let keyword: Ident = content.parse()?;
            if keyword != "pass" {
                return Err(syn::Error::new_spanned(keyword, "expected `pass`"));
            }
            let name: Ident = content.parse()?;
            let pass: PassCfgAst = {
                let inner;
                braced!(inner in content);
                let mut pipelines = Vec::new();
                while !inner.is_empty() {
                    let key: Ident = inner.parse()?;
                    inner.parse::<Token![:]>()?;
                    match key.to_string().as_str() {
                        "pipelines" => {
                            let bracketed;
                            syn::bracketed!(bracketed in inner);
                            while !bracketed.is_empty() {
                                let kw: Ident = bracketed.parse()?;
                                if kw != "pipeline" {
                                    return Err(syn::Error::new_spanned(kw, "expected `pipeline`"));
                                }
                                let pname: Ident = bracketed.parse()?;
                                let p:
                                    PipelineCfgAst = {
                                        let pcontent; braced!(pcontent in bracketed);
                                        let mut vs = None; let mut fs = None; let mut topology = None; let mut depth = None;
                                        while !pcontent.is_empty() {
                                            let k: Ident = pcontent.parse()?;
                                            pcontent.parse::<Token![:]>()?;
                                            match k.to_string().as_str() {
                                                "vs" => vs = Some(pcontent.parse()?),
                                                "fs" => fs = Some(pcontent.parse()?),
                                                "topology" => topology = Some(pcontent.parse()?),
                                                "depth" => depth = Some(pcontent.parse()?),
                                                _ => return Err(syn::Error::new_spanned(k, "unknown pipeline key")),
                                            }
                                            let _ = pcontent.parse::<Token![,]>();
                                        }
                                        PipelineCfgAst { name: pname, vs, fs, topology, depth }
                                    };
                                pipelines.push(p);
                                let _ = bracketed.parse::<Token![,]>();
                            }
                        }
                        _ => return Err(syn::Error::new_spanned(key, "unknown key in pass")),
                    }
                    let _ = inner.parse::<Token![,]>();
                }
                PassCfgAst { name, pipelines }
            };
            passes.push(pass);
            let _ = content.parse::<Token![,]>();
        }
        Ok(Self { passes })
    }
}

// ===================== Codegen helpers =====================
struct CfgTokens { cfg_mod: proc_macro2::TokenStream }

impl EngineCfgAst {
    fn to_tokens(&self) -> CfgTokens {
        let app_title = self
            .app
            .as_ref()
            .map(|s| s.value())
            .unwrap_or_else(|| "macrokid Vulkan App".to_string());

        let (w, h, vsync) = self
            .window
            .as_ref()
            .map(|w| {
                (
                    w.width.as_ref().map(|x| x.base10_parse::<u32>().unwrap_or(1280)).unwrap_or(1280),
                    w.height.as_ref().map(|x| x.base10_parse::<u32>().unwrap_or(720)).unwrap_or(720),
                    w.vsync.as_ref().map(|x| x.value).unwrap_or(true),
                )
            })
            .unwrap_or((1280, 720, true));

        // Flatten pipelines for a simple runtime description
        let mut pp = Vec::new();
        if let Some(graph) = &self.graph {
            for pass in &graph.passes {
                let pname = pass.name.to_string();
                for p in &pass.pipelines {
                    let nm = p.name.to_string();
                    let vs = p.vs.as_ref().map(|s| s.value()).unwrap_or_default();
                    let fs = p.fs.as_ref().map(|s| s.value()).unwrap_or_default();
                    let topo = p.topology.as_ref().map(|i| i.to_string()).unwrap_or("TriangleList".into());
                    let depth = p.depth.as_ref().map(|b| b.value).unwrap_or(true);
                    pp.push((pname.clone(), nm, vs, fs, topo, depth));
                }
            }
        }

        let pass_defs = pp.iter().map(|(pass, name, vs, fs, topo, depth)| {
            let pass_lit = LitStr::new(pass, Span::call_site());
            let name_lit = LitStr::new(name, Span::call_site());
            let vs_lit = LitStr::new(vs, Span::call_site());
            let fs_lit = LitStr::new(fs, Span::call_site());
            let topo_ident = Ident::new(topo, Span::call_site());
            let depth_b = LitBool::new(*depth, Span::call_site());
            quote! {
                PipelineDesc {
                    pass: #pass_lit,
                    name: #name_lit,
                    shaders: ShaderPaths { vs: #vs_lit, fs: #fs_lit },
                    topology: Topology::#topo_ident,
                    depth: #depth_b,
                }
            }
        });

        // Trait, impl and engine scaffolding using ImplBuilder for flavor
        let cfg_mod = quote! {
            #[allow(non_camel_case_types)]
            pub mod mgfx_cfg {
                pub use gfx_dsl_support::ir::*;
                pub const CONFIG: EngineConfig = EngineConfig {
                    app: #app_title,
                    window: WindowCfg { width: #w, height: #h, vsync: #vsync },
                    pipelines: &[ #( #pass_defs ),* ],
                };
            }
        };
        CfgTokens { cfg_mod }
    }
}

// ===================== vk_engine! =====================
#[proc_macro]
pub fn vk_engine(input: TokenStream) -> TokenStream {
    let cfg = parse_macro_input!(input as EngineCfgAst);
    let parts = cfg.to_tokens();
    let CfgTokens { cfg_mod } = parts;
    let out = quote! {
        #cfg_mod
    };
    out.into()
}
