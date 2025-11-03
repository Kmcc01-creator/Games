# Macrokid Documentation Index

Quick reference to all project documentation.

## 📋 Start Here

- **[SESSION_SUMMARY.md](SESSION_SUMMARY.md)** - Latest session overview (Jan 2025)
- **[TODO.md](TODO.md)** - Current tasks and priorities
- **[RECENT_UPDATES.md](RECENT_UPDATES.md)** - Recent changes and updates

## 📚 Framework Documentation

### Core Concepts
- **[OPTION_B_COMPLETE.md](OPTION_B_COMPLETE.md)** - Framework features summary
  - ResultCodeGen trait system
  - StaticSliceDerive pattern
  - Derive DSL macros

- **[FRAMEWORK_IMPROVEMENTS.md](FRAMEWORK_IMPROVEMENTS.md)** - Architecture overview
  - Design patterns
  - Migration guides
  - Best practices

### Tutorials & Guides
- **[CODEGEN_TUTORIAL.md](CODEGEN_TUTORIAL.md)** - Complete code generation tutorial
  - CodeGen trait basics
  - ResultCodeGen for fallible operations
  - Combinators and composition
  - Real-world examples

- **[CODEGEN_QUICKREF.md](CODEGEN_QUICKREF.md)** - Quick reference
  - Cheat sheets
  - Common patterns
  - API reference

## 🔬 Performance & Analysis

- **[PARSE_BENCHMARK_ANALYSIS.md](PARSE_BENCHMARK_ANALYSIS.md)** - Benchmarking study
  - Custom vs syn parsing comparison
  - Performance metrics
  - Decision rationale
  - Optimization recommendations

## 📝 Module Documentation

### macrokid_core
- **Location:** `macrokid_core/src/`
- **Features:**
  - IR (Intermediate Representation) types
  - Attribute schema validation
  - Code generation combinators
  - Derive pattern traits

**Key APIs:**
- `AttrSchema` - Attribute validation (supports int, str, bool, float)
- `TypeSpec` - AST representation
- `CodeGen` / `ResultCodeGen` - Generator traits
- `StaticSliceDerive` - Derive pattern trait

### macrokid_graphics
- **Location:** `macrokid_graphics/src/`
- **Features:**
  - Vulkan rendering abstractions
  - Resource management
  - Pipeline configuration
  - Asset generation

**Key Modules:**
- `resources` - Buffer layouts, bindings, descriptors
- `pipeline` - Graphics pipeline configuration
- `assets` - Procedural mesh/texture generation
- `engine` - High-level rendering API

### macrokid_graphics_derive
- **Location:** `macrokid_graphics_derive/src/`
- **Derives:**
  - `ResourceBinding` - GPU resource bindings
  - `BufferLayout` - Vertex buffer layouts
  - `GraphicsPipeline` - Pipeline configuration
  - `RenderEngine` - Engine setup
  - `RenderPass` - Render graph nodes

**TODO:** Asset derives (ProceduralMesh, ProceduralTexture, AssetBundle)
- See `TODO.md` for status

### macrokid_parse_bench
- **Location:** `macrokid_parse_bench/`
- **Purpose:** Performance benchmarking
- **Results:** See `PARSE_BENCHMARK_ANALYSIS.md`

## 📖 Examples

### Current Examples
- `examples/graphics_demo/` - Basic graphics rendering
- `examples/custom_derive/` - Custom derive patterns
- `examples/gfx_dsl/` - Graphics DSL showcase
- `examples/render_resources/` - Resource management
- `examples/threads_demo/` - Threading examples

### Example Specs (Not Implemented)
- `examples/pbr_showcase.rs` - PBR material system
- `examples/deferred_clustered_lighting.md` - Deferred rendering spec
- `examples/texture_showcase.rs` - Texture generation
- `examples/advanced_pbr_showcase.rs` - Advanced PBR
- `examples/derive_based_assets.rs` - Asset derive usage

## 🔧 Development Guides

### Getting Started
1. Read `SESSION_SUMMARY.md` for current state
2. Review `TODO.md` for tasks
3. Follow `CODEGEN_TUTORIAL.md` for framework usage
4. Check `CODEGEN_QUICKREF.md` for API reference

### Contributing
1. Check `TODO.md` for open tasks
2. Read relevant documentation for the area
3. Follow existing patterns (see `OPTION_B_COMPLETE.md`)
4. Test changes with `cargo test`
5. Update documentation as needed

### Common Tasks

**Add a new derive:**
1. Read `CODEGEN_TUTORIAL.md` sections 1-3
2. Use `StaticSliceDerive` pattern if field-based
3. Follow `ResourceBinding` example in `lib.rs:19-144`
4. Add to `macrokid_graphics_derive/src/lib.rs` at root level

**Add attribute parameter support:**
1. See `AttrSchema` in `macrokid_core/src/common/attr_schema.rs`
2. Use `req_*()` for required, `opt_*()` for optional
3. Supported types: `str`, `int`, `bool`, `float`

**Optimize performance:**
1. Profile first - don't assume
2. See `PARSE_BENCHMARK_ANALYSIS.md` for methodology
3. Focus on code generation (80% of time)
4. Parsing is well-optimized (use syn)

## 📊 Project Stats

### Code Quality
- **Test Coverage:** Core features tested
- **Documentation:** 10+ comprehensive docs
- **Examples:** 5+ working examples

### Build Status (Current)
- ✅ macrokid_core - Clean build
- ✅ macrokid_graphics - Builds with warnings
- ✅ macrokid_graphics_derive - Builds with warnings
- ✅ macrokid_parse_bench - All tests passing

### Known Issues
- 🔴 Assets module disabled (proc_macro location - see TODO.md)
- 🟡 10 unused variable warnings (low priority)
- 🟡 Float tests needed (new feature)

## 🔗 Quick Links

### Documentation Files
- [SESSION_SUMMARY.md](SESSION_SUMMARY.md) - Latest work
- [TODO.md](TODO.md) - Task list
- [RECENT_UPDATES.md](RECENT_UPDATES.md) - Change log
- [OPTION_B_COMPLETE.md](OPTION_B_COMPLETE.md) - Framework summary
- [CODEGEN_TUTORIAL.md](CODEGEN_TUTORIAL.md) - Tutorial
- [CODEGEN_QUICKREF.md](CODEGEN_QUICKREF.md) - Quick ref
- [PARSE_BENCHMARK_ANALYSIS.md](PARSE_BENCHMARK_ANALYSIS.md) - Benchmarks
- [FRAMEWORK_IMPROVEMENTS.md](FRAMEWORK_IMPROVEMENTS.md) - Architecture

### Code Locations
- Core Framework: `macrokid_core/src/`
- Graphics Runtime: `macrokid_graphics/src/`
- Derive Macros: `macrokid_graphics_derive/src/`
- Examples: `examples/`
- Benchmarks: `macrokid_parse_bench/`

### External Resources
- Rust Proc Macro Guide: https://doc.rust-lang.org/reference/procedural-macros.html
- syn Documentation: https://docs.rs/syn/
- quote Documentation: https://docs.rs/quote/

## 📅 Document History

- **2025-01**: Performance analysis, float support, BufferLayout refactoring
- **2024-12**: Framework enhancements (Option B), derive patterns
- **2024-11**: Graphics rendering, lighting system
- **2024-10**: Initial framework development

---

**Last Updated:** January 2025

For questions or issues, see `TODO.md` or review `SESSION_SUMMARY.md` for recent work.
