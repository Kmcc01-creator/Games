# Render Resources Derives

This example crate provides two derives to describe GPU-facing resources in a backend-agnostic way:

- `#[derive(ResourceBinding)]` — describes descriptor set/binding layout for a material-like struct.
- `#[derive(BufferLayout)]` — describes vertex buffer layout (attributes, offsets, sizes, stride, step mode).

## ResourceBinding

Annotate fields with one of:
- `#[uniform(set = N, binding = M)]`
- `#[texture(set = N, binding = M)]`
- `#[sampler(set = N, binding = M)]`

Validations:
- Exactly one kind per field.
- Required keys `set` and `binding` present and integers.
- Duplicate `(set, binding)` pairs rejected.

Codegen:
- Adds an inherent method `fn describe_bindings() -> &'static [BindingDesc]`.
- Implements `render_resources_support::ResourceBindings`:
  - `fn bindings() -> &'static [BindingDesc]`.

## BufferLayout

Struct-level attributes:
- `#[buffer(stride = N, step = "vertex" | "instance")]` (both optional)

Field-level attributes:
- `#[vertex(location = N, format = "vec3" | "auto")]`

Validations & Inference:
- Duplicate `location` rejected.
- Per-attribute `offset`/`size` computed in `location` order.
  - If `format != "auto"`: sizes inferred from common formats (f32/u32/i32=4, vec2=8, vec3=12, vec4=16, mat4=64).
  - If `format = "auto"`: infer from field type (e.g., `[f32; 3]` -> 12, `f32` -> 4). Errors if unknown.
- `stride` defaults to sum of attribute sizes; can be overridden.
- `step` defaults to `Vertex` (per-vertex); `Instance` if specified.

Codegen:
- Adds inherent:
  - `fn describe_vertex_layout() -> &'static [VertexAttr]`
  - `fn describe_vertex_buffer() -> VertexBufferDesc`
- Implements `render_resources_support::VertexLayout` with the same data.

## Integration Example

```rust
use render_resources::{ResourceBinding, BufferLayout};
use render_resources_support::{ResourceBindings, VertexLayout};

#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0)] matrices: Matrices,
    #[texture(set = 0, binding = 1)] albedo: Texture2D,
    #[sampler(set = 0, binding = 2)] albedo_sampler: Sampler,
}
#[derive(BufferLayout)]
#[buffer(step = "instance")]
struct Vertex {
    #[vertex(location = 0, format = "vec3")] pos: [f32; 3],
    #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
    #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
}

fn use_in_engine<E: Engine>(engine: &E, cfg: &EngineConfig) {
    // Validate then initialize
    engine.validate_pipelines_with::<Material, Vertex>(cfg).unwrap();
    engine.init_pipelines(cfg);
}
```

## Notes & Follow‑Ups
- Alignment rules are not modeled yet — offsets are sequential; add alignment if needed.
- `format` is a string today; consider a typed enum for formats and validation.
- Consider per-field overrides: `#[vertex(offset = N, size = M)]`.
- The runtime mapping to actual backend structs remains outside this crate.

