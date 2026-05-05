# 开发环境初始化

目标：

- clone 后运行一个脚本，补齐仓库级依赖与资源
- `npm run tauri dev`、`npm test`、`cargo test --manifest-path src-tauri/Cargo.toml` 使用同一套模型和 runtime

以下系统级前置依赖仍需手工安装：

- Git
- Node.js 与 npm
- Rust 与 cargo（建议通过 rustup）
- Tauri 所需系统依赖

## 资源目录

以下目录不会进入 Git，但会被运行时和打包流程使用：

- `src-tauri/models/`：CLIP / OCR 模型、词表及配套 `.data` 文件
- `src-tauri/libs/`：ONNX Runtime 动态库

当前代码使用 Rust `ort` crate 的动态加载模式。仅安装 Python 的 `onnxruntime` 不能保证 Tauri / Rust 测试找到运行时。开发与打包统一通过 `src-tauri/libs/` 提供动态库。

- 模型资产：[`models-manifest.json`](../models-manifest.json)
- Runtime 资产：[`runtime-manifest.json`](../runtime-manifest.json)

## Windows

Windows 使用 PowerShell 脚本：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

脚本会执行：

- 检查 `npm`、`cargo` 是否可用
- 运行 `npm install`
- 运行 `cargo fetch --manifest-path src-tauri/Cargo.toml`
- 从 `models-manifest.json` 读取模型压缩包地址和 SHA256
- 下载并解压模型到 `src-tauri/models/`
- 校验模型压缩包 SHA256（如存在）
- 下载并提取 `onnxruntime.dll` 到 `src-tauri/libs/`
- 从 `runtime-manifest.json` 读取 runtime 下载地址、目标文件名和 SHA256
- 校验 runtime 压缩包 SHA256（如存在）
- 校验模型文件 SHA256

可选参数：

- `-SkipNpmInstall`
- `-SkipCargoFetch`
- `-ModelsUrl <url>`：覆盖 `models-manifest.json`
- `-OrtUrl <url>`：覆盖 `runtime-manifest.json`

更细说明见 [SETUP_WINDOWS.md](./SETUP_WINDOWS.md)。

## Linux

Linux 使用 Bash 脚本：

```bash
bash scripts/setup-linux.sh
```

脚本会执行：

- 检查 `npm`、`cargo`、`curl`、`unzip`、`tar`、`sha256sum` 是否可用
- 运行 `npm install`
- 运行 `cargo fetch --manifest-path src-tauri/Cargo.toml`
- 从 `models-manifest.json` 读取模型压缩包地址和 SHA256
- 下载并解压模型到 `src-tauri/models/`
- 校验模型压缩包 SHA256（如存在）
- 下载并提取 `libonnxruntime.so` 或 `libonnxruntime.dylib` 到 `src-tauri/libs/`
- 从 `runtime-manifest.json` 读取 runtime 下载地址、目标文件名和 SHA256
- 校验 runtime 压缩包 SHA256（如存在）
- 校验模型文件 SHA256

可选参数：

- `--skip-npm-install`
- `--skip-cargo-fetch`
- `--repo-root <path>`
- `--models-url <url>`：覆盖 `models-manifest.json`
- `--ort-url <url>`：覆盖 `runtime-manifest.json`

Linux 仍需手工安装 Tauri 运行库和系统构建依赖。

## 初始化完成后的验证

建议至少执行以下命令：

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

## 边界

初始化脚本只负责仓库级依赖与资源，不负责系统环境安装。

## Runtime Manifest

[`runtime-manifest.json`](../runtime-manifest.json) 记录：

- 各平台 ONNX Runtime 版本
- 各平台下载地址
- 目标动态库文件名
- 资产 SHA256

## Models Manifest

[`models-manifest.json`](../models-manifest.json) 记录：

- 模型压缩包名
- 模型压缩包下载地址
- 模型压缩包 SHA256
- 每个模型文件的 SHA256
