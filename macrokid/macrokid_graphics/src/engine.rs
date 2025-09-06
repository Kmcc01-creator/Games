//! Minimal engine/runtime scaffolding inspired by the gfx DSL support crate,
//! ported into macrokid_graphics and aligned with current macrokid_core patterns.
//!
//! Goals:
//! - Keep runtime types close to the derives provided by macrokid_graphics_derive
//! - Provide a simple backend trait and an Engine wrapper
//! - Offer validation helpers that work with ResourceBindings and VertexLayout
//!
//! Notes:
//! - This module intentionally avoids any windowing or device lifetimes; it focuses on
//!   structuring and validating pipeline descriptions.

use crate::pipeline::PipelineDesc;
use macrokid_core::common::validate::Validator;
use crate::resources::{ResourceBindings, VertexLayout};

#[derive(Clone, Debug)]
pub struct WindowCfg { pub width: u32, pub height: u32, pub vsync: bool }

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub app: &'static str,
    pub window: WindowCfg,
    pub pipelines: Vec<PipelineDesc>,
}

/// Backend abstraction for creating pipelines and presenting frames.
pub trait RenderBackend {
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
        let vb = VL::vertex_buffer();
        if rb.is_empty() { return Err(EngineValidateError::NoBindings); }
        if vl.is_empty() { return Err(EngineValidateError::NoVertexAttrs); }
        for p in cfg.pipelines.iter() {
            // In a real system, validate descriptor sets vs shader reflection and vertex layout vs shader inputs.
            println!("validate '{}': bindings={} attrs={} stride={} step={:?} shaders=({}, {}) topo={:?}",
                p.name, rb.len(), vl.len(), vb.stride, vb.step, p.shaders.vs, p.shaders.fs, p.topology);
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
}

impl EngineBuilder {
    pub fn new() -> Self { Self { app: None, window: None, pipelines: Vec::new() } }
    pub fn app(mut self, name: &'static str) -> Self { self.app = Some(name); self }
    pub fn window(mut self, width: u32, height: u32, vsync: bool) -> Self { self.window = Some(WindowCfg { width, height, vsync }); self }
    pub fn add_pipeline(mut self, desc: PipelineDesc) -> Self { self.pipelines.push(desc); self }
    pub fn build(self) -> Result<EngineConfig, ConfigError> {
        let cfg = EngineConfig {
            app: self.app.unwrap_or("Untitled"),
            window: self.window.unwrap_or(WindowCfg { width: 1280, height: 720, vsync: true }),
            pipelines: self.pipelines,
        };
        validate_config(&cfg)?;
        Ok(cfg)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{PipelineDesc, ShaderPaths, Topology};

    #[test]
    fn builder_and_validation_work() {
        let cfg = EngineBuilder::new()
            .app("Demo")
            .window(800, 600, true)
            .add_pipeline(PipelineDesc { name: "triangle", shaders: ShaderPaths { vs: "vs", fs: "fs" }, topology: Topology::TriangleList, depth: true })
            .build()
            .expect("valid");
        assert_eq!(cfg.window.width, 800);
        assert_eq!(cfg.pipelines.len(), 1);
        // Validate RB/VL heuristics using types from resources module would be integration-level; unit test basic only.
    }
}
