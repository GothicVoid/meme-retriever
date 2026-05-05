# meme-retriever

本地表情包太多，总是找不到想发的那张？  
`meme-retriever` 是一个本地表情包检索桌面应用，支持导入、搜索和图库整理。

目前仅支持 Windows 平台。  
下载： [GitHub Releases](https://github.com/GothicVoid/meme-retriever/releases)

如果你经常遇到这些问题，这个工具就是给你用的：

- 表情包都在本地文件夹里，但越来越难找
- 记不清文件名，只记得图的大概意思
- 想边找图边把图库整理干净

### 适合谁

- 经常使用本地表情包的人
- 收藏了很多聊天图、梗图、反应图的人
- 想把“找图”和“整理图库”放在一个工具里完成的人

不太适合：

- 只想直接在线搜图的人
- 想把它当成聊天软件插件直接使用的人

### 如何开始

1. 从发布页下载并安装应用
2. 首次打开后先导入你的本地图片
3. 在搜索页输入关键词或一句描述开始找图

详细使用说明见 [docs/USER_GUIDE.md](./docs/USER_GUIDE.md)。

### 界面预览

搜索页：

支持用关键词，或直接输入一句更自然的描述来找图。

<img src="./docs/images/README-search.png" alt="搜索页截图" width="760" />

图库管理页：

适合查看已导入内容，并继续做删除、清理和整理。

<img src="./docs/images/README-library.png" alt="图库管理页截图" width="760" />

### 主要能力

- 把本地表情包文件夹导入进来统一管理
- 用关键词或一句描述快速找图
- 在图库管理页删除不想要的图片，清理和整理图库
- 按需使用标记增强等更进阶的能力

---

以下内容面向开发者与仓库协作者。

### 项目定位

`meme-retriever` 是一个本地优先的表情包工作台，不依赖在线图库。当前产品主路径只有两条：

- `搜索`
- `图库管理`

更完整的产品口径、能力边界和主 Spec 说明见 [specs/README.md](./specs/README.md)。

### 技术栈

前端：

- `Vue 3`
- `TypeScript`
- `Pinia`
- `Vite`

桌面与后端：

- `Tauri 2`
- `Rust`
- `SQLite / sqlx`
- `ONNX Runtime`
- `ort`

测试：

- `Vitest + jsdom`
- `cargo test`

### 目录结构

```text
.
├─ src/                  # Vue 3 前端
│  ├─ components/        # 可复用组件
│  ├─ views/             # 页面级视图
│  ├─ stores/            # Pinia 状态
│  ├─ composables/       # 共享逻辑
│  └─ test/              # 前端测试
├─ public/               # 静态资源
├─ src/assets/           # 前端资源
├─ src-tauri/            # Rust / Tauri 后端
│  ├─ commands/          # Tauri 命令入口
│  ├─ db/                # 数据库相关
│  ├─ indexer/           # 索引流程
│  ├─ search/            # 搜索能力
│  ├─ kb/                # 知识库 / 角色库相关
│  ├─ ml/                # 模型推理相关
│  ├─ migrations/        # 数据库迁移
│  └─ tests/             # 后端集成测试
├─ docs/                 # 环境、发布、使用说明
├─ specs/                # 需求与架构规范
├─ models-manifest.json  # 模型资产约定
└─ runtime-manifest.json # Runtime 资产约定
```

### 环境要求

开发环境需要：

- `Node.js >= 20 < 23`
- `Rust` stable toolchain
- 可用的 Tauri 开发环境
- 模型文件与 ONNX Runtime 动态库

环境准备、资源约定和平台差异说明见：

- [docs/SETUP.md](./docs/SETUP.md)
- [docs/SETUP_WINDOWS.md](./docs/SETUP_WINDOWS.md)
- [models-manifest.json](./models-manifest.json)
- [runtime-manifest.json](./runtime-manifest.json)

### 快速开始

开发者最短路径：

1. 安装依赖：`npm install`
2. 准备模型与运行时资源
3. 启动桌面联调：`npm run tauri dev`

资源初始化和平台细节不要直接猜，按文档执行：

- [docs/SETUP.md](./docs/SETUP.md)
- [docs/SETUP_WINDOWS.md](./docs/SETUP_WINDOWS.md)

### 常用命令

```bash
# 前端开发
npm run dev

# 桌面联调
npm run tauri dev

# 类型检查 + 前端构建
npm run build

# 本地预览构建结果
npm run preview

# 前端 Lint
npm run lint

# 前端测试
npm test

# 前端测试（监听模式）
npm run test:watch

# Rust 后端测试
cargo test --manifest-path src-tauri/Cargo.toml

# 打包模型资产
npm run package:models -- 2026.04.28
```

### 文档导航

- 最终用户使用说明： [docs/USER_GUIDE.md](./docs/USER_GUIDE.md)
- 发布流程： [docs/RELEASE.md](./docs/RELEASE.md)
- 产品口径与主 Spec： [specs/README.md](./specs/README.md)
- 环境初始化： [docs/SETUP.md](./docs/SETUP.md)
- Windows 初始化： [docs/SETUP_WINDOWS.md](./docs/SETUP_WINDOWS.md)
- 贡献与协作规范： [CONTRIBUTING.md](./CONTRIBUTING.md)

### 参与开发

开发前请先看 [specs/README.md](./specs/README.md) 确认当前产品口径。  
协作流程、提交规范、测试要求和 Spec 驱动开发约定见 [CONTRIBUTING.md](./CONTRIBUTING.md)。
