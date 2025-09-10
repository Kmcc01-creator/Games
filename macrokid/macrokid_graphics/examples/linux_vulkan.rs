//! Minimal Linux Vulkan example using the `vulkan-linux` feature.
//! Run with:
//!   cargo run -p macrokid_graphics --example linux_vulkan --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() {
    use macrokid_graphics::engine::EngineBuilder;
    use macrokid_graphics::pipeline::{PipelineDesc, ShaderPaths, Topology};
    use macrokid_graphics::vk_linux::run_vulkan_linux_app_with;
    
    let tri = PipelineDesc {
        name: "triangle",
        shaders: ShaderPaths { vs: concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/triangle.vert"), fs: concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/triangle.frag") },
        topology: Topology::TriangleList,
        depth: false,
        raster: None,
        blend: None,
        samples: None,
        depth_stencil: None,
        dynamic: None,
        push_constants: None,
    };

    let cfg = EngineBuilder::new()
        .app("MacroKid Vulkan (Linux)")
        .window(960, 540, true)
        .add_pipeline(tri)
        .build()
        .expect("valid config");

    // Derive-generated bindings and vertex layout
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout};

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

    run_vulkan_linux_app_with::<Material, Vertex>(&cfg).expect("vulkan linux app ran");
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' feature.\nTry: cargo run -p macrokid_graphics --example linux_vulkan --features vulkan-linux");
}
