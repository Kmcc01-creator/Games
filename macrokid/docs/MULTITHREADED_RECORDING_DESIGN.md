# Multi-threaded Command Buffer Recording Design

## Overview

Enable parallel Vulkan command buffer recording using the existing `macrokid_core::threads` infrastructure with secondary command buffers.

## Architecture

### Current State (Single-threaded)

```rust
// vk_linux.rs:1323-1335
command_pool: vk::CommandPool,  // Single pool, RESET_COMMAND_BUFFER flag
command_buffers: Vec<vk::CommandBuffer>,  // Primary buffers (one per frame)
```

**Limitation**: Command pools are NOT thread-safe. Cannot record from same pool on multiple threads.

### Target State (Multi-threaded)

```rust
// Per-thread command pool infrastructure
struct ThreadLocalPools {
    pools: Vec<vk::CommandPool>,           // One pool per thread
    secondary_buffers: Vec<Vec<vk::CommandBuffer>>,  // [thread][buffer]
}

// Main VkCore keeps:
primary_command_buffers: Vec<vk::CommandBuffer>,  // One per frame (unchanged)
thread_pools: ThreadLocalPools,
```

**Vulkan guarantees**:
- ✅ Command pools can be used from one thread at a time
- ✅ Secondary command buffers can be recorded in parallel
- ✅ Primary buffer executes secondaries via `vkCmdExecuteCommands`

---

## Integration with macrokid_core::threads

### Phase 1: Extend `Frame` trait

```rust
// engine.rs (current):
pub trait Frame {
    type CommandCtx;  // Backend-specific, likely !Send
    fn encoder_for(&self, pass: &'static str) -> Self::CommandCtx;
}
```

**Change to**:

```rust
pub trait Frame {
    type CommandCtx;  // Now: SecondaryCommandBuffer wrapper
    type PrimaryCtx;  // For final submission

    /// Acquire a secondary command buffer for parallel recording
    /// Thread-safe: each call returns a different buffer from thread-local pool
    fn encoder_for(&self, pass: &'static str) -> Self::CommandCtx;

    /// Finalize frame: execute all secondary buffers in primary
    fn finalize(self) -> Self::PrimaryCtx;
}
```

### Phase 2: ThreadLocal Command Pool Allocation

```rust
// vk_linux.rs implementation
struct VkFrame {
    frame_idx: usize,
    primary_cb: vk::CommandBuffer,
    thread_pool_allocator: Arc<ThreadLocalPools>,
    recorded_secondaries: Arc<Mutex<Vec<vk::CommandBuffer>>>,
}

impl VkFrame {
    fn encoder_for(&self, pass: &'static str) -> VkCommandEncoder {
        // Get thread-local pool
        let thread_id = std::thread::current().id();
        let pool_idx = thread_id_to_index(thread_id);  // Hash or TLS

        // Allocate secondary command buffer from thread's pool
        let secondary = self.thread_pool_allocator.allocate_secondary(pool_idx);

        VkCommandEncoder {
            cb: secondary,
            pass_name: pass,
            parent: Arc::clone(&self.recorded_secondaries),
        }
    }

    fn finalize(self) -> vk::CommandBuffer {
        // Collect all recorded secondaries
        let secondaries = self.recorded_secondaries.lock().unwrap();

        // Primary command buffer execution
        device.cmd_execute_commands(self.primary_cb, &secondaries);

        self.primary_cb
    }
}
```

### Phase 3: Schedule Derive Integration

```rust
// User code: Multi-threaded render pass recording
#[derive(Job, System)]
#[reads(MeshData)]
#[writes(RenderCommands)]
struct RecordGeometryPass {
    frame: Arc<VkFrame>,  // Shared across jobs
    meshes: Vec<Mesh>,
}

impl RecordGeometryPass {
    fn run(self) {
        // Each job runs on different thread
        let encoder = self.frame.encoder_for("geometry");

        // Record Vulkan commands (thread-safe due to secondary CB)
        encoder.bind_pipeline(...);
        for mesh in self.meshes {
            encoder.bind_vertex_buffer(mesh.vbo);
            encoder.draw(mesh.vertex_count, ...);
        }

        encoder.finish();  // Adds CB to recorded_secondaries
    }
}

#[derive(Schedule)]
struct ParallelRenderSchedule {
    // All three jobs execute in parallel
    #[stage(name = "record")]
    record: (RecordGeometryPass, RecordLightingPass, RecordShadowPass),
}

fn render_frame(renderer: &VkRenderer, schedule: &ParallelRenderSchedule) {
    let frame = renderer.begin_frame();  // Returns Arc<VkFrame>

    schedule.run(&thread_pool);  // Parallel recording via macrokid_core::threads

    let primary = frame.finalize();  // Collect + execute secondaries
    renderer.end_frame(primary);     // Submit to queue
}
```

---

## Vulkan Implementation Details

### 1. Command Pool Creation (Per-Thread)

```rust
// At init time:
let num_threads = thread_pool.worker_count();
let mut pools = Vec::new();
for _ in 0..num_threads {
    let pool = device.create_command_pool(
        &vk::CommandPoolCreateInfo::builder()
            .queue_family_index(qfi)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
        None
    )?;
    pools.push(pool);
}
```

**Key**: No `TRANSIENT` flag - buffers may be reused across frames.

### 2. Secondary Buffer Allocation

```rust
// Per frame, per thread:
let secondary_alloc = vk::CommandBufferAllocateInfo::builder()
    .command_pool(pools[thread_idx])
    .level(vk::CommandBufferLevel::SECONDARY)  // ← Secondary, not Primary
    .command_buffer_count(max_passes_per_thread);

let secondaries = device.allocate_command_buffers(&secondary_alloc)?;
```

### 3. Secondary Recording with Inheritance

```rust
// When recording secondary:
let inheritance = vk::CommandBufferInheritanceInfo::builder()
    .render_pass(render_pass)
    .subpass(0)
    .framebuffer(framebuffer);  // Optional, but enables FB-local optimization

let begin = vk::CommandBufferBeginInfo::builder()
    .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)  // ← Must set this
    .inheritance_info(&inheritance);

device.begin_command_buffer(secondary_cb, &begin)?;

// Record commands...
device.cmd_bind_pipeline(secondary_cb, ...);
device.cmd_draw(secondary_cb, ...);

device.end_command_buffer(secondary_cb)?;
```

### 4. Primary Execution

```rust
// In primary command buffer:
device.begin_command_buffer(primary_cb, &begin_info)?;
device.cmd_begin_render_pass(primary_cb, &render_pass_begin, SECONDARY_COMMAND_BUFFERS);

// Execute all secondaries in order
device.cmd_execute_commands(primary_cb, &[secondary1, secondary2, secondary3]);

device.cmd_end_render_pass(primary_cb);
device.end_command_buffer(primary_cb)?;
```

---

## Thread Safety Guarantees

| Component | Thread-Safety | Notes |
|-----------|---------------|-------|
| **Command Pool** | ❌ NOT thread-safe | Must have one per thread |
| **Secondary CB** | ✅ Thread-safe | Each thread records different CB |
| **Primary CB** | ❌ NOT thread-safe | Only recorded on main thread |
| **Device** | ✅ Thread-safe | Vulkan devices are externally synchronized |
| **Queue** | ❌ NOT thread-safe | Submission is single-threaded |

**Pattern**: Parallel recording, serial submission.

---

## Performance Considerations

### Benefits

1. **CPU parallelism**: Distribute draw call encoding across cores
2. **Reduced frame latency**: Recording doesn't block on single thread
3. **Scales with core count**: 4 threads → ~4x recording throughput

### Costs

1. **Memory overhead**: N command pools + N secondary buffers
2. **Synchronization overhead**: `join_all()` barrier between record and submit
3. **Vulkan overhead**: `vkCmdExecuteCommands` has small dispatch cost

### When to Use

✅ **Use when**:
- Rendering 1000+ draw calls per frame
- Complex state changes (many pipeline binds, descriptor updates)
- CPU is bottleneck (profile with `VK_LAYER_LUNARG_monitor`)

❌ **Don't use when**:
- Simple scenes (<100 draw calls)
- GPU-bound (changing recording won't help)
- Single-core systems

---

## Migration Path

### Step 1: Add ThreadLocalPools to VkCore

```diff
  struct VkCore {
      command_pool: vk::CommandPool,
      command_buffers: Vec<vk::CommandBuffer>,
+     thread_pools: Option<ThreadLocalPools>,  // None = single-threaded
  }
```

### Step 2: Implement VkFrame + VkCommandEncoder

New types in `vk_linux.rs`:
- `VkFrame`: Per-frame context (Send + Sync)
- `VkCommandEncoder`: Wrapper around secondary CB (!Send)

### Step 3: Extend Renderer Trait

```rust
impl Renderer for VkRenderer {
    type Frame = Arc<VkFrame>;  // ← Arc enables sharing across threads

    fn begin_frame(&self) -> Self::Frame {
        // Allocate secondary buffers from thread pools
        Arc::new(VkFrame::new(self.vk_core, frame_idx))
    }

    fn end_frame(&self, frame: Self::Frame) {
        // finalize() called internally, submit primary CB
    }
}
```

### Step 4: User Adoption (Opt-in)

```rust
// Single-threaded (unchanged):
let frame = renderer.begin_frame();
let encoder = frame.encoder_for("main");
encoder.draw(...);
renderer.end_frame(frame);

// Multi-threaded (new):
#[derive(Schedule)]
struct ParallelRender { /* jobs */ }

let frame = renderer.begin_frame();
parallel_render.run(&thread_pool);  // Uses frame internally
renderer.end_frame(frame);
```

---

## Open Questions

1. **Secondary buffer pooling**: Reuse across frames or reallocate?
   - **Recommendation**: Pool with `RESET_COMMAND_BUFFER`, reuse per thread

2. **Pass ordering**: How to enforce dependencies between secondaries?
   - **Option A**: Schedule derive ensures ordering (stages run sequentially)
   - **Option B**: Explicit barriers between secondaries (see Part 2)

3. **Dynamic vs static threading**: Always use N threads or adapt to workload?
   - **Recommendation**: Static pools (size = worker count), dynamic job dispatch

4. **Compute pipelines**: Can they use secondary buffers?
   - **Answer**: No, compute must use primary. Could dispatch compute from main thread while graphics records in parallel.

---

## Next Steps

**Phase 1.1**: Implement ThreadLocalPools (Vulkan infrastructure)
**Phase 1.2**: Add VkFrame + VkCommandEncoder (API surface)
**Phase 1.3**: Extend Schedule derive to pass `Arc<Frame>` to jobs
**Phase 1.4**: Write example: parallel_recording_demo

**Dependencies**: None - can implement incrementally without breaking existing code.
