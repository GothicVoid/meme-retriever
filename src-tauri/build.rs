fn main() {
    println!("cargo::rerun-if-changed=libs");

    // 开发环境下优先指向仓库内的 ONNX Runtime 动态库；
    // 打包后的正式应用会在运行时改写 ORT_DYLIB_PATH 到资源目录。
    let libs_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("libs");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let dylib_name = if target_os == "windows" {
        "onnxruntime.dll"
    } else if target_os == "macos" {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    };

    let dylib_path = libs_dir.join(dylib_name);
    if dylib_path.exists() {
        println!(
            "cargo::rustc-env=ORT_DYLIB_PATH={}",
            dylib_path.display()
        );
    }

    tauri_build::build()
}
