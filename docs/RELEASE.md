# 发布约定

源码仓库只保存源码、脚本、文档和元数据，不保存以下大文件：

- `src-tauri/models/`
- `src-tauri/libs/`
- 用户运行时数据

大文件统一走 `GitHub Releases`。

## Release 拆分

### 1. 应用 Release

用于给最终用户下载可执行程序。

- tag 示例：`v0.1.0`
- 资产示例：
  - `meme-retriever_0.1.0_x64-setup.exe`
  - `meme-retriever_0.1.0_x64_zh-CN.msi`

### 2. 模型 Release

用于开发机、构建机、CI 拉取模型资源。

- tag 示例：`models-2026.04.28`
- 资产示例：
  - `meme-retriever-models-2026.04.28.zip`

### 3. 平台运行时 Release

用于开发机、构建机、CI 拉取 ONNX Runtime。

- tag 示例：`runtime-onnx-2026.05.05`
- 资产示例：
  - `onnxruntime-win-x64-1.25.20260427.4.8a77e45.zip`
  - `onnxruntime-linux-x64-1.24.4.tgz`

## 版本建议

- 应用版本独立递增，例如 `v0.1.0`
- 模型版本按发布日期或语义版本管理，例如 `models-2026.04.28`
- Runtime 版本按发布日期管理，例如 `runtime-onnx-2026.05.05`

## 模型打包流程

在已有 `src-tauri/models/` 的机器上执行：

```bash
npm run package:models -- 2026.04.28
```

该命令会：

- 校验必需模型文件是否齐全
- 在 `clip_*` 与 `vit-b-16.*.onnx` 候选名之间自动选择实际存在的模型文件
- 自动把模型伴随的 `.data` / `.extra_file` 一并打包
- 更新仓库根目录 `models-manifest.json`
- 生成 `release-assets/models/`
- 输出模型 zip、`models-manifest.json`、`SHA256SUMS.txt`

模型 Release 只需要上传：

- `release-assets/models/meme-retriever-models-<version>.zip`

然后回填仓库根目录 [`models-manifest.json`](../models-manifest.json)：

- `distribution.artifactUrl`
- `distribution.artifactSha256`

## Runtime 发布流程

Runtime Release 只需要上传目标资产文件，例如：

- `onnxruntime-win-x64-1.25.20260427.4.8a77e45.zip`
- `onnxruntime-linux-x64-1.24.4.tgz`

然后回填仓库根目录 [`runtime-manifest.json`](../runtime-manifest.json)：

- 各平台 `artifactUrl`
- 各平台 `sha256`

## 正式应用发布建议

当前项目优先采用“应用包内自带模型和运行时库”的模式：

- 构建机先执行资源初始化脚本
- 保证 `src-tauri/models/` 和 `src-tauri/libs/` 都已就绪
- 再执行 `npm run tauri build`

这样打出的 Windows 安装包能直接运行，用户无需首次启动再下载模型。
