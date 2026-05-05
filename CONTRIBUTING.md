# 贡献指南

本文档面向准备参与 `meme-retriever` 开发的人，集中说明开发流程、Spec 约定、测试要求和提交规范。

## 开发前先看什么

默认采用“先确定 spec，再进行开发”的流程。

开始开发前，先按以下顺序确认依据：

1. 阅读 [specs/README.md](./specs/README.md)
2. 找到对应 `Spec ID`
3. 打开对应主 Spec，确认当前产品口径
4. 若改动找不到对应 spec，先补或收口 spec，再开始开发

## 什么时候先改 Spec

以下改动都应先更新对应 spec，再开始开发：

- 页面结构变化
- 入口、按钮、导航变化
- 状态切换变化
- 交互流程变化
- 用户可见能力边界变化
- 验收口径变化

以下情况通常可以不先改 spec：

- 纯实现重构
- 性能优化但不改变外部行为
- 内部清理
- 非用户可见的工程化调整

## 当前主 Spec ID

- `SPEC-SEARCH`：搜索、首页、结果展示与高频反馈
- `SPEC-LIBRARY`：导入、图库治理、删除与详情
- `SPEC-TAGGING`：标记增强与详情
- `SPEC-ROLELIB`：私有角色库维护
- `SPEC-IA-WINDOW`：信息架构与窗口
- `SPEC-SETTINGS`：设置模块优化与高级能力收口

主入口： [specs/README.md](./specs/README.md)

## 项目结构

前端主目录在 `src/`：

- `components/`：可复用组件
- `views/`：页面级视图
- `stores/`：Pinia 状态
- `composables/`：共享逻辑
- `test/`：前端测试

桌面与后端主目录在 `src-tauri/`：

- `commands/`：Tauri 命令入口
- `db/`：数据库相关
- `indexer/`：索引流程
- `search/`：搜索能力
- `kb/`：角色库 / 知识库相关
- `ml/`：模型推理相关
- `migrations/`：数据库迁移
- `tests/`：后端集成测试

其他重要目录：

- `docs/`：环境、发布、使用说明
- `specs/`：需求与架构文档

## 环境与资源

开始开发前，先准备以下内容：

- Node.js `>= 20 < 23`
- Rust stable toolchain
- Tauri 开发环境
- 模型资源
- ONNX Runtime 动态库

相关文档：

- [docs/SETUP.md](./docs/SETUP.md)
- [docs/SETUP_WINDOWS.md](./docs/SETUP_WINDOWS.md)
- [models-manifest.json](./models-manifest.json)
- [runtime-manifest.json](./runtime-manifest.json)

## 常用命令

```bash
# 前端开发
npm run dev

# 桌面联调
npm run tauri dev

# 类型检查 + 前端构建
npm run build

# 前端 Lint
npm run lint

# 前端测试
npm test

# 前端测试（监听模式）
npm run test:watch

# Rust 后端测试
cargo test --manifest-path src-tauri/Cargo.toml
```

## 编码风格

- Vue 模板与样式使用 2 空格缩进
- TypeScript 使用双引号和分号
- Vue 单文件组件采用 `<script setup lang="ts">`
- 组件与页面使用 PascalCase，例如 `SearchView.vue`
- 组合式函数使用 `useXxx.ts`
- 状态模块与普通 TypeScript 文件使用 camelCase
- Rust 模块、函数与文件名使用 `snake_case`
- 修改时优先遵循邻近文件的现有风格

项目启用了 ESLint，但没有单独配置格式化工具。提交前请至少确保改动代码与周边风格一致，并通过相关检查。

## 测试要求

前端测试基于 `Vitest + jsdom`，初始化位于 `src/test/setup.ts`。  
后端测试位于 `src-tauri/tests/`。

新增或修改功能时：

- 前端改动优先补 `src/test/*.test.ts`
- 后端改动优先覆盖入库、搜索、删除、迁移等完整流程
- 提交前至少运行与改动相关的测试

最低建议：

- 前端改动：`npm test`
- 后端改动：`cargo test --manifest-path src-tauri/Cargo.toml`

## 提交规范

Git 历史采用 Conventional Commits 风格，优先使用：

- `feat`
- `fix`
- `docs`
- `refactor`
- `test`

提交信息使用中文，且应保持单次提交聚焦。

涉及行为变更时，提交信息必须带对应 `Spec ID`，例如：

```text
feat(spec:SPEC-SEARCH): 实现首页继续使用区
fix(spec:SPEC-LIBRARY): 修复冷启动导入后列表未刷新
```

若一个提交同时影响多个 spec，优先拆分提交；确实无法拆时，在提交信息里列出主要 spec。

## Pull Request 要求

PR 描述建议至少包含：

- 改动目的
- 影响范围
- 对应 Spec ID 或 spec 链接
- 已执行的验证命令
- 如涉及界面修改，附截图或录屏

若改动对应需求文档、问题单或讨论记录，也应一并链接。
