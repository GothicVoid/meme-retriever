# Spec 系统索引

本目录采用“当前口径优先、历史可追踪、变更可回写”的组织方式。

## 阅读顺序

1. 先看 [总览索引](./spec.md)
2. 再看当前模块文档：
   - [搜索模块](./product/modules/search.md)
   - [图库管理模块](./product/modules/library.md)
   - [私有角色库模块](./product/modules/private-role-library.md)
3. 若需要理解一次具体能力或交互方案，再看对应 feature spec
4. 若发现新旧口径切换，优先查看 `changes/` 下的收口文档
5. 若需追历史方案，再看 `archive/` 或旧编号文档顶部提示

## 根目录编号文档说明

`specs/` 根目录中的历史 `5.x.x` 编号文档当前分为两类：

- 兼容入口：仅保留原编号路径，正文只包含“当前正文已迁移到哪里”的跳转说明
- 总览文档：少量仍保留在根目录，作为某一组 feature 的总览入口，例如 [`5_3_2.md`](./5_3_2.md)

`archive/` 与根目录兼容入口的区别：

- `archive/` 用于保留完整历史正文
- 根目录兼容入口用于维持旧链接稳定，不再承载历史正文

## 文档状态

正式 spec 统一使用以下状态：

- `draft`：草稿，不能作为开发依据
- `approved`：已确认，可指导开发
- `implemented`：已实现并验证
- `superseded`：已被替代，不再作为当前依据
- `archived`：历史归档，仅供追溯

`当前性` 统一使用：

- `当前有效`
- `部分失效`
- `历史兼容入口`
- `历史归档`

## 当前主口径

### 模块级

- 搜索与首页职责：[product/modules/search.md](./product/modules/search.md)
- 图库管理职责：[product/modules/library.md](./product/modules/library.md)
- 私有角色库定位：[product/modules/private-role-library.md](./product/modules/private-role-library.md)
- 设置模块优化与下线：[FEAT-设置模块优化与高级能力收口功能](./product/features/FEAT-设置模块优化与高级能力收口功能.md)

### 当前关键 feature

- 记忆式搜索主路与排序原则：[FEAT-5.1.1-记忆式搜索功能](./product/features/FEAT-5.1.1-记忆式搜索功能.md)
- 结果展示：[FEAT-5.1.2-结果列表展示功能](./product/features/FEAT-5.1.2-结果列表展示功能.md)
- 首页启动态与冷启动首页：[FEAT-5.1.3-首页启动态功能](./product/features/FEAT-5.1.3-首页启动态功能.md)
- 搜索体验增强：[FEAT-5.1.4-搜索体验增强功能](./product/features/FEAT-5.1.4-搜索体验增强功能.md)
- 导入协议与入口分流：[FEAT-5.2.1-添加图片](./product/features/FEAT-5.2.1-添加图片.md)
- 删除图片与批量删除：[FEAT-5.2.2-删除图片功能](./product/features/FEAT-5.2.2-删除图片功能.md)
- 清空图库内部重置能力：[FEAT-5.2.3-清空图库功能](./product/features/FEAT-5.2.3-清空图库功能.md)
- 图库图片列表与分页浏览：[FEAT-5.2.4-图片列表功能](./product/features/FEAT-5.2.4-图片列表功能.md)
- 标签编辑与单图标记维护：[FEAT-5.3.1-标签编辑功能](./product/features/FEAT-5.3.1-标签编辑功能.md)
- 标记对象与私有角色卡片模型：[FEAT-5.3.2.2-标签分类与存储模型](./product/features/FEAT-5.3.2.2-标签分类与存储模型.md)
- 标记提权规则：[FEAT-5.3.2.3-标记提权规则](./product/features/FEAT-5.3.2.3-标记提权规则.md)
- 图片详情查看与快捷操作：[FEAT-5.4.1-详情页展示功能](./product/features/FEAT-5.4.1-详情页展示功能.md)
- 基础设置兼容入口：[FEAT-5.5.1-基础设置功能](./product/features/FEAT-5.5.1-基础设置功能.md)
- 导航与信息架构分层：[FEAT-5.6.1-导航与信息架构重构功能](./product/features/FEAT-5.6.1-导航与信息架构重构功能.md)
- 窗口双态与布局：[FEAT-5.6.2-聊天伴随型侧边栏工作台与窗口布局](./product/features/FEAT-5.6.2-聊天伴随型侧边栏工作台与窗口布局.md)
- 冷启动导入闭环与图库管理解耦：[FEAT-5.6.3-导入与图库管理解耦及冷启动导入闭环](./product/features/FEAT-5.6.3-导入与图库管理解耦及冷启动导入闭环.md)
- 高频使用体验与状态反馈：[FEAT-5.7.1-高频使用体验与状态反馈功能](./product/features/FEAT-5.7.1-高频使用体验与状态反馈功能.md)
- 私有角色库维护工具：[FEAT-5.5.2-私有角色库维护工具](./product/features/FEAT-5.5.2-私有角色库维护工具.md)
- 设置模块优化与高级能力收口：[FEAT-设置模块优化与高级能力收口功能](./product/features/FEAT-设置模块优化与高级能力收口功能.md)

### 当前关键 change note

- 信息架构重构后的模块映射：[CHG-2026-04-20-信息架构迁移.md](./changes/CHG-2026-04-20-信息架构迁移.md)
- 首页导入口径收口：[CHG-2026-04-20-首页导入口径收口.md](./changes/CHG-2026-04-20-首页导入口径收口.md)

## 工作流

1. 先起草或更新 spec，再进入开发
2. 单次 spec 改动尽量只处理一个明确主题
3. 若后续迭代改变旧行为，必须同步回写受影响文档或将其标记为 `superseded`
4. 提交信息应带 spec 编号或 `docs(spec)` 前缀
5. 正文只表达当前有效口径，历史说明放到修订记录或 `changes/`
