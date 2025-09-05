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
pub struct Shading {
    pub bands: u32,
    pub face_shadow_threshold: f32,
    pub cloth_shadow_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lines {
    pub width_px: f32,
    pub crease_angle_deg: f32,
}

