# Macrokid Experiments: Graphics + Cross‑Language Tooling

This document tracks experimental work to make macro authoring more “plug‑and‑play,” build a graphics‑focused DSL, and explore cross‑language codegen by orchestrating Clang from Rust.

Status: experimental, evolving rapidly. APIs and behavior may change.

## Goals

- Reduce boilerplate in proc‑macro development via reusable primitives.
- Provide a production‑quality graphics DSL (resources, vertex layout, pipelines).
- Explore cross‑language generation and analysis (C/C++ headers) from Rust.

## What’s Implemented

### 1) macrokid_core improvements
- Derive entry + helpers
  - `derive_entry!`: one‑line proc_macro_derive wrappers with attrs.
  - `derive::{with_type_spec, impl_for_trait}` utilities.
- Collections, codegen, diagnostics
  - `collect::{from_named_fields, unique_by, require_non_empty}`
  - `codegen::{static_slice_mod, impl_trait_method_static_slice, impl_inherent_methods}`
  - `diag::Collector` to aggregate multiple errors.
- Typed attribute schemas
  - `attr_schema::{AttrSchema, AttrSchemaSet, scope}`
  - `exclusive_schemas!` macro sugar for mutually exclusive attr blocks.
- Walkers + templates
  - `walk::{bind_variant, bind_struct, FieldCtx, VariantCtx}`
  - `templates::{display, debug}` for common impls.

### 2) macrokid_graphics crates
- Runtime (`macrokid_graphics`)
  - `resources`: ResourceKind/BindingDesc, VertexLayout types (VertexAttr, VertexBufferDesc, StepMode).
  - `pipeline`: Topology, ShaderPaths, PipelineDesc, PipelineInfo trait.
- Derives (`macrokid_graphics_derive`)
  - `#[derive(ResourceBinding)]`: typed schemas + collection/codegen helpers.
  - `#[derive(BufferLayout)]`: infers sizes/offsets, emits VertexLayout impl + helpers.
  - `#[derive(GraphicsPipeline)]`: parses type‑level pipeline attrs, emits static PipelineDesc + trait.
- Demo (`examples/graphics_demo`)
  - Shows ResourceBinding, BufferLayout, and GraphicsPipeline working together.

### 3) Exec‑based Clang PoC (`macrokid_clang_exec`)
- AST → IR (JSON)
  - `analyze_header`: best‑effort AST walker (struct + field + attributes).
  - `emit_cpp_header`: emit a simple C++ header from IR (with attribute comments).
- Typed mk:: annotation parsing
  - `parse_all_mk`: turns `__attribute__((annotate("mk::...")))` into typed `MkAnnotation{ kind, args }`.
- C‑only IR + macros (good fit for Vulkan)
  - `analyze_header_c`: C structs/unions, enums, typedefs, functions.
  - `analyze_macros_c`: preprocessor `#define` dump (-dM -E).
- Build integration (graphics_demo/build.rs)
  - Controlled by `CLANG_EXEC_DEMO=1`.
  - Env‑driven includes/defines: `CLANG_INCLUDES`, `CLANG_DEFS` → `-I`/`-D` flags.
  - Writes to `OUT_DIR`: `generated_demo.hpp`, `generated_demo.json`, `parsed_mk.json`, `c_ir.json`, `macros.json`.

## Why This Matters

- Macro DX: Boilerplate disappears; authors focus on transformation logic.
- Graphics DSL: Safer, declarative resource/layout/pipeline definitions with compile‑time validation.
- Cross‑language bridges: Analyze/generate C headers, JSON, TS, shader snippets from one source of truth.
- System boundaries: Orchestrate Clang from Rust for legacy code, or export Rust IR to other ecosystems.

## Experimental Nature & Limitations

- APIs are not yet stable; modules and names may change.
- Clang PoC is best‑effort: JSON shapes vary by Clang version; coverage is incremental.
- mk:: annotation format is a convention; parsers are intentionally tolerant but may evolve.
- BufferLayout size inference supports common shapes only (scalars, fixed arrays, vecN/mat4).

## How To Try Things Locally

- Graphics demo
  - `cargo run -p graphics_demo` (compiles; prints bindings/layouts/pipeline in stdout).
- Clang PoC (requires `clang`)
  - `CLANG_EXEC_DEMO=1 cargo build -p graphics_demo`
  - Optional: `CLANG_INCLUDES="/path/to/sdk/include" CLANG_DEFS="VK_USE_PLATFORM_XLIB_KHR" ...`
  - Check `target/debug/build/graphics_demo-*/out/` for generated files.

## Potential Applications

- Rust → C/C++ headers for interop (graphics engines, tooling, plugins).
- C headers → Rust IR for FFI and validation (align structs/enums/functions).
- Unified JSON IR for editor/asset pipelines across Rust/Python/Node.
- Shader interface stubs from Rust types (and vice‑versa) with validation.

## Roadmap (Proposed)

- Graphics
  - RenderPass/Engine derive to aggregate pipelines/resources/layouts.
  - ShaderInterface derive (declarative first; optional GLSL/HLSL reflection later).
  - Cross‑validation: VertexLayout ↔ Shader inputs; ResourceBinding ↔ descriptor sets.
- macrokid_core
  - More codegen templates (static_trait_impl!, static_slice! helpers).
  - Richer schema sugar and validation chains with aggregated diagnostics.
- Clang/Interop
  - Env‑driven `-std=c11/c17` and platform defines; include discovery helpers.
  - Optional in‑process `clang-sys` backend reusing the same public API.
  - vk.xml reader as an alternate Vulkan source of truth.

## Contributing / Extending

- Add derives in `macrokid_graphics_derive` using `attr_schema`, `collect`, and `codegen` helpers.
- Extend Clang PoC by adding new node handlers; keep IR additive and documented.
- Keep changes small and focused; document new user‑facing features here.

