#[derive(Clone, Debug)]
pub enum PassKind { Graphics, Compute }

#[derive(Clone, Debug)]
pub enum SizeSpec {
    Abs { width: u32, height: u32 },
    Rel { sx: f32, sy: f32 },
    Swapchain,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct UsageMask: u32 {
        const COLOR = 1 << 0;
        const DEPTH = 1 << 1;
        const SAMPLED = 1 << 2;
        const STORAGE = 1 << 3;
        const TRANSFER_SRC = 1 << 4;
        const TRANSFER_DST = 1 << 5;
    }
}

#[derive(Clone, Debug)]
pub struct TextureDesc {
    pub name: &'static str,
    pub format: &'static str,
    pub size: SizeSpec,
    pub usage: UsageMask,
    pub samples: u32,
}

#[derive(Clone, Debug)]
pub struct OutputDesc {
    pub name: &'static str,
    pub format: &'static str,
    pub size: SizeSpec,
    pub usage: UsageMask,
    pub samples: u32,
    pub is_depth: bool,
}

#[derive(Clone, Debug)]
pub struct PassDesc {
    pub name: &'static str,
    pub kind: PassKind,
    // Legacy compatibility (may be None when outputs are used)
    pub color: Option<&'static [crate::pipeline::ColorTargetDesc]>,
    pub depth: Option<crate::pipeline::DepthTargetDesc>,
    pub inputs: Option<&'static [&'static str]>,
    // Preferred attachment description with names/sizes/usages
    pub outputs: Option<&'static [OutputDesc]>,
}

pub trait PassInfo { fn pass_desc() -> &'static PassDesc; }

#[derive(Clone, Debug)]
pub struct GraphPass {
    pub pass: &'static PassDesc,
    pub pipeline: &'static crate::pipeline::PipelineDesc,
}

#[derive(Clone, Debug, Default)]
pub struct RenderGraphDesc { pub passes: Vec<GraphPass> }

pub struct RenderGraphBuilder { desc: RenderGraphDesc }

impl RenderGraphBuilder {
    pub fn new() -> Self { Self { desc: RenderGraphDesc::default() } }
    pub fn add_pass(mut self, pass: &'static PassDesc, pipeline: &'static crate::pipeline::PipelineDesc) -> Self {
        self.desc.passes.push(GraphPass { pass, pipeline }); self
    }
    pub fn build(self) -> RenderGraphDesc { self.desc }
}

#[derive(Clone, Debug)]
pub struct ResourcePlan {
    pub name: &'static str,
    pub format: &'static str,
    pub size: SizeSpec,
    pub usage: UsageMask,
    pub samples: u32,
}

#[derive(Clone, Debug)]
pub struct PassPlan {
    pub name: &'static str,
    pub colors: Vec<&'static str>,
    pub depth: Option<&'static str>,
}

pub fn compute_actual_size(size: &SizeSpec, swap_w: u32, swap_h: u32) -> (u32, u32) {
    match size {
        SizeSpec::Swapchain => (swap_w, swap_h),
        SizeSpec::Rel { sx, sy } => {
            let w = ((*sx * swap_w as f32).max(1.0)).round() as u32;
            let h = ((*sy * swap_h as f32).max(1.0)).round() as u32;
            (w, h)
        }
        SizeSpec::Abs { width, height } => (*width, *height),
    }
}

/// Very simple planner: flattens all pass outputs into resources and creates per-pass bindings.
/// Does not alias or validate overlaps yet — goal is to land resource planning structure first.
pub fn plan_resources(desc: &RenderGraphDesc) -> (Vec<ResourcePlan>, Vec<PassPlan>) {
    use std::collections::BTreeMap;
    let mut by_name: BTreeMap<&'static str, ResourcePlan> = BTreeMap::new();
    let mut pass_plans: Vec<PassPlan> = Vec::new();
    for gp in &desc.passes {
        let mut colors: Vec<&'static str> = Vec::new();
        let mut depth: Option<&'static str> = None;
        if let Some(outs) = gp.pass.outputs {
            for o in outs {
                // Promote to static names; PassDesc holds &'static already
                let name: &'static str = Box::leak(o.name.to_string().into_boxed_str());
                let rp = ResourcePlan { name, format: o.format, size: o.size.clone(), usage: o.usage, samples: o.samples };
                by_name.entry(name).or_insert(rp);
                if o.is_depth { depth = Some(name); } else { colors.push(name); }
            }
        } else {
            // Legacy: synthesize names for color/depth
            if let Some(cols) = gp.pass.color { for (i, c) in cols.iter().enumerate() {
                let name: &'static str = Box::leak(format!("{}_col{}", gp.pass.name, i).into_boxed_str());
                let rp = ResourcePlan { name, format: c.format, size: SizeSpec::Swapchain, usage: UsageMask::COLOR, samples: 1 };
                by_name.entry(name).or_insert(rp); colors.push(name);
            } }
            if let Some(d) = &gp.pass.depth { let name: &'static str = Box::leak(format!("{}_depth", gp.pass.name).into_boxed_str()); let rp = ResourcePlan { name, format: d.format, size: SizeSpec::Swapchain, usage: UsageMask::DEPTH, samples: 1 }; by_name.entry(name).or_insert(rp); depth = Some(name); }
        }
        pass_plans.push(PassPlan { name: gp.pass.name, colors, depth });
    }
    let mut resources: Vec<ResourcePlan> = by_name.into_values().collect();
    resources.sort_by_key(|r| r.name);
    (resources, pass_plans)
}

/// Convenience planner when only pass descriptors are available.
pub fn plan_resources_from_passes(passes: &[&PassDesc]) -> (Vec<ResourcePlan>, Vec<PassPlan>) {
    use std::collections::BTreeMap;
    let mut by_name: BTreeMap<&'static str, ResourcePlan> = BTreeMap::new();
    let mut pass_plans: Vec<PassPlan> = Vec::new();
    for p in passes {
        let mut colors: Vec<&'static str> = Vec::new();
        let mut depth: Option<&'static str> = None;
        if let Some(outs) = p.outputs {
            for o in outs {
                let name: &'static str = o.name;
                let rp = ResourcePlan { name, format: o.format, size: o.size.clone(), usage: o.usage, samples: o.samples };
                by_name.entry(name).or_insert(rp);
                if o.is_depth { depth = Some(name); } else { colors.push(name); }
            }
        } else {
            if let Some(cols) = p.color { for (i, c) in cols.iter().enumerate() {
                let name: &'static str = Box::leak(format!("{}_col{}", p.name, i).into_boxed_str());
                let rp = ResourcePlan { name, format: c.format, size: SizeSpec::Swapchain, usage: UsageMask::COLOR, samples: 1 };
                by_name.entry(name).or_insert(rp); colors.push(name);
            } }
            if let Some(d) = &p.depth { let name: &'static str = Box::leak(format!("{}_depth", p.name).into_boxed_str()); let rp = ResourcePlan { name, format: d.format, size: SizeSpec::Swapchain, usage: UsageMask::DEPTH, samples: 1 }; by_name.entry(name).or_insert(rp); depth = Some(name); }
        }
        pass_plans.push(PassPlan { name: p.name, colors, depth });
    }
    let mut resources: Vec<ResourcePlan> = by_name.into_values().collect();
    resources.sort_by_key(|r| r.name);
    (resources, pass_plans)
}
