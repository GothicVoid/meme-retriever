# 表情包检索器 Spec 总览

- 状态：approved
- 当前性：当前有效

本文件不再承担所有细行为定义，而是作为总览和索引入口使用。具体交互与实现口径请进入模块文档、feature spec 和 change note。

## 产品目标

- 面向随手收藏型用户提供本地记忆式找图能力
- 不要求用户维护文件名或系统性标签
- 为少量冷门角色或私有对象提供受控增强能力

## 当前模块结构

- 搜索模块：空查询首页、冷启动首页、非空搜索结果
- 图库管理模块：导入后的整理、修复和批量治理
- 私有角色库模块：高级检索增强能力，不是普通用户主路径

当前模块主文档：

- [搜索模块](./product/modules/search.md)
- [图库管理模块](./product/modules/library.md)
- [私有角色库模块](./product/modules/private-role-library.md)

## 当前关键 feature 文档

### 搜索与首页

- [FEAT-5.1.1 记忆式搜索功能](./product/features/FEAT-5.1.1-记忆式搜索功能.md)
- [FEAT-5.1.2 结果列表展示功能](./product/features/FEAT-5.1.2-结果列表展示功能.md)
- [FEAT-5.1.3 首页启动态功能](./product/features/FEAT-5.1.3-首页启动态功能.md)
- [FEAT-5.1.4 搜索体验增强功能](./product/features/FEAT-5.1.4-搜索体验增强功能.md)

### 图库管理

- [FEAT-5.2.1 添加图片功能](./product/features/FEAT-5.2.1-添加图片.md)
- [FEAT-5.2.2 删除图片功能](./product/features/FEAT-5.2.2-删除图片功能.md)
- [FEAT-5.2.3 清空图库内部重置能力](./product/features/FEAT-5.2.3-清空图库功能.md)
- [FEAT-5.2.4 图片列表功能](./product/features/FEAT-5.2.4-图片列表功能.md)

### 增强能力

- [FEAT-5.3.1 标签编辑功能](./product/features/FEAT-5.3.1-标签编辑功能.md)
- [5.3.2 标记增强与自动提示词](./5_3_2.md)
- [FEAT-5.3.2.2 标签分类与存储模型](./product/features/FEAT-5.3.2.2-标签分类与存储模型.md)
- [FEAT-5.3.2.3 标记提权规则](./product/features/FEAT-5.3.2.3-标记提权规则.md)
- [FEAT-5.4.1 详情页展示功能](./product/features/FEAT-5.4.1-详情页展示功能.md)
- [FEAT-5.5.1 基础设置功能（兼容入口）](./product/features/FEAT-5.5.1-基础设置功能.md)
- [FEAT-5.5.2 私有角色库维护工具](./product/features/FEAT-5.5.2-私有角色库维护工具.md)

### 信息架构与窗口

- [FEAT-5.6.1 导航与信息架构重构功能](./product/features/FEAT-5.6.1-导航与信息架构重构功能.md)
- [FEAT-5.6.2 聊天伴随型侧边栏工作台与窗口布局功能](./product/features/FEAT-5.6.2-聊天伴随型侧边栏工作台与窗口布局.md)
- [FEAT-5.6.3 导入与图库管理解耦及冷启动导入闭环功能](./product/features/FEAT-5.6.3-导入与图库管理解耦及冷启动导入闭环.md)
- [FEAT-设置模块优化与高级能力收口功能](./product/features/FEAT-设置模块优化与高级能力收口功能.md)

### 体验与设计规范

- [FEAT-5.7.1 高频使用体验与状态反馈功能](./product/features/FEAT-5.7.1-高频使用体验与状态反馈功能.md)
- [UI 设计规范](./ui-guidelines.md)

## 当前关键口径

- 记忆式搜索主路、结果分层和私有角色主路切换以 [`FEAT-5.1.1`](./product/features/FEAT-5.1.1-记忆式搜索功能.md) 为准
- 搜索结果展示以 [`FEAT-5.1.2`](./product/features/FEAT-5.1.2-结果列表展示功能.md) 为准
- 首页启动态、冷启动首页和普通首页切换规则以 [`FEAT-5.1.3`](./product/features/FEAT-5.1.3-首页启动态功能.md) 为准
- 搜索输入引导、失败反馈和历史复用以 [`FEAT-5.1.4`](./product/features/FEAT-5.1.4-搜索体验增强功能.md) 为准
- 冷启动首页导入行为以 [`FEAT-5.6.3`](./product/features/FEAT-5.6.3-导入与图库管理解耦及冷启动导入闭环.md) 与 [`FEAT-5.2.1`](./product/features/FEAT-5.2.1-添加图片.md) 为准
- 删除图片与批量删除行为以 [`FEAT-5.2.2`](./product/features/FEAT-5.2.2-删除图片功能.md) 为准
- 清空图库内部重置能力与保护约束以 [`FEAT-5.2.3`](./product/features/FEAT-5.2.3-清空图库功能.md) 为准
- 图库图片列表、分页加载与失效图片清理入口以 [`FEAT-5.2.4`](./product/features/FEAT-5.2.4-图片列表功能.md) 为准
- 标签编辑与单图标记维护以 [`FEAT-5.3.1`](./product/features/FEAT-5.3.1-标签编辑功能.md) 为准
- 标记对象与私有角色卡片的模型边界以 [`FEAT-5.3.2.2`](./product/features/FEAT-5.3.2.2-标签分类与存储模型.md) 为准
- 标记提权与搜索辅路关系以 [`FEAT-5.3.2.3`](./product/features/FEAT-5.3.2.3-标记提权规则.md) 为准
- 图片详情查看、异常态处理与快捷操作以 [`FEAT-5.4.1`](./product/features/FEAT-5.4.1-详情页展示功能.md) 为准
- 基础设置历史边界与兼容入口见 [`FEAT-5.5.1`](./product/features/FEAT-5.5.1-基础设置功能.md)
- 窗口双态与布局以 [`FEAT-5.6.2`](./product/features/FEAT-5.6.2-聊天伴随型侧边栏工作台与窗口布局.md) 为准
- 设置模块下线、私有角色库定位、调试与维护能力分层以 [`FEAT-设置模块优化与高级能力收口功能`](./product/features/FEAT-设置模块优化与高级能力收口功能.md) 为准
- 私有角色库入口层级、导航分层与维护工具以 [`FEAT-5.6.1`](./product/features/FEAT-5.6.1-导航与信息架构重构功能.md) 与 [`FEAT-5.5.2`](./product/features/FEAT-5.5.2-私有角色库维护工具.md) 为准
- 高频使用交互、快速预览和关键状态反馈以 [`FEAT-5.7.1`](./product/features/FEAT-5.7.1-高频使用体验与状态反馈功能.md) 为准
- 信息架构迁移与旧口径映射以 [`changes/`](./changes/) 下文档为准

## 阅读建议

1. 先读 [README.md](./README.md)
2. 再读模块文档
3. 最后按具体主题进入 feature spec
