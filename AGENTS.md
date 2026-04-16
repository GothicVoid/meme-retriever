总是使用中文

# Repository Guidelines

## 项目结构与模块组织
`src/` 为 Vue 3 前端主目录：`components/` 放可复用组件，`views/` 放页面级视图，`stores/` 管理 Pinia 状态，`composables/` 封装共享逻辑，`test/` 存放前端测试。静态资源位于 `public/` 与 `src/assets/`。`src-tauri/` 为 Rust 后端，包含 `commands/`、`db/`、`indexer/`、`search/`、`kb/`、`ml/` 等核心模块，数据库迁移在 `src-tauri/migrations/`，集成测试在 `src-tauri/tests/`。需求与架构说明见 `specs/`。

## 构建、测试与开发命令
`npm run dev`：启动 Vite 前端开发环境。  
`npm run tauri dev`：启动完整桌面应用，联调前后端。  
`npm run build`：执行 `vue-tsc` 类型检查并构建前端产物。  
`npm run preview`：本地预览构建结果。  
`npm run lint`：检查 `src/` 下的 TypeScript 与 Vue 代码。  
`npm test`：一次性运行 Vitest 测试。  
`npm run test:watch`：监听模式运行前端测试。  
`cargo test --manifest-path src-tauri/Cargo.toml`：运行 Rust 后端测试。

## 编码风格与命名约定
遵循现有代码风格：Vue 模板与样式使用 2 空格缩进，TypeScript 使用双引号和分号，Vue 单文件组件采用 `<script setup lang="ts">`。组件与页面使用 PascalCase，例如 `SearchView.vue`、`ImageCard.vue`；组合式函数使用 `useXxx.ts`；状态模块与普通 TypeScript 文件使用 camelCase。Rust 模块、函数与文件名使用 `snake_case`。项目启用 ESLint，但未单独配置格式化工具，修改时以邻近文件风格为准。

## 测试指南
前端测试基于 Vitest + jsdom，公共测试初始化位于 `src/test/setup.ts`。新增测试统一放在 `src/test/`，命名采用 `*.test.ts`，并尽量体现功能范围，例如 `SearchView.pagination.test.ts`。后端测试位于 `src-tauri/tests/`，优先覆盖入库、搜索、删除、数据库迁移等完整流程。提交前至少运行与改动相关的前端或后端测试。

## 提交与 Pull Request 规范
Git 历史采用 Conventional Commits 风格，如 `feat: ...`、`fix(search): ...`。请保持单次提交聚焦，前缀优先使用 `feat`、`fix`、`docs`、`refactor`、`test`。提交信息使用中文写。对于进行了 spec 改动的提交，提交信息里要带上影响的 spec 范围，比如： `feat(5.3.1): 实现了xxx功能`。提交 PR 时应说明改动目的、影响范围、已执行的验证命令；若涉及界面修改，附截图或录屏；若对应需求文档或问题单，请一并链接。
