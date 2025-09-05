use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDNA {
    pub id: String,
    pub proportions: Proportions,
    pub hair: Hair,
    pub clothes: Clothes,
    pub palette: Palette,
    pub shading: Shading,
    pub lines: Lines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proportions {
    pub head_scale: f32,
    pub eye_scale: f32,
    pub limb_len: LimbLen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbLen {
    pub arm: f32,
    pub leg: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hair {
    pub style: String,
    pub strands: u32,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clothes {
    pub top: String,
    pub skirt_folds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub skin: Vec<String>,
    pub hair: Vec<String>,
    pub cloth: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Shading {
    pub bands: u32,
    pub face_shadow_threshold: f32,
    pub cloth_shadow_threshold: f32,
    // Style extensions (optional): hue shifts, saturation scales, rim, spec, softness
    pub hue_shift_shadow_deg: f32,
    pub hue_shift_light_deg: f32,
    pub sat_scale_shadow: f32,
    pub sat_scale_light: f32,
    pub rim_strength: f32,
    pub rim_width: f32,
    pub spec_threshold: f32,
    pub spec_intensity: f32,
    pub band_softness: f32,
}

impl Default for Shading {
    fn default() -> Self {
        Self {
            bands: 3,
            face_shadow_threshold: 0.63,
            cloth_shadow_threshold: 0.55,
            hue_shift_shadow_deg: 0.0,
            hue_shift_light_deg: 0.0,
            sat_scale_shadow: 1.0,
            sat_scale_light: 1.0,
            rim_strength: 0.2,
            rim_width: 0.35,
            spec_threshold: 0.86,
            spec_intensity: 0.22,
            band_softness: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lines {
    pub width_px: f32,
    pub crease_angle_deg: f32,
}
