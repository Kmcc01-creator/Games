#[derive(Clone, Debug)]
pub enum Topology { TriangleList, LineList, PointList }

#[derive(Clone, Debug)]
pub struct ShaderPaths { pub vs: &'static str, pub fs: &'static str }

#[derive(Clone, Debug)]
pub struct PipelineDesc {
    pub name: &'static str,
    pub shaders: ShaderPaths,
    pub topology: Topology,
    pub depth: bool,
}

pub trait PipelineInfo { fn pipeline_desc() -> &'static PipelineDesc; }

