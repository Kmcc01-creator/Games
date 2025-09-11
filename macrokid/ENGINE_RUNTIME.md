# Engine Runtime and Threading Design

This document outlines a forward‑looking design for the macrokid graphics engine/runtime with a focus on:
- Threading models (multi‑threaded command buffer recording, single‑threaded submission)
- Subsystem boundaries (renderer vs. ECS/AI/physics)
- API scaffolding (`Renderer`, `Frame`) and lifetime management
- `PhantomData` choices and auto‑trait coupling
- Future: a scheduled full job system and potential `macrokid_threads` crate

## Goals

- Keep the renderer focused on GPU device/queues, per‑frame resources, and executing a render graph.
- Allow multi‑threaded recording of command buffers with single‑threaded submission.
- Decouple game logic systems (ECS/AI/physics) from the renderer via an Extract → Prepare → Record pipeline.
- Preserve compile‑time guarantees through zero‑cost type markers and traits.

## Recommended Split

- Renderer (graphics): Owns GPU device, queues, per‑frame allocators, render‑graph execution.
- EngineRuntime (or Orchestrator): Coordinates frame stages and scheduling, wires subsystems together.
- Systems (ECS/AI/physics): Run independently; “Extract” emits render‑friendly snapshots for the renderer.

## Threading Models

- Multi‑threaded recording: Workers record secondary/parallel command buffers using per‑pass encoders.
- Single‑threaded submission: One thread owns queue/device submission; workers submit recorded bundles.
- Staged frame pipeline: Update → Extract → Prepare → Record → Submit → Present.

## Minimal API Scaffolding

Traits to shape the API without locking in backend internals:

- `Renderer`: thread‑safe façade responsible for frames.
- `Frame`: per‑frame lifetime guard; creates encoders for passes/subpasses.
- `CommandCtx` (associated type): recording context (likely !Send) created by a Send handle.

This allows:

```
let frame = renderer.begin_frame();
let ctx = frame.encoder_for("gbuffer");
// workers record on ctx in parallel (actual type backend‑dependent)
renderer.end_frame(frame);
```

## PhantomData Guidance

- Engine<B> using `PhantomData<B>` couples Engine’s auto‑traits (Send/Sync) to the backend B.
  - Keep `PhantomData<B>` if you want Engine’s Send/Sync to reflect backend constraints.
  - Use `PhantomData<*const B>` if you want a pure type‑level tag without auto‑trait coupling.
- Validator/Combinator types typically use `PhantomData<*const T>` when they should not inherit auto‑traits from T.
- For lighting helpers, returning `PhantomData<Model::RB>` is optional; callers can name the RB type explicitly.

## Scheduled Full Job System

- Introduce a scheduler that runs Systems (ECS/AI/physics) and the Render stages as jobs.
- Clear stage boundaries with data handoff:
  - Extract: Copy ECS state into immutable, render‑friendly buffers.
  - Prepare: Build draw lists, material/resource bindings.
  - Record: Issue command buffers (potentially in parallel) via pass encoders.
  - Submit/Present: Single‑threaded submission and present.
- Consider a dedicated crate `macrokid_threads` for lightweight job abstractions (traits, thread pools, work‑stealing) and derive helpers.
  - Alternatively, host small thread helpers in `macrokid_core` behind a feature flag and promote to a crate when mature.

## Current Status and Next Steps

Implemented:
- Backend trait bounds tightened (`RenderBackend: Send + Sync + 'static`).
- Minimal `Renderer` and `Frame` trait scaffolding in `macrokid_graphics`.
- Feature‑gated scheduler runtime in `macrokid_core::threads` (`Scheduler`, `ThreadPool`, `join_all`, `JobRun`, `SpawnExt`, `ResourceAccess`).
- Derives in `macrokid_threads_derive`:
  - `#[derive(Job)]` for job behavior wiring.
  - `#[derive(System)]` to declare `reads()`/`writes()` metadata.
  - `#[derive(Schedule)]` with:
    - Stage dependencies: `after = "a, b"` and sugar `before = "c, d"`.
    - Conflict‑aware batching within a stage based on `ResourceAccess` (greedy grouping of non‑conflicting systems).
    - `topo_groups()` debug helper returning topological layers.

Next:
- Integrate a Scheduler into Renderer flows (parallel recording hooks) while preserving single‑threaded submission.
- Expand examples to include record‑pass jobs using `Frame::encoder_for(..)` when a backend is ready.
