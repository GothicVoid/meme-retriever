# Windows 开发机初始化

本文档说明 Windows 侧的具体初始化方法。跨平台整体约定见 [SETUP.md](./SETUP.md)。

## 前提

- 已安装 Git、Node.js、Rust
- 已安装 Tauri for Windows 所需前置环境
- 已克隆本仓库并进入仓库根目录

## 推荐流程

1. 运行初始化脚本：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

2. 启动桌面应用：

```powershell
npm run tauri dev
```

## 脚本行为

`scripts/setup-windows.ps1` 会执行：

- 运行 `npm install`
- 运行 `cargo fetch --manifest-path src-tauri/Cargo.toml`
- 从 `models-manifest.json` 读取模型压缩包地址和 SHA256
- 下载并解压模型压缩包到 `src-tauri/models/`
- 从 `runtime-manifest.json` 读取 runtime 压缩包或 DLL 地址
- 自动提取 `onnxruntime.dll` 到 `src-tauri/libs/`
- 校验模型压缩包 SHA256（如存在）
- 校验 runtime 压缩包 SHA256（如存在）
- 校验模型文件 SHA256

如需跳过依赖安装，可追加：

```powershell
-SkipNpmInstall -SkipCargoFetch
```

如需临时覆盖 `runtime-manifest.json` 中的 runtime 地址，可额外传入：

```powershell
-OrtUrl "https://example.com/onnxruntime-win-x64.zip"
```

如需临时覆盖 `models-manifest.json` 中的模型地址，可继续显式传入：

```powershell
-ModelsUrl "https://example.com/meme-retriever-models.zip"
```

## 常见约定

- 模型 Release tag：`models-2026.04.28`
- 应用 Release tag：`v0.1.0`
- Runtime Release tag：`runtime-onnx-2026.05.05`

## 注意

- Windows 编译不能直接复用 Linux 的 `libonnxruntime.so`
- 正式打包时，`src-tauri/tauri.conf.json` 会把 `models/*` 和 `libs/*` 一起带入 bundle 资源目录
