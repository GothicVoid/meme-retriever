# meme-retriever

本项目是一个基于 Tauri + Vue 3 + TypeScript 的本地表情包检索工具。

## 开发入口

- 前端开发：`npm run dev`
- 桌面联调：`npm run tauri dev`
- 前端测试：`npm test`
- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml`

## 大文件与发布

模型文件、ONNX Runtime 动态库和用户运行时数据不会直接进入源码仓库。

- Windows 初始化说明：`docs/SETUP_WINDOWS.md`
- 发布约定：`docs/RELEASE.md`
- 模型打包命令：`npm run package:models -- 2026.04.28`
