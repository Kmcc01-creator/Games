# macrokid_graphics: Domain-Specific Graphics Framework

## Overview

`macrokid_graphics` is a specialized crate that extends `macrokid_core` to provide ergonomic, type-safe DSLs for graphics programming. It supports multiple rendering backends (Vulkan, Wayland, D3D12) while maintaining a unified API design.

## Architecture Philosophy

### Separation of Concerns
```
macrokid_core     → Universal building blocks (IR, attrs, builders)
macrokid_graphics → Graphics domain expertise (pipelines, shaders, backends)
```

This design keeps the core lightweight (~50KB) while enabling rich domain-specific functionality without bloating the foundational framework.

### Key Principles
- **Single Source, Multiple Targets**: One DSL definition → multiple backend implementations
- **Performance by Design**: Built-in support for Data-Oriented Design patterns
- **Type Safety**: Compile-time validation of graphics resource relationships
- **Developer Experience**: Clear error messages with actionable suggestions

## Crate Structure

```
macrokid_graphics/
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── pipeline/           # Rendering pipeline DSL
│   │   ├── mod.rs
│   │   ├── builder.rs      # Type-state pipeline builders
│   │   ├── validation.rs   # Cross-validation of pipeline components
│   │   └── codegen.rs      # Backend-specific code generation
│   ├── resources/          # Resource binding and management
│   │   ├── mod.rs
│   │   ├── buffers.rs      # Vertex/index buffer layout generation
│   │   ├── textures.rs     # Texture binding and sampling
│   │   └── uniforms.rs     # Uniform buffer layout
│   ├── backends/           # Backend-specific implementations
│   │   ├── mod.rs
│   │   ├── vulkan.rs       # Vulkan API generation
│   │   ├── wayland.rs      # Wayland compositor integration
│   │   └── traits.rs       # Common backend abstractions
│   ├── heat/               # Data-Oriented Design support
│   │   ├── mod.rs
│   │   ├── analysis.rs     # Access pattern profiling
│   │   └── layout.rs       # Memory layout optimization
│   └── validation/         # Graphics-specific validation
│       ├── mod.rs
│       ├── shaders.rs      # Shader interface compatibility
│       └── resources.rs    # Resource binding validation
```

## Implementation Plan

### Phase 1: Core Graphics DSL Migration
**Goal**: Extract existing graphics work from `examples/` into production-ready crate

#### 1.1 Pipeline DSL (`pipeline/`)
Migrate and enhance the current `vk_engine!` macro:

```rust
// Current examples/gfx_dsl approach
vk_engine! {
    {
        app: "Demo",
        window: { width: 1024, height: 768 },
        graph: {
            pass main {
                pipelines: [
                    pipeline triangle {
                        vs: "shaders/triangle.vert",
                        fs: "shaders/triangle.frag",
                        topology: TriangleList,
                    }
                ]
            }
        }
    }
}

// Enhanced macrokid_graphics approach  
#[derive(RenderEngine)]
#[backend(vulkan)]
pub struct GameEngine {
    #[window(width = 1024, height = 768)]
    display: WindowConfig,
    
    #[render_pass(name = "main")]
    geometry_pass: GeometryPass,
}

#[derive(RenderPass)]
pub struct GeometryPass {
    #[pipeline(vs = "triangle.vert", fs = "triangle.frag")]
    triangle_pipeline: GraphicsPipeline<TriangleVertex>,
    
    #[pipeline(vs = "lines.vert", fs = "lines.frag", topology = "LineList")]  
    wireframe_pipeline: GraphicsPipeline<LineVertex>,
}
```

**Migration Steps**:
1. Copy `examples/gfx_dsl/src/lib.rs` → `macrokid_graphics/src/pipeline/mod.rs`
2. Replace manual parsing with `macrokid_core::attrs` validation
3. Enhance error messages using `macrokid_core::diag`
4. Add type-state builders from `examples/gfx_dsl_support/src/builder.rs`

#### 1.2 Resource Management (`resources/`)
Migrate resource binding work from `examples/render_resources*`:

```rust
// Enhanced resource binding with validation
#[derive(ResourceLayout)]
pub struct MaterialUniforms {
    #[uniform(binding = 0, stage = "vertex")]
    mvp_matrix: Mat4,
    
    #[uniform(binding = 1, stage = "fragment")]
    material_params: MaterialParams,
    
    #[texture(binding = 2, format = "RGBA8")]
    albedo_texture: Texture2D,
    
    #[sampler(binding = 3, filter = "linear")]
    texture_sampler: Sampler,
}

// Cross-validation with shader interfaces
#[derive(ShaderInterface)]
pub struct VertexShader {
    #[input(location = 0)] position: Vec3,
    #[input(location = 1)] normal: Vec3,
    #[input(location = 2)] uv: Vec2,
    
    #[uniform(set = 0)] material: MaterialUniforms,  // Validates compatibility
}
```

**Migration Steps**:
1. Copy resource binding derives from `examples/render_resources/`
2. Integrate with `macrokid_core::TypeSpec` for enhanced analysis
3. Add cross-validation between resource layouts and shader interfaces
4. Generate binding descriptor code for multiple backends

#### 1.3 Backend Abstraction (`backends/`)
Create unified backend trait with specialized implementations:

```rust
// Common backend interface
pub trait RenderBackend {
    type Device;
    type Pipeline;
    type CommandBuffer;
    
    fn create_pipeline(desc: &PipelineDesc) -> Self::Pipeline;
    fn begin_render_pass(cmd: &mut Self::CommandBuffer, pass: &RenderPassDesc);
}

// Vulkan implementation (for games/applications)
impl RenderBackend for VulkanBackend {
    // Rich graphics pipeline creation
    // Complex resource binding
    // Multi-threaded command recording
}

// Wayland implementation (for compositors/system graphics)
impl RenderBackend for WaylandBackend {
    // Lightweight surface composition  
    // Buffer sharing protocols
    // Damage tracking optimization
}
```

### Phase 2: Advanced Features
**Goal**: Add sophisticated graphics programming capabilities

#### 2.1 Data-Oriented Design Support (`heat/`)
Add automated performance optimization:

```rust
#[derive(RenderData)]
#[layout(data_oriented)]
pub struct SceneData {
    // Ultra-hot: accessed every frame
    #[heat(ultra, simd_align)]
    mesh_ids: Vec<u32>,
    
    #[heat(ultra, simd_align)]  
    transform_indices: Vec<u32>,
    
    // Hot: accessed per render pass
    #[heat(hot)]
    material_ids: Vec<u32>,
    
    // Warm: state changes only
    #[heat(warm)]
    pipeline_states: Vec<PipelineState>,
    
    // Cold: debug/development
    #[heat(cold)]
    debug_names: Vec<String>,
}

// Generates optimized SoA layout:
pub struct SceneDataSoA {
    // Cache-friendly hot data
    pub ultra_data: UltraHotData,   // 64-byte aligned, SIMD-ready
    pub hot_data: HotData,          // Separate cache lines
    pub warm_data: Vec<WarmData>,   // On-demand loading
    pub cold_data: Vec<ColdData>,   // Debug builds only
}
```

#### 2.2 Multi-Backend Code Generation
Generate backend-specific optimizations from single source:

```rust
#[derive(GraphicsPipeline)]
#[backends(vulkan, d3d12, wayland)]
pub struct MaterialPipeline {
    #[vulkan(descriptor_set = 0)]
    #[d3d12(root_parameter = 0)]
    #[wayland(uniform_buffer)]
    material_uniforms: MaterialData,
}

// Generates three separate implementations:
// impl VulkanPipeline for MaterialPipeline { ... }
// impl D3D12Pipeline for MaterialPipeline { ... }  
// impl WaylandPipeline for MaterialPipeline { ... }
```

#### 2.3 Shader Interface Validation
Compile-time shader compatibility checking:

```rust
#[derive(VertexLayout)]
struct Vertex {
    #[location(0)] pos: Vec3,
    #[location(1)] normal: Vec3,
    #[location(2)] uv: Vec2,
}

#[derive(Pipeline)]
#[vertex_shader("triangle.vert")]    // Parsed at compile time
#[fragment_shader("triangle.frag")]  // Interface extracted  
struct TrianglePipeline<V: VertexLayout> {
    vertex_data: V,
}

// Compile error if vertex layout doesn't match shader inputs
type MyPipeline = TrianglePipeline<Vertex>;  // ✅ Compatible
type BadPipeline = TrianglePipeline<DifferentVertex>;  // ❌ Compile error
```

### Phase 3: Developer Experience
**Goal**: Production-ready developer tools and optimization

#### 3.1 Enhanced Error Messages
Leverage `macrokid_core::diag` for actionable graphics errors:

```rust
error: shader interface mismatch
  --> src/pipeline.rs:15:1
   |
15 | #[derive(Pipeline)]
   | ^^^^^^^^^^^^^^^^^^^ vertex shader expects Vec3 at location 1, but vertex layout provides Vec4
   |
help: change vertex layout to match shader
   |
12 | #[location(1)] normal: Vec3,  // Change Vec4 to Vec3
   |                        ^^^^
```

#### 3.2 Performance Analysis Tools
Built-in profiling and optimization suggestions:

```rust
#[derive(RenderData)]
#[profile(collect_heat_data)]  // Instruments field access
pub struct SceneData {
    // Profiler automatically tracks access patterns
    mesh_ids: Vec<u32>,        // Accessed 10,000x per frame → ultra-hot
    debug_names: Vec<String>,  // Accessed 2x per session → cold
}

// cargo run --features=heat-profiling
// Generates heat_profile.json with actual usage data
// Suggestions for optimal memory layout
```

#### 3.3 Code Generation Debugging
Tools for understanding generated code:

```bash
# Expand macros to see generated code
cargo expand --features=macrokid-debug

# Validate resource bindings
cargo check --features=validate-shaders

# Analyze memory layouts  
cargo run --bin analyze-layout --features=heat-analysis
```

## API Design Principles

### 1. Progressive Disclosure
```rust
// Simple case: minimal configuration
#[derive(RenderEngine)]
struct SimpleEngine {
    #[pipeline(vs = "quad.vert", fs = "texture.frag")]
    blit_pipeline: Pipeline,
}

// Complex case: full control when needed
#[derive(RenderEngine)] 
#[backend(vulkan)]
#[validation(strict)]
struct ComplexEngine {
    #[render_pass(
        attachments = [ColorAttachment::RGBA8, DepthAttachment::D24S8],
        samples = 4
    )]
    geometry_pass: GeometryRenderPass,
    
    #[render_pass(load_op = "load", store_op = "store")]
    postprocess_pass: PostProcessRenderPass,
}
```

### 2. Type Safety with Ergonomics
```rust
// Type-safe resource binding
struct Material {
    #[texture] albedo: Texture2D,      // Strong typing
    #[texture] normal: NormalMap,      // Semantic types
    #[uniform] params: MaterialParams, // Layout validation
}

// Compile-time compatibility checking
struct Pipeline<V, M> 
where 
    V: VertexLayout,
    M: MaterialLayout,
{
    vertex_format: V,
    material: M,
}
```

### 3. Backend Abstraction
```rust
// Same API, different backends
let vulkan_engine = Engine::<VulkanBackend>::new(&config);
let wayland_engine = Engine::<WaylandBackend>::new(&config);  
let d3d12_engine = Engine::<D3D12Backend>::new(&config);

// Backend-specific optimizations automatically applied
vulkan_engine.draw_indexed();    // Uses vkCmdDrawIndexed
wayland_engine.composite_surface(); // Uses wl_surface protocol
```

## Migration from Examples

### Step 1: Create Crate Structure
```bash
cargo new macrokid_graphics --lib
cd macrokid_graphics

# Copy existing work as baseline
cp -r ../examples/gfx_dsl/src/* src/pipeline/
cp -r ../examples/gfx_dsl_support/src/* src/resources/
cp -r ../examples/render_resources/src/* src/resources/
```

### Step 2: Refactor with macrokid_core
```rust
// Replace manual parsing
// OLD: examples/gfx_dsl/src/lib.rs
impl Parse for WindowCfgAst {
    fn parse(input: ParseStream) -> Result<Self> {
        // 40+ lines of manual parsing...
    }
}

// NEW: macrokid_graphics/src/pipeline/mod.rs  
use macrokid_core::attrs::{validate_attrs, AttrSpec, AttrType};

const WINDOW_SCHEMA: &[AttrSpec] = &[
    AttrSpec { key: "width", required: false, ty: AttrType::Int },
    AttrSpec { key: "height", required: false, ty: AttrType::Int },
    AttrSpec { key: "vsync", required: false, ty: AttrType::Bool },
];

fn parse_window_config(attrs: &[Attribute]) -> syn::Result<WindowConfig> {
    let values = validate_attrs(attrs, "window", WINDOW_SCHEMA)?;
    // Clean, validated parsing with good error messages
}
```

### Step 3: Add Domain Expertise
```rust
// Add graphics-specific validation
impl GraphicsTypeSpec for TypeSpec {
    fn validate_shader_compatibility(&self, shader_path: &str) -> syn::Result<()> {
        // Parse shader file and validate interface compatibility
        // This wouldn't belong in macrokid_core
    }
    
    fn generate_binding_descriptors(&self) -> Vec<DescriptorBinding> {
        // Graphics-specific resource analysis
    }
}
```

### Step 4: Enhance Developer Experience
```rust
// Better error messages
#[derive(Pipeline)]
#[shader(vs = "missing.vert")]  // File doesn't exist
struct BrokenPipeline;

// Error: shader file not found
//   --> src/pipeline.rs:2:1
//    |
//  2 | #[shader(vs = "missing.vert")]
//    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ shader file 'missing.vert' not found
//    |
//    help: create the shader file or check the path
//       expected location: shaders/missing.vert
```

## Benefits Over Examples Approach

### Maintainability
- **Single Source of Truth**: One implementation vs scattered examples
- **Consistent API**: Unified patterns across all graphics functionality
- **Better Testing**: Comprehensive test suite vs example-only validation

### Usability  
- **Clear Documentation**: API docs vs example comments
- **IDE Integration**: Full autocomplete and error highlighting
- **Semantic Versioning**: Stable API contracts vs example drift

### Performance
- **Optimized Builds**: Release-ready code generation
- **Feature Flags**: Users only pay for what they use
- **Compile-time Optimization**: More analysis time budget vs examples

### Ecosystem Integration
- **Cargo Integration**: Standard dependency management
- **Documentation**: docs.rs integration with examples
- **Community**: Crates.io discoverability vs hidden examples

The `macrokid_graphics` crate transforms experimental graphics work into a production-ready framework while maintaining the innovative DSL approach developed in the examples.