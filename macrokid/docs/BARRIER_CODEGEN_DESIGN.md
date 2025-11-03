# Memory Barrier Code Generation Design

## Overview

Leverage `#[derive(System)]` resource tracking to automatically generate:
1. **Barrier hint comments** - Human-readable synchronization requirements
2. **Barrier validation code** - Runtime checks for missing barriers
3. **Barrier generation code** (optional) - Actual Vulkan barrier emission

This extends CPU-side conflict detection to GPU-side synchronization.

---

## Motivation

**Current problem**: Vulkan barriers are manual and error-prone

```rust
// User must remember:
device.cmd_pipeline_barrier(
    cb,
    vk::PipelineStageFlags::TRANSFER,       // What wrote last?
    vk::PipelineStageFlags::FRAGMENT_SHADER, // What reads next?
    &[vk::ImageMemoryBarrier {
        src_access_mask: vk::AccessFlags::TRANSFER_WRITE,  // Which operations?
        dst_access_mask: vk::AccessFlags::SHADER_READ,
        old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,  // Which layout?
        new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image: render_target,
        // ...
    }]
);
```

**Proposed solution**: Generate hints from derive attributes

```rust
#[derive(System)]
#[reads(GpuBuffer<Transform>)]  // ← GPU resource type
#[writes(GpuImage<RenderTarget>)]
struct RenderPass;

// Generated:
impl RenderPass {
    fn run(self) {
        // BARRIER HINT: GpuBuffer<Transform>
        //   Previous: TRANSFER stage, TRANSFER_WRITE access
        //   Current:  VERTEX_SHADER stage, SHADER_READ access
        //   Action:   Insert buffer barrier before vertex shader access

        // BARRIER HINT: GpuImage<RenderTarget>
        //   Previous: UNDEFINED layout
        //   Current:  COLOR_ATTACHMENT_OPTIMAL layout, COLOR_ATTACHMENT_WRITE access
        //   Action:   Insert layout transition before render pass

        // User's actual rendering code...
    }
}
```

---

## Design: GPU Resource Type System

### 1. Define GPU Resource Traits

```rust
// macrokid_graphics/src/resources.rs

/// Marker trait for GPU-accessible resources
pub trait GpuResource: Send + Sync + 'static {
    /// Vulkan resource handle (for barrier construction)
    fn handle(&self) -> GpuHandle;

    /// Default pipeline stage for reads
    fn read_stage() -> vk::PipelineStageFlags;

    /// Default pipeline stage for writes
    fn write_stage() -> vk::PipelineStageFlags;

    /// Default access mask for reads
    fn read_access() -> vk::AccessFlags;

    /// Default access mask for writes
    fn write_access() -> vk::AccessFlags;
}

/// GPU resource handle (type-erased)
pub enum GpuHandle {
    Buffer(vk::Buffer),
    Image(vk::Image, ImageLayoutInfo),
}

pub struct ImageLayoutInfo {
    pub current_layout: vk::ImageLayout,
    pub target_layout: vk::ImageLayout,
}
```

### 2. Implement for Concrete Types

```rust
pub struct GpuBuffer<T> {
    buffer: vk::Buffer,
    _phantom: PhantomData<T>,
}

impl<T: 'static> GpuResource for GpuBuffer<T> {
    fn handle(&self) -> GpuHandle {
        GpuHandle::Buffer(self.buffer)
    }

    fn read_stage() -> vk::PipelineStageFlags {
        // Infer from T's usage
        if type_name::<T>().contains("Vertex") {
            vk::PipelineStageFlags::VERTEX_INPUT
        } else if type_name::<T>().contains("Uniform") {
            vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER
        } else {
            vk::PipelineStageFlags::ALL_COMMANDS  // Conservative
        }
    }

    fn write_stage() -> vk::PipelineStageFlags {
        vk::PipelineStageFlags::TRANSFER  // Most writes are uploads
    }

    fn read_access() -> vk::AccessFlags {
        if type_name::<T>().contains("Vertex") {
            vk::AccessFlags::VERTEX_ATTRIBUTE_READ
        } else {
            vk::AccessFlags::SHADER_READ
        }
    }

    fn write_access() -> vk::AccessFlags {
        vk::AccessFlags::TRANSFER_WRITE
    }
}

pub struct GpuImage<T> {
    image: vk::Image,
    current_layout: Cell<vk::ImageLayout>,  // Tracked at runtime
    _phantom: PhantomData<T>,
}

impl<T: 'static> GpuResource for GpuImage<T> {
    fn handle(&self) -> GpuHandle {
        let target = if type_name::<T>().contains("RenderTarget") {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        } else if type_name::<T>().contains("Texture") {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        } else {
            vk::ImageLayout::GENERAL
        };

        GpuHandle::Image(self.image, ImageLayoutInfo {
            current_layout: self.current_layout.get(),
            target_layout: target,
        })
    }

    fn read_stage() -> vk::PipelineStageFlags {
        vk::PipelineStageFlags::FRAGMENT_SHADER
    }

    fn write_stage() -> vk::PipelineStageFlags {
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
    }

    fn read_access() -> vk::AccessFlags {
        vk::AccessFlags::SHADER_READ
    }

    fn write_access() -> vk::AccessFlags {
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE
    }
}
```

---

## Design: Extend ResourceAccess Trait

### Current (CPU-only):

```rust
// macrokid_core/src/common/threads.rs
pub trait ResourceAccess {
    fn reads() -> &'static [TypeId];
    fn writes() -> &'static [TypeId];
}
```

### Extended (GPU-aware):

```rust
pub trait ResourceAccess {
    fn reads() -> &'static [TypeId];
    fn writes() -> &'static [TypeId];

    // NEW: GPU resource metadata
    fn gpu_reads() -> &'static [GpuResourceMeta] { &[] }
    fn gpu_writes() -> &'static [GpuResourceMeta] { &[] }
}

pub struct GpuResourceMeta {
    pub type_id: TypeId,
    pub resource_kind: &'static str,  // "Buffer" | "Image"
    pub read_stage: vk::PipelineStageFlags,
    pub write_stage: vk::PipelineStageFlags,
    pub read_access: vk::AccessFlags,
    pub write_access: vk::AccessFlags,
}
```

---

## Design: Derive Macro Changes

### 1. Detect GPU Resources in Attributes

```rust
// macrokid_threads_derive/src/lib.rs

#[derive(System)]
#[reads(CpuTransform, GpuBuffer<Vertex>)]  // ← Mix CPU and GPU
#[writes(GpuImage<RenderTarget>)]
struct RenderPass;
```

**Parsing logic**:
```rust
fn parse_resource_attr(attr: &Attribute) -> Vec<ResourceType> {
    // Parse: reads(A, B, C) or writes(D, E)
    let types = extract_types(attr);

    types.into_iter().map(|ty| {
        if is_gpu_type(&ty) {  // Check for GpuBuffer<T>, GpuImage<T>, etc.
            ResourceType::Gpu {
                inner_type: extract_inner_type(&ty),  // Extract T from GpuBuffer<T>
                kind: extract_kind(&ty),  // "Buffer" or "Image"
            }
        } else {
            ResourceType::Cpu(ty)
        }
    }).collect()
}
```

### 2. Generate GPU Metadata

```rust
// Generated code for RenderPass:

impl ResourceAccess for RenderPass {
    fn reads() -> &'static [TypeId] {
        // CPU resources only
        &[TypeId::of::<CpuTransform>()]
    }

    fn writes() -> &'static [TypeId] {
        &[]  // No CPU writes
    }

    fn gpu_reads() -> &'static [GpuResourceMeta] {
        static GPU_READS: [GpuResourceMeta; 1] = [
            GpuResourceMeta {
                type_id: TypeId::of::<GpuBuffer<Vertex>>(),
                resource_kind: "Buffer",
                read_stage: vk::PipelineStageFlags::VERTEX_INPUT,
                write_stage: vk::PipelineStageFlags::TRANSFER,
                read_access: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
                write_access: vk::AccessFlags::TRANSFER_WRITE,
            }
        ];
        &GPU_READS
    }

    fn gpu_writes() -> &'static [GpuResourceMeta] {
        static GPU_WRITES: [GpuResourceMeta; 1] = [
            GpuResourceMeta {
                type_id: TypeId::of::<GpuImage<RenderTarget>>(),
                resource_kind: "Image",
                read_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                write_stage: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                read_access: vk::AccessFlags::SHADER_READ,
                write_access: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            }
        ];
        &GPU_WRITES
    }
}
```

---

## Design: Barrier Hint Generation

### Option 1: Comment Injection (Least Invasive)

```rust
// Generated in RenderPass::run():

fn run(self) {
    /* ═══ GPU BARRIER ANALYSIS ═══
     *
     * GpuBuffer<Vertex>:
     *   READ access required
     *   Ensure previous TRANSFER_WRITE is complete before VERTEX_INPUT
     *   Insert: vkCmdPipelineBarrier(
     *     srcStage: TRANSFER,
     *     dstStage: VERTEX_INPUT,
     *     buffer_barrier: { src_access: TRANSFER_WRITE, dst_access: VERTEX_ATTRIBUTE_READ }
     *   )
     *
     * GpuImage<RenderTarget>:
     *   WRITE access required
     *   Transition layout: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
     *   Insert: vkCmdPipelineBarrier(
     *     srcStage: TOP_OF_PIPE,
     *     dstStage: COLOR_ATTACHMENT_OUTPUT,
     *     image_barrier: {
     *       old_layout: UNDEFINED,
     *       new_layout: COLOR_ATTACHMENT_OPTIMAL,
     *       src_access: empty,
     *       dst_access: COLOR_ATTACHMENT_WRITE
     *     }
     *   )
     *
     * ════════════════════════════ */

    // User's actual code here...
}
```

**Pros**:
- Non-invasive, doesn't change runtime behavior
- Helps developers understand synchronization needs
- Copy-paste barrier code directly

**Cons**:
- Still requires manual implementation
- Comments can become stale

### Option 2: Validation Hooks (Debug Builds)

```rust
// Generated:
impl RenderPass {
    fn run(self, ctx: &mut RenderContext) {
        #[cfg(debug_assertions)]
        self.validate_barriers(ctx);

        // User's code...
    }

    #[cfg(debug_assertions)]
    fn validate_barriers(&self, ctx: &RenderContext) {
        // Check if required barriers were inserted
        for meta in Self::gpu_reads() {
            let handle = ctx.get_resource_state(meta.type_id);
            if !handle.is_ready_for_read(meta.read_stage) {
                panic!(
                    "MISSING BARRIER: {} not ready for read at {:?} stage. \
                     Previous writer: {:?}. \
                     Insert barrier between {:?} -> {:?}",
                    type_name_of_id(meta.type_id),
                    meta.read_stage,
                    handle.last_writer_stage,
                    handle.last_writer_stage,
                    meta.read_stage
                );
            }
        }
        // Similar for writes...
    }
}
```

**Pros**:
- Catches missing barriers at runtime
- Provides actionable error messages
- Only runs in debug mode (zero production cost)

**Cons**:
- Requires resource state tracking infrastructure
- Runtime overhead in debug builds

### Option 3: Automatic Barrier Emission (Highest Abstraction)

```rust
// Generated:
impl RenderPass {
    fn run(self, ctx: &mut CommandContext) {
        // Automatically insert required barriers
        ctx.barrier_tracker.ensure_ready(
            Self::gpu_reads(),
            Self::gpu_writes(),
        );

        // User's code...
    }
}

// In CommandContext:
impl CommandContext {
    pub fn ensure_ready(&mut self, reads: &[GpuResourceMeta], writes: &[GpuResourceMeta]) {
        for meta in reads {
            if let Some(barrier) = self.compute_barrier_for_read(meta) {
                self.emit_barrier(barrier);
            }
        }
        for meta in writes {
            if let Some(barrier) = self.compute_barrier_for_write(meta) {
                self.emit_barrier(barrier);
            }
        }
    }

    fn compute_barrier_for_read(&self, meta: &GpuResourceMeta) -> Option<VkBarrier> {
        let state = self.resource_states.get(&meta.type_id)?;

        // If last access was write, need barrier
        if state.last_access == AccessType::Write && state.stage != meta.read_stage {
            Some(VkBarrier::Buffer {
                src_stage: state.stage,
                dst_stage: meta.read_stage,
                src_access: meta.write_access,  // From last write
                dst_access: meta.read_access,
                buffer: state.handle.as_buffer(),
            })
        } else {
            None  // Already synchronized
        }
    }

    fn emit_barrier(&mut self, barrier: VkBarrier) {
        match barrier {
            VkBarrier::Buffer { src_stage, dst_stage, src_access, dst_access, buffer } => {
                let barrier = vk::BufferMemoryBarrier::builder()
                    .src_access_mask(src_access)
                    .dst_access_mask(dst_access)
                    .buffer(buffer)
                    .offset(0)
                    .size(vk::WHOLE_SIZE);

                self.device.cmd_pipeline_barrier(
                    self.command_buffer,
                    src_stage,
                    dst_stage,
                    vk::DependencyFlags::empty(),
                    &[], &[barrier.build()], &[]
                );
            }
            VkBarrier::Image { /* ... */ } => {
                // Similar for images...
            }
        }

        // Update state
        self.resource_states.insert(meta.type_id, ResourceState {
            last_access: AccessType::Read,
            stage: dst_stage,
            handle: /* ... */,
        });
    }
}
```

**Pros**:
- Zero manual barrier code
- Correct by construction
- Handles complex dependencies automatically

**Cons**:
- Hidden magic (harder to debug)
- Runtime overhead (tracking + emission)
- May insert redundant barriers (conservative)

---

## Recommended Approach: Hybrid

**Phase 1**: Comment injection (Option 1)
- Low implementation cost
- Immediate value for developers
- No runtime changes

**Phase 2**: Debug validation (Option 2)
- Add validation layer infrastructure
- Catch missing barriers in tests
- Gated behind `#[cfg(debug_assertions)]`

**Phase 3**: Optional auto-emission (Option 3)
- Opt-in via attribute: `#[derive(System, AutoBarriers)]`
- Uses validation infrastructure from Phase 2
- User can choose manual vs automatic

---

## Implementation: Comment Generation

### Derive Macro Changes

```rust
// macrokid_threads_derive/src/lib.rs

fn derive_system_impl(input: DeriveInput) -> TokenStream {
    let attrs = parse_resource_attrs(&input.attrs);
    let gpu_reads = attrs.gpu_reads;
    let gpu_writes = attrs.gpu_writes;

    // Generate barrier hints
    let barrier_comments = generate_barrier_hints(&gpu_reads, &gpu_writes);

    quote! {
        impl #name {
            // Optionally inject into user's run() method if we control it,
            // or provide a separate validation method:
            #[doc = "Generated barrier requirements for this system"]
            pub fn barrier_requirements() -> &'static str {
                #barrier_comments
            }
        }

        // Standard ResourceAccess impl...
        impl ResourceAccess for #name {
            // ... (as shown above)
        }
    }
}

fn generate_barrier_hints(reads: &[GpuResourceType], writes: &[GpuResourceType]) -> String {
    let mut hints = String::from("GPU Barrier Requirements:\n\n");

    for read in reads {
        hints.push_str(&format!(
            "  READ: {}\n\
                 Stage: {:?} → {:?}\n\
                 Access: {:?} → {:?}\n\
                 Action: Insert buffer/image barrier\n\n",
            read.type_name,
            read.write_stage,  // Previous stage (assumption)
            read.read_stage,   // Current stage
            read.write_access,
            read.read_access
        ));
    }

    for write in writes {
        hints.push_str(&format!(
            "  WRITE: {}\n\
                 Stage: {:?}\n\
                 Access: {:?}\n",
            write.type_name,
            write.write_stage,
            write.write_access
        ));

        if write.kind == "Image" {
            hints.push_str(&format!(
                "    Layout: UNDEFINED → {:?}\n",
                write.target_layout
            ));
        }
        hints.push_str("\n");
    }

    hints
}
```

### Usage

```rust
#[derive(System)]
#[reads(GpuBuffer<Vertex>)]
#[writes(GpuImage<RenderTarget>)]
struct RenderGeometry;

fn main() {
    println!("{}", RenderGeometry::barrier_requirements());
    // Prints:
    // GPU Barrier Requirements:
    //
    //   READ: GpuBuffer<Vertex>
    //     Stage: TRANSFER → VERTEX_INPUT
    //     Access: TRANSFER_WRITE → VERTEX_ATTRIBUTE_READ
    //     Action: Insert buffer/image barrier
    //
    //   WRITE: GpuImage<RenderTarget>
    //     Stage: COLOR_ATTACHMENT_OUTPUT
    //     Access: COLOR_ATTACHMENT_WRITE
    //     Layout: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
}
```

---

## Integration with Schedule Derive

### Dependency Graph Analysis

```rust
#[derive(Schedule)]
struct FramePipeline {
    #[stage(name = "upload")]
    upload: (UploadMeshes,),  // Writes: GpuBuffer<Vertex>

    #[stage(name = "render", after = "upload")]
    render: (RenderGeometry,),  // Reads: GpuBuffer<Vertex>
}
```

**Generated barrier hint**:
```rust
impl FramePipeline {
    fn run<S: Scheduler>(&self, sched: &S) {
        // Stage: upload
        /* BARRIER HINT: After this stage completes, GpuBuffer<Vertex> will be in:
         *   Stage: TRANSFER
         *   Access: TRANSFER_WRITE
         *
         * Before next stage (render), insert barrier:
         *   vkCmdPipelineBarrier(
         *     TRANSFER → VERTEX_INPUT,
         *     buffer_barrier: { TRANSFER_WRITE → VERTEX_ATTRIBUTE_READ }
         *   )
         */
        self.upload.0.run();

        // ← INSERT BARRIER HERE (user's responsibility)

        // Stage: render
        self.render.0.run();
    }
}
```

---

## Examples

### Example 1: Upload → Render Pipeline

```rust
#[derive(System)]
#[writes(GpuBuffer<Vertex>)]
struct UploadMeshes {
    data: Vec<Vertex>,
}

impl UploadMeshes {
    fn run(self, ctx: &mut VkContext) {
        // Upload to staging buffer, copy to GPU
        ctx.copy_buffer(staging, gpu_buffer);
        // State: TRANSFER stage, TRANSFER_WRITE access
    }
}

#[derive(System)]
#[reads(GpuBuffer<Vertex>)]
struct RenderMeshes {
    pipeline: vk::Pipeline,
}

impl RenderMeshes {
    fn run(self, ctx: &mut VkContext) {
        // Generated comment:
        /* BARRIER: Ensure GpuBuffer<Vertex> ready for VERTEX_INPUT
         * Insert before bind_vertex_buffer:
         *   cmd_pipeline_barrier(
         *     TRANSFER → VERTEX_INPUT,
         *     buffer_barrier: { TRANSFER_WRITE → VERTEX_ATTRIBUTE_READ }
         *   )
         */

        ctx.cmd_bind_vertex_buffer(/* ... */);
        ctx.cmd_draw(/* ... */);
    }
}
```

### Example 2: Render Target Transitions

```rust
#[derive(System)]
#[writes(GpuImage<RenderTarget>)]
struct RenderScene;

impl RenderScene {
    fn run(self, ctx: &mut VkContext) {
        /* BARRIER: Transition GpuImage<RenderTarget>
         * From: UNDEFINED (initial state)
         * To:   COLOR_ATTACHMENT_OPTIMAL
         *
         * Insert before render pass:
         *   cmd_pipeline_barrier(
         *     TOP_OF_PIPE → COLOR_ATTACHMENT_OUTPUT,
         *     image_barrier: {
         *       old_layout: UNDEFINED,
         *       new_layout: COLOR_ATTACHMENT_OPTIMAL,
         *       src_access: empty,
         *       dst_access: COLOR_ATTACHMENT_WRITE
         *     }
         *   )
         */

        ctx.cmd_begin_render_pass(/* ... */);
        // ...
    }
}

#[derive(System)]
#[reads(GpuImage<RenderTarget>)]
struct PostProcess;

impl PostProcess {
    fn run(self, ctx: &mut VkContext) {
        /* BARRIER: Transition GpuImage<RenderTarget>
         * From: COLOR_ATTACHMENT_OPTIMAL (written by RenderScene)
         * To:   SHADER_READ_ONLY_OPTIMAL
         *
         * Insert before bind_image:
         *   cmd_pipeline_barrier(
         *     COLOR_ATTACHMENT_OUTPUT → FRAGMENT_SHADER,
         *     image_barrier: {
         *       old_layout: COLOR_ATTACHMENT_OPTIMAL,
         *       new_layout: SHADER_READ_ONLY_OPTIMAL,
         *       src_access: COLOR_ATTACHMENT_WRITE,
         *       dst_access: SHADER_READ
         *     }
         *   )
         */

        ctx.bind_image_as_texture(/* ... */);
        ctx.cmd_draw(/* ... */);
    }
}
```

---

## Open Questions

1. **Layout tracking**: Who tracks current image layout?
   - **Option A**: User-managed (Cell/RefCell in GpuImage)
   - **Option B**: Context-managed (global resource state map)
   - **Recommendation**: Option B for automatic mode, Option A for manual

2. **Cross-frame dependencies**: How to handle resources reused across frames?
   - **Answer**: Semaphores (CPU-GPU sync) are separate from barriers (GPU-GPU sync)
   - Barriers are per-frame; semaphores handle frame N → frame N+1

3. **Redundant barriers**: How to avoid over-synchronization?
   - **Answer**: Phase 2 validation layer tracks actual state, only emits when needed
   - Comment generation (Phase 1) may suggest redundant barriers (user's choice)

4. **Interaction with multi-threading**: Do barriers need thread awareness?
   - **Answer**: No - secondary command buffers are independent
   - Primary execution order determines barrier placement

---

## Performance Impact

### Comment Injection (Phase 1)
- **Compile time**: +5% (extra string generation in derive)
- **Runtime**: Zero (comments are stripped)

### Debug Validation (Phase 2)
- **Compile time**: +10% (validation code generation)
- **Runtime (debug)**: +5-10% (state tracking overhead)
- **Runtime (release)**: Zero (gated by `cfg(debug_assertions)`)

### Auto-Emission (Phase 3)
- **Compile time**: +15% (barrier emission code)
- **Runtime**: +2-5% (state lookups + conditional barriers)
- **Trade-off**: Correctness vs manual optimization

---

## Next Steps

**Phase 1.1**: Define GpuResource trait + concrete types (GpuBuffer, GpuImage)
**Phase 1.2**: Extend ResourceAccess with gpu_reads/gpu_writes
**Phase 1.3**: Modify System derive to detect GPU resources
**Phase 1.4**: Implement comment generation in derive macro
**Phase 1.5**: Write examples: barrier_hints_demo

**Phase 2**: Validation infrastructure (separate effort)

**Dependencies**: None for Phase 1 - pure codegen changes
