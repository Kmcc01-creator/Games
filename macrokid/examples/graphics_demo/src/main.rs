use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};

// Dummy types
struct Matrices;
struct Texture2D;
struct Sampler;

#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0)] matrices: Matrices,
    #[texture(set = 0, binding = 1)] albedo: Texture2D,
    #[sampler(set = 0, binding = 2)] albedo_sampler: Sampler,
}

#[derive(BufferLayout)]
#[buffer(step = "vertex")]
struct Vertex {
    #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
    #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
    #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
}

fn main() {
    // Touch fields so examples reflect real usage and avoid dead code:
    // - Material fields are consumed by derives at compile time; at runtime we read them here.
    fn touch_material_fields(m: &Material) { let _ = (&m.matrices, &m.albedo, &m.albedo_sampler); }
    fn touch_vertex_fields(v: &Vertex) { let _ = (&v.pos, &v.normal, &v.uv); }

    println!("== Resource Bindings ==");
    for b in Material::describe_bindings() {
        println!("field={} set={} binding={} kind={:?}", b.field, b.set, b.binding, b.kind);
    }

    println!("\n== Vertex Layout ==");
    for a in Vertex::describe_vertex_layout() {
        println!("field={} location={} format={} offset={} size={}", a.field, a.location, a.format, a.offset, a.size);
    }
    let vb = Vertex::describe_vertex_buffer();
    println!("buffer: stride={} step={:?}", vb.stride, vb.step);

    // Construct sample values to ensure fields are read in this demo
    let m = Material { matrices: Matrices, albedo: Texture2D, albedo_sampler: Sampler };
    touch_material_fields(&m);

    let v = Vertex { pos: [0.0; 3], normal: [0.0; 3], uv: [0.0; 2] };
    touch_vertex_fields(&v);

    #[derive(GraphicsPipeline)]
    #[pipeline(vs = "shaders/triangle.vert", fs = "shaders/triangle.frag", topology = "TriangleList", depth = true)]
    struct TrianglePipeline;

    let p = TrianglePipeline::describe_pipeline();
    println!("\n== Pipeline ==\nname={} vs={} fs={} topo={:?} depth={}", p.name, p.shaders.vs, p.shaders.fs, p.topology, p.depth);
}
