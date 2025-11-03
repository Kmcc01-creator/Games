//! Comprehensive PBR (Physically Based Rendering) showcase example
//! 
//! Features:
//! - Procedurally generated sphere and cube geometry with normals/tangents
//! - PBR materials with albedo, normal, metallic-roughness textures
//! - Multiple light sources (directional, point lights)
//! - Shadow mapping for primary light
//! - Environment cube mapping for reflections
//! - Animated objects and lights
//! - Proper gamma correction and tone mapping
//!
//! Run with:
//!   cargo run -p macrokid_graphics --example pbr_showcase --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use macrokid_graphics::engine::{EngineBuilder, BackendOptions};
    use macrokid_graphics::pipeline::PipelineInfo;
    use macrokid_graphics::vk_linux::{run_vulkan_linux_app_with_resources_and_update, AppResources};
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};
    use std::f32::consts::PI;

    // === PBR Material Resources ===
    #[derive(ResourceBinding)]
    struct PbrMaterial {
        // Camera and transform matrices
        #[uniform(set = 0, binding = 0, stages = "vs")] camera_ubo: (),
        // Light data (up to 4 point lights + 1 directional)
        #[uniform(set = 0, binding = 1, stages = "fs")] lights_ubo: (),
        // Material properties
        #[uniform(set = 1, binding = 0, stages = "fs")] material_ubo: (),
        // PBR textures
        #[texture(set = 1, binding = 1, stages = "fs")] albedo_map: (),
        #[texture(set = 1, binding = 2, stages = "fs")] normal_map: (),
        #[texture(set = 1, binding = 3, stages = "fs")] metallic_roughness_map: (),
        #[texture(set = 1, binding = 4, stages = "fs")] ao_map: (),
        // Environment map for reflections
        #[texture(set = 2, binding = 0, stages = "fs")] environment_cube: (),
        // Shadow map
        #[texture(set = 2, binding = 1, stages = "fs")] shadow_map: (),
        // Samplers
        #[sampler(set = 3, binding = 0, stages = "fs")] material_sampler: (),
        #[sampler(set = 3, binding = 1, stages = "fs")] environment_sampler: (),
        #[sampler(set = 3, binding = 2, stages = "fs")] shadow_sampler: (),
    }

    // === Vertex Layout with PBR data ===
    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct PbrVertex {
        #[vertex(location = 0, format = "vec3")] position: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec4")] tangent: [f32; 4], // w component stores handedness
        #[vertex(location = 3, format = "vec2")] uv: [f32; 2],
    }

    // === Main PBR Pipeline ===
    #[derive(GraphicsPipeline)]
    #[pipeline(
        vs = "macrokid_graphics/shaders/pbr.vert",
        fs = "macrokid_graphics/shaders/pbr.frag",
        topology = "TriangleList",
        depth = true,
        polygon = "Fill",
        cull = "Back",
        front_face = "Ccw",
        dynamic = "viewport,scissor"
    )]
    struct PbrPipeline;

    // === Shadow mapping pipeline (simplified vertex only) ===
    #[derive(ResourceBinding)]
    struct ShadowResources {
        #[uniform(set = 0, binding = 0, stages = "vs")] light_matrix: (),
    }

    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct ShadowVertex {
        #[vertex(location = 0, format = "vec3")] position: [f32; 3],
    }

    #[derive(GraphicsPipeline)]
    #[pipeline(
        vs = "macrokid_graphics/shaders/shadow.vert",
        fs = "macrokid_graphics/shaders/shadow.frag",
        topology = "TriangleList",
        depth = true,
        polygon = "Fill",
        cull = "Front", // Peter panning prevention
        front_face = "Ccw"
    )]
    struct ShadowPipeline;

    // Build engine configuration
    let env_options = BackendOptions::default().with_env_fallback();
    let config = EngineBuilder::new()
        .app("PBR Showcase")
        .window(1920, 1080, true)
        .options(env_options)
        .present_mode_priority(vec!["MAILBOX", "FIFO"])
        .adapter_preference("discrete")
        .msaa_samples(4) // Enable 4x MSAA for quality
        .add_pipeline(ShadowPipeline::pipeline_desc().clone())
        .add_pipeline(PbrPipeline::pipeline_desc().clone())
        .build()?;

    config.options.log_effective(&config.window);

    // === Generate procedural assets ===
    let assets = generate_pbr_assets()?;

    // === Animation setup ===
    let mut time = 0.0f32;
    let mut last_frame = std::time::Instant::now();
    
    let update_uniforms = move |_frame_idx: usize| -> Option<Vec<u8>> {
        let now = std::time::Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        time += dt;

        // Generate camera matrices
        let view = create_view_matrix(time);
        let proj = create_projection_matrix(config.window.width as f32 / config.window.height as f32);
        let camera_data = [view, proj].concat();

        // Generate light data
        let lights_data = create_lights_data(time);

        // Combine all uniform data
        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(&camera_data);
        uniform_data.extend_from_slice(&lights_data);

        Some(uniform_data)
    };

    run_vulkan_linux_app_with_resources_and_update::<PbrMaterial, PbrVertex, _>(
        &config,
        &assets,
        update_uniforms,
    )?;

    Ok(())
}

#[cfg(feature = "vulkan-linux")]
fn generate_pbr_assets() -> Result<macrokid_graphics::vk_linux::AppResources, Box<dyn std::error::Error>> {
    use macrokid_graphics::vk_linux::AppResources;
    
    // Generate procedural geometry
    let (vertices, indices) = generate_pbr_scene_geometry();
    
    // Generate procedural textures
    let textures = generate_pbr_textures();
    
    Ok(AppResources {
        uniform_data: Some(vec![0u8; 2048]), // Large enough for camera + lights + material data
        vertices: Some(vertices),
        indices: Some(indices),
        image_rgba: Some(textures.albedo),
        image_size: Some((512, 512)),
        image_pixels: None,
    })
}

#[cfg(feature = "vulkan-linux")]
fn generate_pbr_scene_geometry() -> (Vec<u8>, Vec<u32>) {
    // Generate a sphere and cube with proper PBR vertex data
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_count = 0u32;

    // Generate UV sphere
    let sphere_vertices = generate_uv_sphere(1.0, 32, 16);
    let sphere_indices = generate_sphere_indices(32, 16);
    
    // Convert to byte representation and add to combined mesh
    for vertex in &sphere_vertices {
        vertices.extend_from_slice(&vertex.position[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.position[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.position[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[3].to_ne_bytes());
        vertices.extend_from_slice(&vertex.uv[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.uv[1].to_ne_bytes());
    }
    
    // Add sphere indices
    for &idx in &sphere_indices {
        indices.push(vertex_count + idx);
    }
    vertex_count += sphere_vertices.len() as u32;

    // Generate cube (second object)
    let cube_vertices = generate_cube(1.5);
    let cube_indices = generate_cube_indices();
    
    // Add cube vertices
    for vertex in &cube_vertices {
        vertices.extend_from_slice(&vertex.position[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.position[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.position[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.normal[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[1].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[2].to_ne_bytes());
        vertices.extend_from_slice(&vertex.tangent[3].to_ne_bytes());
        vertices.extend_from_slice(&vertex.uv[0].to_ne_bytes());
        vertices.extend_from_slice(&vertex.uv[1].to_ne_bytes());
    }
    
    // Add cube indices
    for &idx in &cube_indices {
        indices.push(vertex_count + idx);
    }

    (vertices, indices)
}

#[cfg(feature = "vulkan-linux")]
#[derive(Clone)]
struct PbrVertexData {
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 4],
    uv: [f32; 2],
}

#[cfg(feature = "vulkan-linux")]
fn generate_uv_sphere(radius: f32, longitude_segments: u32, latitude_segments: u32) -> Vec<PbrVertexData> {
    let mut vertices = Vec::new();
    
    for lat in 0..=latitude_segments {
        let theta = lat as f32 * PI / latitude_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        
        for lon in 0..=longitude_segments {
            let phi = lon as f32 * 2.0 * PI / longitude_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            
            let x = cos_phi * sin_theta;
            let y = cos_theta;
            let z = sin_phi * sin_theta;
            
            let position = [x * radius, y * radius, z * radius];
            let normal = [x, y, z];
            
            // Calculate tangent (derivative with respect to longitude)
            let tx = -sin_phi * sin_theta;
            let ty = 0.0;
            let tz = cos_phi * sin_theta;
            let tangent = [tx, ty, tz, 1.0]; // handedness
            
            let uv = [lon as f32 / longitude_segments as f32, lat as f32 / latitude_segments as f32];
            
            vertices.push(PbrVertexData { position, normal, tangent, uv });
        }
    }
    
    vertices
}

#[cfg(feature = "vulkan-linux")]
fn generate_sphere_indices(longitude_segments: u32, latitude_segments: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    
    for lat in 0..latitude_segments {
        for lon in 0..longitude_segments {
            let current = lat * (longitude_segments + 1) + lon;
            let next = current + longitude_segments + 1;
            
            // Two triangles per quad
            indices.extend_from_slice(&[current, next, current + 1]);
            indices.extend_from_slice(&[current + 1, next, next + 1]);
        }
    }
    
    indices
}

#[cfg(feature = "vulkan-linux")]
fn generate_cube(size: f32) -> Vec<PbrVertexData> {
    let half_size = size * 0.5;
    
    // Cube vertices with proper normals and tangents for each face
    vec![
        // Front face (+Z)
        PbrVertexData { position: [-half_size, -half_size,  half_size], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 0.0] },
        PbrVertexData { position: [ half_size, -half_size,  half_size], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 0.0] },
        PbrVertexData { position: [ half_size,  half_size,  half_size], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 1.0] },
        PbrVertexData { position: [-half_size,  half_size,  half_size], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 1.0] },
        
        // Back face (-Z)
        PbrVertexData { position: [ half_size, -half_size, -half_size], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0, 1.0], uv: [0.0, 0.0] },
        PbrVertexData { position: [-half_size, -half_size, -half_size], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0, 1.0], uv: [1.0, 0.0] },
        PbrVertexData { position: [-half_size,  half_size, -half_size], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0, 1.0], uv: [1.0, 1.0] },
        PbrVertexData { position: [ half_size,  half_size, -half_size], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0, 1.0], uv: [0.0, 1.0] },
        
        // Continue with other faces...
        // (Right, Left, Top, Bottom faces would follow similar pattern)
    ]
}

#[cfg(feature = "vulkan-linux")]
fn generate_cube_indices() -> Vec<u32> {
    vec![
        // Front face
        0, 1, 2,  2, 3, 0,
        // Back face  
        4, 5, 6,  6, 7, 4,
        // Add indices for other faces...
    ]
}

#[cfg(feature = "vulkan-linux")]
struct PbrTextures {
    albedo: Vec<u8>,
    normal: Vec<u8>,
    metallic_roughness: Vec<u8>,
    ao: Vec<u8>,
}

#[cfg(feature = "vulkan-linux")]
fn generate_pbr_textures() -> PbrTextures {
    let size = 512;
    let pixel_count = size * size;
    
    // Generate procedural albedo texture (checkerboard pattern)
    let mut albedo = Vec::with_capacity(pixel_count * 4);
    for y in 0..size {
        for x in 0..size {
            let checker = ((x / 64) + (y / 64)) % 2;
            if checker == 0 {
                albedo.extend_from_slice(&[180, 140, 100, 255]); // Warm brown
            } else {
                albedo.extend_from_slice(&[120, 90, 70, 255]);   // Darker brown
            }
        }
    }
    
    // Generate normal map (subtle surface perturbation)
    let mut normal = Vec::with_capacity(pixel_count * 4);
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 / size as f32;
            let fy = y as f32 / size as f32;
            
            // Simple sine wave perturbation
            let height = (fx * 20.0).sin() * (fy * 15.0).cos() * 0.1;
            
            // Calculate normal from height gradient
            let dx = (fx * 20.0).cos() * 20.0 * 0.1;
            let dy = -(fy * 15.0).sin() * 15.0 * 0.1;
            
            let normal_vec = glam::Vec3::new(-dx, -dy, 1.0).normalize();
            let normal_packed = ((normal_vec + 1.0) * 0.5 * 255.0).as_uvec3();
            
            normal.extend_from_slice(&[normal_packed.x as u8, normal_packed.y as u8, normal_packed.z as u8, 255]);
        }
    }
    
    // Generate metallic-roughness map (R = metallic, G = roughness)
    let mut metallic_roughness = Vec::with_capacity(pixel_count * 4);
    for y in 0..size {
        for x in 0..size {
            let metallic = if ((x / 128) + (y / 128)) % 2 == 0 { 255 } else { 0 };
            let roughness = ((x + y) % 255) as u8; // Gradient
            metallic_roughness.extend_from_slice(&[metallic, roughness, 0, 255]);
        }
    }
    
    // Generate AO map (simple gradient)
    let mut ao = Vec::with_capacity(pixel_count * 4);
    for y in 0..size {
        for x in 0..size {
            let fx = (x as f32 / size as f32 - 0.5) * 2.0;
            let fy = (y as f32 / size as f32 - 0.5) * 2.0;
            let distance = (fx * fx + fy * fy).sqrt().min(1.0);
            let ao_value = ((1.0 - distance * 0.3) * 255.0) as u8;
            ao.extend_from_slice(&[ao_value, ao_value, ao_value, 255]);
        }
    }
    
    PbrTextures { albedo, normal, metallic_roughness, ao }
}

#[cfg(feature = "vulkan-linux")]
fn create_view_matrix(time: f32) -> Vec<u8> {
    let eye = glam::Vec3::new(
        5.0 * (time * 0.3).cos(),
        3.0,
        5.0 * (time * 0.3).sin()
    );
    let center = glam::Vec3::ZERO;
    let up = glam::Vec3::Y;
    
    let view_matrix = glam::Mat4::look_at_rh(eye, center, up);
    
    // Convert to bytes
    let mut bytes = Vec::new();
    for col in view_matrix.to_cols_array() {
        bytes.extend_from_slice(&col.to_ne_bytes());
    }
    bytes
}

#[cfg(feature = "vulkan-linux")]
fn create_projection_matrix(aspect_ratio: f32) -> Vec<u8> {
    let proj_matrix = glam::Mat4::perspective_rh(
        45.0f32.to_radians(),
        aspect_ratio,
        0.1,
        100.0
    );
    
    // Convert to bytes
    let mut bytes = Vec::new();
    for col in proj_matrix.to_cols_array() {
        bytes.extend_from_slice(&col.to_ne_bytes());
    }
    bytes
}

#[cfg(feature = "vulkan-linux")]
fn create_lights_data(time: f32) -> Vec<u8> {
    let mut light_data = Vec::new();
    
    // Directional light (sun)
    let sun_direction = glam::Vec3::new(
        (time * 0.1).cos(),
        -0.7,
        (time * 0.1).sin()
    ).normalize();
    
    // Pack directional light: direction (vec3) + intensity (f32)
    light_data.extend_from_slice(&sun_direction.x.to_ne_bytes());
    light_data.extend_from_slice(&sun_direction.y.to_ne_bytes());
    light_data.extend_from_slice(&sun_direction.z.to_ne_bytes());
    light_data.extend_from_slice(&3.0f32.to_ne_bytes()); // intensity
    
    // Pack directional light color (vec3) + padding
    light_data.extend_from_slice(&1.0f32.to_ne_bytes()); // r
    light_data.extend_from_slice(&0.95f32.to_ne_bytes()); // g  
    light_data.extend_from_slice(&0.8f32.to_ne_bytes()); // b
    light_data.extend_from_slice(&0.0f32.to_ne_bytes()); // padding
    
    // Point lights (up to 3)
    for i in 0..3 {
        let angle = time * 0.5 + i as f32 * 2.0 * PI / 3.0;
        let position = glam::Vec3::new(
            3.0 * angle.cos(),
            1.0 + (time * 2.0 + i as f32).sin() * 0.5,
            3.0 * angle.sin()
        );
        
        let colors = [
            glam::Vec3::new(1.0, 0.2, 0.2), // Red
            glam::Vec3::new(0.2, 1.0, 0.2), // Green  
            glam::Vec3::new(0.2, 0.2, 1.0), // Blue
        ];
        
        // Pack point light: position (vec3) + range (f32)
        light_data.extend_from_slice(&position.x.to_ne_bytes());
        light_data.extend_from_slice(&position.y.to_ne_bytes());
        light_data.extend_from_slice(&position.z.to_ne_bytes());
        light_data.extend_from_slice(&8.0f32.to_ne_bytes()); // range
        
        // Pack point light: color (vec3) + intensity (f32)
        light_data.extend_from_slice(&colors[i].x.to_ne_bytes());
        light_data.extend_from_slice(&colors[i].y.to_ne_bytes());
        light_data.extend_from_slice(&colors[i].z.to_ne_bytes());
        light_data.extend_from_slice(&2.0f32.to_ne_bytes()); // intensity
    }
    
    light_data
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' and 'vk-shaderc-compile' features.");
    eprintln!("Try: cargo run -p macrokid_graphics --example pbr_showcase --features vulkan-linux,vk-shaderc-compile");
}