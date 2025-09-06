use gfx_dsl::vk_engine;
use gfx_dsl_support::{Engine, VulkanBackend};
use render_resources::{ResourceBinding, BufferLayout};
// Traits are used by derive expansions; no direct imports needed here.

vk_engine! {
    {
        app: "MacroKid Vulkan Demo",
        window: { width: 1024, height: 600, vsync: true },
        graph: {
            pass main {
                pipelines: [
                    pipeline triangle {
                        vs: "shaders/triangle.vert",
                        fs: "shaders/triangle.frag",
                        topology: TriangleList,
                        depth: false,
                    },
                    pipeline lines {
                        vs: "shaders/lines.vert",
                        fs: "shaders/lines.frag",
                        topology: LineList,
                        depth: false,
                    }
                ]
            }
        }
    }
}

fn main() {
    println!("App: {} ({}x{}, vsync={})",
        mgfx_cfg::CONFIG.app,
        mgfx_cfg::CONFIG.window.width,
        mgfx_cfg::CONFIG.window.height,
        mgfx_cfg::CONFIG.window.vsync,
    );

    // Demo: resource bindings and vertex layout definitions
    #[derive(ResourceBinding)]
    struct Material {
        #[uniform(set = 0, binding = 0)] matrices: Matrices,
        #[texture(set = 0, binding = 1)] albedo: Texture2D,
        #[sampler(set = 0, binding = 2)] albedo_sampler: Sampler,
    }
    struct Matrices; struct Texture2D; struct Sampler;

    #[derive(BufferLayout)]
    #[buffer(step = "instance")]
    struct Vertex {
        #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

    println!("\n== Resource Bindings ==");
    // Touch fields to ensure example reads them at runtime
    fn touch_material_fields(m: &Material) { let _ = (&m.matrices, &m.albedo, &m.albedo_sampler); }
    fn touch_vertex_fields(v: &Vertex) { let _ = (&v.pos, &v.normal, &v.uv); }
    for b in Material::describe_bindings() {
        println!("field={} set={} binding={} kind={:?}", b.field, b.set, b.binding, b.kind);
    }
    println!("\n== Vertex Layout ==");
    for a in Vertex::describe_vertex_layout() {
        println!("field={} location={} format={} offset={} size={}", a.field, a.location, a.format, a.offset, a.size);
    }
    let vb = Vertex::describe_vertex_buffer();
    println!("buffer: stride={} step={:?}", vb.stride, vb.step);
    let m = Material { matrices: Matrices, albedo: Texture2D, albedo_sampler: Sampler };
    touch_material_fields(&m);
    let v = Vertex { pos: [0.0; 3], normal: [0.0; 3], uv: [0.0; 2] };
    touch_vertex_fields(&v);

    // Create engine using Vulkan backend and validate + initialize pipelines
    let engine = Engine::<VulkanBackend>::new_from_config(&mgfx_cfg::CONFIG);
    // Validate Graphics resources against pipeline configs
    engine.validate_pipelines_with::<Material, Vertex>(&mgfx_cfg::CONFIG).unwrap();
    engine.init_pipelines(&mgfx_cfg::CONFIG);
    engine.frame();
}
