//! Rigging foundations (skeleton, IK, blendspaces) - stubs.

#[derive(Debug, Clone)]
pub struct Skeleton {
    pub bone_count: usize,
}

impl Default for Skeleton {
    fn default() -> Self { Self { bone_count: 0 } }
}

