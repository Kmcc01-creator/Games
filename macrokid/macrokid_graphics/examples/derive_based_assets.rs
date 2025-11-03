//! Derive-Based Asset Generation Example
//!
//! This example demonstrates how to use derive macros for compile-time asset generation:
//! - `#[derive(ProceduralMesh)]` for geometry generation
//! - `#[derive(ProceduralTexture)]` for texture generation  
//! - `#[derive(AssetBundle)]` for combining assets
//!
//! Benefits of derive-based approach:
//! - Compile-time validation and generation
//! - Type-safe asset references
//! - Zero-cost abstractions
//! - Declarative, readable asset definitions
//! - Automatic caching and lazy initialization
//!
//! Run with:
//!   cargo run -p macrokid_graphics --example derive_based_assets --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use macrokid_graphics::engine::{EngineBuilder, BackendOptions};
    use macrokid_graphics::pipeline::PipelineInfo;
    use macrokid_graphics::vk_linux::{run_vulkan_linux_app_with_resources_and_update, AppResources};
    use macrokid_graphics::assets::*;
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline, ProceduralMesh, ProceduralTexture, AssetBundle};
    use glam::{Vec3, Vec4, Mat4};

    // ==================== DERIVE-BASED ASSET DEFINITIONS ====================

    /// A procedural sphere mesh with compile-time generation
    #[derive(ProceduralMesh)]
    #[primitive(type = "sphere", radius = 1.5, rings = 16, sectors = 32)]
    #[transform(translate = "0.0,2.0,0.0", rotate = "0.0,0.0,0.0")]
    struct HeroSphere;

    /// A procedural cube mesh, scaled and positioned
    #[derive(ProceduralMesh)]
    #[primitive(type = "cube", size = 2.0)]
    #[transform(translate = "-3.0,1.0,0.0", rotate = "0.0,45.0,0.0", scale = "1.0,1.5,1.0")]
    struct StretchedCube;

    /// A ground plane with high tessellation for detail
    #[derive(ProceduralMesh)]
    #[primitive(type = "plane", width = 20.0, height = 20.0, segments = 8)]
    #[transform(translate = "0.0,-1.0,0.0")]
    struct GroundPlane;

    /// A decorative cylinder
    #[derive(ProceduralMesh)]
    #[primitive(type = "cylinder", radius = 0.8, height = 3.0, segments = 12)]
    #[transform(translate = "3.0,0.5,0.0")]
    struct DecorativePillar;

    /// Checkerboard texture for the sphere
    #[derive(ProceduralTexture)]
    #[texture(type = "checkerboard", width = 512, height = 512)]
    struct CheckerTexture;

    /// Noise-based texture for interesting surfaces
    #[derive(ProceduralTexture)]
    #[texture(type = "noise", width = 256, height = 256)]
    #[noise(scale = 6.0, octaves = 4)]
    struct NoiseTexture;

    /// Solid color texture for simple materials
    #[derive(ProceduralTexture)]
    #[texture(type = "solid", width = 128, height = 128)]
    struct RedTexture;

    /// Gradient texture for interesting effects
    #[derive(ProceduralTexture)]
    #[texture(type = "gradient", width = 512, height = 512)]
    struct GradientTexture;

    /// Asset bundle combining meshes and textures
    #[derive(AssetBundle)]
    struct SceneAssets {
        #[mesh_ref] sphere: HeroSphere,
        #[mesh_ref] cube: StretchedCube,
        #[mesh_ref] ground: GroundPlane,
        #[mesh_ref] pillar: DecorativePillar,
        
        #[texture_ref] checker: CheckerTexture,
        #[texture_ref] noise: NoiseTexture,
        #[texture_ref] red: RedTexture,
        #[texture_ref] gradient: GradientTexture,
    }

    // ==================== RENDERING SETUP ====================

    #[derive(BufferLayout)]
    #[buffer(step = "vertex")]
    struct DeriveVertex {
        #[vertex(location = 0, format = "vec3")] position: [f32; 3],
        #[vertex(location = 1, format = "vec3")] normal: [f32; 3],
        #[vertex(location = 2, format = "vec2")] uv: [f32; 2],
    }

    impl From<SimpleVertex> for DeriveVertex {
        fn from(v: SimpleVertex) -> Self {
            Self {
                position: v.position.to_array(),
                normal: v.normal.to_array(),
                uv: v.uv.to_array(),
            }
        }
    }

    #[derive(ResourceBinding)]
    struct SceneMaterial {
        #[uniform(set = 0, binding = 0, stages = "vs")] camera_ubo: (),
        #[uniform(set = 0, binding = 1, stages = "fs")] light_ubo: (),
        #[texture(set = 1, binding = 0, stages = "fs")] diffuse_texture: (),
        #[sampler(set = 1, binding = 1, stages = "fs")] diffuse_sampler: (),
    }

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
    struct DeriveBasedPipeline;

    // ==================== ENGINE SETUP ====================

    let env_options = BackendOptions::default().with_env_fallback();
    let config = EngineBuilder::new()
        .app("Derive-Based Asset Generation")
        .window(1920, 1080, true)
        .options(env_options)
        .present_mode_priority(vec!["MAILBOX", "FIFO"])
        .adapter_preference("discrete")
        .msaa_samples(4)
        .add_pipeline(DeriveBasedPipeline::pipeline_desc().clone())
        .build()?;

    config.options.log_effective(&config.window);

    // ==================== ASSET COLLECTION ====================

    println!("Generating derive-based assets...");
    let app_resources = collect_derive_based_assets()?;

    // ==================== ANIMATION LOOP ====================

    let mut time = 0.0f32;
    let mut last_frame = std::time::Instant::now();
    
    let update_uniforms = move |_frame_idx: usize| -> Option<Vec<u8>> {
        let now = std::time::Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        time += dt;

        // Dynamic camera movement
        let radius = 12.0 + (time * 0.5).sin() * 3.0;
        let height = 8.0 + (time * 0.3).cos() * 2.0;
        let camera_pos = Vec3::new(
            radius * (time * 0.2).cos(),
            height,
            radius * (time * 0.2).sin(),
        );
        
        let view = Mat4::look_at_rh(camera_pos, Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), 16.0/9.0, 0.1, 100.0);
        
        // Dynamic lighting
        let light_dir = Vec3::new(
            (time * 0.4).cos(),
            -0.6,
            (time * 0.4).sin()
        ).normalize();
        
        // Pack uniform data
        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(&view.to_cols_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&proj.to_cols_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&light_dir.to_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&(2.0f32 + (time * 2.0).sin() * 0.5).to_ne_bytes()); // animated intensity
        
        let light_color = Vec3::new(
            0.8 + (time * 1.1).sin() * 0.2,
            0.9 + (time * 1.3).cos() * 0.1,
            1.0 + (time * 0.9).sin() * 0.1,
        );
        uniform_data.extend_from_slice(&light_color.to_array().map(f32::to_ne_bytes).concat());
        uniform_data.extend_from_slice(&0.0f32.to_ne_bytes()); // padding

        Some(uniform_data)
    };

    run_vulkan_linux_app_with_resources_and_update::<SceneMaterial, DeriveVertex, _>(
        &config,
        &app_resources,
        update_uniforms,
    )?;

    Ok(())
}

#[cfg(feature = "vulkan-linux")]
fn collect_derive_based_assets() -> Result<macrokid_graphics::vk_linux::AppResources, Box<dyn std::error::Error>> {
    use macrokid_graphics::vk_linux::AppResources;
    use macrokid_graphics::assets::*;

    // ==================== DEMONSTRATE DERIVE USAGE ====================

    println!("  Asset bundle contains {} assets", SceneAssets::asset_count());
    println!("  Available assets: {:?}", SceneAssets::list_assets());

    // Access derive-generated meshes (zero-cost, compile-time generated)
    let sphere_mesh = HeroSphere::mesh();
    let cube_mesh = StretchedCube::mesh();  
    let ground_mesh = GroundPlane::mesh();
    let pillar_mesh = DecorativePillar::mesh();

    println!("  Sphere mesh: {} vertices, {} indices", sphere_mesh.vertices.len(), sphere_mesh.indices.len());
    println!("  Cube mesh: {} vertices, {} indices", cube_mesh.vertices.len(), cube_mesh.indices.len());
    println!("  Ground mesh: {} vertices, {} indices", ground_mesh.vertices.len(), ground_mesh.indices.len());
    println!("  Pillar mesh: {} vertices, {} indices", pillar_mesh.vertices.len(), pillar_mesh.indices.len());

    // Access derive-generated textures (cached, lazy-loaded)
    let checker_texture = CheckerTexture::texture();
    let noise_texture = NoiseTexture::texture();
    let red_texture = RedTexture::texture();
    let gradient_texture = GradientTexture::texture();

    println!("  Checker texture: {}x{} ({} bytes)", checker_texture.width, checker_texture.height, checker_texture.data.len());
    println!("  Noise texture: {}x{} ({} bytes)", noise_texture.width, noise_texture.height, noise_texture.data.len());
    
    // ==================== COMBINE INTO SINGLE MESH ====================

    // Combine all meshes into a single draw call
    let mut combined_vertices = Vec::new();
    let mut combined_indices = Vec::new();
    let mut vertex_offset = 0u32;

    let meshes = [sphere_mesh, cube_mesh, ground_mesh, pillar_mesh];
    
    for mesh in &meshes {
        // Convert and add vertices
        for vertex in &mesh.vertices {
            let derive_vertex = DeriveVertex::from(vertex.clone());
            combined_vertices.extend_from_slice(&derive_vertex.to_bytes());
        }
        
        // Add indices with offset
        for &index in &mesh.indices {
            combined_indices.push(vertex_offset + index);
        }
        vertex_offset += mesh.vertices.len() as u32;
    }

    // Use the primary texture (checkerboard for demonstration)
    let primary_texture = checker_texture;

    Ok(AppResources {
        uniform_data: Some(vec![0u8; 1024]),
        vertices: Some(combined_vertices),
        indices: Some(combined_indices),
        image_rgba: Some(primary_texture.data.clone()),
        image_size: Some((primary_texture.width, primary_texture.height)),
        image_pixels: None,
    })
}

// ==================== TRAIT IMPLEMENTATIONS FOR DERIVE EXAMPLES ====================

impl macrokid_graphics::assets::Vertex for DeriveVertex {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.position[0].to_ne_bytes());
        bytes.extend_from_slice(&self.position[1].to_ne_bytes());
        bytes.extend_from_slice(&self.position[2].to_ne_bytes());
        bytes.extend_from_slice(&self.normal[0].to_ne_bytes());
        bytes.extend_from_slice(&self.normal[1].to_ne_bytes());
        bytes.extend_from_slice(&self.normal[2].to_ne_bytes());
        bytes.extend_from_slice(&self.uv[0].to_ne_bytes());
        bytes.extend_from_slice(&self.uv[1].to_ne_bytes());
        bytes
    }
    
    fn byte_size() -> usize { 32 } // 3*4 + 3*4 + 2*4
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() {
    eprintln!("This example requires the 'vulkan-linux' and 'vk-shaderc-compile' features.");
    eprintln!("Try: cargo run -p macrokid_graphics --example derive_based_assets --features vulkan-linux,vk-shaderc-compile");
    
    // Even without Vulkan, we can demonstrate compile-time asset generation
    #[cfg(feature = "derive-demo")]
    demo_compile_time_assets();
}

#[cfg(all(not(feature = "vulkan-linux"), feature = "derive-demo"))]
fn demo_compile_time_assets() {
    use macrokid_graphics_derive::{ProceduralMesh, ProceduralTexture};
    
    println!("=== Compile-Time Asset Generation Demo ===");
    
    #[derive(ProceduralMesh)]
    #[primitive(type = "sphere", radius = 2.0)]
    struct DemoSphere;
    
    #[derive(ProceduralTexture)]
    #[texture(type = "checkerboard", width = 256, height = 256)]
    struct DemoTexture;
    
    let mesh = DemoSphere::mesh();
    let texture = DemoTexture::texture();
    
    println!("Generated sphere mesh: {} vertices", mesh.vertices.len());
    println!("Generated texture: {}x{} pixels", texture.width, texture.height);
    println!("All assets generated at compile-time with zero runtime cost!");
}