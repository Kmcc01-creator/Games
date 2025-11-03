//! Procedural Asset Generation Framework
//! 
//! This module provides a unified system for generating:
//! - Procedural geometry (meshes, primitives)  
//! - Procedural textures (noise, patterns, PBR maps)
//! - Asset pipelines and combinations
//!
//! Design goals:
//! - Type-safe vertex layouts via BufferLayout derives
//! - Composable texture generation pipeline
//! - Efficient memory layout for GPU upload
//! - Extensible primitive and pattern libraries

use glam::{Vec2, Vec3, Vec4};
use std::f32::consts::{PI, TAU};

// ============================================================================
// CORE TRAITS AND TYPES
// ============================================================================

/// Generic vertex trait that all procedural geometry must implement
pub trait Vertex: Clone + Copy + Send + Sync + 'static {
    /// Convert vertex to raw bytes for GPU buffer upload
    fn to_bytes(&self) -> Vec<u8>;
    /// Size of vertex in bytes
    fn byte_size() -> usize;
}

/// Mesh data container for any vertex type
#[derive(Debug, Clone)]
pub struct Mesh<V: Vertex> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub primitive_topology: PrimitiveTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    TriangleList,
    LineList, 
    PointList,
}

/// Standard vertex types for common use cases
#[derive(Debug, Clone, Copy)]
pub struct SimpleVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct PbrVertex {
    pub position: Vec3,
    pub normal: Vec3, 
    pub tangent: Vec4, // w = handedness
    pub uv: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorVertex {
    pub position: Vec3,
    pub color: Vec4,
}

impl Vertex for SimpleVertex {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.position.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.normal.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.uv.to_array().map(f32::to_ne_bytes).concat());
        bytes
    }
    fn byte_size() -> usize { 32 } // 3*4 + 3*4 + 2*4
}

impl Vertex for PbrVertex {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.position.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.normal.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.tangent.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.uv.to_array().map(f32::to_ne_bytes).concat());
        bytes
    }
    fn byte_size() -> usize { 48 } // 3*4 + 3*4 + 4*4 + 2*4
}

impl Vertex for ColorVertex {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.position.to_array().map(f32::to_ne_bytes).concat());
        bytes.extend_from_slice(&self.color.to_array().map(f32::to_ne_bytes).concat());
        bytes
    }
    fn byte_size() -> usize { 28 } // 3*4 + 4*4
}

// ============================================================================
// GEOMETRY GENERATION
// ============================================================================

/// Mesh builder with fluent API
pub struct MeshBuilder<V: Vertex> {
    vertices: Vec<V>,
    indices: Vec<u32>,
    topology: PrimitiveTopology,
}

impl<V: Vertex> MeshBuilder<V> {
    pub fn new(topology: PrimitiveTopology) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            topology,
        }
    }
    
    pub fn add_vertex(&mut self, vertex: V) -> u32 {
        let index = self.vertices.len() as u32;
        self.vertices.push(vertex);
        index
    }
    
    pub fn add_vertices(&mut self, vertices: &[V]) -> Vec<u32> {
        let start_index = self.vertices.len() as u32;
        self.vertices.extend_from_slice(vertices);
        (start_index..start_index + vertices.len() as u32).collect()
    }
    
    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }
    
    pub fn add_quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        // Two triangles: ABC, ACD  
        self.indices.extend_from_slice(&[a, b, c, a, c, d]);
    }
    
    pub fn build(self) -> Mesh<V> {
        Mesh {
            vertices: self.vertices,
            indices: self.indices,
            primitive_topology: self.topology,
        }
    }
}

/// Primitive geometry generators
pub struct Primitives;

impl Primitives {
    /// Generate UV sphere with customizable resolution
    pub fn uv_sphere<V: From<SimpleVertex> + Vertex>(radius: f32, long_segments: u32, lat_segments: u32) -> Mesh<V> {
        let mut builder = MeshBuilder::new(PrimitiveTopology::TriangleList);
        
        // Generate vertices
        for lat in 0..=lat_segments {
            let theta = lat as f32 * PI / lat_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();
            
            for lon in 0..=long_segments {
                let phi = lon as f32 * TAU / long_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();
                
                let x = cos_phi * sin_theta;
                let y = cos_theta;
                let z = sin_phi * sin_theta;
                
                let position = Vec3::new(x, y, z) * radius;
                let normal = Vec3::new(x, y, z);
                let uv = Vec2::new(lon as f32 / long_segments as f32, lat as f32 / lat_segments as f32);
                
                let vertex = SimpleVertex { position, normal, uv };
                builder.add_vertex(vertex.into());
            }
        }
        
        // Generate indices
        for lat in 0..lat_segments {
            for lon in 0..long_segments {
                let current = lat * (long_segments + 1) + lon;
                let next = current + long_segments + 1;
                
                builder.add_triangle(current, next, current + 1);
                builder.add_triangle(current + 1, next, next + 1);
            }
        }
        
        builder.build()
    }
    
    /// Generate cube with proper face normals
    pub fn cube<V: From<SimpleVertex> + Vertex>(size: f32) -> Mesh<V> {
        let half = size * 0.5;
        let mut builder = MeshBuilder::new(PrimitiveTopology::TriangleList);
        
        // Define cube faces with normals and UVs
        let faces = [
            // Front (+Z)
            ([
                SimpleVertex { position: Vec3::new(-half, -half,  half), normal: Vec3::Z, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new( half, -half,  half), normal: Vec3::Z, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new( half,  half,  half), normal: Vec3::Z, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new(-half,  half,  half), normal: Vec3::Z, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
            // Back (-Z)
            ([
                SimpleVertex { position: Vec3::new( half, -half, -half), normal: Vec3::NEG_Z, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new(-half, -half, -half), normal: Vec3::NEG_Z, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new(-half,  half, -half), normal: Vec3::NEG_Z, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new( half,  half, -half), normal: Vec3::NEG_Z, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
            // Right (+X)
            ([
                SimpleVertex { position: Vec3::new( half, -half,  half), normal: Vec3::X, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new( half, -half, -half), normal: Vec3::X, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new( half,  half, -half), normal: Vec3::X, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new( half,  half,  half), normal: Vec3::X, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
            // Left (-X)
            ([
                SimpleVertex { position: Vec3::new(-half, -half, -half), normal: Vec3::NEG_X, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new(-half, -half,  half), normal: Vec3::NEG_X, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new(-half,  half,  half), normal: Vec3::NEG_X, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new(-half,  half, -half), normal: Vec3::NEG_X, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
            // Top (+Y)
            ([
                SimpleVertex { position: Vec3::new(-half,  half,  half), normal: Vec3::Y, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new( half,  half,  half), normal: Vec3::Y, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new( half,  half, -half), normal: Vec3::Y, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new(-half,  half, -half), normal: Vec3::Y, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
            // Bottom (-Y)
            ([
                SimpleVertex { position: Vec3::new(-half, -half, -half), normal: Vec3::NEG_Y, uv: Vec2::new(0.0, 0.0) },
                SimpleVertex { position: Vec3::new( half, -half, -half), normal: Vec3::NEG_Y, uv: Vec2::new(1.0, 0.0) },
                SimpleVertex { position: Vec3::new( half, -half,  half), normal: Vec3::NEG_Y, uv: Vec2::new(1.0, 1.0) },
                SimpleVertex { position: Vec3::new(-half, -half,  half), normal: Vec3::NEG_Y, uv: Vec2::new(0.0, 1.0) },
            ], [0, 1, 2, 2, 3, 0]),
        ];
        
        for (face_verts, face_indices) in &faces {
            let converted: Vec<V> = face_verts.iter().map(|v| (*v).into()).collect();
            let vert_indices = builder.add_vertices(&converted);
            for &idx in face_indices {
                builder.indices.push(vert_indices[idx]);
            }
        }

        builder.build()
    }
    
    /// Generate plane with customizable subdivision
    pub fn plane<V: From<SimpleVertex> + Vertex>(width: f32, height: f32, w_segments: u32, h_segments: u32) -> Mesh<V> {
        let mut builder = MeshBuilder::new(PrimitiveTopology::TriangleList);
        
        for y in 0..=h_segments {
            for x in 0..=w_segments {
                let u = x as f32 / w_segments as f32;
                let v = y as f32 / h_segments as f32;
                
                let position = Vec3::new(
                    (u - 0.5) * width,
                    0.0,
                    (v - 0.5) * height
                );
                
                let vertex = SimpleVertex {
                    position,
                    normal: Vec3::Y,
                    uv: Vec2::new(u, v),
                };
                
                builder.add_vertex(vertex.into());
            }
        }
        
        // Generate indices for quads
        for y in 0..h_segments {
            for x in 0..w_segments {
                let i = y * (w_segments + 1) + x;
                let next_row = i + w_segments + 1;
                
                builder.add_quad(i, i + 1, next_row + 1, next_row);
            }
        }
        
        builder.build()
    }
    
    /// Generate cylinder with customizable resolution  
    pub fn cylinder<V: From<SimpleVertex> + Vertex>(radius: f32, height: f32, segments: u32) -> Mesh<V> {
        let mut builder = MeshBuilder::new(PrimitiveTopology::TriangleList);
        let half_height = height * 0.5;
        
        // Generate side vertices
        for i in 0..=segments {
            let theta = i as f32 * TAU / segments as f32;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();
            
            let x = cos_theta * radius;
            let z = sin_theta * radius;
            let normal = Vec3::new(cos_theta, 0.0, sin_theta);
            let u = i as f32 / segments as f32;
            
            // Bottom vertex
            let bottom = SimpleVertex {
                position: Vec3::new(x, -half_height, z),
                normal,
                uv: Vec2::new(u, 0.0),
            };
            
            // Top vertex  
            let top = SimpleVertex {
                position: Vec3::new(x, half_height, z),
                normal,
                uv: Vec2::new(u, 1.0),
            };
            
            builder.add_vertex(bottom.into());
            builder.add_vertex(top.into());
        }
        
        // Generate side indices
        for i in 0..segments {
            let bottom_curr = i * 2;
            let top_curr = bottom_curr + 1;
            let bottom_next = ((i + 1) % segments) * 2;
            let top_next = bottom_next + 1;
            
            builder.add_quad(bottom_curr, bottom_next, top_next, top_curr);
        }
        
        // TODO: Add end caps
        
        builder.build()
    }
}

// ============================================================================
// TEXTURE GENERATION  
// ============================================================================

/// Texture format specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8,
    RG8, 
    RGB8,
    RGBA8,
    R16F,
    RG16F,
    RGB16F,
    RGBA16F,
    R32F,
    RG32F,
    RGB32F,
    RGBA32F,
}

impl TextureFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::RG8 => 2,
            TextureFormat::RGB8 => 3,
            TextureFormat::RGBA8 => 4,
            TextureFormat::R16F => 2,
            TextureFormat::RG16F => 4,
            TextureFormat::RGB16F => 6,
            TextureFormat::RGBA16F => 8,
            TextureFormat::R32F => 4,
            TextureFormat::RG32F => 8,
            TextureFormat::RGB32F => 12,
            TextureFormat::RGBA32F => 16,
        }
    }
    
    pub fn channel_count(&self) -> u32 {
        match self {
            TextureFormat::R8 | TextureFormat::R16F | TextureFormat::R32F => 1,
            TextureFormat::RG8 | TextureFormat::RG16F | TextureFormat::RG32F => 2,
            TextureFormat::RGB8 | TextureFormat::RGB16F | TextureFormat::RGB32F => 3,
            TextureFormat::RGBA8 | TextureFormat::RGBA16F | TextureFormat::RGBA32F => 4,
        }
    }
}

/// 2D texture container
#[derive(Debug, Clone)]
pub struct Texture2D {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub data: Vec<u8>,
}

impl Texture2D {
    pub fn new(width: u32, height: u32, format: TextureFormat) -> Self {
        let size = (width * height) as usize * format.bytes_per_pixel();
        Self {
            width,
            height, 
            format,
            data: vec![0; size],
        }
    }
    
    /// Sample texture at normalized coordinates (bilinear filtering)
    pub fn sample(&self, u: f32, v: f32) -> Vec4 {
        let x = (u * self.width as f32).clamp(0.0, self.width as f32 - 1.0);
        let y = (v * self.height as f32).clamp(0.0, self.height as f32 - 1.0);
        
        // Simple nearest neighbor for now
        let px = x as u32;
        let py = y as u32;
        
        self.get_pixel(px, py)
    }
    
    /// Get pixel value as normalized Vec4
    pub fn get_pixel(&self, x: u32, y: u32) -> Vec4 {
        if x >= self.width || y >= self.height {
            return Vec4::ZERO;
        }
        
        let index = ((y * self.width + x) as usize) * self.format.bytes_per_pixel();
        
        match self.format {
            TextureFormat::RGBA8 => {
                let r = self.data[index] as f32 / 255.0;
                let g = self.data[index + 1] as f32 / 255.0;
                let b = self.data[index + 2] as f32 / 255.0;
                let a = self.data[index + 3] as f32 / 255.0;
                Vec4::new(r, g, b, a)
            }
            TextureFormat::RGB8 => {
                let r = self.data[index] as f32 / 255.0;
                let g = self.data[index + 1] as f32 / 255.0;
                let b = self.data[index + 2] as f32 / 255.0;
                Vec4::new(r, g, b, 1.0)
            }
            // TODO: Implement other formats
            _ => Vec4::ZERO,
        }
    }
    
    /// Set pixel value from normalized Vec4
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Vec4) {
        if x >= self.width || y >= self.height {
            return;
        }
        
        let index = ((y * self.width + x) as usize) * self.format.bytes_per_pixel();
        
        match self.format {
            TextureFormat::RGBA8 => {
                self.data[index] = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                self.data[index + 1] = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                self.data[index + 2] = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
                self.data[index + 3] = (color.w.clamp(0.0, 1.0) * 255.0) as u8;
            }
            TextureFormat::RGB8 => {
                self.data[index] = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                self.data[index + 1] = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                self.data[index + 2] = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
            }
            // TODO: Implement other formats
            _ => {}
        }
    }
}

/// Procedural texture generators
pub struct TextureGenerator;

impl TextureGenerator {
    /// Generate solid color texture
    pub fn solid_color(width: u32, height: u32, color: Vec4) -> Texture2D {
        let mut texture = Texture2D::new(width, height, TextureFormat::RGBA8);
        
        for y in 0..height {
            for x in 0..width {
                texture.set_pixel(x, y, color);
            }
        }
        
        texture
    }
    
    /// Generate checkerboard pattern
    pub fn checkerboard(width: u32, height: u32, checker_size: u32, color1: Vec4, color2: Vec4) -> Texture2D {
        let mut texture = Texture2D::new(width, height, TextureFormat::RGBA8);
        
        for y in 0..height {
            for x in 0..width {
                let checker_x = (x / checker_size) % 2;
                let checker_y = (y / checker_size) % 2;
                let color = if (checker_x + checker_y) % 2 == 0 { color1 } else { color2 };
                texture.set_pixel(x, y, color);
            }
        }
        
        texture
    }
    
    /// Generate gradient texture
    pub fn gradient(width: u32, height: u32, start_color: Vec4, end_color: Vec4, horizontal: bool) -> Texture2D {
        let mut texture = Texture2D::new(width, height, TextureFormat::RGBA8);
        
        for y in 0..height {
            for x in 0..width {
                let t = if horizontal {
                    x as f32 / (width - 1) as f32
                } else {
                    y as f32 / (height - 1) as f32
                };
                
                let color = start_color.lerp(end_color, t);
                texture.set_pixel(x, y, color);
            }
        }
        
        texture
    }
    
    /// Generate Perlin noise texture  
    pub fn perlin_noise(width: u32, height: u32, scale: f32, octaves: u32) -> Texture2D {
        let mut texture = Texture2D::new(width, height, TextureFormat::RGBA8);
        
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32 * scale;
                let fy = y as f32 / height as f32 * scale;
                
                let mut noise_value = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut max_value = 0.0;
                
                for _ in 0..octaves {
                    noise_value += simple_noise(fx * frequency, fy * frequency) * amplitude;
                    max_value += amplitude;
                    amplitude *= 0.5;
                    frequency *= 2.0;
                }
                
                noise_value /= max_value;
                let normalized = (noise_value + 1.0) * 0.5; // [-1,1] -> [0,1]
                
                let color = Vec4::new(normalized, normalized, normalized, 1.0);
                texture.set_pixel(x, y, color);
            }
        }
        
        texture
    }
    
    /// Generate normal map from height data
    pub fn normal_map_from_height(height_map: &Texture2D, strength: f32) -> Texture2D {
        let mut normal_map = Texture2D::new(height_map.width, height_map.height, TextureFormat::RGBA8);
        
        for y in 0..height_map.height {
            for x in 0..height_map.width {
                // Sample neighboring pixels for gradient calculation
                let left = if x > 0 { height_map.get_pixel(x - 1, y).x } else { height_map.get_pixel(x, y).x };
                let right = if x < height_map.width - 1 { height_map.get_pixel(x + 1, y).x } else { height_map.get_pixel(x, y).x };
                let up = if y > 0 { height_map.get_pixel(x, y - 1).x } else { height_map.get_pixel(x, y).x };
                let down = if y < height_map.height - 1 { height_map.get_pixel(x, y + 1).x } else { height_map.get_pixel(x, y).x };
                
                // Calculate gradient
                let dx = (right - left) * strength;
                let dy = (down - up) * strength;
                
                // Calculate normal
                let normal = Vec3::new(-dx, -dy, 1.0).normalize();
                
                // Pack normal into [0,1] range
                let packed = (normal + 1.0) * 0.5;
                let color = Vec4::new(packed.x, packed.y, packed.z, 1.0);
                
                normal_map.set_pixel(x, y, color);
            }
        }
        
        normal_map
    }
}

// Simple noise function (replace with proper Perlin/Simplex noise if needed)
fn simple_noise(x: f32, y: f32) -> f32 {
    let n = ((x * 12.9898 + y * 78.233).sin() * 43758.5453).fract();
    (n - 0.5) * 2.0 // [-1, 1]
}

// ============================================================================
// ASSET COMBINATIONS AND PIPELINES
// ============================================================================

/// Asset bundle for common rendering scenarios
#[derive(Debug)]
pub struct AssetBundle<V: Vertex> {
    pub mesh: Mesh<V>,
    pub textures: Vec<Texture2D>,
}

/// PBR material asset generator
pub struct PbrAssets;

impl PbrAssets {
    /// Generate complete PBR material set
    pub fn generate_material_set(
        base_color: Vec4,
        metallic: f32,
        roughness: f32,
        texture_size: u32,
    ) -> (Texture2D, Texture2D, Texture2D, Texture2D) {
        // Albedo map with subtle color variation
        let mut albedo = TextureGenerator::solid_color(texture_size, texture_size, base_color);
        
        // Add some noise variation to albedo
        let noise = TextureGenerator::perlin_noise(texture_size, texture_size, 4.0, 3);
        for y in 0..texture_size {
            for x in 0..texture_size {
                let base = albedo.get_pixel(x, y);
                let noise_val = noise.get_pixel(x, y).x * 0.1; // Small variation
                let varied = base + Vec4::splat(noise_val);
                albedo.set_pixel(x, y, varied);
            }
        }
        
        // Metallic-Roughness map (R = metallic, G = roughness)
        let mut metallic_roughness = Texture2D::new(texture_size, texture_size, TextureFormat::RGBA8);
        for y in 0..texture_size {
            for x in 0..texture_size {
                let color = Vec4::new(metallic, roughness, 0.0, 1.0);
                metallic_roughness.set_pixel(x, y, color);
            }
        }
        
        // Normal map from noise
        let height_noise = TextureGenerator::perlin_noise(texture_size, texture_size, 8.0, 4);
        let normal = TextureGenerator::normal_map_from_height(&height_noise, 0.5);
        
        // AO map (simple radial gradient)
        let mut ao = Texture2D::new(texture_size, texture_size, TextureFormat::RGBA8);
        let center = texture_size as f32 * 0.5;
        for y in 0..texture_size {
            for x in 0..texture_size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let distance = (dx * dx + dy * dy).sqrt() / center;
                let ao_value = 1.0 - (distance * 0.3).min(1.0);
                let color = Vec4::new(ao_value, ao_value, ao_value, 1.0);
                ao.set_pixel(x, y, color);
            }
        }
        
        (albedo, normal, metallic_roughness, ao)
    }
}

// ============================================================================
// TRAIT DEFINITIONS FOR DERIVE SUPPORT
// ============================================================================

/// Trait for types that provide procedural meshes
pub trait MeshProvider {
    type Vertex: Vertex;
    fn mesh() -> &'static Mesh<Self::Vertex>;
}

/// Trait for types that provide procedural textures
pub trait TextureProvider {
    fn texture() -> &'static Texture2D;
}

/// Trait for asset bundle types
pub trait BundleProvider {
    fn asset_count() -> usize;
}

/// Mesh transformation utilities for derive macros
pub mod transform {
    use super::*;
    use glam::{Vec3, Vec4, Mat4};
    
    /// Apply translation transform to mesh vertices
    pub fn translate_mesh<V: From<SimpleVertex> + Vertex>(mesh: Mesh<SimpleVertex>, translation: Vec3) -> Mesh<V> {
        let transformed_vertices: Vec<V> = mesh.vertices.into_iter().map(|mut v| {
            v.position += translation;
            v.into()
        }).collect();
        
        Mesh {
            vertices: transformed_vertices,
            indices: mesh.indices,
            primitive_topology: mesh.primitive_topology,
        }
    }
    
    /// Apply rotation transform to mesh vertices and normals
    pub fn rotate_mesh<V: From<SimpleVertex> + Vertex>(mesh: Mesh<SimpleVertex>, euler_angles: Vec3) -> Mesh<V> {
        let rotation_matrix = Mat4::from_euler(glam::EulerRot::XYZ, euler_angles.x, euler_angles.y, euler_angles.z);
        
        let transformed_vertices: Vec<V> = mesh.vertices.into_iter().map(|mut v| {
            let pos_4 = Vec4::new(v.position.x, v.position.y, v.position.z, 1.0);
            let normal_4 = Vec4::new(v.normal.x, v.normal.y, v.normal.z, 0.0);
            
            v.position = (rotation_matrix * pos_4).truncate();
            v.normal = (rotation_matrix * normal_4).truncate().normalize();
            v.into()
        }).collect();
        
        Mesh {
            vertices: transformed_vertices,
            indices: mesh.indices,
            primitive_topology: mesh.primitive_topology,
        }
    }
    
    /// Apply scale transform to mesh vertices
    pub fn scale_mesh<V: From<SimpleVertex> + Vertex>(mesh: Mesh<SimpleVertex>, scale: Vec3) -> Mesh<V> {
        let transformed_vertices: Vec<V> = mesh.vertices.into_iter().map(|mut v| {
            v.position *= scale;
            // Note: normals should be scaled by inverse transpose for non-uniform scaling
            if scale.x != scale.y || scale.y != scale.z {
                v.normal = (v.normal / scale).normalize();
            }
            v.into()
        }).collect();
        
        Mesh {
            vertices: transformed_vertices,
            indices: mesh.indices,
            primitive_topology: mesh.primitive_topology,
        }
    }
}

// ============================================================================
// UTILITY EXTENSIONS 
// ============================================================================

/// Add tangent calculation to meshes for normal mapping
impl Mesh<SimpleVertex> {
    /// Convert to PBR vertex format with calculated tangents
    pub fn with_tangents(self) -> Mesh<PbrVertex> {
        let mut pbr_vertices = Vec::with_capacity(self.vertices.len());
        
        // Initialize all tangents to zero
        let mut tangents = vec![Vec3::ZERO; self.vertices.len()];
        let mut bitangents = vec![Vec3::ZERO; self.vertices.len()];
        
        // Calculate tangents for each triangle
        for chunk in self.indices.chunks(3) {
            if chunk.len() != 3 { continue; }
            
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;
            
            let v0 = &self.vertices[i0];
            let v1 = &self.vertices[i1];
            let v2 = &self.vertices[i2];
            
            let delta_pos1 = v1.position - v0.position;
            let delta_pos2 = v2.position - v0.position;
            
            let delta_uv1 = v1.uv - v0.uv;
            let delta_uv2 = v2.uv - v0.uv;
            
            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;
            
            tangents[i0] += tangent;
            tangents[i1] += tangent;
            tangents[i2] += tangent;
            
            bitangents[i0] += bitangent;
            bitangents[i1] += bitangent;
            bitangents[i2] += bitangent;
        }
        
        // Convert vertices with calculated tangents
        for (i, vertex) in self.vertices.iter().enumerate() {
            let normal = vertex.normal;
            let mut tangent = tangents[i];
            
            // Gram-Schmidt orthogonalization
            tangent = (tangent - normal * normal.dot(tangent)).normalize();
            
            // Calculate handedness
            let handedness = if normal.cross(tangent).dot(bitangents[i]) < 0.0 { -1.0 } else { 1.0 };
            
            let pbr_vertex = PbrVertex {
                position: vertex.position,
                normal: vertex.normal,
                tangent: Vec4::new(tangent.x, tangent.y, tangent.z, handedness),
                uv: vertex.uv,
            };
            
            pbr_vertices.push(pbr_vertex);
        }
        
        Mesh {
            vertices: pbr_vertices,
            indices: self.indices,
            primitive_topology: self.primitive_topology,
        }
    }
}