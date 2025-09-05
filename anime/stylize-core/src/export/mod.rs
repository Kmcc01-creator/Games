//! Export utilities (PNG/WebP sprite sheets + metadata) - stubs.

#[derive(Debug, Clone, serde::Serialize)]
pub struct AtlasMeta {
    pub cols: u32,
    pub rows: u32,
    pub frames: u32,
    pub anchor_px: (i32, i32),
}

impl Default for AtlasMeta {
    fn default() -> Self { Self { cols: 4, rows: 4, frames: 16, anchor_px: (0, 0) } }
}

