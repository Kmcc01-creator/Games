# Getting Started with macrokid_graphics

## Prerequisites

- Rust 1.70+ (2021 edition)
- Vulkan SDK (for Linux backend)
- GLSL shader compiler (or use runtime compilation feature)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
macrokid_graphics = { path = "../macrokid_graphics" }
macrokid_graphics_derive = { path = "../macrokid_graphics_derive" }
```

## Your First Render

### 1. Define Your Vertex Type

```rust
use macrokid_graphics_derive::BufferLayout;

#[derive(BufferLayout)]
struct Vertex {
    #[location(0)]
    position: [f32; 3],
    #[location(1)]
    color: [f32; 3],
}
```

### 2. Define Resources (Uniforms)

```rust
use macrokid_graphics_derive::ResourceBinding;

#[derive(ResourceBinding)]
#[binding(set = 0, binding = 0, uniform)]
struct CameraUniforms {
    view_proj: [[f32; 4]; 4],
}
```

### 3. Create a Pipeline

```rust
use macrokid_graphics_derive::GraphicsPipeline;

#[derive(GraphicsPipeline)]
#[vertex_shader("shaders/basic.vert")]
#[fragment_shader("shaders/basic.frag")]
struct BasicPipeline {
    #[vertex]
    vertex: Vertex,
    #[resource]
    camera: CameraUniforms,
}
```

### 4. Set Up the Engine

```rust
use macrokid_graphics_derive::RenderEngine;

#[derive(RenderEngine)]
#[app(name = "My First Render")]
#[window(width = 800, height = 600, vsync = true)]
struct MyEngine {
    #[use_pipeline]
    basic: BasicPipeline,
}

fn main() {
    let config = MyEngine::engine_config();
    // Use config to initialize your rendering backend
}
```

## Running Examples

**Basic triangle:**
```bash
cargo run -p macrokid_graphics --example linux_vulkan \
    --features vulkan-linux,vk-shaderc-compile
```

**Animated scene:**
```bash
cargo run -p macrokid_graphics --example animated_demo \
    --features vulkan-linux,vk-shaderc-compile
```

## Next Steps

- [Architecture Overview](architecture.md) - Understand the design
- [Examples Guide](examples.md) - Explore more complex examples
- [Vulkan Notes](vulkan-notes.md) - Platform-specific details

## Common Issues

**Shader compilation fails:**
- Ensure `vk-shaderc-compile` feature is enabled
- Check shader paths are relative to project root
- Verify GLSL syntax

**Vulkan initialization fails:**
- Install Vulkan SDK
- Check for graphics driver support
- Run `vulkaninfo` to verify installation

**Stride/layout errors:**
- BufferLayout derives automatically calculate stride
- Use `#[repr(C)]` on your vertex structs for predictable layout
- Check alignment requirements for your data types
