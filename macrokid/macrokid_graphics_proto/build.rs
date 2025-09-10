fn main() {
    // Use vendored protoc to avoid system dependency
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc vendored");
    std::env::set_var("PROTOC", protoc);
    let proto_dir = std::path::PathBuf::from("proto");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    let files = vec![proto_dir.join("graphics_config.proto")];
    println!("cargo:rerun-if-changed={}", files[0].display());
    println!("cargo:rerun-if-changed={}", proto_dir.display());
    prost_build::compile_protos(&files, &[proto_dir]).expect("compile protos");
}
