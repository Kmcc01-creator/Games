# Changelog

All notable changes to the Macrokid project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [November 2025] - Multi-threaded Vulkan & GPU Barrier Generation

### Added - GPU Resource Tracking
- **GpuResource trait system**: Type-safe wrappers for GPU buffers and images
- **GpuBuffer<T>** and **GpuImage<T>**: Generic GPU resource types with type-level inference
- **Automatic barrier hint generation**: Human-readable Vulkan synchronization requirements
- **GpuResourceAccess trait**: Extends System derive with GPU metadata
- **Type-level inference**: Automatically infer pipeline stages from type names
  - Example: `GpuBuffer<VertexData>` → `VERTEX_INPUT` stage
  - Example: `GpuImage<RenderTarget>` → `COLOR_ATTACHMENT_OUTPUT` stage

### Added - Multi-threaded Command Buffer Recording
- **ThreadLocalPools**: Per-thread Vulkan command pool infrastructure
- **Secondary command buffer allocation**: Pre-allocated buffers per thread
- **Thread-safe design**: Isolated command pools per worker thread (Vulkan requirement)
- **Opt-in complexity**: Multi-threading disabled by default, enable when needed

### Changed - System Derive
- **GPU resource detection**: System derive now detects `GpuBuffer<T>` and `GpuImage<T>` types
- **Dual impl generation**: Generates both `ResourceAccess` (CPU) and `GpuResourceAccess` (GPU)
- **Mixed resource tracking**: Support for both CPU and GPU resources in same system
- **Barrier requirements method**: Automatic `barrier_requirements()` helper for debugging

### Changed - Graphics Infrastructure
- **GpuImage thread safety**: Uses `AtomicU32` for layout tracking instead of `Cell`
- **VkCore integration**: Added optional `ThreadLocalPools` field
- **Resource cleanup**: Proper Drop impl for thread pools

### Documentation
- Added `MULTITHREADED_RECORDING_DESIGN.md` - Complete architectural design
- Added `BARRIER_CODEGEN_DESIGN.md` - Three-phase barrier generation approach
- Added `PARALLEL_IMPLEMENTATION_SUMMARY.md` - Implementation status and roadmap
- Added `USAGE_EXAMPLES.md` - 7 practical examples
- Updated main README with new features section
- Updated sub-project READMEs (macrokid_graphics, macrokid_threads_derive)

### Performance
- Multi-threaded recording: Expected 2-4x throughput on 4+ core systems
- Barrier generation: Zero runtime cost (Phase 1 - comments only)
- Type inference: Zero-cost compile-time heuristics

### Status
- Core infrastructure: 70% complete (6/10 tasks)
- ThreadLocalPools: ✅ Complete
- GPU resource types: ✅ Complete
- System derive GPU detection: ✅ Complete
- Barrier hint generation: ✅ Complete
- VkFrame/VkCommandEncoder: 🔄 In progress
- Examples: 🔄 Pending

### Future Work
- Phase 2: Debug validation layer (runtime barrier checking)
- Phase 3: Automatic barrier emission (opt-in)
- Schedule derive inter-stage barrier hints
- Transfer queue integration

## [January 2025] - Documentation Restructuring

### Documentation
- Restructured project documentation with dedicated `docs/` folders
- Created sub-project README files (macrokid_graphics, macrokid_core, macrokid_threads_derive)
- Organized documentation into architecture, guides, reference, and design sections

## [January 2025] - Performance Analysis & BufferLayout Refactoring

### Added
- `macrokid_parse_bench` crate for attribute parsing benchmarks
- Float type support for `AttrSchema` (`req_float()`, `opt_float()`, `get_float()`)
- Helper functions for BufferLayout derive to improve maintainability

### Changed
- **BufferLayout Derive**: Refactored from 161 lines to 91 lines (43% reduction)
  - Extracted helper functions: `collect_vertex_attrs()`, `compute_offsets()`, `compute_strides()`
  - Introduced `VertexAttrRec` struct for clarity
- Moved `glam` from dev-dependencies to dependencies in `macrokid_graphics`
- Added `+ Vertex` trait bounds to mesh generation functions

### Fixed
- Fixed `quote!` macro issues in assets module (pre-computed collections)
- Fixed unused imports in mesh generation code
- Fixed type conversion in `cube()` function

### Performance
- Benchmark Results:
  - Custom attribute parser: 883ns (13.9x faster than syn)
  - Decision: Keep using syn (parsing is only 10-20% of total time)
  - See `docs/design/parse-benchmark.md` for detailed analysis

### Documentation
- Added `PARSE_BENCHMARK_ANALYSIS.md` (now in `docs/design/`)
- Added `RECENT_UPDATES.md` (now archived)
- Updated framework documentation

### Known Issues
- Assets module derives must be moved to root level (proc_macro limitation)
- 10 unused variable warnings in `macrokid_graphics_derive` (low priority cleanup)

## [December 2024] - Framework Enhancements

### Added
- `StaticSliceDerive` pattern for code generation
- `ResultCodeGen` trait for fallible generators
- `exclusive_schemas!` macro for attribute validation
- Derive DSL macros for graphics programming

### Changed
- Enhanced `ImplBuilder` with method and associated item support
- Improved attribute schema validation
- Extended combinator system

### Documentation
- Added `FRAMEWORK_IMPROVEMENTS.md` (now in `docs/design/`)
- Added `OPTION_B_COMPLETE.md` (now archived)
- Added `CODEGEN_TUTORIAL.md` and `CODEGEN_QUICKREF.md` (now in `docs/guides/`)

## [November 2024] - Graphics & Lighting

### Added
- Graphics rendering system (`macrokid_graphics`)
- `ResourceBinding`, `BufferLayout`, `GraphicsPipeline` derives
- `RenderEngine` derive for engine configuration
- Basic PBR rendering support
- Lighting system foundation

### Features
- Vulkan backend for Linux
- Runtime GLSL shader compilation
- Per-frame uniform updates
- Custom texture loading

## [October 2024] - Threading System

### Added
- Threaded scheduling utilities (`macrokid_core::threads`)
- `Job`, `System`, `Schedule` derives
- Stage dependency specification
- Resource conflict detection
- Conflict-aware batching

### Features
- `Scheduler`, `ThreadPool`, `join_all` utilities
- `ResourceAccess` tracking
- Topological stage ordering

## [September 2024] - Initial Framework

### Added
- Core framework (`macrokid_core`)
- Intermediate Representation (IR) system
- `TypeSpec`, `FieldSpec`, `VariantSpec` types
- Attribute parsing helpers
- `ImplBuilder` and `MatchArmBuilder`
- Pattern DSL (feature-gated)

### Features
- Modern syn 2.x API integration
- Support for structs and enums
- Field type introspection
- Visibility and span tracking
- Discriminant preservation

### Documentation
- Initial README
- Architecture documentation
- API reference
- Getting started guide

---

**Note:** This changelog started January 2025. Previous changes are reconstructed from project history.

For detailed recent updates, see archived documentation in `ARCHIVE/`.
