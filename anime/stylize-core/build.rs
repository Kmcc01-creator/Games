use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let candidates = [
        manifest_dir.join("assets").join("shaders"),
        manifest_dir.join("..").join("assets").join("shaders"),
    ];
    let shader_dir = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .expect("Could not find assets/shaders in crate or workspace root");

    println!("cargo:rerun-if-changed={}", shader_dir.display());

    // Map extensions to shader kinds
    let compiler = shaderc::Compiler::new().expect("shaderc compiler");
    let mut options = shaderc::CompileOptions::new().expect("shaderc opts");
    options.set_target_env(shaderc::TargetEnv::Vulkan, shaderc::EnvVersion::Vulkan1_3 as u32);
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);

    // Debug info when not release
    let is_debug = env::var("PROFILE").map(|p| p != "release").unwrap_or(true);
    if is_debug { options.set_generate_debug_info(); }

    // Collect .vert and .frag files
    let entries = fs::read_dir(&shader_dir).unwrap_or_else(|_| panic!("missing {:?}", shader_dir));
    for entry in entries {
        let path = entry.unwrap().path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue; };
        let kind = match ext {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            _ => continue,
        };

        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));
        let filename = path.file_name().unwrap().to_string_lossy();

        let bin = compiler
            .compile_into_spirv(&src, kind, &filename, "main", Some(&options))
            .unwrap_or_else(|e| panic!("shaderc failed for {:?}: {}", path, e));

        let mut out_path = out_dir.clone();
        out_path.push(format!("{}.spv", filename));
        fs::write(&out_path, bin.as_binary_u8()).expect("write spv");
        println!("cargo:warning=Compiled {:?} -> {}", path, out_path.display());
    }
}
