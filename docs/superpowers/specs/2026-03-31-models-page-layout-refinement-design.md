# 模型页面布局优化设计

日期：2026-03-31

## 背景

模型页面（`/models`）当前存在两个问题：

1. **"转写服务"与 Providers 缺乏层级区隔**：两者是平级的 section，但逻辑上"转写服务"调用 Provider 提供的模型，不是对等关系。且"转写服务"所在的区域未来会扩展为包含翻译服务等更大的"AI 服务"区块。
2. **SenseVoice 本地模型操作冗余**：卡片内有两个删除入口（模型状态区的"删除模型"按钮 + Models 标签区的 ✕ 按钮），且本地模型不应支持添加、测试等操作。

## 设计方案

### 变更 1：AI 服务容器分组

**当前结构**：

```
转写服务          ← section，rounded-xl border 卡片
  转写模型 / 润色开关 / 润色模型

Providers         ← section，grid 布局
  Provider 卡片 1 | Provider 卡片 2
```

**目标结构**：

```
┌─ AI 服务 ──────────────────────────────────────┐
│                                                  │
│  转写服务                                        │ ← 子标题（无卡片边框）
│  转写模型: [选择模型]                             │
│  启用润色  [开关]                                 │
│  润色模型: [选择模型]                             │
│                                                  │
│  （未来扩展：翻译服务等）                          │
└──────────────────────────────────────────────────┘

Providers                                         ← 独立 section
  Provider 卡片 1 | Provider 卡片 2
```

**实现细节**：

- 外层容器：`rounded-xl border border-border bg-background-alt p-5`，`mb-10`
- 容器标题"AI 服务"：`text-sm font-semibold text-foreground mb-4`
- 内部子标题"转写服务"：保持当前 section 标题样式 `text-xs text-muted-foreground uppercase tracking-wider mb-3`
- 移除"转写服务"原有的 `rounded-xl border bg-background-alt p-5` 卡片包裹
- Providers section 保持不变

### 变更 2：SenseVoice 卡片操作精简

**移除的操作**：

- Models 标签中模型名称旁的 `✕` 删除按钮 → 对 `sensevoice` 类型条件跳过渲染
- Models 标签区的「+ 添加模型」按钮 → 对 `sensevoice` 类型条件跳过
- Models 标签区的「测试全部」按钮 → 对 `sensevoice` 类型条件跳过

**保留的操作**：

- 模型状态区的「删除模型」按钮（仅在模型已就绪时显示）→ 保留，这是管理本地模型文件下载/删除生命周期的唯一入口

**SenseVoice 卡片 Models 区域最终效果**：

- 只读显示模型名称 + 能力标签（如 `T` 标记）
- 可点击模型标签进行连通性测试（保留 `onclick={() => testModel(...)}`）
- 不显示 `✕` 删除、`+ 添加模型`、`测试全部`

## 涉及文件

- `src/routes/models/+page.svelte`：页面布局和条件渲染逻辑

## 不涉及

- 后端逻辑
- 配置数据结构
- 新增组件
