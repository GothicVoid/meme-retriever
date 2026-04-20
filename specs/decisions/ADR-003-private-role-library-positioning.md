# ADR-003 私有角色库降级为受控高级能力

- 状态：accepted
- 当前性：当前有效
- 日期：2026-04-20

## 背景

早期知识库/角色库能力过于显眼，容易打断普通用户主路径，也让设置与导航膨胀。

## 决策

- 私有角色库不再作为普通用户一级导航
- 对外语义统一为“私有角色库”
- 默认只在设置的高级能力区、开发模式或受控入口中出现
- 普通用户主路径不要求理解该概念

## 当前主文档

- 模块层：[private-role-library.md](../product/modules/private-role-library.md)
- feature 层：[`FEAT-5.5.2-私有角色库维护工具.md`](../product/features/FEAT-5.5.2-私有角色库维护工具.md)、[`FEAT-5.6.1-导航与信息架构重构功能.md`](../product/features/FEAT-5.6.1-导航与信息架构重构功能.md)
