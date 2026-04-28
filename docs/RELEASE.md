# 发布约定

## 总体原则

源码仓库只保存源码、脚本、文档和元数据，不保存以下大文件：

- `src-tauri/models/`
- `src-tauri/libs/`
- 用户运行时数据

大文件统一走 `GitHub Releases`。

## Release 拆分

建议至少维护两类 Release。

### 1. 应用 Release

用于给最终用户下载可执行程序。

- tag 示例：`v0.1.0`
- 资产示例：
  - `meme-retriever_0.1.0_windows_x64_setup.msi`
  - `meme-retriever_0.1.0_windows_x64_portable.zip`
  - `SHA256SUMS.txt`

### 2. 模型 Release

用于开发机、构建机、CI 拉取模型资源。

- tag 示例：`models-2026.04.28`
- 资产示例：
  - `meme-retriever-models-2026.04.28.zip`
  - `models-manifest.json`
  - `SHA256SUMS.txt`

### 3. 平台运行时 Release

如果你不想把 ONNX Runtime 直接提交到仓库，可以为不同平台单独发运行时资源。

- tag 示例：`runtime-windows-x64`
- 资产示例：
  - `onnxruntime-win-x64.zip`
  - `SHA256SUMS.txt`

## 建议的版本映射

- 应用版本独立递增，例如 `v0.1.0`
- 模型版本按发布日期或语义版本管理，例如 `models-2026.04.28`
- 当应用版本不变、模型更新时，只更新模型 Release

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
- 输出模型 zip 与 `SHA256SUMS.txt`

建议将以下文件上传到模型 Release：

- `release-assets/models/meme-retriever-models-<version>.zip`
- `release-assets/models/models-manifest.json`
- `release-assets/models/SHA256SUMS.txt`

## 正式应用发布建议

当前项目更适合优先采用“应用包内自带模型和运行时库”的模式：

- 构建机先执行资源初始化脚本
- 保证 `src-tauri/models/` 和 `src-tauri/libs/` 都已就绪
- 再执行 `npm run tauri build`

这样打出的 Windows 安装包能直接运行，用户无需首次启动再下载模型。
