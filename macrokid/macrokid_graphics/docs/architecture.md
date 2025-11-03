# macrokid_graphics Architecture

## Design Philosophy

### Separation of Concerns
```
macrokid_core     → Universal building blocks (IR, attrs, builders)
macrokid_graphics → Graphics domain expertise (pipelines, shaders, backends)
```

This keeps the core lightweight while enabling rich domain-specific functionality.

### Key Principles

1. **Single Source, Multiple Targets**: One DSL definition → multiple backend implementations
2. **Performance by Design**: Built-in support for Data-Oriented Design patterns
3. **Type Safety**: Compile-time validation of graphics resource relationships
4. **Developer Experience**: Clear error messages with actionable suggestions

## Crate Structure

```
macrokid_graphics/
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── pipeline.rs         # Pipeline configuration
│   ├── resources.rs        # Resource management (buffers, bindings)
│   ├── engine.rs           # High-level engine API
│   ├── proto.rs            # Protobuf integration (feature-gated)
│   ├── vk_bridge.rs        # Vulkan abstraction layer
│   └── vk_linux.rs         # Linux-specific Vulkan implementation
```

## Derive System

### BufferLayout

Automatically generates vertex/index buffer layouts with:
- Stride calculation (automatic or explicit via `#[stride]`)
- Step rate inference (per-vertex by default, per-instance via `#[step(instance)]`)
- Location binding via `#[location(N)]`
- Size calculation for allocations

```rust
#[derive(BufferLayout)]
struct Vertex {
    #[location(0)]
    position: [f32; 3],
    #[location(1)]
    normal: [f32; 3],
}
// Generated: stride = 24, step = PerVertex, size = 24
```

### ResourceBinding

Generates descriptor set bindings for:
- Uniform buffers (`#[binding(set = S, binding = B, uniform)]`)
- Storage buffers (`#[binding(..., storage)]`)
- Textures/samplers (`#[binding(..., texture)]`)

```rust
#[derive(ResourceBinding)]
#[binding(set = 0, binding = 0, uniform)]
struct SceneData {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
}
```

### GraphicsPipeline

Combines vertex layouts, resources, and shader stages:

```rust
#[derive(GraphicsPipeline)]
#[vertex_shader("shaders/pbr.vert")]
#[fragment_shader("shaders/pbr.frag")]
struct PbrPipeline {
    #[vertex]
    vertex: PbrVertex,
    #[resource]
    scene: SceneData,
    #[resource]
    material: MaterialData,
}
```

### RenderEngine

Declarative engine configuration:

```rust
#[derive(RenderEngine)]
#[app(name = "My Game")]
#[window(width = 1920, height = 1080, vsync = true)]
struct GameEngine {
    #[use_pipeline]
    geometry: GeometryPipeline,
    #[use_pipeline]
    lighting: LightingPipeline,
}

// Generates: impl RenderEngineInfo with engine_config() -> EngineConfig
```

## Backend Integration

### Vulkan Path

1. **Derive Processing**: Macros analyze struct definitions
2. **IR Generation**: Convert to internal representation
3. **Validation**: Check resource/layout compatibility
4. **Code Generation**: Emit Vulkan-specific initialization code
5. **Runtime Reflection**: Create descriptor layouts, pipeline states

### Protobuf Path (Optional)

Parallel data-first approach using `.proto` definitions:

```protobuf
message EngineConfig {
  AppConfig app = 1;
  WindowConfig window = 2;
  repeated PipelineConfig pipelines = 3;
}
```

Load at runtime:
```bash
cargo run --example load_proto --features proto -- config.pb
```

## Code Generation Flow

```
User Code (derives)
    ↓
syn parsing → TypeSpec (IR)
    ↓
Attribute validation
    ↓
Code generation (quote!)
    ↓
Generated impl blocks
    ↓
Runtime reflection → Vulkan objects
```

## Resource Management

- **Buffers**: Managed via `BufferLayout` with automatic stride/alignment
- **Descriptors**: Grouped by set/binding from `ResourceBinding`
- **Pipelines**: Composed from vertex layouts + resources + shaders
- **Lifetime**: Framework handles Vulkan object lifecycle

## Performance Considerations

- **Compile-time validation**: Catch errors before runtime
- **Zero-cost abstractions**: Derives emit optimal code
- **Minimal runtime overhead**: Direct Vulkan API usage
- **Efficient layouts**: Automatic alignment and packing

## Extension Points

1. **Custom Backends**: Implement backend traits
2. **Asset Procedural Generation**: Extend asset derives (planned)
3. **Render Passes**: Define custom render graph nodes
4. **Validation Rules**: Add domain-specific checks

## Future Directions

- Advanced lighting (deferred, clustered)
- Asset pipeline integration
- Multi-threading support
- Hot shader reloading
- Frame graph optimization

For implementation details, see the [main graphics documentation](../../MACROKID_GRAPHICS.md) (archived).
