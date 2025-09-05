//! Silhouette/outline pass stubs. See assets/shaders/outline.vert for reference.

#[derive(Debug, Clone, Copy)]
pub struct OutlineParams {
    pub width_px: f32,
    pub crease_angle_deg: f32,
}

impl Default for OutlineParams {
    fn default() -> Self {
        Self { width_px: 2.0, crease_angle_deg: 42.0 }
    }
}

pub fn describe() -> &'static str { "Pass 3: mesh backface expansion + optional crease edges" }

