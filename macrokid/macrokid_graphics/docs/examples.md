# macrokid_graphics Examples Guide

## Available Examples

### linux_vulkan
**Location:** `examples/linux_vulkan.rs`

Basic Vulkan rendering example demonstrating:
- Window creation
- Swapchain setup
- Simple triangle rendering
- Basic vertex/fragment shaders

**Run:**
```bash
cargo run -p macrokid_graphics --example linux_vulkan \
    --features vulkan-linux,vk-shaderc-compile
```

### animated_demo
**Location:** `examples/animated_demo.rs`

Demonstrates dynamic rendering:
- Per-frame uniform updates
- Custom texture loading
- Animation loops
- Camera transformations

**Run:**
```bash
cargo run -p macrokid_graphics --example animated_demo \
    --features vulkan-linux,vk-shaderc-compile
```

### load_proto (Data-First Path)
**Location:** `examples/load_proto.rs`

Protobuf-based configuration:
- Load engine config from `.pb` files
- Runtime pipeline construction
- Data-driven rendering

**Run:**
```bash
# First, create a protobuf config (or use existing)
cargo run -p macrokid_graphics --example load_proto \
    --features vulkan-linux,proto,vk-shaderc-compile -- path/to/config.pb
```

### PBR Showcase (Planned)
**Location:** `examples/pbr_showcase.rs` (not yet implemented)

Will demonstrate:
- Physically-based rendering
- Multiple light sources
- Material system
- Shadow mapping

### Texture Showcase (Planned)
**Location:** `examples/texture_showcase.rs` (not yet implemented)

Will demonstrate:
- Procedural texture generation
- Texture loading and sampling
- Mipmaps and filtering
- Texture arrays

## Example Structure

Most examples follow this pattern:

```rust
use macrokid_graphics_derive::{BufferLayout, ResourceBinding, GraphicsPipeline, RenderEngine};

// 1. Define data structures with derives
#[derive(BufferLayout)]
struct Vertex { /* ... */ }

#[derive(ResourceBinding)]
#[binding(set = 0, binding = 0, uniform)]
struct Uniforms { /* ... */ }

#[derive(GraphicsPipeline)]
#[vertex_shader("path/to/vert.glsl")]
#[fragment_shader("path/to/frag.glsl")]
struct MyPipeline {
    #[vertex]
    vertex: Vertex,
    #[resource]
    uniforms: Uniforms,
}

// 2. Set up engine
#[derive(RenderEngine)]
#[app(name = "Example")]
#[window(width = 800, height = 600)]
struct MyEngine {
    #[use_pipeline]
    pipeline: MyPipeline,
}

// 3. Initialize and run
fn main() {
    let config = MyEngine::engine_config();
    // ... backend-specific initialization
}
```

## Building Your Own

### Step 1: Create Vertex Layout

Start with your vertex data:

```rust
#[derive(BufferLayout)]
struct MyVertex {
    #[location(0)]
    position: [f32; 3],
    #[location(1)]
    normal: [f32; 3],
    #[location(2)]
    uv: [f32; 2],
}
```

### Step 2: Define Resources

Add any uniforms or resources:

```rust
#[derive(ResourceBinding)]
#[binding(set = 0, binding = 0, uniform)]
struct Camera {
    view_proj: [[f32; 4]; 4],
}

#[derive(ResourceBinding)]
#[binding(set = 0, binding = 1, uniform)]
struct Model {
    transform: [[f32; 4]; 4],
}
```

### Step 3: Create Pipeline

Combine vertex + resources + shaders:

```rust
#[derive(GraphicsPipeline)]
#[vertex_shader("shaders/my.vert")]
#[fragment_shader("shaders/my.frag")]
struct MyPipeline {
    #[vertex]
    vertex: MyVertex,
    #[resource]
    camera: Camera,
    #[resource]
    model: Model,
}
```

### Step 4: Write Shaders

Create corresponding GLSL shaders:

**shaders/my.vert:**
```glsl
#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;
};

layout(set = 0, binding = 1) uniform Model {
    mat4 transform;
};

void main() {
    gl_Position = view_proj * transform * vec4(position, 1.0);
}
```

**shaders/my.frag:**
```glsl
#version 450

layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(1.0, 0.5, 0.2, 1.0);
}
```

### Step 5: Set Up Engine

```rust
#[derive(RenderEngine)]
#[app(name = "My Example")]
#[window(width = 1024, height = 768, vsync = true)]
struct MyEngine {
    #[use_pipeline]
    main_pipeline: MyPipeline,
}
```

### Step 6: Initialize Backend

```rust
fn main() {
    let config = MyEngine::engine_config();
    // Use config to set up Vulkan backend
    // (See linux_vulkan.rs for full initialization)
}
```

## Common Patterns

### Multiple Pipelines

```rust
#[derive(RenderEngine)]
struct MultiPipelineEngine {
    #[use_pipeline]
    geometry: GeometryPipeline,
    #[use_pipeline]
    lighting: LightingPipeline,
    #[use_pipeline]
    post_process: PostProcessPipeline,
}
```

### Per-Instance Data

```rust
#[derive(BufferLayout)]
struct InstanceData {
    #[location(4)]
    #[step(instance)]  // Per-instance, not per-vertex
    model_matrix: [[f32; 4]; 4],
}
```

### Custom Stride

```rust
#[derive(BufferLayout)]
#[stride(64)]  // Explicitly set stride (e.g., for padding)
struct PaddedVertex {
    #[location(0)]
    position: [f32; 3],
    // 52 bytes of padding implied
}
```

## Troubleshooting

**"Shader not found":**
- Paths are relative to project root
- Ensure shader files exist
- Check feature flags (`vk-shaderc-compile` for runtime compilation)

**"Binding mismatch":**
- Verify shader bindings match `ResourceBinding` attributes
- Check set/binding numbers align
- Ensure shader reflects latest derive changes (rebuild if needed)

**"Stride incorrect":**
- BufferLayout calculates stride automatically
- Use `#[stride(N)]` for explicit control
- Check `#[repr(C)]` on vertex structs

For more examples, see the `examples/` directory and existing showcase files.
