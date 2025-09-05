//! Compute sprite atlas packing stubs.

#[derive(Debug, Clone, Copy)]
pub struct AtlasConfig {
    pub cols: u32,
    pub rows: u32,
}

impl Default for AtlasConfig {
    fn default() -> Self { Self { cols: 4, rows: 4 } }
}

pub fn describe() -> &'static str { "Pass 5: pack frames to atlas" }

