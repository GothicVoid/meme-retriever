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

**MVP 已完整实现**，所有核心功能已上线，53 个 Rust 测试 + 前端测试全部通过。

**已完成功能：**
- ✅ 混合检索引擎（语义 + 关键词加权合并）
- ✅ CLIP 真实推理（文本/图像编码，ONNX Runtime）
- ✅ OCR 文本提取（PaddleOCR-ONNX）
- ✅ 入库流水线（缩略图 + 进度条）
- ✅ 知识库查询扩展
- ✅ FTS5 全文搜索 + 标签系统
- ✅ 全部 9 个 IPC 命令
- ✅ 前端 UI（搜索/图库/设置/调试面板）
- ✅ 批量添加文件夹图片

**重要修复（commit 32fc981 + 6b01f34）：**
- 修复 CLIP 图像预处理参数错误（使用 CLIP 专用归一化值，非 ImageNet）
- 修复搜索流程的三个准确性问题
- 已有图片需通过设置页"重新生成图像索引"按钮重新计算 embedding

**模型文件路径**（通过环境变量 `CLIP_MODEL_DIR` 配置，默认 `./models`）：
- `models/clip_text.onnx`：Chinese-CLIP ViT-B/16 文本编码器
- `models/clip_image.onnx`：Chinese-CLIP ViT-B/16 图像编码器
- `models/ocr.onnx`：PaddleOCR 文本识别模型
- `models/vocab.txt`：BERT tokenizer 词表（CLS=101, SEP=102, PAD=0）

**ONNX Runtime 配置：**
- 需设置环境变量 `ORT_DYLIB_PATH` 指向 `libonnxruntime.so`
- 测试时示例路径：`/home/void/projects/Chinese-CLIP/venv/lib/python3.12/site-packages/onnxruntime/capi/libonnxruntime.so.1.24.4`
