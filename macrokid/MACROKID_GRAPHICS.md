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

### Current Engine Process & Readiness

- Derives produce descriptors and trait impls:
  - `#[derive(ResourceBinding)]` → static slice of `BindingDesc` + `ResourceBindings` impl.
  - `#[derive(BufferLayout)]` → static slice of `VertexAttr` + `VertexLayout` impl.
  - `#[derive(GraphicsPipeline)]` → `PipelineDesc` and `PipelineInfo` impl.
    - New optional attributes add backend-agnostic pipeline state:
      - `polygon = "Fill|Line"`, `cull = "None|Front|Back"`, `front_face = "Cw|Ccw"`
      - `blend = true|false`, `samples = 1|2|4|8`
      - Example: `#[pipeline(vs = "triangle.vert", fs = "triangle.frag", topology = "TriangleList", polygon = "Fill", cull = "Back", front_face = "Cw", blend = false, samples = 1)]`
- Runtime flow in `macrokid_graphics::engine`:
  - Build `EngineConfig` (via `EngineBuilder` or manual Vec<PipelineDesc>`).

## Backend Options (Current Implementation)

To keep the runtime malleable and avoid hard-coded build parameters, `EngineConfig` now includes `options: BackendOptions` honored by the Vulkan backend on Linux. Defaults preserve existing behavior.

Supported fields:
- `present_mode`: "FIFO" | "MAILBOX" | "IMMEDIATE" | "FIFO_RELAXED" (default derived from `window.vsync`)
- `present_mode_priority`: priority list; first supported is used (e.g., "MAILBOX,IMMEDIATE,FIFO")
- `swapchain_images`: preferred image count (clamped to surface caps)
- `color_format`: e.g. "B8G8R8A8_SRGB"; `color_space`: e.g. "SRGB_NONLINEAR"
- `depth_format`: e.g. "D32_SFLOAT"
- `msaa_samples`: 1, 2, 4, 8, ...
- `dynamic_viewport` / `dynamic_scissor`: override dynamic states if desired
 - `adapter_index`: preferred device index
 - `adapter_preference`: "discrete" | "integrated" | "virtual" | "cpu"

Example:

```rust
use macrokid_graphics::engine::EngineBuilder;
use macrokid_graphics::pipeline::PipelineInfo;

#[pipeline(vs = "shaders/triangle.vert", fs = "shaders/triangle.frag", topology = "TriangleList")]
struct Triangle;

let cfg = EngineBuilder::new()
    .app("Demo")
    .window(1280, 720, true)
    .present_mode("FIFO")
    .swapchain_images(3)
    .msaa_samples(4)
    .add_pipeline(Triangle::pipeline_desc().clone())
    .build()?;
```

Notes:
- When `present_mode` is not set, `vsync = true` prefers FIFO; otherwise MAILBOX if available.
- `msaa_samples` overrides pipeline samples for the Vulkan attachments; ensure consistency across pipelines.

### Configure via Environment Variables

You can also populate `BackendOptions` from env with `BackendOptions::from_env()`.

Recognized env vars:
- `MK_PRESENT_MODE` (e.g., `MAILBOX`)
- `MK_PRESENT_MODE_PRIORITY` (comma-separated, e.g., `MAILBOX,IMMEDIATE,FIFO`)
- `MK_SWAPCHAIN_IMAGES` (integer)
- `MK_COLOR_FORMAT`, `MK_COLOR_SPACE`, `MK_DEPTH_FORMAT`
- `MK_MSAA_SAMPLES` (integer)
- `MK_DYNAMIC_VIEWPORT`, `MK_DYNAMIC_SCISSOR` (true/false/1/0)
- `MK_ADAPTER_INDEX` (integer), `MK_ADAPTER_PREFERENCE` (discrete|integrated|virtual|cpu)

Example:

```bash
MK_PRESENT_MODE_PRIORITY=MAILBOX,FIFO \
MK_ADAPTER_PREFERENCE=discrete \
MK_MSAA_SAMPLES=4 \
cargo run -p macrokid_graphics --example render_engine --features vulkan-linux
```

### Inspecting Effective Options

You can print the effective `BackendOptions` (after env merge and with implied defaults) using:

```rust
cfg.options.log_effective(&cfg.window);
```

The Vulkan backend also logs the selected adapter and present mode at startup, e.g.:

```
[vk-linux] adapter='NVIDIA RTX' type=DiscreteGPU present_mode=MAILBOX
```

Current project state:
- Builder and env-driven configuration across swapchain format/space, present mode, image count, MSAA, dynamics, and adapter selection.
- Vulkan Linux path honors these options with safe fallbacks.
- Examples `animated_demo` and `render_engine` demonstrate env+override and log effective settings.

  - Optionally `validate_config()` for structural checks (non-empty, no duplicates, shader paths present).
  - Validate pipelines against derives using either:
    - `engine.validate_pipelines_with::<RB, VL>(&cfg)` or
    - `cfg.validate_with::<GraphicsValidator<RB, VL>>()` (macrokid_core::common::validate facade).
  - Initialize pipelines and present (currently logs via the `RenderBackend` default methods).

Readiness for Vulkan (testing/hobby)
- The current engine provides structure and validation; the backend is a logging stub.
- Implementing a minimal Vulkan backend (e.g., with `ash`) is viable for hobby/testing but requires:
  - Device/swapchain setup (surface creation via `winit`/`raw-window-handle`).
  - Shader module creation from SPIR-V; map `ShaderPaths` to compiled assets.
  - Pipeline layout from `ResourceBindings`; descriptor set layouts + pools and allocation.
  - Vertex input state from `VertexLayout` (locations, formats, strides, step mode).
  - Render pass, framebuffers, command buffers, submission, and synchronization.
- Recommendation: For a fast path to drawing, integrate `wgpu` behind a `wgpu` backend that translates `PipelineDesc`/`ResourceBindings`/`VertexLayout` into wgpu pipeline descriptors. This reduces boilerplate while keeping the same derives + engine.

Summary: The abstractions and validation are in place for hobby/testing. A Vulkan backend can be implemented incrementally; wgpu offers a quicker route for early visuals. The derives (resources/layout/pipeline) already encode the data needed to wire either backend.

## Running

- Linux Vulkan example (feature-gated):
  - `cargo run -p macrokid_graphics --example linux_vulkan --features vulkan-linux,vk-shaderc-compile`
- Engine + derives demo:
  - `cargo run -p macrokid_graphics --example engine_demo`
- Animated uniform + texture demo:
  - `cargo run -p macrokid_graphics --example animated_demo --features vulkan-linux,vk-shaderc-compile`

## Vulkan (Linux) Backend

- Feature flags:
  - `vulkan-linux`: enables the Linux-facing Vulkan backend (ash + winit + ash-window).
  - `vk-shaderc-compile`: compiles inline GLSL at runtime via `shaderc` for the minimal triangle example.
- What it does today:
  - Creates instance, surface, physical device, logical device + queue (graphics/present).
  - Builds swapchain + image views; color+depth render pass; framebuffers.
  - Creates descriptor set layouts from `ResourceBindings`; allocates descriptor sets per-frame.
  - Builds graphics pipeline from `VertexLayout` and `PipelineDesc` state (topology, raster, blend, samples, optional depth/stencil, dynamic states, push constants).
  - Loads shaders from file paths in `PipelineDesc`:
    - `.spv` is read directly as SPIR‑V
    - `.vert`/`.frag` compiled at runtime with `shaderc` when `vk-shaderc-compile` is enabled
  - Uploads a demo texture via staging (1×1 RGBA or arbitrary size via AppResources) and binds descriptors.
  - Records command buffers that clear, bind descriptor sets/vertex buffers, draw; presents with per-frame sync.
- What derives drive:
  - `ResourceBindings` → Vulkan `DescriptorSetLayoutBinding`s (Uniform/Texture/Sampler) and pipeline layout.
  - `VertexLayout` → `VertexInputBindingDescription` (binding 0) and `VertexInputAttributeDescription`s.
  - `PipelineDesc` (from `GraphicsPipeline` derive or manual) → topology, raster state, blend enable, samples.
- Limitations:
  - No swapchain recreation on resize yet.
  - Descriptor writes are demo-oriented; a fuller resource API is in progress.
  - No shader reflection; validation relies on derive/runtime metadata.

## Derives: Graphics Attributes (Current)

- `#[derive(ResourceBinding)]` on a struct with fields annotated by:
  - `#[uniform(set = N, binding = M, stages = "vs|fs|cs")]`
  - `#[texture(set = N, binding = M, stages = "vs|fs|cs")]`
  - `#[sampler(set = N, binding = M, stages = "vs|fs|cs")]`
  - `#[combined(set = N, binding = M, stages = "vs|fs|cs")]` (Vulkan combined image sampler)
  - Emits: `impl ResourceBindings` with a static slice of `BindingDesc { set, binding, kind, stages: Option<BindingStages> }`.
- `#[derive(BufferLayout)]` on a struct with fields annotated by:
  - `#[vertex(location = L, binding = B, format = "vec2|vec3|vec4|f32|u32|i32|rgba8_unorm|u8x4_norm|..." )]`
  - Optional type-level: `#[buffer(binding = B?, stride = S?, step = "vertex|instance")]`.
  - Emits: `impl VertexLayout` with static `VertexAttr[]` and `VertexBufferDesc[]`.
- `#[derive(GraphicsPipeline)]` on a zero-sized type with:
  - `#[pipeline(vs = "path.vert", fs = "path.frag", topology = "TriangleList|LineList|PointList", depth = true|false, polygon = "Fill|Line", cull = "None|Front|Back", front_face = "Cw|Ccw", blend = true|false, samples = 1|2|4|8)]`
  - Depth/stencil: `depth_test`, `depth_write`, `depth_compare = "Less|LEqual|..."`
  - Dynamic states: `dynamic = "viewport,scissor"`
  - Push constants: `push_constants_size = N`, `push_constants_stages = "vs|fs|cs"`
  - Emits: `impl PipelineInfo` returning a static `PipelineDesc` with shader paths and optional pipeline state.

### RenderEngine derive

- `#[derive(RenderEngine)]` on a struct builds an engine config at compile time:
  - `#[app(name = "...")]`, `#[window(width = W, height = H, vsync = bool)]`
  - `#[use_pipeline]` on fields whose types implement `PipelineInfo`
  - Emits: `impl RenderEngineInfo` with `fn engine_config() -> EngineConfig`

## PipelineDesc Fields (Runtime)

- `name`: static identifier for deduping/logging.
- `shaders`: vertex/fragment paths (or, in future, bytes).
- `topology`: TriangleList, LineList, PointList.
- `depth`: whether to enable depth (Vulkan depth/stencil state pending).
- `raster`: optional `RasterState { polygon, cull, front_face }`.
- `blend`: optional `ColorBlendState { enable }`.
- `samples`: optional multisample count (1,2,4,8 supported).

## Protobuf Config Path

- New crate: `macrokid_graphics_proto` (prost + vendored protoc) with schema `proto/graphics_config.proto`.
  - Messages: `EngineConfig`, `WindowCfg`, `PipelineDesc`, `ShaderPaths`, `Topology`, `RasterState`, `ColorBlendState`.
  - Optional authorable: `ResourceBindingSet`, `VertexLayout` (planned integration).
- Feature flag `proto` in `macrokid_graphics` enables conversions:
  - `impl TryFrom<pb::EngineConfig> for engine::EngineConfig`.
  - `impl TryFrom[pb::PipelineDesc] for pipeline::PipelineDesc`.
  - `ShaderPaths`: currently supports path strings; SPIR-V bytes are reserved for a follow-up.
- Example loader:
  - `cargo run -p macrokid_graphics --example load_proto --features vulkan-linux,proto,vk-shaderc-compile -- <path.pb>`
  - Without an argument, it builds a small in-memory proto config and runs.
- Why protobuf:
  - Cross-language authoring and tooling.
  - Versioned, evolvable config assets.
  - Parallel to derive-driven (compile-time) configs; both validated by the same runtime.

## App-Supplied Resources (Vulkan demo)

For quick demos, `vk_linux` exposes `AppResources` and convenience runners:

- `AppResources` fields:
  - `uniform_data: Option<Vec<u8>>` (initial contents for a 64‑byte per-frame UBO)
  - `image_rgba: Option<[u8;4]>` (1×1 color) or `image_size + image_pixels` for RGBA8 images
- Runners:
  - `run_vulkan_linux_app_with_resources<RB, VL>(cfg, &resources)`
  - `run_vulkan_linux_app_with_resources_and_update<RB, VL, F>(cfg, &resources, update)`
    - `update(frame_idx) -> Option<Vec<u8>>` updates the current frame’s UBO before submit

## Mapping To Vulkan

- Descriptor set layouts: from `ResourceBindings` kinds → VK descriptor types; merged by set index.
- Pipeline layout: vector of set layouts; push constants TBD.
- Vertex input:
  - Binding 0: `VertexBufferDesc { stride, step }` → input rate.
  - Attributes: `VertexAttr { location, format, offset }` → VK formats (vec2/vec3/vec4 mapped to R32G32/R32G32B32/R32G32B32A32_SFLOAT).
- Raster/blend/samples: pulled from `PipelineDesc` if present; otherwise uses defaults.
- Frame graph: single subpass render pass with one color attachment (clear+store+present).

## Roadmap (Focused)

- Vulkan
  - Swapchain recreation on resize.
  - Descriptor pool, allocate descriptor sets, and bind minimal UBO/texture/sampler.
  - Support loading SPIR-V bytes (from proto) and files (from desc) interchangeably.
  - Depth/stencil, per-attachment blending, dynamic state, and push constants.
- Derives
  - `ResourceBinding`: add `stages = "vs|fs|..."` support; consider `CombinedImageSampler` kind.
  - `BufferLayout`: extend `format` vocabulary; consider multiple vertex buffers.
  - `GraphicsPipeline`: add `resources = Type, vertex = Type` to tie pipeline to RB/VL types.
- Protobuf
  - Add ResourceBindingSet and VertexLayout conversions into runtime traits.
  - Round-trip helpers: export derives to proto and load back.

## Recommended Next Steps (Derives + Bridges)

- ResourceBinding
  - Add `stages = "vs|fs|..."` on fields to control visibility masks.
  - Extend `BindingDesc` to carry an optional stage mask. Default remains VS|FS.
  - Add `CombinedImageSampler` in addition to separate `Texture`/`Sampler` kinds; allow both styles.

- VertexLayout
  - Expand `format` vocabulary: `u32`, `i32`, `rgba8_unorm`, `u8x4_norm`, `u16x2`, etc.
  - Support multiple buffers: buffer-level `binding` in `#[buffer(binding = N, step = "vertex|instance", stride = S)]`.
  - Update Vulkan mapping to emit multiple `VertexInputBindingDescription`s.

- GraphicsPipeline
  - Add depth/stencil controls: `depth_test`, `depth_write`, `depth_compare = "Less|LEqual|Equal|..."`.
  - Per-attachment blending description and write masks.
  - Dynamic states: `dynamic = "viewport,scissor"`.
  - Push constants: accept a push-constant type + range, generate pipeline layout integration.
  - Associate RB/VL types: `#[pipeline(resources = Material, vertex = Vertex)]` to build end-to-end from a single type.

- Vulkan Bridge Helpers (non-proc-macro)
  - Extract helpers from `vk_linux`:
    - `vk_descriptor_set_layouts<RB: ResourceBindings>() -> Vec<vk::DescriptorSetLayoutBinding>`
    - `vk_vertex_input<VL: VertexLayout>() -> (Vec<vk::VertexInputBindingDescription>, Vec<vk::VertexInputAttributeDescription>)`
  - Centralize pipeline creation:
    - `build_pipeline_from<T: PipelineInfo>(...)` where `T` optionally carries `resources = R, vertex = V` via trait bounds.
  - Benefit: reusable, testable mapping independent of window/swapchain code.


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
