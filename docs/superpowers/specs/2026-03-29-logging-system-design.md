# TalkShow 日志系统设计规格

## 背景

当前 TalkShow 应用缺少结构化日志系统。Rust 后端使用 `println!/eprintln!` 输出到控制台，前端使用 `console.error()` 记录错误。模型连通性测试失败时，用户只能看到 `verified.status === 'error'` 和一条简短的 `message`，无法获取详细的错误信息（HTTP 状态码、响应体、请求耗时等）。

本设计引入一套统一的日志系统，不仅记录模型连通性测试日志，还支持未来所有模块的日志输出，并在前端提供可浏览、可过滤的日志页面。

## 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 存储方式 | 独立日志文件 | 最轻量，与项目无数据库的架构一致 |
| 文件格式 | JSON Lines (.jsonl) | 结构化数据，前端解析零成本，支持任意 meta 字段 |
| 文件轮转 | 每次启动一个文件 | 便于按会话查找问题 |
| 自动清理 | 保留最近 10 个文件 | 控制磁盘占用，约一周的使用记录 |
| 日志分类 | 按功能模块 | 便于按业务场景过滤 |
| 前端功能 | 查看 + 简单过滤 | 满足诊断需求，不做过度设计 |

## 后端设计

### 新增文件

`src-tauri/src/logger.rs` — 日志模块

### 数据结构

```rust
#[derive(Serialize, Clone)]
pub struct LogEntry {
    pub ts: String,                        // ISO 8601，精确到毫秒，带时区
    pub module: String,                    // "connectivity" | "recording" | "ai" | "system"
    pub level: String,                     // "info" | "warn" | "error"
    pub msg: String,                       // 人类可读的消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,   // 可选的结构化附加信息
}

pub struct Logger {
    log_dir: PathBuf,
    current_file: Mutex<File>,
    current_filename: String,
}
```

### 核心 API

- `Logger::new(app_data_dir: &Path) -> Result<Self>` — 创建 `logs/` 目录，打开新文件，执行自动清理
- `logger.log(module, level, msg, meta)` — 追加一行 JSON 到当前文件
- 便捷方法：`logger.info(module, msg, meta)` / `logger.warn(...)` / `logger.error(...)`

### 文件命名与轮转

- 文件名格式：`talkshow-2026-03-29_14-30-00.jsonl`（启动时的时间戳）
- 存放路径：`{app_data_dir}/logs/`
- 初始化时扫描 `logs/` 目录，按修改时间排序，删除超出 10 个的旧文件

### Tauri 命令

| 命令 | 参数 | 返回值 | 用途 |
|------|------|--------|------|
| `get_log_sessions` | 无 | `Vec<LogSession>` | 获取历史日志文件列表 |
| `get_log_content` | `session_file: Option<String>` | `Vec<LogEntry>` | 获取指定会话的日志内容（默认当前会话） |

```rust
#[derive(Serialize)]
pub struct LogSession {
    pub filename: String,      // "talkshow-2026-03-29_14-30-00.jsonl"
    pub size_bytes: u64,       // 文件大小
    pub entry_count: usize,    // 日志条目数
    pub first_ts: Option<String>,  // 首条日志时间戳
    pub is_current: bool,      // 是否为当前会话
}
```

### 模块分类

| module 值 | 用途 | 初期调用点 |
|---|---|---|
| `connectivity` | 模型连通性测试 | `test_model_connectivity` 命令 |
| `recording` | 录音相关 | `recording.rs` 录音开始/完成/失败 |
| `ai` | AI 请求处理 | `ai.rs` 请求发送/响应/错误 |
| `system` | 应用系统事件 | 启动/关闭/配置变更 |

### Logger 集成方式

Logger 作为 Tauri managed state 注册：

```rust
// lib.rs setup 中
let logger = Logger::new(&app_data_dir)?;
app_handle.manage(logger);
```

在任何 Tauri 命令中通过 `app_handle.state::<Logger>()` 获取使用。

### 改造 test_model_connectivity

当前流程保持不变，在关键节点追加日志记录：

1. **开始测试** → `logger.info("connectivity", "开始测试模型连通性", {provider_id, model_name})`
2. **测试成功** → `logger.info("connectivity", "测试成功", {provider_id, model_name, latency_ms, response_summary})`
3. **测试失败** → `logger.error("connectivity", "测试失败", {provider_id, model_name, error, http_status, response_body, latency_ms})`

前端测试失败后可引导用户前往日志页查看详情。

## 前端设计

### 新增路由

`/logs` — `src/routes/logs/+page.svelte`

### 侧边栏更新

`src/routes/+layout.svelte` 新增第四个导航项：
- 图标：`ScrollText`（来自 lucide-svelte）
- 标签：「日志」
- 路由：`/logs`

### 页面结构

#### 顶部工具栏

- **会话切换**：当前会话 / 历史记录（类似 tab 切换样式）
- **模块过滤标签**：全部 / connectivity / recording / ai / system
- **右侧信息**：当前文件时间范围 · 条目数

#### 当前会话视图

日志条目列表，每行包含：
- 时间戳（`YYYY-MM-DD HH:mm:ss.SSS`，灰色列）
- 级别图标（info ● 蓝色 / warn ● 黄色 / error ● 红色）
- 模块名（带颜色区分：connectivity 蓝色 / recording 紫色 / ai 琥珀色 / system 灰色）
- 消息内容（错误消息红色高亮）
- 右侧 meta 摘要（如 `openai / gpt-4o · 1657ms`）

使用等宽字体（`font-mono`）呈现日志列表。

#### 历史记录视图

文件列表，每项显示：
- 会话时间（从文件名解析）
- 文件大小
- 条目数
- 点击后展开显示该会话的日志内容（复用当前会话的日志列表组件）

### 前端数据流

```
页面加载
  ├── 默认加载当前会话
  │     └── invoke('get_log_content', { session_file: null })
  │           → 渲染日志列表
  ├── 模型页测试完成 → 自动刷新日志
  │     └── 页面可见时 invoke('get_log_content') 重新加载
  └── 切换到历史记录
        └── invoke('get_log_sessions')
              → 渲染文件列表
              → 点击文件 → invoke('get_log_content', { session_file })
```

### 前端过滤逻辑

前端侧过滤（不增加后端复杂度）：
- 模块过滤：`entries.filter(e => e.module === selectedModule)`
- 客户端过滤对桌面应用的日志量来说性能完全足够

## 文件变更清单

| 操作 | 文件 | 说明 |
|------|------|------|
| 新增 | `src-tauri/src/logger.rs` | 日志模块（Logger 结构体、文件管理、日志写入） |
| 修改 | `src-tauri/src/lib.rs` | 注册 Logger state、新增 2 个 Tauri 命令、改造 test_model_connectivity 添加日志 |
| 修改 | `src-tauri/Cargo.toml` | 无需新增依赖（已使用 serde_json、chrono 可用于时间戳格式化） |
| 新增 | `src/routes/logs/+page.svelte` | 日志页面 |
| 修改 | `src/routes/+layout.svelte` | 侧边栏新增日志导航项 |

## 未来扩展

- `recording` 模块：在录音开始/完成/失败时记录日志
- `ai` 模块：在 AI 请求发送/响应/错误时记录日志
- `system` 模块：应用启动/关闭/配置变更时记录日志
- 这些模块的日志接入只需在对应代码位置调用 `logger.info/warn/error` 即可，无需修改日志框架
