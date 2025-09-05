#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

// Generate a unit-radius UV sphere centered at origin, scaled by radius.
// stacks: latitude segments (>= 3), slices: longitude segments (>= 3)
pub fn generate_uv_sphere(radius: f32, stacks: u32, slices: u32) -> (Vec<Vertex>, Vec<u32>) {
    let stacks = stacks.max(3);
    let slices = slices.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=stacks {
        let v = i as f32 / stacks as f32; // 0..1
        let theta = v * std::f32::consts::PI; // 0..PI
        let sin_t = theta.sin();
        let cos_t = theta.cos();
        for j in 0..=slices {
            let u = j as f32 / slices as f32; // 0..1
            let phi = u * std::f32::consts::PI * 2.0; // 0..2PI
            let sin_p = phi.sin();
            let cos_p = phi.cos();

            let nx = sin_t * cos_p;
            let ny = cos_t;
            let nz = sin_t * sin_p;
            let pos = [radius * nx, radius * ny, radius * nz];
            let normal = [nx, ny, nz];
            vertices.push(Vertex { pos, normal });
        }
    }

    let stride = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * stride + j;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            indices.extend_from_slice(&[a as u32, c as u32, b as u32]);
            indices.extend_from_slice(&[b as u32, c as u32, d as u32]);
        }
    }

    (vertices, indices)
}

