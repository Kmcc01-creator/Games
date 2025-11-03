# macrokid_graphics

> Derive-based Vulkan rendering abstractions for the Macrokid framework

A high-level graphics runtime built on Vulkan, providing procedural macros and abstractions for resource management, pipeline configuration, and rendering.

## Features

- **Resource Management**: `#[derive(ResourceBinding)]` for GPU resource bindings
- **Vertex Layouts**: `#[derive(BufferLayout)]` for vertex buffer layouts with automatic stride/step inference
- **Pipeline Configuration**: `#[derive(GraphicsPipeline)]` for declarative pipeline setup
- **Engine Setup**: `#[derive(RenderEngine)]` for ergonomic engine configuration
- **Procedural Assets**: Built-in mesh and texture generators (experimental)
- **Vulkan Backend**: Direct Vulkan integration with Linux support
- **GPU Resource Tracking** (NEW): Type-safe GPU buffers/images with automatic barrier hint generation
- **Multi-threaded Recording** (NEW): Infrastructure for parallel Vulkan command buffer recording

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
macrokid_graphics = { path = "../macrokid_graphics" }
macrokid_graphics_derive = { path = "../macrokid_graphics_derive" }
```

Define resources, layouts, and pipelines:

```rust
use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};

#[derive(ResourceBinding)]
#[binding(set = 0, binding = 0, uniform)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[derive(BufferLayout)]
struct Vertex {
    #[location(0)]
    position: [f32; 3],
    #[location(1)]
    normal: [f32; 3],
}

#[derive(GraphicsPipeline)]
#[vertex_shader("shaders/pbr.vert")]
#[fragment_shader("shaders/pbr.frag")]
struct PbrPipeline {
    #[vertex]
    vertex: Vertex,
    #[resource]
    uniforms: Uniforms,
}
```

## GPU Resource Tracking (NEW)

Track GPU resources and generate Vulkan barrier hints automatically:

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuImage, GpuResourceAccess};
use macrokid_threads_derive::System;

// Define marker types
struct VertexData;
struct RenderTarget;

// System with GPU resources
#[derive(System)]
#[reads(GpuBuffer<VertexData>)]
#[writes(GpuImage<RenderTarget>)]
struct RenderGeometry;

// Get automatic barrier hints
println!("{}", RenderGeometry::barrier_requirements());
```

**Features:**
- **Type-level inference**: `GpuBuffer<Vertex>` → `VERTEX_INPUT` stage automatically
- **Thread-safe tracking**: Atomic image layout tracking
- **Mixed CPU/GPU**: Track both CPU and GPU resources in same system
- **Barrier hints**: Human-readable Vulkan synchronization requirements

See [Barrier Generation Design](../docs/BARRIER_CODEGEN_DESIGN.md) for details.

## Multi-threaded Recording (NEW)

Infrastructure for parallel Vulkan command buffer recording:

```rust
// ThreadLocalPools: Per-thread command pools
let pools = ThreadLocalPools::new(&device, queue_family, num_threads, 8)?;

// Each thread gets isolated command pool (Vulkan requirement)
let secondary_cb = pools.get_secondary(thread_idx, buffer_idx);
```

**Status:** Core infrastructure complete. High-level `VkFrame`/`VkCommandEncoder` API in progress.

See [Multi-threaded Recording Design](../docs/MULTITHREADED_RECORDING_DESIGN.md) for details.

## Running Examples

**Basic Vulkan example:**
```bash
cargo run -p macrokid_graphics --example linux_vulkan \
    --features vulkan-linux,vk-shaderc-compile
```

**Animated demo (per-frame updates + custom textures):**
```bash
cargo run -p macrokid_graphics --example animated_demo \
    --features vulkan-linux,vk-shaderc-compile
```

**Protobuf-based configuration (data-first path):**
```bash
cargo run -p macrokid_graphics --example load_proto \
    --features vulkan-linux,proto,vk-shaderc-compile -- <path.pb>
```

## Documentation

- **[Getting Started](docs/getting-started.md)** - Setup and first render
- **[Architecture](docs/architecture.md)** - Design and internals
- **[Examples Guide](docs/examples.md)** - Detailed example walkthrough
- **[Vulkan Notes](docs/vulkan-notes.md)** - Platform-specific details

For complete framework documentation, see the [main README](../README.md) and [docs/](../docs/).

## Features Flags

- `vulkan-linux` - Enable Vulkan backend for Linux
- `vk-shaderc-compile` - Runtime GLSL shader compilation
- `proto` - Protobuf-based configuration support

## Current Status

**Working:**
- Resource binding derives
- Buffer layout derives with inference
- Pipeline configuration
- Basic PBR rendering
- Per-frame uniform updates
- Custom texture loading
- GPU resource type system (GpuBuffer, GpuImage)
- Automatic barrier hint generation
- ThreadLocalPools infrastructure

**Experimental:**
- Procedural mesh/texture generation
- Asset derives
- Advanced lighting (deferred, clustered)
- Multi-threaded command buffer recording (infrastructure complete, high-level API in progress)

## License

Part of the Macrokid project. See [LICENSE](../LICENSE) for details.
