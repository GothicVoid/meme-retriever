# meme-retriever

本地表情包越存越多，真正想发的时候却翻不到？  
`meme-retriever` 是一个帮你管理和查找本地表情包的桌面应用。你可以先把散落在各个文件夹里的图片导进来，再用搜索快速找到想发的那一张。

目前仅支持 Windows 平台。  
下载： [GitHub Releases](https://github.com/GothicVoid/meme-retriever/releases)

如果你经常遇到这些问题，这个工具会更适合你：

- 表情包都在本地，分散在不同文件夹里，越存越难找
- 记不清文件名，只记得大概是什么场景、情绪或梗
- 想先把图找出来，用的时候顺手整理一下图库

### 适合谁

- 经常使用本地表情包，想更快找到图的人
- 收藏了很多聊天图、梗图、反应图的人
- 想把“找图”和“整理图库”放在一个工具里完成的人

不太适合：

- 只想直接在线搜图的人
- 想把它当成聊天软件插件直接使用的人

### 如何开始

1. 从发布页下载并安装应用
2. 首次打开后先导入你的本地图片
3. 打开搜索页，用关键词或一句描述开始找图

详细使用说明见 [docs/USER_GUIDE.md](./docs/USER_GUIDE.md)。

### 界面预览

搜索页：

可以用关键词找图，也可以直接输入一句更自然的描述。

<img src="./docs/images/README-search.gif" alt="搜索页截图" width="600" />

图库管理页：

适合查看已经导入的内容，也方便继续删除、清理和整理图库。

<img src="./docs/images/README-library.png" alt="图库管理页截图" width="600" />

### 主要能力

- 把分散在本地文件夹里的表情包导入进来，集中管理
- 用关键词或一句描述快速找图
- 在图库管理页删除不想要的图片，慢慢把图库整理干净
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
