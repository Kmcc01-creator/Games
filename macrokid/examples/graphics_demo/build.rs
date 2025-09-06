use std::env;

fn collect_clang_args() -> Vec<String> {
    // Read CLANG_INCLUDES and CLANG_DEFS environment variables and expand into
    // clang command-line flags. This allows pointing to SDK headers and setting
    // preprocessor symbols without hardcoding paths.
    // - CLANG_INCLUDES: a list of include directories (space- or semicolon-separated)
    // - CLANG_DEFS: a list of preprocessor defines like NAME or NAME=VALUE (space- or semicolon-separated)
    let mut args: Vec<String> = Vec::new();

    if let Ok(includes) = env::var("CLANG_INCLUDES") {
        for item in includes.split([' ', ';']) {
            let it = item.trim();
            if it.is_empty() { continue; }
            args.push(format!("-I{}", it));
        }
    }
    if let Ok(defs) = env::var("CLANG_DEFS") {
        for item in defs.split([' ', ';']) {
            let it = item.trim();
            if it.is_empty() { continue; }
            // Accept NAME or NAME=VALUE forms directly
            args.push(format!("-D{}", it));
        }
    }
    args
}

fn main() {
    // Best-effort PoC: try to analyze a header if CLANG_EXEC_DEMO is set
    if std::env::var("CLANG_EXEC_DEMO").ok().as_deref() != Some("1") {
        println!("cargo:warning=CLANG_EXEC_DEMO not set; skipping clang exec PoC");
        return;
    }
    let header_rel = "examples/graphics_demo/include/demo.hpp";
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let header = std::path::Path::new(&manifest_dir).join("..").join("..").join(header_rel);
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_cpp = std::path::Path::new(&out_dir).join("generated_demo.hpp");
    let out_json = std::path::Path::new(&out_dir).join("generated_demo.json");
    let out_mk = std::path::Path::new(&out_dir).join("parsed_mk.json");
    let out_cir = std::path::Path::new(&out_dir).join("c_ir.json");
    let out_macros = std::path::Path::new(&out_dir).join("macros.json");

    let extra = collect_clang_args();
    let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
    match macrokid_clang_exec::analyze_header(&header, &extra_refs) {
        Ok(ir) => {
            let hdr = macrokid_clang_exec::emit_cpp_header(&ir, Some("mk"));
            std::fs::write(&out_cpp, hdr).ok();
            let _ = std::fs::write(&out_json, serde_json::to_string_pretty(&ir).unwrap());
            println!("cargo:warning=generated C++ header at {}", out_cpp.display());
            // Parse mk:: annotations to typed IR and emit as JSON
            let mk = macrokid_clang_exec::parse_all_mk(&ir);
            let _ = std::fs::write(&out_mk, serde_json::to_string_pretty(&mk).unwrap());
            println!("cargo:warning=parsed mk annotations at {}", out_mk.display());

            // C-only IR + macros
            if let Ok(cir) = macrokid_clang_exec::analyze_header_c(&header, &extra_refs) {
                let _ = std::fs::write(&out_cir, serde_json::to_string_pretty(&cir).unwrap());
                println!("cargo:warning=c-only IR at {}", out_cir.display());
            }
            if let Ok(macros) = macrokid_clang_exec::analyze_macros_c(&header, &extra_refs) {
                let _ = std::fs::write(&out_macros, serde_json::to_string_pretty(&macros).unwrap());
                println!("cargo:warning=macros at {}", out_macros.display());
            }
        }
        Err(e) => {
            println!("cargo:warning=clang exec PoC failed: {}", e);
        }
    }
}
