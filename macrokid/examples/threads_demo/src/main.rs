use macrokid_core::threads::ThreadPool;
use macrokid_threads_derive::{Job, System, Schedule};

// Dummy resource types for ResourceAccess metadata
struct Transform;
struct PhysicsState;
struct RenderData;
struct DrawList;

#[derive(Clone, Job, System)]
#[reads(Transform)]
struct Extract;
impl Extract { fn run(self) { println!("[extract] running"); } }

#[derive(Clone, Job, System)]
#[reads(PhysicsState)]
struct PhysicsSim;
impl PhysicsSim { fn run(self) { println!("[physics] running"); } }

#[derive(Clone, Job, System)]
#[reads(RenderData)]
#[writes(DrawList)]
struct Prepare;
impl Prepare { fn run(self) { println!("[prepare] running"); } }

#[derive(Clone, Job, System)]
#[reads(DrawList)]
struct Record;
impl Record { fn run(self) { println!("[record] running"); } }

#[derive(Schedule)]
struct FrameSchedule {
    #[stage(name = "extract")] extract: (Extract,),
    #[stage(name = "physics")] physics: (PhysicsSim,),
    #[stage(name = "prepare", after = "extract")] prepare: (Prepare,),
    #[stage(name = "record",  after = "prepare, physics")]  record:  (Record,),
}

fn main() {
    let sched = ThreadPool::new(4);
    let frame = FrameSchedule { extract: (Extract,), physics: (PhysicsSim,), prepare: (Prepare,), record: (Record,) };
    frame.run(&sched);
}
