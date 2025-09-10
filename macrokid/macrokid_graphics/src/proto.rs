#![cfg(feature = "proto")]
use crate::engine::{EngineConfig, WindowCfg};
use crate::pipeline::{PipelineDesc, ShaderPaths, Topology, RasterState, PolygonMode, CullMode, FrontFace, ColorBlendState};
use macrokid_graphics_proto::proto as pb;

#[derive(Debug)]
pub enum ConvertError { MissingField(&'static str), Invalid(&'static str) }

impl From<pb::Topology> for Topology {
    fn from(t: pb::Topology) -> Self {
        match t {
            pb::Topology::TriangleList => Topology::TriangleList,
            pb::Topology::LineList => Topology::LineList,
            pb::Topology::PointList => Topology::PointList,
            _ => Topology::TriangleList,
        }
    }
}

fn map_shader_paths(sp: &pb::ShaderPaths) -> Result<ShaderPaths, ConvertError> {
    let vs: &'static str = match &sp.vs {
        Some(pb::shader_paths::Vs::VsPath(p)) => Box::leak(p.clone().into_boxed_str()),
        Some(pb::shader_paths::Vs::VsSpirv(_)) => return Err(ConvertError::Invalid("vs_spirv not supported yet")),
        None => return Err(ConvertError::MissingField("vs")),
    };
    let fs: &'static str = match &sp.fs {
        Some(pb::shader_paths::Fs::FsPath(p)) => Box::leak(p.clone().into_boxed_str()),
        Some(pb::shader_paths::Fs::FsSpirv(_)) => return Err(ConvertError::Invalid("fs_spirv not supported yet")),
        None => return Err(ConvertError::MissingField("fs")),
    };
    Ok(ShaderPaths { vs, fs })
}

fn map_raster(r: &pb::RasterState) -> RasterState {
    let polygon = match r.polygon() {
        pb::raster_state::PolygonMode::Fill => PolygonMode::Fill,
        pb::raster_state::PolygonMode::Line => PolygonMode::Line,
        _ => PolygonMode::Fill,
    };
    let cull = match r.cull() {
        pb::raster_state::CullMode::None => CullMode::None,
        pb::raster_state::CullMode::Front => CullMode::Front,
        pb::raster_state::CullMode::Back => CullMode::Back,
        _ => CullMode::Back,
    };
    let front_face = match r.front_face() {
        pb::raster_state::FrontFace::Cw => FrontFace::Cw,
        pb::raster_state::FrontFace::Ccw => FrontFace::Ccw,
        _ => FrontFace::Cw,
    };
    RasterState { polygon, cull, front_face }
}

impl TryFrom<pb::PipelineDesc> for PipelineDesc {
    type Error = ConvertError;
    fn try_from(v: pb::PipelineDesc) -> Result<Self, Self::Error> {
        let shaders = map_shader_paths(v.shaders.as_ref().ok_or(ConvertError::MissingField("shaders"))?)?;
        let topology: Topology = v.topology().into();
        let raster = v.raster.map(|r| map_raster(&r));
        let blend = v.blend.map(|b| ColorBlendState { enable: b.enable });
        let samples = if v.samples == 0 { None } else { Some(v.samples) };
        Ok(PipelineDesc { name: Box::leak(v.name.into_boxed_str()), shaders, topology, depth: v.depth, raster, blend, samples, depth_stencil: None, dynamic: None, push_constants: None })
    }
}

impl TryFrom<pb::EngineConfig> for EngineConfig {
    type Error = ConvertError;
    fn try_from(v: pb::EngineConfig) -> Result<Self, Self::Error> {
        let w = v.window.ok_or(ConvertError::MissingField("window"))?;
        let window = WindowCfg { width: w.width, height: w.height, vsync: w.vsync };
        let mut pipelines = Vec::with_capacity(v.pipelines.len());
        for p in v.pipelines.into_iter() { pipelines.push(p.try_into()?); }
        Ok(EngineConfig { app: Box::leak(v.app.into_boxed_str()), window, pipelines })
    }
}
