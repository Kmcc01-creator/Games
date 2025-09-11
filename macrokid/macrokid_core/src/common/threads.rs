//! Minimal, zero-dependency job scheduling primitives (feature `threads`).
//!
//! Design goals
//! - Keep types simple and decoupled from backends/ECS; just scheduling.
//! - Provide a direct (immediate) scheduler and a tiny thread pool.
//! - Offer a scoped API to spawn jobs and wait for completion without leaking joins.
//!
//! This module is intentionally small to allow promotion to a dedicated crate later
//! (e.g., `macrokid_threads`) without breaking users. The API here focuses on
//! closures as jobs; more advanced traits can layer above.

use std::sync::{mpsc, Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::any::TypeId;

/// A unit of work. Implemented as a boxed `FnOnce()` for ergonomics.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// A scheduler is able to accept jobs. Implementations may run jobs immediately
/// (direct) or distribute to workers (thread pool).
pub trait Scheduler: Send + Sync + 'static {
    fn schedule(&self, job: Job);
}

/// Runs jobs immediately on the calling thread.
#[derive(Default, Clone, Copy)]
pub struct Direct;
impl Scheduler for Direct {
    fn schedule(&self, job: Job) { (job)(); }
}

enum Message {
    Run(Job),
    Shutdown,
}

/// A tiny thread pool with a fixed number of worker threads.
pub struct ThreadPool {
    tx: mpsc::Sender<Message>,
    workers: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    /// Create a pool with `workers` threads.
    pub fn new(workers: usize) -> Self {
        assert!(workers > 0, "thread pool requires at least one worker");
        let (tx, rx) = mpsc::channel::<Message>();
        let rx = Arc::new(Mutex::new(rx));
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let rx_cloned = Arc::clone(&rx);
            handles.push(thread::spawn(move || loop {
                let msg = { rx_cloned.lock().unwrap().recv().unwrap() };
                match msg {
                    Message::Run(job) => { (job)(); }
                    Message::Shutdown => break,
                }
            }));
        }
        Self { tx, workers: handles }
    }

    /// Spawn a scope, allowing jobs to be scheduled and then joined before returning.
    pub fn scope<F>(&self, f: F)
    where
        F: FnOnce(&Scope<'_>),
    {
        let state = Arc::new(ScopeState::new());
        let scope = Scope { pool: self, state: Arc::clone(&state) };
        f(&scope);
        // Wait for all jobs spawned via this scope to finish.
        state.wait_all();
    }
}

impl Scheduler for ThreadPool {
    fn schedule(&self, job: Job) { let _ = self.tx.send(Message::Run(job)); }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers { let _ = self.tx.send(Message::Shutdown); }
        while let Some(h) = self.workers.pop() { let _ = h.join(); }
    }
}

struct ScopeState {
    remaining: AtomicUsize,
    pair: (Mutex<()>, Condvar),
}

impl ScopeState {
    fn new() -> Self { Self { remaining: AtomicUsize::new(0), pair: (Mutex::new(()), Condvar::new()) } }
    fn incr(&self) { self.remaining.fetch_add(1, Ordering::AcqRel); }
    fn decr(&self) {
        if self.remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
            let (lock, cv) = &self.pair; let _g = lock.lock().unwrap(); cv.notify_all();
        }
    }
    fn wait_all(&self) {
        let (lock, cv) = &self.pair; let mut g = lock.lock().unwrap();
        while self.remaining.load(Ordering::Acquire) != 0 { g = cv.wait(g).unwrap(); }
    }
}

/// A scope that allows spawning jobs tied to a join point at the end of the scope.
pub struct Scope<'p> {
    pool: &'p ThreadPool,
    state: Arc<ScopeState>,
}

impl<'p> Scope<'p> {
    /// Spawn a job into the pool; scope will wait for its completion.
    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.state.incr();
        let state = Arc::clone(&self.state);
        self.pool.schedule(Box::new(move || { f(); state.decr(); }));
    }
}

/// Schedule a set of jobs and wait for completion using any `Scheduler`.
///
/// This provides a per-stage barrier without requiring a pool-specific scope API.
pub fn join_all<S, I>(sched: &S, jobs: I)
where
    S: Scheduler,
    I: IntoIterator<Item = Job>,
{
    let state = Arc::new(ScopeState::new());
    let st2 = state.clone();
    for job in jobs {
        st2.incr();
        let st3 = st2.clone();
        sched.schedule(Box::new(move || { (job)(); st3.decr(); }));
    }
    state.wait_all();
}

// ===========================
// Job traits for derive usage
// ===========================

/// Trait implemented by types that can run as jobs.
///
/// Derives or manual impls should provide the body for `run(self)`.
pub trait JobRun {
    fn run(self);
}

/// Convenience extension to spawn jobs on any Scheduler.
///
/// - `spawn(self, sched)`: moves the job and schedules it.
/// - `spawn_ref(&self, sched)`: clones the job and schedules it (requires Clone).
pub trait SpawnExt: JobRun + Sized {
    fn spawn<S: Scheduler>(self, sched: &S)
    where
        Self: Send + 'static,
    {
        sched.schedule(Box::new(move || self.run()));
    }

    fn spawn_ref<S: Scheduler>(&self, sched: &S)
    where
        Self: Clone + Send + 'static,
    {
        let cloned = self.clone();
        sched.schedule(Box::new(move || cloned.run()));
    }
}

impl<T: JobRun> SpawnExt for T {}

// ===========================
// System resource access metadata
// ===========================

/// Declares which resource types a system reads and writes.
///
/// The default derive populates static sets using `TypeId::of::<T>()` for listed types.
pub trait ResourceAccess {
    fn reads() -> &'static [TypeId] { &[] }
    fn writes() -> &'static [TypeId] { &[] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn direct_runs_inline() {
        let s = Direct;
        let flag = Arc::new(AtomicUsize::new(0));
        let f2 = flag.clone();
        s.schedule(Box::new(move || f2.fetch_add(1, Ordering::AcqRel)));
        assert_eq!(flag.load(Ordering::Acquire), 1);
    }

    #[test]
    fn pool_runs_and_scope_joins() {
        let pool = ThreadPool::new(2);
        let n = Arc::new(AtomicUsize::new(0));
        pool.scope(|scope| {
            for _ in 0..8 {
                let n2 = n.clone();
                scope.spawn(move || { n2.fetch_add(1, Ordering::AcqRel); });
            }
        });
        assert_eq!(n.load(Ordering::Acquire), 8);
    }
}
