# TalkShow — Tauri v2 + Svelte 基础框架设计

## 概述

TalkShow 是一个基于 Tauri v2 + Svelte + TypeScript 的工具型桌面应用。本文档描述最基础的项目框架搭建方案。

## 技术栈

| 层 | 技术 |
|---|---|
| 框架 | Tauri v2 |
| 前端 | Svelte + TypeScript |
| 构建 | Vite |
| 包管理 | npm |

## 项目结构

```
talkshow/
├── src-tauri/            # Rust 后端
│   ├── src/
│   │   ├── main.rs       # 入口
│   │   └── lib.rs        # 应用逻辑
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json   # Tauri 配置
│   ├── capabilities/     # 权限配置
│   └── icons/
├── src/                  # Svelte 前端
│   ├── app.html          # HTML 模板
│   ├── app.css           # 全局样式
│   └── lib/              # 共享组件
├── static/               # 静态资源
├── package.json
├── vite.config.js
├── svelte.config.js
└── tsconfig.json
```

## 关键配置

- **窗口**：默认 800x600，可调整大小，居中显示
- **权限**：最小权限原则，仅基础窗口操作
- **Rust 侧**：`main.rs` + `lib.rs` 骨架，预留命令扩展点
- **前端**：最小 Svelte 骨架

## 搭建方式

使用 `create-tauri-app` 官方脚手架生成项目骨架，确保标准结构和最新兼容性。

## 不包含（YAGNI）

- UI 组件库
- 状态管理
- 测试框架
- 路由库

以上按需在后续迭代中加入。
