//! G-buffer pass stubs. Future: Vulkan dynamic rendering setup.

#[derive(Debug, Clone, Copy)]
pub struct GBufferFormats {
    pub albedo_region: u32,
    pub normal: u32,
    pub depth: u32,
    pub motion: u32,
    pub flags: u32,
}

impl Default for GBufferFormats {
    fn default() -> Self {
        Self { albedo_region: 0, normal: 0, depth: 0, motion: 0, flags: 0 }
    }
}

pub fn describe() -> &'static str {
    "Pass 1: populate albedo/region, normal, depth, motion, flags"
}

