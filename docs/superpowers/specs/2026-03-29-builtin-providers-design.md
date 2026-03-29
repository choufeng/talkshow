# 内置 Provider 管理

## 目标

为系统设定内置 Provider 项，这些项不能被用户删除，但可以重置为默认值。用户自建的 Provider 删除时需确认。

## 数据模型

`ProviderConfig` 接口不变，不添加新字段。

## BUILTIN_PROVIDERS 常量

前后端各维护一份完整的内置 Provider 默认数据，作为合并和重置的唯一数据源。

当前内置列表：

| id | type | name | endpoint | models |
|----|------|------|----------|--------|
| vertex | vertex | Vertex AI | https://aiplatform.googleapis.com/v1 | gemini-2.0-flash |
| dashscope | openai-compatible | 阿里云 | https://dashscope.aliyuncs.com/compatible-mode/v1 | qwen2-audio-instruct |

## 行为规则

### 内置 Provider

- **不可删除**：卡片右上角不显示删除按钮
- **可重置**：卡片右上角显示重置按钮（↺），点击弹出确认框，确认后用 `BUILTIN_PROVIDERS` 中对应 ID 的完整数据覆盖（endpoint、models 等全部恢复，api_key 清空）

### 非内置 Provider

- **可删除**：卡片右上角显示删除按钮（✕），点击弹出确认框，确认后删除

### 自动合并

应用启动加载用户配置时，遍历 `BUILTIN_PROVIDERS`，将用户配置中缺失的内置 Provider（按 ID 匹配）补到 `providers` 数组最前面。不覆盖已有的同 ID 项。

## 改动文件

### 前端 `src/lib/stores/config.ts`

1. 提取 `BUILTIN_PROVIDERS` 常量（从 store 初始值解耦）
2. 导出 `isBuiltinProvider(id: string)` 函数
3. 在 `load()` 方法中增加合并逻辑
4. 修复 vertex name: `"VTX"` → `"Vertex AI"`

### 后端 `src-tauri/src/config.rs`

1. 提取 `builtin_providers()` 函数，返回内置 Provider 列表
2. 在 `load_config()` 中增加合并逻辑
3. 修复 vertex name: `"VTX"` → `"Vertex AI"`

### UI `src/routes/models/+page.svelte`

1. 导入 `isBuiltinProvider`
2. 内置 Provider：渲染重置按钮 + 确认对话框
3. 非内置 Provider：渲染删除按钮 + 确认对话框
4. 新增 `handleResetProvider()` 函数
