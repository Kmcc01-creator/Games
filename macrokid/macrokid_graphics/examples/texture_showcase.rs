//! Texture Generation Showcase
//!
//! This example demonstrates the texture generation capabilities:
//! - Solid colors, gradients, checkerboards
//! - Perlin noise textures  
//! - Normal maps generated from height data
//! - PBR material texture sets
//! - Multiple objects with different procedural textures
//!
//! Run with:
//!   cargo run -p macrokid_graphics --example texture_showcase --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use macrokid_graphics::engine::{EngineBuilder, BackendOptions};
    use macrokid_graphics::pipeline::PipelineInfo;
    use macrokid_graphics::vk_linux::{run_vulkan_linux_app_with_resources_and_update, AppResources};
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};
    use macrokid_graphics::assets::*;
    use glam::{Vec3, Vec4, Mat4};

    // === Simple vertex layout for texture showcase ===
    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct TextureVertex {
        #[vertex(location = 0, format = "vec3")] position: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

    impl From<SimpleVertex> for TextureVertex {
        fn from(v: SimpleVertex) -> Self {
            Self {
                position: v.position.to_array(),
                normal: v.normal.to_array(),
                uv: v.uv.to_array(),
            }
        }
    }

    // === Simple material for texture display ===
    #[derive(ResourceBinding)]
    struct TextureMaterial {
        #[uniform(set = 0, binding = 0, stages = "vs")] camera_ubo: (),
        #[uniform(set = 0, binding = 1, stages = "fs")] light_ubo: (),
        #[texture(set = 1, binding = 0, stages = "fs")] diffuse_texture: (),
        #[sampler(set = 1, binding = 1, stages = "fs")] diffuse_sampler: (),
    }

    // === Simple lighting pipeline ===
    #[derive(GraphicsPipeline)]
    #[pipeline(
        vs = "macrokid_graphics/shaders/simple_lit.vert",
        fs = "macrokid_graphics/shaders/simple_lit.frag",
        topology = "TriangleList",
        depth = true,
        polygon = "Fill",
        cull = "Back",
        front_face = "Ccw",
        dynamic = "viewport,scissor"
    )]
    struct SimpleLitPipeline;

    // Build engine configuration
    let env_options = BackendOptions::default().with_env_fallback();
    let config = EngineBuilder::new()
        .app("Texture Generation Showcase")
        .window(1920, 1080, true)
        .options(env_options)
        .present_mode_priority(vec!["MAILBOX", "FIFO"])
        .adapter_preference("discrete")
        .msaa_samples(4)
        .add_pipeline(SimpleLitPipeline::pipeline_desc().clone())
        .build()?;

    config.options.log_effective(&config.window);

    // === Generate scene with various textures ===
    println!("Generating texture showcase scene...");
    let scene_assets = generate_texture_showcase_scene()?;

    // === Simple camera animation ===
    let mut time = 0.0f32;
    let mut last_frame = std::time::Instant::now();
    
    let update_uniforms = move |_frame_idx: usize| -> Option<Vec<u8>> {
        let now = std::time::Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        time += dt;

        // Orbiting camera
        let radius = 12.0;
        let height = 6.0;
        let camera_pos = Vec3::new(
            radius * (time * 0.2).cos(),
            height,
            radius * (time * 0.2).sin(),
        );
        
        let view = Mat4::look_at_rh(camera_pos, Vec3::new(0.0, 2.0, 0.0), Vec3::Y);
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), 16.0/9.0, 0.1, 100.0);
        
        // Pack matrices and basic lighting data
        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(&view.to_cols_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&proj.to_cols_array().map(f32::to_ne_bytes).concat());
        
        // Simple directional light
        let light_dir = Vec3::new(0.3, -0.7, 0.2).normalize();
        uniform_data.extend_from_slice(&light_dir.to_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&2.0f32.to_ne_bytes()); // intensity
        
        let light_color = Vec3::new(1.0, 0.95, 0.8);
        uniform_data.extend_from_slice(&light_color.to_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&0.0f32.to_ne_bytes()); // padding

        Some(uniform_data)
    };

    run_vulkan_linux_app_with_resources_and_update::<TextureMaterial, TextureVertex, _>(
        &config,
        &scene_assets,
        update_uniforms,
    )?;

    Ok(())
}

#[cfg(feature = "vulkan-linux")]
fn generate_texture_showcase_scene() -> Result<macrokid_graphics::vk_linux::AppResources, Box<dyn std::error::Error>> {
    use macrokid_graphics::vk_linux::AppResources;
    use macrokid_graphics::assets::*;
    use glam::{Vec3, Vec4, Mat4};

    // === Generate different primitive meshes ===
    
    println!("  Creating geometry for texture display...");
    
    // Create a grid of cubes to show different textures
    let cube_base = Primitives::cube::<SimpleVertex>(1.8);
    let mut combined_vertices = Vec::new();
    let mut combined_indices = Vec::new();
    let mut vertex_offset = 0u32;

    // 4x2 grid of cubes
    let grid_width = 4;
    let grid_height = 2;
    let spacing = 3.0;
    
    for row in 0..grid_height {
        for col in 0..grid_width {
            let x = (col as f32 - (grid_width - 1) as f32 * 0.5) * spacing;
            let y = 1.0;
            let z = (row as f32 - (grid_height - 1) as f32 * 0.5) * spacing;
            
            let transform = Mat4::from_translation(Vec3::new(x, y, z));
            
            // Transform vertices
            for vertex in &cube_base.vertices {
                let pos = Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
                let world_pos = transform * pos;
                
                let transformed_vertex = SimpleVertex {
                    position: world_pos.truncate(),
                    normal: vertex.normal,
                    uv: vertex.uv,
                };
                
                // Convert to bytes
                combined_vertices.extend_from_slice(&transformed_vertex.to_bytes());
            }
            
            // Add indices with offset
            for &index in &cube_base.indices {
                combined_indices.push(vertex_offset + index);
            }
            vertex_offset += cube_base.vertices.len() as u32;
        }
    }

    // Add a ground plane
    let ground = Primitives::plane::<SimpleVertex>(20.0, 20.0, 4, 4);
    let ground_transform = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0));
    
    for vertex in &ground.vertices {
        let pos = Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
        let world_pos = ground_transform * pos;
        
        let transformed_vertex = SimpleVertex {
            position: world_pos.truncate(),
            normal: vertex.normal,
            uv: vertex.uv * 4.0, // Tile the texture
        };
        
        combined_vertices.extend_from_slice(&transformed_vertex.to_bytes());
    }
    
    for &index in &ground.indices {
        combined_indices.push(vertex_offset + index);
    }

    // === Generate showcase textures ===
    println!("  Generating showcase textures...");
    
    let textures = generate_texture_variety(512);
    
    // For now, use the first texture as primary
    let primary_texture = &textures[0];

    Ok(AppResources {
        uniform_data: Some(vec![0u8; 1024]), // Buffer for camera and lighting
        vertices: Some(combined_vertices),
        indices: Some(combined_indices),
        image_rgba: Some(primary_texture.data.clone()),
        image_size: Some((primary_texture.width, primary_texture.height)),
        image_pixels: None,
    })
}

#[cfg(feature = "vulkan-linux")]
fn generate_texture_variety(size: u32) -> Vec<macrokid_graphics::assets::Texture2D> {
    use macrokid_graphics::assets::*;
    use glam::Vec4;

    let mut textures = Vec::new();

    println!("    - Solid colors");
    textures.push(TextureGenerator::solid_color(size, size, Vec4::new(1.0, 0.3, 0.3, 1.0))); // Red
    textures.push(TextureGenerator::solid_color(size, size, Vec4::new(0.3, 1.0, 0.3, 1.0))); // Green
    textures.push(TextureGenerator::solid_color(size, size, Vec4::new(0.3, 0.3, 1.0, 1.0))); // Blue

    println!("    - Checkerboards");
    textures.push(TextureGenerator::checkerboard(
        size, size, size / 8,
        Vec4::new(0.9, 0.9, 0.9, 1.0),
        Vec4::new(0.1, 0.1, 0.1, 1.0)
    ));

    println!("    - Gradients");
    textures.push(TextureGenerator::gradient(
        size, size,
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
        true // horizontal
    ));
    textures.push(TextureGenerator::gradient(
        size, size,
        Vec4::new(1.0, 1.0, 0.0, 1.0),
        Vec4::new(1.0, 0.0, 1.0, 1.0),
        false // vertical
    ));

    println!("    - Perlin noise textures");
    textures.push(TextureGenerator::perlin_noise(size, size, 4.0, 3));
    textures.push(TextureGenerator::perlin_noise(size, size, 8.0, 5));

    println!("    - Generated normal maps");
    if !textures.is_empty() {
        // Use the first noise texture as height map
        let height_map = &textures[textures.len() - 2]; // Second-to-last noise
        let normal_map = TextureGenerator::normal_map_from_height(height_map, 1.0);
        textures.push(normal_map);
    }

    println!("    Generated {} textures", textures.len());
    textures
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' and 'vk-shaderc-compile' features.");
    eprintln!("Try: cargo run -p macrokid_graphics --example texture_showcase --features vulkan-linux,vk-shaderc-compile");
}