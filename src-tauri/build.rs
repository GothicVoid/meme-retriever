fn main() {
    // 将项目内的 libonnxruntime.so 路径设置为编译时环境变量，
    // 使 ort load-dynamic 在运行时能找到它，无需手动设置 ORT_DYLIB_PATH。
    let libs_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("libs");
    let so_path = libs_dir.join("libonnxruntime.so");
    if so_path.exists() {
        println!("cargo::rustc-env=ORT_DYLIB_PATH={}", so_path.display());
    }

    tauri_build::build()
}
