//! Bridge smoke test: maps derives to Vulkan structs without creating a device.
//! Run with: cargo run -p macrokid_graphics --example bridge_smoke --features vulkan-linux

#[cfg(feature = "vulkan-linux")]
fn main() {
    use macrokid_graphics::vk_bridge as bridge;
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};
    use macrokid_graphics::pipeline::PipelineInfo;

    struct Matrices; struct Texture2D; struct Sampler;

    #[derive(ResourceBinding)]
    struct Material {
        #[uniform(set = 0, binding = 0, stages = "vs|fs")] matrices: Matrices,
        #[combined(set = 0, binding = 1, stages = "fs")] albedo: Texture2D,
    }

    #[derive(BufferLayout)]
    #[buffer(step = "vertex")] // single binding by default
    struct Vertex {
        #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

    #[derive(GraphicsPipeline)]
    #[pipeline(vs = "shaders/triangle.vert.spv", fs = "shaders/triangle.frag.spv", topology = "TriangleList", depth = true, depth_test = true, depth_write = true, depth_compare = "LEqual", polygon = "Fill", cull = "Back", front_face = "Cw", blend = false, samples = 1, dynamic = "viewport,scissor", push_constants_size = 64, push_constants_stages = "vs")]
    struct P;

    // Map to Vulkan structs
    let by_set = bridge::descriptor_bindings_from::<Material>();
    println!("sets:{}", by_set.len());
    for (set, binds) in by_set.iter() { println!("set {}: {} bindings", set, binds.len()); }

    let (vbb, vat) = bridge::vertex_input_from::<Vertex>();
    println!("vbufs={} attrs={}", vbb.len(), vat.len());

    let desc = P::pipeline_desc();
    let (poly, cull, ff) = bridge::raster_state_from(desc);
    let samples = bridge::samples_from(desc);
    let dyns = bridge::dynamic_states_from(desc);
    let pcs = bridge::push_constant_ranges_from(desc);
    let ds = bridge::depth_stencil_from(desc);
    println!("raster={:?}/{:?}/{:?} samples={:?} dyn={} pcs={} depth_test={}", poly.as_raw(), cull.as_raw(), ff.as_raw(), samples.as_raw(), dyns.len(), pcs.len(), ds.depth_test_enable);
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() { eprintln!("enable feature: vulkan-linux"); }

