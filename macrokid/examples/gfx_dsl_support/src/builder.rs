use crate::ir::*;

#[derive(Default)]
pub struct EngineConfigBuilder {
    app: Option<&'static str>,
    window: Option<WindowCfg>,
    pipelines: Vec<PipelineDesc>,
}

impl EngineConfigBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn app(mut self, name: &'static str) -> Self { self.app = Some(name); self }
    pub fn window(mut self, width: u32, height: u32, vsync: bool) -> Self {
        self.window = Some(WindowCfg { width, height, vsync });
        self
    }
    pub fn begin_pass(self, name: &'static str) -> PassBuilder { PassBuilder { parent: self, pass: name } }
    pub fn build(self) -> EngineConfig {
        let app = self.app.unwrap_or("Untitled");
        let window = self.window.unwrap_or(WindowCfg { width: 1280, height: 720, vsync: true });
        let pipelines_box = self.pipelines;
        // Move to 'static by leaking; examples-only convenience
        // In real app, prefer owned data or codegen static.
        let pipelines: &'static [PipelineDesc] = Box::leak(pipelines_box.into_boxed_slice());
        EngineConfig { app, window, pipelines }
    }
}

pub struct PassBuilder {
    parent: EngineConfigBuilder,
    pass: &'static str,
}
impl PassBuilder {
    pub fn pipeline(self, name: &'static str) -> PipelineBuilder { PipelineBuilder { parent: self, name, shaders: ShaderPaths { vs: "", fs: "" }, topology: Topology::TriangleList, depth: true } }
    pub fn finish(self) -> EngineConfigBuilder { self.parent }
}

pub struct PipelineBuilder {
    parent: PassBuilder,
    name: &'static str,
    shaders: ShaderPaths,
    topology: Topology,
    depth: bool,
}
impl PipelineBuilder {
    pub fn shaders(mut self, vs: &'static str, fs: &'static str) -> Self { self.shaders = ShaderPaths { vs, fs }; self }
    pub fn topology(mut self, topology: Topology) -> Self { self.topology = topology; self }
    pub fn depth(mut self, enabled: bool) -> Self { self.depth = enabled; self }
    pub fn finish(self) -> PassBuilder {
        let desc = PipelineDesc { pass: self.parent.pass, name: self.name, shaders: self.shaders, topology: self.topology, depth: self.depth };
        let mut parent = self.parent;
        parent.parent.pipelines.push(desc);
        parent
    }
}

