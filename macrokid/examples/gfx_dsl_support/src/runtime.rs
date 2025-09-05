use crate::ir::{EngineConfig, PipelineDesc};

pub trait RenderBackend {
    fn name() -> &'static str;
    fn create_device() { println!("[{}] create_device()", Self::name()); }
    fn create_pipeline(desc: &PipelineDesc) {
        println!(
            "[{}] create_pipeline: {} in pass {} (vs={}, fs={}, topo={:?}, depth={})",
            Self::name(), desc.name, desc.pass, desc.shaders.vs, desc.shaders.fs, desc.topology, desc.depth
        );
    }
    fn present() { println!("[{}] present()", Self::name()); }
}

pub struct VulkanBackend;
impl RenderBackend for VulkanBackend { fn name() -> &'static str { "vulkan" } }

pub struct Engine<B: RenderBackend> { backend: core::marker::PhantomData<B> }
impl<B: RenderBackend> Engine<B> {
    pub fn new_from_config(_cfg: &EngineConfig) -> Self {
        B::create_device();
        Self { backend: core::marker::PhantomData }
    }
    pub fn init_pipelines(&self, cfg: &EngineConfig) {
        for p in cfg.pipelines.iter() { B::create_pipeline(p); }
    }
    pub fn frame(&self) { B::present(); }
}

