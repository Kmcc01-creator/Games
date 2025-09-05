# GFX DSL — Project Status & Roadmap

## Intentions
- Vulkan-first DSL to describe render graphs, pipelines, and windowing succinctly.
- Minimize boilerplate by generating engine scaffolding (config + runtime wiring).
- Keep Macrokid generic: use it to build domain DSLs without baking graphics into core.
- Support two authoring modes that unify on a single IR:
  - Declarative token DSL (`vk_engine!`) → static `EngineConfig`.
  - Programmatic builder (type-state, validated) → runtime `EngineConfig`.
- Longer-term: accept config files (text/TOML/YAML) that map to the same IR.

## Current State
- Crates
  - `examples/gfx_dsl` (proc macro): `vk_engine!` builds `mgfx_cfg::CONFIG` using shared IR.
  - `examples/gfx_dsl_support` (normal lib): shared IR, runtime, and a type-state builder.
  - `examples/gfx_dsl/examples/demo.rs`: shows macro-driven config + runtime usage.
- Macro (`vk_engine!`)
  - Emits only `mgfx_cfg::{CONFIG}` and re-uses IR types from support crate.
  - Parser improved with `syn::custom_keyword!` for clearer, keyword-driven errors.
- Support IR/Runtime
  - IR: `WindowCfg`, `ShaderPaths`, `Topology`, `PipelineDesc`, `EngineConfig`.
  - Runtime: `RenderBackend` trait, `VulkanBackend` stub, `Engine<B>` with `new_from_config`, `init_pipelines`, `frame`.
- Builder (type-state + validation)
  - `EngineBuilder<State>`: `Empty → HasApp → HasWindow → HasGraph → build()`.
  - Nested: `GraphBuilder → PassBuilder → PipelineBuilder`.
  - Validation: non-empty pipelines, non-empty shader paths, duplicate pipeline detection.
  - Tests included for builder happy-path and failures.
- Macrokid alignment
  - Uses `macrokid_core` builder patterns where helpful.
  - Core `pattern_dsl` bug fixed and covered by tests (struct patterns with optional `..`).

## Recent Enhancements
- Derive-powered builders
  - `#[derive(FluentBuilder)]` (examples/gfx_dsl_builder_derive) now supports:
    - Field setters via `#[builder]`, rename via `#[builder(name = "...")]`.
    - Tuple setters via `#[builder(tuple = "(a: TyA, b: TyB) => Expr", name = "...")]`.
    - Type-state transitions via `#[builder_transition(method = "...", to = "Type", receiver = "self|&mut self", body = "{ ... }")]`.
  - Applied to `GraphBuilder.finish()`, `PassBuilder.finish_pass()`, `PipelineBuilder.finish()`.
- Resource & Vertex Layout derives
  - `#[derive(ResourceBinding)]` (examples/render_resources): field-level `#[uniform]`/`#[texture]`/`#[sampler]` with set/binding, duplicate checks, and static `BindingDesc[]` + `ResourceBindings` trait.
  - `#[derive(BufferLayout)]`: struct-level `#[buffer(stride = N, step = "vertex|instance")]`, field-level `#[vertex(location, format)]`, computes per-attribute `offset`/`size` (format or type inference), stride (sum or explicit), and `step` mode; implements `VertexLayout`.
- Engine validation (backend-agnostic)
  - `Engine::validate_pipelines_with<ResourceBindings, VertexLayout>` prints bindings/attrs/stride/step per pipeline and checks non-emptiness.

## Known Gaps & Areas for Improvement
- Type-State Coverage
  - Ensure “graph complete” actually implies at least one pass and per-pass invariants.
  - Consider per-pass builder state (e.g., `HasAtLeastOnePipeline`).
- Validation Breadth
  - Cross-checks for pass names, uniqueness, future attachments/subpass relationships.
  - Stronger shader validation hooks (e.g., non-empty, maybe extension hints).
- Diagnostics
  - Macro errors should leverage span-aware helpers and suggestions.
  - Add `trybuild` tests for macro diagnostics and failure modes.
- Config File Ingestion
  - Add `vk_engine_from_str!(include_str!("app.mgfx"))` with a small grammar, or serde-backed TOML.
  - Reuse the same validation pipeline and IR normalization.
- Reusability & Composition
  - Extract reusable presets (e.g., triangle/no-depth pipeline), allow composing graphs.
  - Provide helpers for common topologies/depth modes to reduce repetition.
- Memory & Ownership
  - Example uses `Box::leak` to produce `'static` slices; good for demos but not production.
  - Options: owned `EngineConfig` for runtime, or macro-side static generation only.
- BufferLayout fidelity
  - Current size inference is heuristic-based and alignment is not modeled. Add alignment rules and optional per-field overrides (e.g., `#[vertex(offset = N, size = M)]`).
- Format validation
  - `format` is a string today; elevate to an enum with validation and conversions.
- Documentation & Examples
  - Add builder-based demo and side-by-side macro/builder example.
  - Expand docs for error scenarios and best practices.
- Long-Term Direction
  - Backend abstraction (keep RenderBackend thin; avoid vendor deps here).
  - Optional ECS integration and a minimal schedule layer in support crate.
  - Resource/attachment graph modeling and validation.

## Next Steps & Goals
- Phase 1: Builder Hardening
  - Add per-pass/pipeline type-state guardrails and richer validation.
  - Expand unit tests for duplicate passes, empty graphs, and topology/depth defaults.
- Phase 2: Macro Diagnostics
  - Replace generic errors with span-aware messages and suggestions.
  - Add `trybuild` UI tests for invalid keys, missing commas, bad values.
- Phase 3: Config Files
  - Introduce `vk_engine_from_str!` with a small `.mgfx` grammar or a TOML loader.
  - Normalize → validate → `EngineConfig` pipeline shared with builder/macro.
- Phase 4: Derive Codegen for Builders
  - Add a derive macro (in examples) to generate repetitive builder glue.
  - Keep surface ergonomic and testable.
- Phase 5: Presets & Composition
  - Publish a few canonical presets and a composed example (multi-pass).
  - Provide guidance on mixing builder and macro inputs.
- Phase 6: Advanced Graph Modeling
  - Plan an attachment/subpass model and semantics, with validation passes.

## How to Use (Today)
- Declarative (macro):
  - Define config with `vk_engine! { { ... } }` and use `mgfx_cfg::CONFIG`.
  - Run with `Engine::<VulkanBackend>::new_from_config(&mgfx_cfg::CONFIG)`.
- Programmatic (builder):
  - Build `EngineConfig` with `EngineBuilder::<Empty>::new()...build()?`.
  - Run with the same `Engine::<VulkanBackend>` runtime.
- Resource/Layout derives:
  - `Type::describe_bindings()`, `Type::describe_vertex_layout()`, `Type::describe_vertex_buffer()`; or use `ResourceBindings` / `VertexLayout` traits.
- Validation:
  - `engine.validate_pipelines_with::<Material, Vertex>(&mgfx_cfg::CONFIG)` then `engine.init_pipelines(&mgfx_cfg::CONFIG)`.

## Notes & Follow‑Ups
- Keep core generic and backend-agnostic; avoid vendor deps in examples.
- Strive for unified IR across macro, builder, and file-based configs.
- Prefer typed enums and validated schemas over raw strings when ergonomics allow.
- Use `macrokid_core::diag` for consistent, span-aware messaging.

## Break Checkpoint
- The current snapshot compiles cleanly and demonstrates:
  - Thin macro → shared IR `CONFIG`.
  - Type-state builders with derive-generated transitions.
  - Resource and vertex layout derives with validation and runtime summaries.
  - A simple engine validation pass that ties the pieces together.
- Good time to pause, gather feedback, and plan alignment/format refinements before expanding the graph model.

## References
- See `RECOMMENDED_IMPROVEMENTS.md` for a deeper design rationale and builder proposal.
- See `examples/gfx_dsl/examples/demo.rs` for the current macro demo.
- Support crate: `examples/gfx_dsl_support` for IR, runtime, and builder implementation.
