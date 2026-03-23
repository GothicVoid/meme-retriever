# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

本地表情包语义检索工具，采用 Tauri 2 + Vue 3 + Rust 技术栈。用户可通过自然语言搜索本地图片，支持语义检索（Chinese-CLIP）和关键词检索（FTS5 + 标签）混合排序。

## 常用命令

```bash
# 启动前端开发服务器（仅前端热更新）
npm run dev

# 启动完整 Tauri 开发环境（前端 + Rust 后端）
npm run tauri dev

# 构建生产版本
npm run tauri build

# 仅检查 TypeScript 类型
npm run build   # 内部执行 vue-tsc --noEmit && vite build

# Rust 后端单独编译检查
cd src-tauri && cargo check
cd src-tauri && cargo build

# 运行 Rust 测试
cd src-tauri && cargo test

# 运行单个 Rust 测试
cd src-tauri && cargo test <test_name>
```

## 架构概览

### 分层结构

```
Vue3 前端 (TypeScript)
    ↕ Tauri IPC (invoke/event)
Rust 核心层
    ├── commands/     IPC 命令处理（前端调用入口）
    ├── search/       混合检索引擎
    ├── indexer/      图片入库流水线
    ├── ml/           ONNX 推理（CLIP + OCR）
    ├── kb/           知识库查询扩展
    └── db/           SQLite 连接池与迁移
```

### 搜索链路

用户输入 → KB 查询扩展 → 并行执行：
- 语义检索：CLIP 文本编码 → 余弦相似度排序
- 关键词检索：FTS5 全文搜索 + 标签精确匹配

加权合并：`score = 0.7 × semantic + 0.3 × keyword`

### 入库流水线

文件复制/引用 → 生成缩略图（256px） → 并行执行 OCR 推理 + CLIP 图像编码 → 写入 SQLite → 通知前端进度

### 数据存储

SQLite 单文件，4 张表：
- `images`：图片元数据（UUID 主键）
- `tags`：标签（区分用户手动 vs 自动生成）
- `embeddings`：CLIP 向量（512 × f32 blob）
- `ocr_texts`：OCR 提取文本
- `ocr_fts`：FTS5 虚拟表，用于全文搜索

迁移文件：`src-tauri/migrations/0001_init.sql`

### 前端状态管理

三个 Pinia store：
- `search.ts`：查询状态与结果列表
- `library.ts`：图库列表与分组视图
- `settings.ts`：持久化配置（localStorage）

三个 composables：
- `useSearch.ts`：封装 `invoke('search')` + 300ms debounce
- `useClipboard.ts`：复制图片到剪贴板
- `useDragDrop.ts`：拖拽导入/拖出

### IPC 命令接口

前端通过 `invoke()` 调用的 Rust 命令（定义在 `src-tauri/src/commands/mod.rs`）：

| 命令 | 参数 | 返回 |
|------|------|------|
| `search` | query, limit | `Vec<SearchResult>` |
| `add_images` | paths | void |
| `delete_image` | id | void |
| `get_images` | page | `Vec<ImageMeta>` |
| `update_tags` | imageId, tags | void |
| `get_tag_suggestions` | prefix | `Vec<String>` |
| `copy_to_clipboard` | id | void |
| `reveal_in_finder` | id | void |
| `increment_use_count` | id | void |

## 关键技术细节

- **ML 模型**：Chinese-CLIP ViT-B/16（ONNX INT8 量化）+ PaddleOCR-ONNX，通过 `ort` crate 加载，使用 `OnceCell` 懒加载
- **向量检索**：内存中暴力余弦相似度（`src-tauri/src/search/vector_store.rs`），无外部向量数据库
- **知识库**：JSON 文件（`app_data/knowledge_base.json`），存储表情包 canonical 名称、别名、标签，用于查询扩展
- **前端路径别名**：`@` → `src/`
- **Tauri 开发端口**：1420（dev server），1421（HMR）

## 项目当前状态

MVP 已完整实现，共 53 个 Rust 测试 + 24 个前端测试全部通过。

**已完成：**
- 搜索引擎（混合检索、KB 扩展、加权合并）、入库流水线、FTS 关键词搜索
- 数据库 CRUD、向量存储、缩略图生成
- 全部 9 个 IPC 命令、前端 UI（搜索/图库/设置）

**剩余 TODO（需要 ONNX 模型文件）：**
- `src-tauri/src/ml/clip.rs` L68/L77：`run_text_inference` / `run_image_inference` 真实推理
- `src-tauri/src/indexer/ocr.rs` L30：`run_inference` OCR 真实推理
- `src-tauri/src/commands/mod.rs` L179：`copy_to_clipboard` 平台特定实现

模型不存在时自动 fallback 到 mock 向量（基于文本长度/文件路径 seed），搜索可运行但语义不准确。

**模型文件路径**（通过环境变量 `CLIP_MODEL_DIR` 配置，默认 `./models`）：
- `models/clip_text.onnx`：Chinese-CLIP ViT-B/16 文本编码器（ONNX INT8）
- `models/clip_image.onnx`：Chinese-CLIP ViT-B/16 图像编码器（ONNX INT8）
- `models/ocr.onnx`：PaddleOCR 文本识别模型
- `models/bpe_simple_vocab_16e6.txt`：CLIP BPE 分词表
