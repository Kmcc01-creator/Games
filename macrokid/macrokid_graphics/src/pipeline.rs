#[derive(Clone, Debug)]
pub enum Topology { TriangleList, LineList, PointList }

#[derive(Clone, Debug)]
pub struct ShaderPaths { pub vs: &'static str, pub fs: &'static str }

// Render target descriptions for flexible attachment configuration
#[derive(Clone, Debug)]
pub struct ColorTargetDesc {
    pub format: &'static str,
    /// Optional per-target blend enable (falls back to pipeline-level blend if None)
    pub blend: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct DepthTargetDesc { pub format: &'static str }

#[derive(Clone, Debug)]
pub struct PipelineDesc {
    pub name: &'static str,
    pub shaders: ShaderPaths,
    pub topology: Topology,
    pub depth: bool,
    // Optional backend-agnostic pipeline state we can use for Vulkan or others
    pub raster: Option<RasterState>,
    pub blend: Option<ColorBlendState>,
    pub samples: Option<u32>,
    pub depth_stencil: Option<DepthState>,
    pub dynamic: Option<DynamicStateDesc>,
    pub push_constants: Option<PushConstantRange>,
    /// Optional list of color targets (MRT). If None or empty, defaults to single swapchain target.
    pub color_targets: Option<&'static [ColorTargetDesc]>,
    /// Optional depth target format (backend picks suitable default if None)
    pub depth_target: Option<DepthTargetDesc>,
}

pub trait PipelineInfo { fn pipeline_desc() -> &'static PipelineDesc; }

#[derive(Clone, Debug)]
pub struct ComputeDesc {
    pub name: &'static str,
    /// Path to compute shader (`.comp`) or SPIR-V (`.spv`).
    pub shader: &'static str,
    /// Dispatch group counts passed to `vkCmdDispatch`.
    pub dispatch: (u32, u32, u32),
    /// Optional push constant range (size and stages) for compute.
    pub push_constants: Option<PushConstantRange>,
    /// Optional explicit descriptor layout for this compute pass (static at compile-time).
    /// Provide from a `ResourceBindings::bindings()` slice when layouts differ between passes.
    pub bindings: Option<&'static [crate::resources::BindingDesc]>,
}

// Backend-agnostic pipeline state (minimal set)
#[derive(Clone, Debug)]
pub enum PolygonMode { Fill, Line }

#[derive(Clone, Debug)]
pub enum CullMode { None, Front, Back }

#[derive(Clone, Debug)]
pub enum FrontFace { Cw, Ccw }

#[derive(Clone, Debug)]
pub struct RasterState {
    pub polygon: PolygonMode,
    pub cull: CullMode,
    pub front_face: FrontFace,
}

#[derive(Clone, Debug)]
pub struct ColorBlendState { pub enable: bool }

#[derive(Clone, Debug)]
pub enum CompareOp { Never, Less, Equal, LessOrEqual, Greater, NotEqual, GreaterOrEqual, Always }

#[derive(Clone, Debug)]
pub struct DepthState { pub test: bool, pub write: bool, pub compare: CompareOp }

#[derive(Clone, Debug)]
pub struct DynamicStateDesc { pub viewport: bool, pub scissor: bool }

#[derive(Clone, Debug)]
pub struct StageMask { pub vs: bool, pub fs: bool, pub cs: bool }

#[derive(Clone, Debug)]
pub struct PushConstantRange { pub size: u32, pub stages: Option<StageMask> }
