//! Engine + derives demo
//!
//! This example uses macrokid_graphics_derive derives to describe GPU-facing resources and
//! vertex layout, builds an EngineConfig, validates the config against those derives, and
//! initializes a simple pipeline.
//!
//! Patterns to note (possible macrokid_core abstractions):
//! - Validation flows often need a generic pattern: `validate_with<TraitA, TraitB>(config)`.
//!   We already do that via `validate_pipelines_with<RB, VL>`. A generic helper trait in
//!   macrokid_core (e.g., a `Validate` facade) could standardize these flows across domains.
//! - Building small constant slices (e.g., pipelines) benefits from static module emitters.
//!   macrokid_core::common::codegen::static_slice_mod is already used in derives; keeping
//!   those helpers idiomatic encourages consistency across crates.

use macrokid_graphics::engine::{Engine, EngineBuilder, VulkanBackend, GraphicsValidator};
use macrokid_core::common::validate::ValidateExt;
use macrokid_graphics::pipeline::{PipelineDesc, ShaderPaths, Topology};
// Derive-generated trait impls are used by the engine; no direct imports needed.
use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};

// Dummy resource types
struct Matrices;
struct Texture2D;
struct Sampler;

// Describe shader resource bindings via derive
#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0)] matrices: Matrices,
    #[texture(set = 0, binding = 1)] albedo: Texture2D,
    #[sampler(set = 0, binding = 2)] albedo_sampler: Sampler,
}

// Describe vertex layout via derive (offsets will be inferred)
#[derive(BufferLayout)]
#[buffer(step = "vertex")]
struct Vertex {
    #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
    #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
    #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
}

// Optionally, describe a pipeline via derive (produces a PipelineDesc at type-level)
#[derive(GraphicsPipeline)]
#[pipeline(vs = "shaders/triangle.vert", fs = "shaders/triangle.frag", topology = "TriangleList", depth = true)]
struct TrianglePipeline;

fn main() {
    // Touch fields so example performs runtime reads (self-documenting usage of derive inputs)
    fn touch_material_fields(m: &Material) { let _ = (&m.matrices, &m.albedo, &m.albedo_sampler); }
    fn touch_vertex_fields(v: &Vertex) { let _ = (&v.pos, &v.normal, &v.uv); }
    // Pipelines can be created by hand or collected from derives
    let tri = PipelineDesc {
        name: "triangle",
        shaders: ShaderPaths { vs: "shaders/triangle.vert", fs: "shaders/triangle.frag" },
        topology: Topology::TriangleList,
        depth: true,
    };

    // Build engine config using the builder (no macros required)
    let cfg = EngineBuilder::new()
        .app("Macrokid Graphics Demo")
        .window(1024, 600, true)
        .add_pipeline(tri)
        .build()
        .expect("valid config");

    // Validate derived resource/vertex information against the engine config
    // Two equivalent paths: direct Engine method or generic Validator facade.
    let engine = Engine::<VulkanBackend>::new_from_config(&cfg);
    engine.validate_pipelines_with::<Material, Vertex>(&cfg).expect("resources/layout validated");
    cfg.validate_with::<GraphicsValidator<Material, Vertex>>().expect("validator facade");

    // Initialize pipelines and present a frame (logs for demo purposes)
    engine.init_pipelines(&cfg);
    engine.frame();

    // Construct sample values to ensure fields are read at runtime
    let m = Material { matrices: Matrices, albedo: Texture2D, albedo_sampler: Sampler };
    touch_material_fields(&m);
    let v = Vertex { pos: [0.0; 3], normal: [0.0; 3], uv: [0.0; 2] };
    touch_vertex_fields(&v);
}
