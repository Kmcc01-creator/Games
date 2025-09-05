//! Secondary motion (verlet chains, cloth-lite) - stubs.

#[derive(Debug, Clone, Copy)]
pub struct ChainParams {
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for ChainParams {
    fn default() -> Self { Self { stiffness: 0.6, damping: 0.12 } }
}

