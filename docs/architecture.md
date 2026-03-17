## 表情包检索器 架构设计文档

**版本** v0.1 | 对应 PRD v0.1

---

## 一、技术选型

| 层级 | 选型 | 理由 |
|---|---|---|
| 桌面框架 | Tauri v2 | 包体小（无需打包 Chromium）、Rust 后端适合 CPU 密集型 ML 推理 |
| 前端 | Vue3 + Vite + TypeScript | 用户指定；Pinia 管理状态，VueUse 提供 debounce 等工具 |
| 后端语言 | Rust | 性能、内存安全，`ort` crate 直接调用 ONNX Runtime |
| ML 模型 | Chinese-CLIP ViT-B/16（ONNX INT8 量化） | 原生支持中文语义，量化后单编码器约 100MB |
| OCR | PaddleOCR-ONNX（轻量版） | 提取图中台词/配文，支持中英文，有 ONNX 导出 |
| 数据库 | SQLite（via `rusqlite`） | 本地嵌入式，元数据 + 向量 blob 统一存储 |
| 向量检索 | 内存暴力搜索（纯 Rust） | 5000 张 × 512 维 ≈ 10MB，余弦搜索 < 10ms，无需 FAISS |
| 知识库 | 本地 JSON 文件 + KnowledgeBaseProvider trait | demo 阶段零依赖，trait 抽象保证后续无缝切换线上版本 |

---

## 二、架构总览

```
┌─────────────────────────────────────────────────────┐
│                   Vue3 前端                          │
│  SearchBar │ ImageGrid │ TagEditor │ Settings        │
└──────────────────────┬──────────────────────────────┘
                       │ Tauri IPC（invoke / event）
┌──────────────────────▼──────────────────────────────┐
│                  Rust 核心层                         │
│                                                     │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ SearchEngine│  │ IndexManager │  │FileWatcher │ │
│  │ (混合检索)  │  │ (入库流水线) │  │(目录监听)  │ │
│  └──────┬──────┘  └──────┬───────┘  └────────────┘ │
│         │                │                          │
│  ┌──────▼──────────▼──────────────────────┐ │
│  │              ML 推理层                          │ │
│  │  ClipTextEncoder │ ClipImageEncoder │ OcrEngine │ │
│  │  （ONNX Runtime via `ort`）                    │ │
│  └────────────────────────────────────────────────┘ │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │              存储层                          │   │
│  │  SQLite（元数据 + 向量 blob + 标签）         │   │
│  │  本地文件系统（缩略图 / 原图引用）           │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### 搜索链路

```
用户输入
    │
    ▼
┌─────────────────┐
│  KB Query       │  ← 新增：命中则扩展查询文本
│  Expander       │
└────────┬────────┘
         │ 扩展后文本
    ┌────▼────────────────────────────────┐
    │           并行检索                  │
    │  CLIP 语义检索  │  FTS 关键词检索   │
    │  (含 OCR 文本)  │  (标签 + OCR FTS) │
    └────────────────┬────────────────────┘
                     │
              加权合并 & 排序

```

---

## 三、数据模型

```sql
-- 图片主表
CREATE TABLE images (
    id            TEXT PRIMARY KEY,  -- UUID
    file_path     TEXT NOT NULL,
    file_name     TEXT NOT NULL,
    format        TEXT NOT NULL,     -- jpg/png/gif/webp
    width         INTEGER,
    height        INTEGER,
    added_at      INTEGER NOT NULL,  -- Unix timestamp
    use_count     INTEGER DEFAULT 0,
    thumbnail_path TEXT
);

-- 标签表
CREATE TABLE tags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id   TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    tag_text   TEXT NOT NULL,
    is_auto    INTEGER DEFAULT 0,    -- 0=用户手动, 1=OCR/自动
    created_at INTEGER NOT NULL
);

-- 向量表（CLIP 图像嵌入）
CREATE TABLE embeddings (
    image_id  TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
    vector    BLOB NOT NULL          -- 512 × f32, little-endian
);

-- OCR 文本表
CREATE TABLE ocr_texts (
    image_id  TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
    content   TEXT NOT NULL          -- 提取的原始文本，用于关键词索引
);
```

索引：
```sql
CREATE INDEX idx_tags_image_id ON tags(image_id);
CREATE INDEX idx_tags_text ON tags(tag_text);
CREATE VIRTUAL TABLE ocr_fts USING fts5(image_id, content); -- 全文检索
```

KB 文件说明（不进 SQLite，独立维护）：

```
app_data/
└── knowledge_base.json     # 本地 KB 文件，随版本更新，结构见下
```

```JSON
{
  "version": "1.0.0",
  "updated_at": "2025-01-01",
  "entries": [
    {
      "id": "meme_001",
      "type": "meme",
      "canonical": "蚌埠住了",
      "aliases": ["绷不住了", "蚌住了"],
      "description": "网络流行语，表示忍不住笑或无法忍受某事",
      "tags": ["搞笑", "崩溃", "无语"],
      "related_ids": []
    }
  ]
}
```

type 枚举：meme | character | work | concept

---

## 四、核心流程

### 4.1 图片入库流水线

```
用户添加图片
     │
     ▼
1. 复制/引用文件 → 生成缩略图（256px）
     │
     ▼
2. OCR 推理 → 提取图中文字 → 写入 ocr_texts + 更新 FTS 索引
     │
     ▼
3. CLIP 图像编码 → 512维向量 → 写入 embeddings
     │
     ▼
4. 写入 images 主表 → 通知前端进度事件
```

步骤 2、3 并行执行，单张图片总耗时约 200~500ms（CPU），入库期间不阻塞搜索。

### 4.2 混合检索流程

```
用户输入查询文本（debounce 300ms）
     │
     ▼
KB 查询扩展
   精确匹配 canonical + aliases
   命中 → 拼接扩展文本：canonical + description + tags.join(" ")
   未命中 → 原样传入
     │
     ▼
并行执行两路检索：
  ┌──────────────────────┐    ┌─────────────────────────┐
  │ 语义检索             │    │ 关键词检索              │
  │ CLIP 文本编码        │    │ FTS 全文搜索（OCR文本） │
  │ → 查询向量           │    │ + 标签精确匹配          │
  │ → 余弦相似度排序     │    │ → BM25 得分             │
  └──────────┬───────────┘    └────────────┬────────────┘
             │                             │
             └──────────┬──────────────────┘
                        ▼
              结果合并 & 加权评分
              score = 0.7 × semantic + 0.3 × keyword
              用户标签命中时 keyword 权重提升至 0.6
                        │
                        ▼
              按 score 降序，返回前 N 张
```

权重系数在设置中可调，初始值通过小样本测试标定。

---

## 五、前端模块设计

```
src/
├── components/
│   ├── SearchBar.vue       # 输入框 + 清空按钮，useDebounce(300ms)
│   ├── ImageGrid.vue       # 虚拟滚动网格，右键菜单
│   ├── ImageCard.vue       # 单图：点击复制、拖拽、右键
│   ├── TagEditor.vue       # 标签弹窗，自动补全
│   └── Toast.vue           # 轻提示
├── stores/
│   ├── search.ts           # 查询状态、结果列表、loading
│   ├── library.ts          # 图库列表、分组视图
│   └── settings.ts         # 持久化配置
├── composables/
│   ├── useSearch.ts        # 封装 invoke('search') + debounce
│   ├── useClipboard.ts     # 复制图片到剪贴板
│   └── useDragDrop.ts      # 拖拽导入 / 拖出到聊天框
└── views/
    ├── SearchView.vue      # 主界面（3.1）
    ├── LibraryView.vue     # 图库管理（3.2）
    └── SettingsView.vue    # 设置（3.5）
```

### IPC 命令清单

```typescript
// 搜索
invoke('search', { query: string, limit: number }) → SearchResult[]

// 图库管理
invoke('add_images', { paths: string[] }) → void
invoke('delete_image', { id: string }) → void
invoke('get_images', { sort, group, page }) → ImageMeta[]

// 标签
invoke('update_tags', { imageId: string, tags: string[] }) → void
invoke('get_tag_suggestions', { prefix: string }) → string[]

// 图片操作
invoke('copy_to_clipboard', { id: string }) → void
invoke('reveal_in_finder', { id: string }) → void
invoke('increment_use_count', { id: string }) → void
```

---

## 六、Rust 后端模块划分

```
src-tauri/src/
├── main.rs
├── commands/           # Tauri command handlers（薄层，只做参数校验和调用）
├── search/
│   ├── engine.rs       # 混合检索主逻辑，权重合并
│   ├── vector_store.rs # 内存向量索引，启动时从 SQLite 加载
│   └── keyword.rs      # FTS 查询封装
├── indexer/
│   ├── pipeline.rs     # 入库流水线，tokio 异步任务队列
│   ├── thumbnail.rs    # 缩略图生成（image crate）
│   └── ocr.rs          # PaddleOCR ONNX 推理封装
├── ml/
│   ├── clip.rs         # CLIP 文本/图像编码器，模型懒加载
│   └── tokenizer.rs    # BPE tokenizer（中文 CLIP 专用）
├── db/
│   ├── schema.rs       # 建表 / 迁移
│   └── repo.rs         # CRUD 封装
├── kb/
│   ├── provider.rs     # KnowledgeBaseProvider trait 定义
│   └── local.rs        # LocalKBProvider，读取 JSON 内容
└── config.rs           # 设置读写
```

**模型懒加载策略**：应用启动时只加载文本编码器（用于搜索），图像编码器在首次入库时加载并常驻内存，避免冷启动慢。

---

## 七、性能设计

| 目标 | 设计措施 |
|---|---|
| 搜索 < 500ms | 向量索引常驻内存；文本编码约 50ms，向量搜索 < 10ms，总链路 < 100ms |
| 入库不阻塞搜索 | 入库走独立 tokio 任务队列，与搜索线程隔离 |
| 索引 < 图片总大小 20% | 5000张：向量 10MB + OCR文本 ~5MB + 缩略图（256px）~50MB，远低于原图总大小 |
| 启动速度 | 向量索引启动时一次性从 SQLite 加载到内存，约 50ms |

---

## 八、目录结构（项目根）

```
meme-retriever/
├── src/                    # Vue3 前端
├── src-tauri/              # Rust 后端
│   ├── models/             # ONNX 模型文件（打包时嵌入）
│   │   ├── clip_text.onnx
│   │   ├── clip_image.onnx
│   │   └── ocr.onnx
│   └── src/
├── vite.config.ts
└── tauri.conf.json
```
