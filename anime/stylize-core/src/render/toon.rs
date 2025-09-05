//! Toon lighting pass stubs. See assets/shaders/toon.frag for reference.

#[derive(Debug, Clone, Copy)]
pub struct ToonParams {
    pub shadow_threshold: f32,
    pub mid_threshold: f32,
    pub rim_strength: f32,
    pub rim_width: f32,
}

impl Default for ToonParams {
    fn default() -> Self {
        Self { shadow_threshold: 0.6, mid_threshold: -1.0, rim_strength: 0.0, rim_width: 0.5 }
    }
}

pub fn describe() -> &'static str {
    "Pass 2: apply toon ramp with region-aware thresholds"
}

