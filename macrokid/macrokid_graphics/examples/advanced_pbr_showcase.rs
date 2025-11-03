//! Advanced PBR Showcase using the new asset generation framework
//!
//! This example demonstrates:
//! - Modular asset generation system
//! - Multiple primitive types (sphere, cube, plane, cylinder) 
//! - Procedural PBR materials with proper texture generation
//! - Tangent-space normal mapping
//! - Various material types (metal, plastic, rough/smooth surfaces)
//! - Environment mapping and lighting scenarios
//!
//! Run with:
//!   cargo run -p macrokid_graphics --example advanced_pbr_showcase --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use macrokid_graphics::engine::{EngineBuilder, BackendOptions};
    use macrokid_graphics::pipeline::PipelineInfo;
    use macrokid_graphics::vk_linux::{run_vulkan_linux_app_with_resources_and_update, AppResources};
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};
    use macrokid_graphics::assets::*;
    use glam::{Vec3, Vec4, Mat4};
    use std::f32::consts::PI;

    // === Define our PBR vertex layout that matches the asset system ===
    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct AdvancedPbrVertex {
        #[vertex(location = 0, format = "vec3")] position: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec4")] tangent: [f32; 4],
        #[vertex(location = 3, format = "vec2")] uv: [f32; 2],
    }

    // Convert our asset system vertex to the derive-compatible format
    impl From<PbrVertex> for AdvancedPbrVertex {
        fn from(v: PbrVertex) -> Self {
            Self {
                position: v.position.to_array(),
                normal: v.normal.to_array(),
                tangent: v.tangent.to_array(),
                uv: v.uv.to_array(),
            }
        }
    }

    // === PBR Material Resources ===
    #[derive(ResourceBinding)]
    struct AdvancedPbrMaterial {
        #[uniform(set = 0, binding = 0, stages = "vs")] camera_ubo: (),
        #[uniform(set = 0, binding = 1, stages = "fs")] lights_ubo: (),
        #[uniform(set = 1, binding = 0, stages = "fs")] material_ubo: (),
        #[texture(set = 1, binding = 1, stages = "fs")] albedo_map: (),
        #[texture(set = 1, binding = 2, stages = "fs")] normal_map: (),
        #[texture(set = 1, binding = 3, stages = "fs")] metallic_roughness_map: (),
        #[texture(set = 1, binding = 4, stages = "fs")] ao_map: (),
        #[texture(set = 2, binding = 0, stages = "fs")] environment_cube: (),
        #[sampler(set = 3, binding = 0, stages = "fs")] material_sampler: (),
        #[sampler(set = 3, binding = 1, stages = "fs")] environment_sampler: (),
    }

    // === Advanced PBR Pipeline ===
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
    struct AdvancedPbrPipeline;

    // Build engine configuration
    let env_options = BackendOptions::default().with_env_fallback();
    let config = EngineBuilder::new()
        .app("Advanced PBR Showcase - Asset Generation")
        .window(1920, 1080, true)
        .options(env_options)
        .present_mode_priority(vec!["MAILBOX", "FIFO"])
        .adapter_preference("discrete")
        .msaa_samples(4)
        .add_pipeline(AdvancedPbrPipeline::pipeline_desc().clone())
        .build()?;

    config.options.log_effective(&config.window);

    // === Generate procedural scene ===
    println!("Generating procedural assets...");
    let scene_assets = generate_advanced_pbr_scene()?;

    // === Animation and camera setup ===
    let mut time = 0.0f32;
    let mut last_frame = std::time::Instant::now();
    
    let update_uniforms = move |_frame_idx: usize| -> Option<Vec<u8>> {
        let now = std::time::Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        time += dt;

        // Orbiting camera
        let radius = 8.0;
        let height = 4.0;
        let camera_pos = Vec3::new(
            radius * (time * 0.3).cos(),
            height + (time * 0.5).sin() * 1.0,
            radius * (time * 0.3).sin(),
        );
        
        let view = Mat4::look_at_rh(camera_pos, Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), 16.0/9.0, 0.1, 100.0);
        
        // Pack camera matrices
        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(&view.to_cols_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&proj.to_cols_array().map(f32::to_ne_bytes).concat());
        
        // Dynamic lighting setup
        let lights_data = create_advanced_lighting_data(time);
        uniform_data.extend_from_slice(&lights_data);

        Some(uniform_data)
    };

    run_vulkan_linux_app_with_resources_and_update::<AdvancedPbrMaterial, AdvancedPbrVertex, _>(
        &config,
        &scene_assets,
        update_uniforms,
    )?;

    Ok(())
}

#[cfg(feature = "vulkan-linux")]
fn generate_advanced_pbr_scene() -> Result<macrokid_graphics::vk_linux::AppResources, Box<dyn std::error::Error>> {
    use macrokid_graphics::vk_linux::AppResources;
    use macrokid_graphics::assets::*;
    use glam::{Vec3, Vec4, Mat4, Quat};

    // === Generate various primitive meshes ===
    
    println!("  Generating sphere geometry...");
    let sphere_simple = Primitives::uv_sphere::<SimpleVertex>(1.2, 32, 16);
    let sphere_pbr = sphere_simple.with_tangents();
    
    println!("  Generating cube geometry...");
    let cube_simple = Primitives::cube::<SimpleVertex>(2.0);
    let cube_pbr = cube_simple.with_tangents();
    
    println!("  Generating plane geometry...");
    let plane_simple = Primitives::plane::<SimpleVertex>(4.0, 4.0, 8, 8);
    let plane_pbr = plane_simple.with_tangents();
    
    println!("  Generating cylinder geometry...");
    let cylinder_simple = Primitives::cylinder::<SimpleVertex>(0.8, 2.5, 16);
    let cylinder_pbr = cylinder_simple.with_tangents();

    // === Combine meshes with different transforms ===
    let mut combined_vertices = Vec::new();
    let mut combined_indices = Vec::new();
    let mut vertex_offset = 0u32;

    // Transform matrices for object placement
    let transforms = [
        Mat4::from_translation(Vec3::new(-3.0, 1.2, 0.0)), // Sphere
        Mat4::from_translation(Vec3::new(3.0, 1.0, 0.0)),  // Cube  
        Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)), // Ground plane
        Mat4::from_translation(Vec3::new(0.0, 1.25, -3.0)), // Cylinder
    ];

    let meshes = [&sphere_pbr, &cube_pbr, &plane_pbr, &cylinder_pbr];
    
    for (mesh, transform) in meshes.iter().zip(transforms.iter()) {
        // Transform vertices
        for vertex in &mesh.vertices {
            let pos = Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
            let world_pos = *transform * pos;
            
            // Transform normal (use inverse transpose for non-uniform scaling)
            let normal_matrix = transform.inverse().transpose();
            let normal = Vec4::new(vertex.normal.x, vertex.normal.y, vertex.normal.z, 0.0);
            let world_normal = (normal_matrix * normal).truncate().normalize();
            
            // Transform tangent  
            let tangent = Vec4::new(vertex.tangent.x, vertex.tangent.y, vertex.tangent.z, 0.0);
            let world_tangent = (normal_matrix * tangent).truncate().normalize();
            
            let transformed_vertex = PbrVertex {
                position: world_pos.truncate(),
                normal: world_normal,
                tangent: Vec4::new(world_tangent.x, world_tangent.y, world_tangent.z, vertex.tangent.w),
                uv: vertex.uv,
            };
            
            // Convert to bytes
            combined_vertices.extend_from_slice(&transformed_vertex.to_bytes());
        }
        
        // Add indices with offset
        for &index in &mesh.indices {
            combined_indices.push(vertex_offset + index);
        }
        vertex_offset += mesh.vertices.len() as u32;
    }

    // === Generate PBR material textures ===
    println!("  Generating PBR textures...");
    
    // Different materials for each object
    let materials = [
        // Sphere: Polished gold
        PbrAssets::generate_material_set(
            Vec4::new(1.0, 0.82, 0.0, 1.0), // Gold albedo
            0.9, // High metallic
            0.1, // Low roughness (shiny)
            512
        ),
        // Cube: Rough plastic
        PbrAssets::generate_material_set(
            Vec4::new(0.8, 0.2, 0.2, 1.0), // Red plastic
            0.0, // Non-metallic
            0.8, // High roughness
            512
        ),
        // Plane: Concrete/stone
        PbrAssets::generate_material_set(
            Vec4::new(0.5, 0.5, 0.5, 1.0), // Gray concrete
            0.0, // Non-metallic
            0.9, // Very rough
            512
        ),
        // Cylinder: Brushed aluminum
        PbrAssets::generate_material_set(
            Vec4::new(0.7, 0.7, 0.8, 1.0), // Aluminum albedo
            0.8, // Mostly metallic
            0.3, // Medium roughness
            512
        ),
    ];

    // For this demo, use the first material set as primary texture
    let (albedo, normal, metallic_roughness, ao) = &materials[0];

    // === Generate environment map ===
    println!("  Generating environment map...");
    let environment = generate_simple_environment_map(256);

    Ok(AppResources {
        uniform_data: Some(vec![0u8; 4096]), // Large buffer for all uniform data
        vertices: Some(combined_vertices),
        indices: Some(combined_indices),
        image_rgba: Some(albedo.data.clone()),
        image_size: Some((albedo.width, albedo.height)),
        image_pixels: None,
    })
}

#[cfg(feature = "vulkan-linux")]
fn generate_simple_environment_map(size: u32) -> macrokid_graphics::assets::Texture2D {
    use macrokid_graphics::assets::*;
    use glam::Vec4;

    // Generate a simple sky gradient for environment mapping
    let sky_top = Vec4::new(0.5, 0.7, 1.0, 1.0);    // Light blue
    let sky_horizon = Vec4::new(0.8, 0.9, 1.0, 1.0); // Lighter blue
    let ground = Vec4::new(0.3, 0.2, 0.1, 1.0);      // Brown ground
    
    let mut texture = Texture2D::new(size, size, TextureFormat::RGBA8);
    
    for y in 0..size {
        let v = y as f32 / (size - 1) as f32;
        
        for x in 0..size {
            let color = if v < 0.5 {
                // Sky gradient (top half)
                let t = v * 2.0; // [0, 0.5] -> [0, 1]
                sky_top.lerp(sky_horizon, t)
            } else {
                // Ground gradient (bottom half) 
                let t = (v - 0.5) * 2.0; // [0.5, 1] -> [0, 1]
                sky_horizon.lerp(ground, t)
            };
            
            texture.set_pixel(x, y, color);
        }
    }
    
    texture
}

#[cfg(feature = "vulkan-linux")]
fn create_advanced_lighting_data(time: f32) -> Vec<u8> {
    use glam::{Vec3, Vec4};
    
    let mut light_data = Vec::new();
    
    // === Directional light (sun) ===
    let sun_direction = Vec3::new(
        0.3 * (time * 0.1).cos(),
        -0.8,
        0.3 * (time * 0.1).sin()
    ).normalize();
    
    // Sun properties
    light_data.extend_from_slice(&sun_direction.to_array().map(f32::to_ne_bytes).concat());
    light_data.extend_from_slice(&4.0f32.to_ne_bytes()); // intensity
    
    let sun_color = Vec3::new(1.0, 0.95, 0.8); // Warm sunlight
    light_data.extend_from_slice(&sun_color.to_array().map(f32::to_ne_bytes).concat());
    light_data.extend_from_slice(&0.0f32.to_ne_bytes()); // padding
    
    // === Point lights (3 dynamic lights) ===
    let colors = [
        Vec3::new(1.0, 0.3, 0.3), // Red
        Vec3::new(0.3, 1.0, 0.3), // Green
        Vec3::new(0.3, 0.3, 1.0), // Blue
    ];
    
    for i in 0..3 {
        let angle = time * 0.8 + i as f32 * std::f32::consts::TAU / 3.0;
        let radius = 4.0 + (time * 1.5 + i as f32).sin() * 1.0;
        let height = 2.0 + (time * 2.0 + i as f32 * 1.7).cos() * 0.8;
        
        let position = Vec3::new(
            radius * angle.cos(),
            height,
            radius * angle.sin()
        );
        
        // Point light position + range
        light_data.extend_from_slice(&position.to_array().map(f32::to_ne_bytes).concat());
        light_data.extend_from_slice(&12.0f32.to_ne_bytes()); // range
        
        // Point light color + intensity  
        let intensity = 1.5 + (time * 3.0 + i as f32 * 2.1).sin() * 0.3; // Flickering
        light_data.extend_from_slice(&colors[i].to_array().map(f32::to_ne_bytes).concat());
        light_data.extend_from_slice(&intensity.to_ne_bytes());
    }
    
    // === Camera position for view-dependent calculations ===
    let radius = 8.0;
    let height = 4.0;
    let camera_pos = Vec3::new(
        radius * (time * 0.3).cos(),
        height + (time * 0.5).sin() * 1.0,
        radius * (time * 0.3).sin(),
    );
    
    light_data.extend_from_slice(&camera_pos.to_array().map(f32::to_ne_bytes).concat());
    light_data.extend_from_slice(&0.0f32.to_ne_bytes()); // padding
    
    light_data
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' and 'vk-shaderc-compile' features.");
    eprintln!("Try: cargo run -p macrokid_graphics --example advanced_pbr_showcase --features vulkan-linux,vk-shaderc-compile");
}