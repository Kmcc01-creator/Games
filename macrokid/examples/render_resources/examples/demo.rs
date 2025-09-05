use render_resources::{ResourceBinding, BufferLayout};

// Dummy types for the example
#[allow(dead_code)]
struct Matrices;
#[allow(dead_code)]
struct Texture2D;
#[allow(dead_code)]
struct Sampler;

#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0)]
    matrices: Matrices,
    #[texture(set = 0, binding = 1)]
    albedo: Texture2D,
    #[sampler(set = 0, binding = 2)]
    albedo_sampler: Sampler,
}

    #[derive(BufferLayout)]
    #[buffer(stride = 32, step = "vertex")]
    struct Vertex {
        #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

fn main() {
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
}
