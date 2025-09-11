//! Demonstrates a pipeline with multiple color targets (MRT) using #[color_target]
//! and a minimal RenderPass descriptor. Requires the `vulkan-linux` feature to run.

#[cfg(feature = "vulkan-linux")]
fn main() {
    use macrokid_graphics::engine::EngineBuilder;
    use macrokid_graphics::vk_linux::run_vulkan_linux_app_with;
    use macrokid_graphics_derive::{GraphicsPipeline, ResourceBinding, BufferLayout, RenderPass};

    // Dummy resource and vertex descriptions (same pattern as other examples)
    struct Matrices; struct Texture2D; struct Sampler;
    #[derive(ResourceBinding)]
    struct Material {
        #[uniform(set = 0, binding = 0, stages = "vs|fs")] matrices: Matrices,
        #[texture(set = 0, binding = 1, stages = "fs")] albedo: Texture2D,
        #[sampler(set = 0, binding = 2, stages = "fs")] albedo_sampler: Sampler,
    }
    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct Vertex {
        #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

    // A pipeline that declares multiple color targets (G-Buffer style)
    // For demo, shaders are the simple triangle; only location 0 writes, but the
    // backend will create N attachments and blit the first to the swapchain.
    #[derive(GraphicsPipeline)]
    #[pipeline(
        vs = "shaders/triangle.vert",
        fs = "shaders/triangle.frag",
        topology = "TriangleList", depth = true, samples = 1,
        polygon = "Fill", cull = "Back", front_face = "Cw"
    )]
    #[color_target(format = "rgba16f")] // Albedo
    #[color_target(format = "rgba16f")] // Normals
    #[color_target(format = "rgba16f")] // Material/roughness
    #[depth_target(format = "D32_SFLOAT")]
    struct DeferredGBuffer;

    // Optional: a minimal pass descriptor for future render-graph use
    #[derive(RenderPass)]
    #[pass(name = "GBuffer", kind = "graphics")]
    #[color_target(format = "rgba16f")]
    #[color_target(format = "rgba16f")]
    #[color_target(format = "rgba16f")]
    #[depth_target(format = "D32_SFLOAT")]
    struct GBufferPass;

    use macrokid_graphics::pipeline::PipelineInfo;
    let cfg = EngineBuilder::new()
        .app("Deferred GBuffer (MRT)")
        .window(1280, 720, true)
        .add_pipeline(<DeferredGBuffer as PipelineInfo>::pipeline_desc().clone())
        .build()
        .expect("valid config");

    // Run using derived ResourceBindings and VertexLayout, via RenderGraph mapping
    let passes: [&macrokid_graphics::render_graph::PassDesc; 1] = [GBufferPass::describe_pass()];
    macrokid_graphics::vk_linux::run_vulkan_linux_app_with_graph::<Material, Vertex>(&cfg, &passes).expect("vulkan MRT app ran");
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' feature.\nTry: cargo run -p macrokid_graphics --example deferred_gbuffer --features vulkan-linux");
}
