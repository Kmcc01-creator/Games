//! RenderEngine derive demo
//! Build and run:
//!   cargo run -p macrokid_graphics --example render_engine

use macrokid_graphics::engine::{Engine, EngineBuilder, VulkanBackend, RenderEngineInfo, BackendOptions};
use macrokid_graphics::pipeline::{PipelineInfo};
use macrokid_graphics_derive::{GraphicsPipeline, ResourceBinding, BufferLayout, RenderEngine};

// Dummy resource and vertex types to exercise derives
struct Matrices; struct Texture2D; struct Sampler;

#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0, stages = "vs|fs")] matrices: Matrices,
}

#[derive(BufferLayout)]
#[buffer(step = "vertex")]
struct Vertex {
    #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
}

#[derive(GraphicsPipeline)]
#[pipeline(vs = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/triangle.vert"), fs = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/triangle.frag"), topology = "TriangleList", depth = true, polygon = "Fill", cull = "Back", front_face = "Cw")]
struct TrianglePipeline;

#[derive(RenderEngine)]
#[app(name = "RenderEngine Demo")]
#[window(width = 800, height = 600, vsync = true)]
struct MyEngine {
    #[use_pipeline]
    _p: TrianglePipeline,
}

fn main() {
    // Build an EngineConfig from the RenderEngine derive
    let mut cfg = MyEngine::engine_config();
    // Merge env-backed options, then override explicitly (builder wins over env)
    cfg.options = cfg.options.clone().with_env_fallback();
    cfg.options.present_mode_priority = Some(vec!["MAILBOX", "FIFO"]);
    cfg.options.adapter_preference = Some("discrete");

    // Log effective options
    cfg.options.log_effective(&cfg.window);
    // Drive minimal engine init/log flow
    let eng = Engine::<VulkanBackend>::new_from_config(&cfg);
    eng.init_pipelines(&cfg);
    eng.frame();
}
