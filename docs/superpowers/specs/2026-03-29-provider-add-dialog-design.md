# Provider 添加弹窗设计

## 目标

在 Models 页面的 Providers 网格中，通过虚位卡片触发 Dialog 弹窗，实现添加新 Provider 的功能。

## 触发方式

在 Providers 网格（`grid grid-cols-2`）的最后一个位置放置一张虚位卡片。点击该卡片打开添加 Provider 的 Dialog 弹窗。

### 虚位卡片设计

- **样式**：虚线边框（`border-dashed`），背景颜色与普通卡片类似但略淡
- **布局**：居中显示 "+" 图标（Lucide `Plus`）和 "添加 Provider" 文字
- **交互**：hover 时背景加深，cursor-pointer

## 弹窗设计

使用 bits-ui `Dialog` 组件实现，保持与现有 UI 组件（select、tag-input）一致的封装风格。

### 新增组件

`src/lib/components/ui/dialog/index.svelte` — 基于 bits-ui Dialog 的通用弹窗组件。

### 表单字段

| 字段 | 组件 | 必填 | 说明 |
|------|------|------|------|
| Name | `<input>` | 是 | 显示名称（如 "阿里云"） |
| Type | `Select` | 是 | 从预定义列表选择 |
| ID | `<input>` | 是 | 根据 name 自动生成 slug，可手动修改 |
| Endpoint | `<input>` | 是 | API 端点 URL |

### Type 预定义选项

| 值 | 显示标签 |
|----|----------|
| `openai-compatible` | OpenAI Compatible |
| `stubbed` | Stubbed |
| `anthropic-compatible` | Anthropic Compatible |

### 表单行为

1. Name 输入时，ID 自动根据 name 生成 slug（小写、空格转连字符、去除特殊字符）
2. 用户可手动修改 ID
3. Type 选择后无联动（所有类型的表单字段相同）
4. 点击 "添加" 时校验所有必填字段，通过后将新 Provider push 到 `config.ai.providers` 并调用 `config.save()`
5. 点击 "取消" 或点击弹窗外部关闭弹窗，不保存

### 校验规则

- 所有字段不可为空
- ID 不可与已有 Provider 的 ID 重复
- ID 仅允许小写字母、数字、连字符

## 涉及文件

### 新增

- `src/lib/components/ui/dialog/index.svelte` — Dialog 通用组件

### 修改

- `src/routes/models/+page.svelte` — 虚位卡片 + 弹窗逻辑 + 添加处理函数

## 不涉及

- API Key 和 Models 的编辑（已有 inline 编辑能力）
- Provider 的编辑/更新功能
- 删除确认弹窗（后续迭代）
