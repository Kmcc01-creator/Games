//! Animated uniform + custom RGBA image demo
//! Run:
//!   cargo run -p macrokid_graphics --example animated_demo --features vulkan-linux,vk-shaderc-compile

#[cfg(feature = "vulkan-linux")]
fn main() {
    use macrokid_graphics::engine::{EngineBuilder, BackendOptions};
    use macrokid_graphics::pipeline::PipelineInfo;
    use macrokid_graphics::vk_linux::{run_vulkan_linux_app_with_resources_and_update, AppResources};
    use macrokid_graphics_derive::{ResourceBinding, BufferLayout, GraphicsPipeline};

    // Resource bindings expected by shaders
    #[derive(ResourceBinding)]
    struct Material {
        #[uniform(set = 0, binding = 0, stages = "fs")] _ubo: (),
        #[combined(set = 0, binding = 1, stages = "fs")] _tex: (),
    }

    // Vertex layout unused (gl_VertexIndex), leave empty
    #[derive(BufferLayout)]
    struct Vertex;

    // Pipeline using custom anim shaders
    #[derive(GraphicsPipeline)]
    #[pipeline(
        vs = "macrokid_graphics/shaders/anim.vert",
        fs = "macrokid_graphics/shaders/anim.frag",
        topology = "TriangleList",
        depth = true,
        polygon = "Fill",
        cull = "Back",
        front_face = "Cw",
        dynamic = "viewport,scissor"
    )]
    struct AnimPipeline;

    // Seed options from env, then override with explicit builder choices (builder wins)
    let env_seed = BackendOptions::default().with_env_fallback();
    let cfg = EngineBuilder::new()
        .app("Animated Uniform + Image")
        .window(960, 540, true)
        .options(env_seed)
        .present_mode_priority(vec!["MAILBOX", "FIFO"]) // prefer MAILBOX, fallback FIFO
        .adapter_preference("discrete")                 // prefer discrete GPUs when available
        .add_pipeline(AnimPipeline::pipeline_desc().clone())
        .build()
        .expect("valid config");

    // Log effective options
    cfg.options.log_effective(&cfg.window);

    // Build a small 2x2 image to prove upload works
    let w = 2u32; let h = 2u32;
    let pixels: Vec<u8> = vec![
        255, 255, 255, 255,  255, 0, 0, 255,
        0, 255, 0, 255,      0, 0, 255, 255,
    ];

    let resources = AppResources {
        uniform_data: Some(vec![0u8; 16]), // initial tint (all zeros; updated per-frame)
        image_rgba: None,
        image_size: Some((w, h)),
        image_pixels: Some(pixels),
    };

    // Animate tint over time (time-based, slower)
    let mut t: f32 = 0.0;
    let mut last = std::time::Instant::now();
    let speed: f32 = 0.3; // radians per second (lower is slower)
    let update = move |_frame_idx: usize| -> Option<Vec<u8>> {
        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        t += dt * speed;
        let r = (0.5 + 0.5 * (t).sin()).clamp(0.0, 1.0);
        let g = (0.5 + 0.5 * (t * 0.7).sin()).clamp(0.0, 1.0);
        let b = (0.5 + 0.5 * (t * 1.3).sin()).clamp(0.0, 1.0);
        let a = 1.0f32;
        // Pack as 4 f32 (16 bytes)
        let pack = |f: f32| f.to_ne_bytes();
        let bytes = [pack(r), pack(g), pack(b), pack(a)].concat();
        Some(bytes)
    };

    run_vulkan_linux_app_with_resources_and_update::<Material, Vertex, _>(&cfg, &resources, update)
        .expect("run");
}

#[cfg(not(feature = "vulkan-linux"))]
fn main() { eprintln!("Enable 'vulkan-linux' and 'vk-shaderc-compile' features"); }
