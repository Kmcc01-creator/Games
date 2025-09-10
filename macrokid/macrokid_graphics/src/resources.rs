#[derive(Clone, Debug)]
pub enum ResourceKind { Uniform, Texture, Sampler, CombinedImageSampler }

#[derive(Clone, Debug)]
pub struct BindingStages { pub vs: bool, pub fs: bool, pub cs: bool }

#[derive(Clone, Debug)]
pub struct BindingDesc {
    pub field: &'static str,
    pub set: u32,
    pub binding: u32,
    pub kind: ResourceKind,
    pub stages: Option<BindingStages>,
}

pub trait ResourceBindings { fn bindings() -> &'static [BindingDesc]; }

// Vertex layout types
#[derive(Clone, Debug)]
pub enum StepMode { Vertex, Instance }

#[derive(Clone, Debug)]
pub struct VertexBufferDesc { pub binding: u32, pub stride: u32, pub step: StepMode }

#[derive(Clone, Debug)]
pub struct VertexAttr { pub field: &'static str, pub binding: u32, pub location: u32, pub format: &'static str, pub offset: u32, pub size: u32 }

pub trait VertexLayout {
    fn vertex_attrs() -> &'static [VertexAttr];
    fn vertex_buffers() -> &'static [VertexBufferDesc];
}
