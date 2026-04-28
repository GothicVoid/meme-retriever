# Windows 开发机初始化

本文档用于在新的 Windows 机器上拉起当前仓库，并补齐不会进入 Git 仓库的大文件资源。

## 前提

- 已安装 Git、Node.js、Rust
- 已安装 Tauri for Windows 所需前置环境
- 已克隆本仓库并进入仓库根目录

## 需要额外准备的资源

以下目录默认不会提交到 Git：

- `src-tauri/models/`
- `src-tauri/libs/`

其中：

- `src-tauri/models/` 放 CLIP / OCR 模型和词表
- `src-tauri/libs/` 放 ONNX Runtime 动态库，Windows 下通常是 `onnxruntime.dll`

## 推荐流程

1. 安装前端依赖：

```powershell
npm install
```

2. 从 GitHub Release 下载模型和运行时库：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1 `
  -ModelsUrl "https://github.com/<owner>/<repo>/releases/download/models-2026.04.28/meme-retriever-models-2026.04.28.zip" `
  -OrtUrl "https://github.com/<owner>/<repo>/releases/download/runtime-windows-x64/onnxruntime-win-x64.zip"
```

3. 启动桌面应用：

```powershell
npm run tauri dev
```

## 脚本行为

`scripts/setup-windows.ps1` 会执行：

- 下载模型压缩包并解压到 `src-tauri/models/`
- 下载 ONNX Runtime 压缩包或 DLL
- 自动提取 `onnxruntime.dll` 到 `src-tauri/libs/`
- 如果仓库根目录存在 `models-manifest.json` 且包含 SHA256，会校验模型文件

## 常见约定

- 模型 Release tag：`models-2026.04.28`
- 应用 Release tag：`v0.1.0`
- Windows 运行时 Release tag：`runtime-windows-x64`

## 注意

- `models-manifest.json` 初始模板里的 `sha256` 为空，需要在有模型文件的机器上运行 `npm run package:models` 生成正式值
- 如果本地使用的是 `vit-b-16.txt.fp32.onnx` / `vit-b-16.img.fp32.onnx` 这类文件名，打包脚本会自动识别，不需要手动改名
- Windows 编译不能直接复用 Linux 的 `libonnxruntime.so`
- 正式打包时，`src-tauri/tauri.conf.json` 会把 `models/*` 和 `libs/*` 一起带入 bundle 资源目录
