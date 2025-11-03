# Usage Examples: Multi-threading + Barrier Generation

This document shows how to use the newly implemented multi-threaded recording and barrier generation features.

---

## Example 1: GPU Resource Types with Barrier Hints

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuImage, GpuResourceAccess};
use macrokid_threads_derive::System;

// Define marker types for GPU resources
struct VertexData;
struct RenderTarget;

// System that reads vertices and writes to render target
#[derive(System)]
#[reads(GpuBuffer<VertexData>)]
#[writes(GpuImage<RenderTarget>)]
struct DrawGeometry;

fn main() {
    // Print generated barrier hints
    println!("{}", DrawGeometry::barrier_requirements());

    /* Output:
     * GPU Barrier Requirements:
     *
     *   READ: GpuBuffer<VertexData>
     *     Resource: Buffer
     *     Stage: VERTEX_INPUT → TRANSFER
     *     Access: VERTEX_ATTRIBUTE_READ → TRANSFER_WRITE
     *     Action: Insert buffer/image barrier before read
     *
     *   WRITE: GpuImage<RenderTarget>
     *     Resource: Image
     *     Stage: COLOR_ATTACHMENT_OUTPUT
     *     Access: COLOR_ATTACHMENT_WRITE
     *     Layout transition may be required
     */
}
```

**What happened**:
1. System derive detected `GpuBuffer` and `GpuImage` types
2. Generated `GpuResourceAccess` impl with metadata
3. Type names inferred pipeline stages:
   - `VertexData` → `VERTEX_INPUT` stage
   - `RenderTarget` → `COLOR_ATTACHMENT_OUTPUT` stage
4. `barrier_requirements()` generated human-readable hints

---

## Example 2: Mixed CPU and GPU Resources

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuImage, GpuResourceAccess};
use macrokid_core::threads::ResourceAccess;
use macrokid_threads_derive::System;

// CPU types
struct Transform;
struct Camera;

// GPU types
struct UniformData;
struct ShadowMap;

#[derive(System)]
#[reads(Transform, Camera, GpuBuffer<UniformData>)]  // Mix CPU and GPU
#[writes(GpuImage<ShadowMap>)]
struct RenderShadows;

fn main() {
    // CPU resources
    println!("CPU reads: {:?}", RenderShadows::reads());
    // Output: [TypeId(Transform), TypeId(Camera)]

    // GPU resources
    println!("{}", RenderShadows::barrier_requirements());
    /* Output:
     * GPU Barrier Requirements:
     *
     *   READ: GpuBuffer<UniformData>
     *     Resource: Buffer
     *     Stage: VERTEX_SHADER | FRAGMENT_SHADER → TRANSFER
     *     Access: UNIFORM_READ → TRANSFER_WRITE
     *     ...
     */
}
```

**Key insight**: CPU and GPU resources are tracked separately, no conflicts.

---

## Example 3: Type-Level Stage Inference

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuResource};
use ash::vk;

// Different type names infer different pipeline stages

struct VertexPositions;
struct IndexBuffer;
struct UniformBlock;
struct StorageBuffer;

fn print_stage<T: 'static + Send + Sync>()
where
    GpuBuffer<T>: GpuResource
{
    let meta = GpuBuffer::<T>::metadata();
    println!("{}: {:?} → {:?}",
             meta.type_name,
             meta.write_stage,
             meta.read_stage);
}

fn main() {
    print_stage::<VertexPositions>();
    // GpuBuffer<VertexPositions>: TRANSFER → VERTEX_INPUT

    print_stage::<IndexBuffer>();
    // GpuBuffer<IndexBuffer>: TRANSFER → VERTEX_INPUT (contains "Index")

    print_stage::<UniformBlock>();
    // GpuBuffer<UniformBlock>: TRANSFER → (VERTEX_SHADER | FRAGMENT_SHADER)

    print_stage::<StorageBuffer>();
    // GpuBuffer<StorageBuffer>: TRANSFER → COMPUTE_SHADER (contains "Storage")
}
```

**Heuristics**:
- Type name contains "Vertex" or "Index" → `VERTEX_INPUT`
- Type name contains "Uniform" or "Ubo" → shader stages
- Type name contains "Storage" → `COMPUTE_SHADER`
- Default → `ALL_COMMANDS` (conservative)

---

## Example 4: Image Layout Tracking

```rust
use macrokid_graphics::resources::{GpuImage, GpuResource};
use ash::vk;

struct RenderTarget;
struct DepthBuffer;
struct Texture;

fn print_layouts<T: 'static + Send + Sync>()
where
    GpuImage<T>: GpuResource
{
    let dummy_image = GpuImage::<T>::new(
        vk::Image::null(),
        vk::Extent2D { width: 1920, height: 1080 },
        vk::Format::R8G8B8A8_SRGB,
    );

    let meta = GpuImage::<T>::metadata();
    let handle = dummy_image.handle();

    if let GpuHandle::Image(_, layout_info) = handle {
        println!("{}: {} → {}",
                 meta.type_name,
                 format!("{:?}", layout_info.current_layout),
                 format!("{:?}", layout_info.target_layout));
    }
}

fn main() {
    print_layouts::<RenderTarget>();
    // GpuImage<RenderTarget>: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL

    print_layouts::<DepthBuffer>();
    // GpuImage<DepthBuffer>: UNDEFINED → DEPTH_STENCIL_ATTACHMENT_OPTIMAL

    print_layouts::<Texture>();
    // GpuImage<Texture>: UNDEFINED → SHADER_READ_ONLY_OPTIMAL
}
```

**Thread Safety**: GpuImage uses `AtomicU32` for layout tracking:
```rust
// Thread-safe updates
image.set_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
let current = image.current_layout();  // Atomic load
```

---

## Example 5: Multi-stage Pipeline with Barriers

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuImage, GpuResourceAccess};
use macrokid_threads_derive::{Job, System, Schedule};
use macrokid_core::threads::ThreadPool;

struct MeshData;
struct LightingData;
struct FinalImage;

// Upload stage: writes GPU buffers
#[derive(Clone, Job, System)]
#[writes(GpuBuffer<MeshData>, GpuBuffer<LightingData>)]
struct UploadResources {
    // mesh_data, lighting_data...
}
impl UploadResources {
    fn run(self) {
        // Upload to GPU buffers (TRANSFER stage)
        println!("Uploading resources...");
    }
}

// Render stage: reads GPU buffers, writes render target
#[derive(Clone, Job, System)]
#[reads(GpuBuffer<MeshData>, GpuBuffer<LightingData>)]
#[writes(GpuImage<FinalImage>)]
struct RenderScene {
    // rendering state...
}
impl RenderScene {
    fn run(self) {
        println!("Barrier hints before rendering:");
        println!("{}", Self::barrier_requirements());

        // Render (needs barriers from TRANSFER → VERTEX_INPUT)
        println!("Rendering scene...");
    }
}

#[derive(Schedule)]
struct FramePipeline {
    #[stage(name = "upload")]
    upload: (UploadResources,),

    #[stage(name = "render", after = "upload")]
    render: (RenderScene,),
}

fn main() {
    let pool = ThreadPool::new(4);

    let pipeline = FramePipeline {
        upload: (UploadResources { /* ... */ },),
        render: (RenderScene { /* ... */ },),
    };

    // CPU-side scheduling with GPU barrier hints
    pipeline.run(&pool);

    /* Output:
     * Uploading resources...
     * Barrier hints before rendering:
     * GPU Barrier Requirements:
     *
     *   READ: GpuBuffer<MeshData>
     *     Stage: TRANSFER → VERTEX_INPUT
     *     Access: TRANSFER_WRITE → VERTEX_ATTRIBUTE_READ
     *     Action: Insert buffer/image barrier before read
     *   ...
     * Rendering scene...
     */
}
```

**Key Point**: Barrier hints tell you to insert barriers **between** upload and render stages.

---

## Example 6: ThreadLocalPools (Low-level API)

```rust
use macrokid_graphics::vk_linux::ThreadLocalPools;
use ash::{vk, Device};

unsafe fn setup_multi_threaded_recording(device: &Device, qfi: u32) -> ThreadLocalPools {
    // Create pools for 4 worker threads
    // Each thread gets 8 secondary command buffers
    let pools = ThreadLocalPools::new(device, qfi, 4, 8).unwrap();

    println!("Created {} thread pools", pools.num_threads());

    pools
}

unsafe fn record_on_thread(pools: &ThreadLocalPools, thread_idx: usize) {
    // Get secondary command buffer for this thread
    let secondary_cb = pools.get_secondary(thread_idx, 0);

    // Begin recording with inheritance info
    let inheritance = vk::CommandBufferInheritanceInfo::builder()
        .render_pass(render_pass)
        .subpass(0);

    let begin_info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
        .inheritance_info(&inheritance);

    device.begin_command_buffer(secondary_cb, &begin_info).unwrap();

    // Record commands...
    device.cmd_bind_pipeline(secondary_cb, vk::PipelineBindPoint::GRAPHICS, pipeline);
    device.cmd_draw(secondary_cb, 3, 1, 0, 0);

    device.end_command_buffer(secondary_cb).unwrap();
}

unsafe fn execute_secondaries(
    device: &Device,
    primary_cb: vk::CommandBuffer,
    pools: &ThreadLocalPools,
) {
    // Collect all recorded secondaries
    let mut secondaries = Vec::new();
    for thread_idx in 0..pools.num_threads() {
        secondaries.push(pools.get_secondary(thread_idx, 0));
    }

    // Execute in primary command buffer
    device.cmd_begin_render_pass(primary_cb, &render_pass_begin, vk::SubpassContents::SECONDARY_COMMAND_BUFFERS);
    device.cmd_execute_commands(primary_cb, &secondaries);
    device.cmd_end_render_pass(primary_cb);
}
```

**Current Status**: Low-level infrastructure complete. High-level VkFrame/VkCommandEncoder API pending.

---

## Example 7: Real-World Integration (Conceptual)

```rust
// What it will look like once VkFrame + VkCommandEncoder are complete

use macrokid_graphics::engine::{Renderer, Frame};
use macrokid_threads_derive::{Job, System, Schedule};
use macrokid_core::threads::ThreadPool;
use std::sync::Arc;

struct MeshData;

#[derive(Job, System)]
#[reads(GpuBuffer<MeshData>)]
struct RecordGeometryPass {
    frame: Arc<VkFrame>,  // Shared across threads
    meshes: Vec<Mesh>,
}

impl RecordGeometryPass {
    fn run(self) {
        // Get thread-safe command encoder
        let encoder = self.frame.encoder_for("geometry");

        // Record Vulkan commands (each thread records to different secondary CB)
        encoder.bind_pipeline(geometry_pipeline);
        for mesh in self.meshes {
            encoder.bind_vertex_buffer(mesh.vbo);
            encoder.draw(mesh.vertex_count, 1, 0, 0);
        }

        encoder.finish();  // Adds CB to frame's recorded list
    }
}

#[derive(Schedule)]
struct ParallelRenderSchedule {
    // All jobs execute in parallel across thread pool
    #[stage(name = "record_passes")]
    passes: (RecordGeometryPass, RecordLightingPass, RecordShadowPass),
}

fn render_frame(renderer: &VkRenderer, schedule: &ParallelRenderSchedule, pool: &ThreadPool) {
    // Begin frame (allocates secondary buffers from thread pools)
    let frame = renderer.begin_frame();

    // Schedule parallel recording (macrokid_core::threads handles distribution)
    schedule.run(pool);

    // Finalize (collects all secondaries, executes in primary CB)
    renderer.end_frame(frame.finalize());
}
```

**Status**: Conceptual - requires Track 1.2 (VkFrame + VkCommandEncoder) implementation.

---

## Best Practices

### 1. Type Naming Conventions

**Good**:
```rust
struct VertexPositions;      // ✅ Clear, infers VERTEX_INPUT
struct UniformMaterial;      // ✅ Clear, infers shader stages
struct StorageParticles;     // ✅ Clear, infers COMPUTE_SHADER
struct RenderTargetHDR;      // ✅ Clear, infers COLOR_ATTACHMENT
```

**Avoid**:
```rust
struct Data;                 // ❌ Too generic, falls back to ALL_COMMANDS
struct Buffer;               // ❌ Doesn't indicate usage
struct Thing;                // ❌ No semantic meaning
```

### 2. Mixing CPU and GPU Resources

**Pattern**:
```rust
#[derive(System)]
#[reads(
    // CPU resources first (convention)
    Transform,
    Camera,
    // GPU resources after
    GpuBuffer<VertexData>,
    GpuBuffer<InstanceData>,
)]
#[writes(
    GpuImage<RenderTarget>
)]
struct MySystem;
```

### 3. Barrier Hint Review

```rust
// During development, check barrier requirements
#[test]
fn review_barrier_requirements() {
    println!("{}", RenderGeometry::barrier_requirements());
    println!("{}", ComputeLighting::barrier_requirements());
    // Review output, implement barriers in rendering code
}
```

### 4. Thread Pool Sizing

```rust
// Match thread pool size to CPU cores (leave 1-2 for OS)
let num_workers = std::thread::available_parallelism()
    .map(|n| n.get() - 1)
    .unwrap_or(4)
    .max(1);

let pool = ThreadPool::new(num_workers);

// ThreadLocalPools should match
let thread_pools = ThreadLocalPools::new(&device, qfi, num_workers, 8)?;
```

### 5. When to Use Multi-threading

**Use multi-threaded recording when**:
- Drawing 1000+ objects per frame
- Many pipeline/descriptor binds
- CPU profiling shows recording as bottleneck

**Stick with single-threaded when**:
- Simple scenes (<100 draw calls)
- GPU-bound (changing recording won't help)
- Single-core targets (embedded)

---

## Debugging Tips

### 1. Verify GPU Resource Detection

```rust
#[test]
fn verify_gpu_detection() {
    // Check if GPU resources were detected
    assert!(!MySystem::gpu_reads().is_empty(), "GPU reads should be detected");
    assert!(!MySystem::gpu_writes().is_empty(), "GPU writes should be detected");

    // Print for manual review
    for meta in MySystem::gpu_reads() {
        println!("Read: {} ({:?})", meta.type_name, meta.resource_kind);
    }
}
```

### 2. Barrier Validation (Future)

```rust
// Phase 2: Debug validation (not yet implemented)
#[cfg(debug_assertions)]
impl MySystem {
    fn validate_barriers(&self, ctx: &RenderContext) {
        // Will check if required barriers were inserted
        // Panic with actionable error if missing
    }
}
```

### 3. Thread Pool Debugging

```rust
unsafe fn debug_thread_pools(pools: &ThreadLocalPools) {
    println!("Thread pools: {}", pools.num_threads());
    println!("Buffers per thread: {}", pools.max_secondary_per_thread);

    // Check if pools are valid
    for i in 0..pools.num_threads() {
        let cb = pools.get_secondary(i, 0);
        assert!(cb != vk::CommandBuffer::null(), "Thread {} CB is null", i);
    }
}
```

---

## Next Steps

1. **Try the examples** above to understand the API
2. **Review barrier hints** for your rendering systems
3. **Plan multi-threaded recording** once VkFrame/VkCommandEncoder are ready
4. **Contribute examples** to `examples/` directory

---

**See Also**:
- [Multi-threaded Recording Design](MULTITHREADED_RECORDING_DESIGN.md)
- [Barrier Code Generation Design](BARRIER_CODEGEN_DESIGN.md)
- [Parallel Implementation Summary](PARALLEL_IMPLEMENTATION_SUMMARY.md)
