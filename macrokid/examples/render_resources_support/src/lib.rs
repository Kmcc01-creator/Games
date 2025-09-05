#[derive(Clone, Debug)]
pub enum ResourceKind { Uniform, Texture, Sampler }
#[derive(Clone, Debug)]
pub struct BindingDesc { pub field: &'static str, pub set: u32, pub binding: u32, pub kind: ResourceKind }

#[derive(Clone, Debug)]
pub enum StepMode { Vertex, Instance }
#[derive(Clone, Debug)]
pub struct VertexBufferDesc { pub stride: u32, pub step: StepMode }
#[derive(Clone, Debug)]
pub struct VertexAttr { pub field: &'static str, pub location: u32, pub format: &'static str, pub offset: u32, pub size: u32 }

pub trait ResourceBindings { fn bindings() -> &'static [BindingDesc]; }
pub trait VertexLayout {
    fn vertex_attrs() -> &'static [VertexAttr];
    fn vertex_buffer() -> VertexBufferDesc;
}
