use gfx_dsl::vk_engine;
use gfx_dsl_support::{Engine, VulkanBackend};
use render_resources::{ResourceBinding, BufferLayout};
use render_resources_support::{ResourceBindings, VertexLayout};

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
    for b in Material::describe_bindings() {
        println!("field={} set={} binding={} kind={:?}", b.field, b.set, b.binding, b.kind);
    }
    println!("\n== Vertex Layout ==");
    for a in Vertex::describe_vertex_layout() {
        println!("field={} location={} format={} offset={} size={}", a.field, a.location, a.format, a.offset, a.size);
    }
    let vb = Vertex::describe_vertex_buffer();
    println!("buffer: stride={} step={:?}", vb.stride, vb.step);

    // Create engine using Vulkan backend and validate + initialize pipelines
    let engine = Engine::<VulkanBackend>::new_from_config(&mgfx_cfg::CONFIG);
    // Validate Graphics resources against pipeline configs
    engine.validate_pipelines_with::<Material, Vertex>(&mgfx_cfg::CONFIG).unwrap();
    engine.init_pipelines(&mgfx_cfg::CONFIG);
    engine.frame();
}
