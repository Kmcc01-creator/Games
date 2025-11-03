# macrokid_threads_derive

> Derive macros for the Macrokid threaded scheduling system

Provides procedural macros for defining jobs, systems, and schedules with automatic resource conflict detection and parallel execution.

## Features

- `#[derive(Job)]` - Define parallelizable work units
- `#[derive(System)]` - Define systems with resource access tracking (CPU and GPU resources)
- `#[derive(Schedule)]` - Define execution schedules with stage dependencies
- **GPU Resource Detection** (NEW) - Automatic Vulkan barrier hint generation

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
macrokid_core = { path = "../macrokid_core", features = ["threads"] }
macrokid_threads_derive = { path = "../macrokid_threads_derive" }
```

Define jobs and systems:

```rust
use macrokid_threads_derive::{Job, System, Schedule};
use macrokid_core::threads::{JobRun, ResourceAccess};

#[derive(Job)]
struct PhysicsUpdate {
    delta_time: f32,
}

#[derive(System)]
#[reads(Transform, Velocity)]
#[writes(Position)]
struct MovementSystem;

#[derive(Schedule)]
struct GameSchedule {
    #[stage(name = "physics")]
    physics: PhysicsStage,

    #[stage(name = "render", after = "physics")]
    render: RenderStage,
}
```

## Stage Dependencies

The `#[stage]` attribute supports dependency specification:

```rust
#[stage(after = "stage_a,stage_b")]  // Runs after both stages
#[stage(before = "stage_c")]         // Sugar for stage_c's after
```

## Resource Conflict Detection

Systems automatically track resource access patterns:
- Systems reading the same resources can run in parallel
- Systems with write conflicts run sequentially
- The scheduler creates batches of non-conflicting systems

## GPU Resource Detection (NEW)

`#[derive(System)]` now detects GPU resources and generates barrier hints:

```rust
use macrokid_graphics::resources::{GpuBuffer, GpuImage, GpuResourceAccess};
use macrokid_threads_derive::System;

// Define marker types
struct VertexData;
struct RenderTarget;

// Mix CPU and GPU resources
#[derive(System)]
#[reads(Transform, GpuBuffer<VertexData>)]  // CPU + GPU
#[writes(GpuImage<RenderTarget>)]
struct RenderGeometry;

// Get CPU resource tracking
println!("CPU reads: {:?}", RenderGeometry::reads());

// Get GPU barrier hints
println!("{}", RenderGeometry::barrier_requirements());
```

**Features:**
- Separates CPU and GPU resources automatically
- Generates both `ResourceAccess` and `GpuResourceAccess` impls
- Type-level inference (e.g., `GpuBuffer<Vertex>` → `VERTEX_INPUT` stage)
- Human-readable Vulkan synchronization requirements

See [Usage Examples](../docs/USAGE_EXAMPLES.md) and [Barrier Generation Design](../docs/BARRIER_CODEGEN_DESIGN.md) for details.

## Documentation

For detailed usage, see:
- [Threading Guide](docs/guide.md)
- [Main Documentation](../docs/)
- [MACROKID_THREADS.md](../MACROKID_THREADS.md) (will be migrated)

## Examples

Run the threading demo:

```bash
cargo run --example threads_demo
```

## Current Status

**Working:**
- Job derive
- System derive with resource tracking (CPU and GPU)
- GPU resource detection and metadata generation
- Automatic barrier hint generation
- Schedule derive with stage dependencies
- Conflict-aware batching
- Topological stage ordering

**In Progress:**
- Schedule derive GPU barrier hints (inter-stage synchronization analysis)

**Experimental:**
- Advanced scheduling patterns
- Dynamic resource registration
- Debug validation layer for GPU barriers

## License

Part of the Macrokid project. See [LICENSE](../LICENSE) for details.
