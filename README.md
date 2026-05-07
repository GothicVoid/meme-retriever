# meme-retriever

本地表情包越存越多，真正想发的时候却翻不到？  
`meme-retriever` 是一个帮你导入、整理并快速搜索本地表情包的桌面应用，面向“图片都已经在本地，但越来越难翻”的使用场景。你可以先把分散在不同文件夹里的表情包导入进来，再通过关键词或一句描述快速搜索。

## 适合谁

### 适合

- 表情包都在本地，分散在不同文件夹里，越存越难找
- 记得图里大概有什么字、角色或场景

### 不太适合

- 只想直接在线搜图的人
- 想把它当成聊天软件插件直接使用的人

## 获取方式

- 目前已提供 Windows 安装包

下载地址： [GitHub Releases](https://github.com/GothicVoid/meme-retriever/releases)

## 如何使用

安装完成后，你只需要先导入一次图库，后续就可以直接搜索和整理表情包。

1. 下载并安装应用

2. 首次打开，先导入图片  

<img src="./docs/images/README-import.gif" alt="导入演示" width="480" />

3. 等导入完成后，到搜索页输入关键词或一句描述开始找图  

<img src="./docs/images/README-search.gif" alt="搜索页演示" width="420" />

4. 如果想继续清理和整理图库，再进入图库管理页  

<img src="./docs/images/README-library.png" alt="图库管理页截图" width="480" />

## 使用边界

- 这是一个本地优先工具，不是在线图库
- 搜索效果依赖你已经导入的本地图片
- 图库为空时，应先导入图片，再开始搜索
- 当前不是聊天软件插件

---
# 开发说明

## 环境与依赖

- `Node.js >= 20 < 23`
- `Rust` 建议装rustup
- `Tauri` 前置环境

## 快速开始

配好环境后，在仓库根目录运行自带的初始化脚本。

### Windows：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

可选参数：

- `-SkipNpmInstall`
- `-SkipCargoFetch`
- `-ModelsUrl <url>`
- `-OrtUrl <url>`

### Linux：

```bash
bash scripts/setup-linux.sh
```

额外依赖：`curl`、`unzip`、`tar`、`sha256sum`

可选参数：

- `--skip-npm-install`
- `--skip-cargo-fetch`
- `--repo-root <path>`
- `--models-url <url>`
- `--ort-url <url>`

### 初始化脚本会：

- 安装 npm 依赖
- 预取 Rust 依赖
- 下载并解压模型到 `src-tauri/models/`
- 下载并提取 ONNX Runtime 到 `src-tauri/libs/`
- 校验模型和 runtime 的 SHA256（如 manifest 提供）

## 常用命令

```bash
# 前端开发
npm run dev

# 桌面联调
npm run tauri dev

# 类型检查 + 前端构建
npm run build

# 前端测试
npm test

# Rust 后端测试
cargo test --manifest-path src-tauri/Cargo.toml
```

## 文档导航

- 发布说明： [docs/RELEASE.md](./docs/RELEASE.md)
- 产品口径与主 Spec： [specs/README.md](./specs/README.md)
- 贡献与协作规范： [CONTRIBUTING.md](./CONTRIBUTING.md)

## 模型来源

- CLIP 模型：[`OFA-Sys/Chinese-CLIP`](https://github.com/OFA-Sys/Chinese-CLIP)
- OCR 模型与相关方案：[`RapidAI/RapidOCR`](https://github.com/RapidAI/RapidOCR)
