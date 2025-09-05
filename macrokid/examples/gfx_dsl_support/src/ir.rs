#[derive(Clone, Debug)]
pub struct WindowCfg { pub width: u32, pub height: u32, pub vsync: bool }

#[derive(Clone, Debug)]
pub struct ShaderPaths { pub vs: &'static str, pub fs: &'static str }

#[derive(Clone, Debug)]
pub enum Topology { TriangleList, LineList, PointList }

#[derive(Clone, Debug)]
pub struct PipelineDesc {
    pub pass: &'static str,
    pub name: &'static str,
    pub shaders: ShaderPaths,
    pub topology: Topology,
    pub depth: bool,
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub app: &'static str,
    pub window: WindowCfg,
    pub pipelines: &'static [PipelineDesc],
}

