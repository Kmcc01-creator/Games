//! Load an EngineConfig from protobuf and run the Vulkan example.
//! Usage:
//!   cargo run -p macrokid_graphics --example load_proto --features vulkan-linux,proto,vk-shaderc-compile -- <path.pb>

#[cfg(all(feature = "vulkan-linux", feature = "proto"))]
fn main() {
    use std::env;
    use std::fs;
    use macrokid_graphics::vk_linux::run_vulkan_linux_app; // wrapper uses NoRB/NoVL
    use macrokid_graphics::engine::EngineConfig as MkEngineConfig;
    use macrokid_graphics_proto::proto as pb;

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let bytes = fs::read(&args[1]).expect("read .pb file");
        let cfg_pb = pb::EngineConfig::decode(bytes.as_slice()).expect("decode EngineConfig");
        let cfg = MkEngineConfig::try_from(cfg_pb).expect("convert to EngineConfig");
        run_vulkan_linux_app(&cfg).expect("run vulkan");
    } else {
        // Build a small in-memory proto config and convert directly
        let vs = format!("{}/shaders/triangle.vert", env!("CARGO_MANIFEST_DIR"));
        let fs = format!("{}/shaders/triangle.frag", env!("CARGO_MANIFEST_DIR"));
        let cfg_pb = pb::EngineConfig {
            app: "Proto Vulkan Demo".into(),
            window: Some(pb::WindowCfg { width: 800, height: 600, vsync: true }),
            pipelines: vec![pb::PipelineDesc {
                name: "triangle".into(),
                shaders: Some(pb::ShaderPaths { vs: Some(pb::shader_paths::Vs::VsPath(vs)), fs: Some(pb::shader_paths::Fs::FsPath(fs)) }),
                topology: pb::Topology::TriangleList as i32,
                depth: false,
                raster: None,
                blend: None,
                samples: 1,
            }],
        };
        let cfg = MkEngineConfig::try_from(cfg_pb).expect("convert to EngineConfig");
        run_vulkan_linux_app(&cfg).expect("run vulkan");
    }
}

#[cfg(not(all(feature = "vulkan-linux", feature = "proto")))]
fn main() {
    eprintln!("This example requires features: vulkan-linux and proto");
}
