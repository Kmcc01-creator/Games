## macrokid_threads: Derive‑Friendly Scheduling

This document shows how to use the generic threading primitives in `macrokid_core` (feature `threads`) together with derive macros from `macrokid_threads_derive` to build parallel job and stage execution.

### Install

- Enable the runtime primitives:

```
# Cargo.toml of your crate
[dependencies]
macrokid_core = { path = "../macrokid_core", features = ["threads"] }
macrokid_threads_derive = { path = "../macrokid_threads_derive" }
```

### Runtime Concepts (macrokid_core::threads)

- `Scheduler`: schedules `Job = Box<dyn FnOnce() + Send + 'static>`.
  - `Direct`: runs jobs inline.
  - `ThreadPool::new(n)`: fixed‑size pool with `scope(|s| s.spawn(..))` and `join_all(sched, jobs)` helper.
- `JobRun`: trait for “things that can run”. `SpawnExt` adds `spawn()`/`spawn_ref()` convenience.
- `ResourceAccess`: optional metadata (`reads()`/`writes()`) for conflict‑aware batching (future enhancement).

### Derives (macrokid_threads_derive)

- `#[derive(Job)]`
  - Implements `JobRun` for the type.
  - Calls `fn run(self)` by default, or override with `#[job(method = "run_impl")]`.
- `#[derive(System)]`
  - Adds `ResourceAccess` based on type‑level attributes:
    - `#[reads(A, B)]` and `#[writes(C, D)]`.
- `#[derive(Schedule)]`
  - On a struct; each field marked `#[stage(...)]` must be a tuple of systems.
  - Generates `fn run<S: Scheduler>(&self, sched: &S)` that executes stages honoring dependencies:
    - `#[stage(after = "a, b")]` means this stage runs after both `a` and `b`.
    - `#[stage(before = "c, d")]` is sugar for `c` and `d` depending on this stage.
    - When no dependencies are provided, declaration order is used.
  - Within a stage, systems run in parallel via `join_all`.

### Examples

#### 1) Simple Job

```
use std::sync::Arc;
use macrokid_core::threads::{ThreadPool, SpawnExt};
use macrokid_threads_derive::Job;

#[derive(Clone, Job)]
struct Build { data: Arc<Vec<u8>> }
impl Build { fn run(self) { /* work */ } }

let pool = ThreadPool::new(4);
Build { data: Arc::new(vec![1,2,3]) }.spawn(&pool);
```

Custom method name:
```
#[derive(Clone, Job)]
#[job(method = "run_impl")]
struct Task;
impl Task { fn run_impl(self) { /* work */ } }
```

#### 2) System with resource metadata

```
use macrokid_core::threads::{ThreadPool, SpawnExt};
use macrokid_threads_derive::{Job, System};

#[derive(Clone, Job, System)]
#[reads(Transform, Light)]
#[writes(DrawList)]
struct Extract;
impl Extract { fn run(self) { /* extract world → render data */ } }

let pool = ThreadPool::new(4);
Extract.spawn(&pool); // runs as a job; metadata available via ResourceAccess
```

#### 3) Schedule with stages and dependencies

```
use macrokid_core::threads::ThreadPool;
use macrokid_threads_derive::{Job, System, Schedule};

#[derive(Clone, Job, System)]
struct Extract; impl Extract { fn run(self) {} }
#[derive(Clone, Job, System)]
struct Prepare; impl Prepare { fn run(self) {} }
#[derive(Clone, Job, System)]
struct Record; impl Record { fn run(self) {} }

#[derive(Schedule)]
struct FrameSchedule {
    #[stage(name = "extract")] extract: (Extract,),
    #[stage(name = "prepare", after = "extract")] prepare: (Prepare,),
    #[stage(name = "physics")] physics: (PhysicsSim,),
    #[stage(name = "record",  after = "prepare", before = "present")]  record:  (Record,),
    #[stage(name = "present", after = "physics")] present: (Present,),
}

let sched = ThreadPool::new(4);
let frame = FrameSchedule { extract: (Extract,), prepare: (Prepare,), record: (Record,) };
frame.run(&sched);
```

Notes:
- Stage execution honors `after` dependencies; absent dependencies, field order applies. Within a stage, tuple elements run in parallel.
- Each system type must be `Clone + Send + 'static` and implement `JobRun` (via `#[derive(Job)]` or manual impl).

### Advanced

- `join_all(&sched, jobs)` can be used directly to parallelize ad‑hoc job vectors and wait for completion.
- `ThreadPool::scope(|s| s.spawn(..))` allows spawning non‑'static work within a lexical scope (custom code only; derives use 'static jobs by default).
- Conflict‑aware batching is built into `Schedule`: within a stage, systems are greedily grouped into batches where no two systems conflict on resource reads/writes (conflict = write/write or write/read on the same type). Each batch runs in parallel, batches run sequentially.
- Debugging: derive adds `fn topo_groups() -> Vec<Vec<&'static str>>` that returns topological layers (stages that can run together). Useful to inspect ordering.

### Limitations / Roadmap

- `Schedule` errors on unknown `after` targets or dependency cycles.
- Derive‑generated jobs capture by cloning the system value; use `Arc<T>` and interior mutability when needed.
- Non‑'static borrows are not supported by derives yet; use pool scopes in hand‑written code.
