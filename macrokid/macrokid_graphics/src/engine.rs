//! Minimal engine/runtime scaffolding aligned with macrokid_core patterns.
//!
//! Goals:
//! - Keep runtime types close to the derives provided by macrokid_graphics_derive.
//! - Provide a simple backend trait and an Engine wrapper.
//! - Offer validation helpers that work with ResourceBindings and VertexLayout.
//!
//! Threading & design overview:
//! - Intended to support multi-threaded recording with single-threaded submission.
//! - See `ENGINE_RUNTIME.md` for the broader plan (Renderer/Frame scaffolding, job system).
//!
//! Notes:
//! - This module intentionally avoids windowing/device lifetimes; it focuses on
//!   structuring and validating pipeline descriptions.

use crate::pipeline::PipelineDesc;
use macrokid_core::common::validate::Validator;
use crate::resources::{ResourceBindings, VertexLayout};

#[derive(Clone, Debug)]
pub struct WindowCfg { pub width: u32, pub height: u32, pub vsync: bool }

/// Generic, backend-agnostic knobs the runtime can honor.
/// Backends may ignore fields they don't support; defaults match current behavior.
#[derive(Clone, Debug, Default)]
pub struct BackendOptions {
    /// Requested present mode, e.g. "FIFO", "MAILBOX", "IMMEDIATE".
    /// When `None`, vsync determines a sensible default.
    pub present_mode: Option<&'static str>,
    /// Preferred swapchain image count; clamped to surface capabilities.
    pub swapchain_images: Option<u32>,
    /// Requested color format, e.g. "B8G8R8A8_SRGB". Fallbacks if unavailable.
    pub color_format: Option<&'static str>,
    /// Requested color space, e.g. "SRGB_NONLINEAR".
    pub color_space: Option<&'static str>,
    /// Requested depth format, e.g. "D32_SFLOAT".
    pub depth_format: Option<&'static str>,
    /// MSAA sample count for attachments/pipelines, e.g. 1,2,4,8.
    pub msaa_samples: Option<u32>,
    /// Enable dynamic viewport state.
    pub dynamic_viewport: Option<bool>,
    /// Enable dynamic scissor state.
    pub dynamic_scissor: Option<bool>,
    /// Preferred present mode order, first supported wins.
    pub present_mode_priority: Option<Vec<&'static str>>,
    /// Preferred adapter index (in enumeration order) if available.
    pub adapter_index: Option<usize>,
    /// Preferred adapter kind: "discrete" | "integrated" | "virtual" | "cpu".
    pub adapter_preference: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub app: &'static str,
    pub window: WindowCfg,
    pub pipelines: Vec<PipelineDesc>,
    pub options: BackendOptions,
}

/// Backend abstraction for creating pipelines and presenting frames.
/// Backend abstraction for creating pipelines and presenting frames.
///
/// Thread-safety: backends are expected to be `Send + Sync + 'static` to enable
/// sharing handles across threads when recording work.
pub trait RenderBackend: Send + Sync + 'static {
    fn name() -> &'static str;
    fn create_device() { println!("[{}] create_device()", Self::name()); }
    fn create_pipeline(desc: &PipelineDesc) {
        println!(
            "[{}] create_pipeline: {} (vs={}, fs={}, topo={:?}, depth={})",
            Self::name(), desc.name, desc.shaders.vs, desc.shaders.fs, desc.topology, desc.depth
        );
    }
    fn present() { println!("[{}] present()", Self::name()); }
}

/// Example Vulkan marker backend.
pub struct VulkanBackend;
impl RenderBackend for VulkanBackend { fn name() -> &'static str { "vulkan" } }

/// Thin façade tagged by backend type.
///
/// `PhantomData<B>` intentionally couples Engine's auto-traits (Send/Sync) to the backend
/// so that Engine can only be sent/shared if the backend supports it.
pub struct Engine<B: RenderBackend> { _backend: core::marker::PhantomData<B> }

impl<B: RenderBackend> Engine<B> {
    pub fn new_from_config(_cfg: &EngineConfig) -> Self {
        B::create_device();
        Self { _backend: core::marker::PhantomData }
    }

    /// Initialize pipelines as described in the config.
    pub fn init_pipelines(&self, cfg: &EngineConfig) {
        for p in cfg.pipelines.iter() { B::create_pipeline(p); }
    }

    /// Validate shader-facing resources and vertex layout against the engine config.
    /// This is a structural validation intended to catch obvious mismatches early.
    pub fn validate_pipelines_with<RB, VL>(&self, cfg: &EngineConfig) -> Result<(), EngineValidateError>
    where
        RB: ResourceBindings,
        VL: VertexLayout,
    {
        let rb = RB::bindings();
        let vl = VL::vertex_attrs();
        let vbs = VL::vertex_buffers();
        if rb.is_empty() { return Err(EngineValidateError::NoBindings); }
        if vl.is_empty() { return Err(EngineValidateError::NoVertexAttrs); }
        for p in cfg.pipelines.iter() {
            // In a real system, validate descriptor sets vs shader reflection and vertex layout vs shader inputs.
            let stride0 = vbs.first().map(|b| b.stride).unwrap_or(0);
            let _step0 = vbs.first().map(|b| &b.step);
            println!("validate '{}': bindings={} attrs={} vbufs={} stride0={} shaders=({}, {}) topo={:?}",
                p.name, rb.len(), vl.len(), vbs.len(), stride0, p.shaders.vs, p.shaders.fs, p.topology);
        }
        Ok(())
    }

    pub fn frame(&self) { B::present(); }
}

#[derive(Debug)]
pub enum EngineValidateError { NoBindings, NoVertexAttrs }

/// Basic config validation akin to the gfx_dsl_support version.
pub fn validate_config(cfg: &EngineConfig) -> Result<(), ConfigError> {
    use std::collections::HashSet;
    if cfg.pipelines.is_empty() { return Err(ConfigError::NoPipelines); }
    let mut seen: HashSet<&'static str> = HashSet::new();
    for p in &cfg.pipelines {
        if p.shaders.vs.is_empty() { return Err(ConfigError::EmptyShaderPath { pipeline: p.name, which: "vs" }); }
        if p.shaders.fs.is_empty() { return Err(ConfigError::EmptyShaderPath { pipeline: p.name, which: "fs" }); }
        if !seen.insert(p.name) { return Err(ConfigError::DuplicatePipeline { pipeline: p.name }); }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    NoPipelines,
    EmptyShaderPath { pipeline: &'static str, which: &'static str },
    DuplicatePipeline { pipeline: &'static str },
}

/// A small, chainable builder to produce EngineConfig without extra macros.
pub struct EngineBuilder {
    app: Option<&'static str>,
    window: Option<WindowCfg>,
    pipelines: Vec<PipelineDesc>,
    options: BackendOptions,
}

impl EngineBuilder {
    pub fn new() -> Self { Self { app: None, window: None, pipelines: Vec::new(), options: BackendOptions::default() } }
    pub fn app(mut self, name: &'static str) -> Self { self.app = Some(name); self }
    pub fn window(mut self, width: u32, height: u32, vsync: bool) -> Self { self.window = Some(WindowCfg { width, height, vsync }); self }
    pub fn add_pipeline(mut self, desc: PipelineDesc) -> Self { self.pipelines.push(desc); self }
    /// Replace all backend options at once.
    pub fn options(mut self, options: BackendOptions) -> Self { self.options = options; self }
    /// Convenience setters for common options
    pub fn present_mode(mut self, mode: &'static str) -> Self { self.options.present_mode = Some(mode); self }
    pub fn swapchain_images(mut self, count: u32) -> Self { self.options.swapchain_images = Some(count); self }
    pub fn color_format(mut self, fmt: &'static str) -> Self { self.options.color_format = Some(fmt); self }
    pub fn color_space(mut self, cs: &'static str) -> Self { self.options.color_space = Some(cs); self }
    pub fn depth_format(mut self, fmt: &'static str) -> Self { self.options.depth_format = Some(fmt); self }
    pub fn msaa_samples(mut self, samples: u32) -> Self { self.options.msaa_samples = Some(samples); self }
    pub fn dynamic_viewport(mut self, enabled: bool) -> Self { self.options.dynamic_viewport = Some(enabled); self }
    pub fn dynamic_scissor(mut self, enabled: bool) -> Self { self.options.dynamic_scissor = Some(enabled); self }
    pub fn present_mode_priority(mut self, modes: Vec<&'static str>) -> Self { self.options.present_mode_priority = Some(modes); self }
    pub fn adapter_index(mut self, index: usize) -> Self { self.options.adapter_index = Some(index); self }
    pub fn adapter_preference(mut self, pref: &'static str) -> Self { self.options.adapter_preference = Some(pref); self }
    pub fn build(self) -> Result<EngineConfig, ConfigError> {
        let cfg = EngineConfig {
            app: self.app.unwrap_or("Untitled"),
            window: self.window.unwrap_or(WindowCfg { width: 1280, height: 720, vsync: true }),
            pipelines: self.pipelines,
            options: self.options,
        };
        validate_config(&cfg)?;
        Ok(cfg)
    }
}

impl BackendOptions {
    /// Populate options from environment variables (best-effort parsing).
    ///
    /// Recognized vars:
    /// - MK_PRESENT_MODE, MK_PRESENT_MODE_PRIORITY (comma-separated)
    /// - MK_SWAPCHAIN_IMAGES, MK_COLOR_FORMAT, MK_COLOR_SPACE, MK_DEPTH_FORMAT, MK_MSAA_SAMPLES
    /// - MK_DYNAMIC_VIEWPORT, MK_DYNAMIC_SCISSOR
    /// - MK_ADAPTER_INDEX, MK_ADAPTER_PREFERENCE
    pub fn from_env() -> Self {
        use std::env;
        fn parse_bool(s: &str) -> Option<bool> {
            match s.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }
        }
        fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }
        let mut opts = BackendOptions::default();
        if let Ok(v) = env::var("MK_PRESENT_MODE") { if !v.is_empty() { opts.present_mode = Some(leak(v)); } }
        if let Ok(v) = env::var("MK_PRESENT_MODE_PRIORITY") {
            if !v.is_empty() {
                let modes: Vec<&'static str> = v.split(',').map(|s| leak(s.trim().to_string())).collect();
                if !modes.is_empty() { opts.present_mode_priority = Some(modes); }
            }
        }
        if let Ok(v) = env::var("MK_SWAPCHAIN_IMAGES") { if let Ok(n) = v.parse::<u32>() { opts.swapchain_images = Some(n); } }
        if let Ok(v) = env::var("MK_COLOR_FORMAT") { if !v.is_empty() { opts.color_format = Some(leak(v)); } }
        if let Ok(v) = env::var("MK_COLOR_SPACE") { if !v.is_empty() { opts.color_space = Some(leak(v)); } }
        if let Ok(v) = env::var("MK_DEPTH_FORMAT") { if !v.is_empty() { opts.depth_format = Some(leak(v)); } }
        if let Ok(v) = env::var("MK_MSAA_SAMPLES") { if let Ok(n) = v.parse::<u32>() { opts.msaa_samples = Some(n); } }
        if let Ok(v) = env::var("MK_DYNAMIC_VIEWPORT") { if let Some(b) = parse_bool(&v) { opts.dynamic_viewport = Some(b); } }
        if let Ok(v) = env::var("MK_DYNAMIC_SCISSOR") { if let Some(b) = parse_bool(&v) { opts.dynamic_scissor = Some(b); } }
        if let Ok(v) = env::var("MK_ADAPTER_INDEX") { if let Ok(n) = v.parse::<usize>() { opts.adapter_index = Some(n); } }
        if let Ok(v) = env::var("MK_ADAPTER_PREFERENCE") { if !v.is_empty() { opts.adapter_preference = Some(leak(v)); } }
        opts
    }

    /// Merge environment-provided options into `self` as fallbacks.
    /// Fields already set on `self` are preserved; unset fields adopt env values.
    pub fn with_env_fallback(mut self) -> Self {
        let env = BackendOptions::from_env();
        macro_rules! take_if_none {
            ($field:ident) => {
                if self.$field.is_none() { self.$field = env.$field; }
            };
        }
        take_if_none!(present_mode);
        take_if_none!(swapchain_images);
        take_if_none!(color_format);
        take_if_none!(color_space);
        take_if_none!(depth_format);
        take_if_none!(msaa_samples);
        take_if_none!(dynamic_viewport);
        take_if_none!(dynamic_scissor);
        take_if_none!(present_mode_priority);
        take_if_none!(adapter_index);
        take_if_none!(adapter_preference);
        self
    }

    /// Print a concise, effective summary of options, including implied defaults.
    pub fn log_effective(&self, window: &WindowCfg) {
        fn or_default<T: std::fmt::Display>(opt: &Option<T>, default: &str) -> String {
            match opt {
                Some(v) => v.to_string(),
                None => default.to_string(),
            }
        }
        let pm_default = if window.vsync { "(default: FIFO via vsync)" } else { "(default: MAILBOX via vsync=false)" };
        let pm = match (&self.present_mode, &self.present_mode_priority) {
            (_, Some(list)) if !list.is_empty() => format!("priority=[{}]", list.join(",")),
            (Some(m), _) => m.to_string(),
            _ => pm_default.to_string(),
        };
        let sc_images = or_default(&self.swapchain_images, "(default: min+1, clamped)");
        let color_fmt = or_default(&self.color_format, "(default: prefer B8G8R8A8_SRGB)");
        let color_space = or_default(&self.color_space, "(default: SRGB_NONLINEAR if available)");
        let depth_fmt = or_default(&self.depth_format, "(default: prefer D32_SFLOAT)");
        let msaa = or_default(&self.msaa_samples, "(default: 1x)");
        let dyn_vp = match self.dynamic_viewport { Some(b) => b.to_string(), None => "(pipeline-driven)".into() };
        let dyn_sc = match self.dynamic_scissor { Some(b) => b.to_string(), None => "(pipeline-driven)".into() };
        let adapter_idx = or_default(&self.adapter_index, "(any)");
        let adapter_pref = or_default(&self.adapter_preference, "(none)");
        println!(
            "[gfx] BackendOptions: present_mode={} | swapchain_images={} | color_format={} | color_space={} | depth_format={} | msaa={} | dynamic_viewport={} | dynamic_scissor={} | adapter_index={} | adapter_preference={}",
            pm, sc_images, color_fmt, color_space, depth_fmt, msaa, dyn_vp, dyn_sc, adapter_idx, adapter_pref
        );
    }
}

/// Generic graphics validator that plugs into macrokid_core::common::validate.
///
/// Usage:
///   cfg.validate_with::<GraphicsValidator<Material, Vertex>>()?;
/// where `Material: ResourceBindings` and `Vertex: VertexLayout` are derive outputs.
pub struct GraphicsValidator<RB, VL>(core::marker::PhantomData<(RB, VL)>);

impl<RB, VL> Validator<EngineConfig> for GraphicsValidator<RB, VL>
where
    RB: crate::resources::ResourceBindings,
    VL: crate::resources::VertexLayout,
{
    type Error = EngineValidateError;
    fn validate(cfg: &EngineConfig) -> Result<(), Self::Error> {
        // Backend choice is irrelevant for validation; we reuse Engine's method.
        let engine = Engine::<VulkanBackend>::new_from_config(cfg);
        engine.validate_pipelines_with::<RB, VL>(cfg)
    }
}

/// Trait for derive-driven engine assembly.
///
/// Types implementing this can produce an `EngineConfig` constructed from
/// statically described pipelines and window/app metadata.
pub trait RenderEngineInfo {
    fn engine_config() -> EngineConfig;
}

// ======================
// Minimal Renderer/Frame scaffolding (forward-looking)
// ======================

/// Thread-safe façade that governs frame lifetimes and encoders.
///
/// Implementations may return backend-specific `Frame`/`CommandCtx` types
/// and are free to parallelize recording internally.
pub trait Renderer: Send + Sync {
    type Frame: Frame;
    fn begin_frame(&self) -> Self::Frame;
    fn end_frame(&self, frame: Self::Frame);
}

/// Per-frame lifetime guard. Creates recording contexts for passes/subpasses.
pub trait Frame {
    /// Backend-specific command recording context (likely !Send).
    type CommandCtx;
    /// Acquire a recording context for a named pass.
    fn encoder_for(&self, _pass: &'static str) -> Self::CommandCtx;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{PipelineDesc, ShaderPaths, Topology};

    #[test]
    fn builder_and_validation_work() {
        let cfg = EngineBuilder::new()
            .app("Demo")
            .window(800, 600, true)
            .add_pipeline(PipelineDesc { name: "triangle", shaders: ShaderPaths { vs: "vs", fs: "fs" }, topology: Topology::TriangleList, depth: true, raster: None, blend: None, samples: None, depth_stencil: None, dynamic: None, push_constants: None, color_targets: None, depth_target: None })
            .build()
            .expect("valid");
        assert_eq!(cfg.window.width, 800);
        assert_eq!(cfg.pipelines.len(), 1);
        // Validate RB/VL heuristics using types from resources module would be integration-level; unit test basic only.
    }
}
