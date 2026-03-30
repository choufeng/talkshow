# TalkShow

> Let your thoughts flow.

TalkShow 是一款 macOS 桌面语音转文字应用。按下全局快捷键即可录音，松开后自动完成语音转写、文本后处理，并将结果直接粘贴到你当前正在使用的应用中——整个过程无需切换窗口，完全在后台完成。

```
全局快捷键录音 → 音频采集(FLAC/WAV) → AI 语音转写 → Skills 后处理 → 自动粘贴到当前应用
```

## 功能特性

- **全局快捷键录音** — 按下快捷键即可开始/停止录音，附带悬浮录音指示器
- **多 AI Provider 支持** — Vertex AI (Gemini)、阿里云 DashScope (Qwen)、SenseVoice 本地模型
- **本地离线转写** — 内置 SenseVoice-Small ONNX 模型，无需联网即可转写（中/英/日）
- **Skills 后处理管线** — 自动去除语气词、修正错别字、润色文本等，支持自定义 Skill
- **一键粘贴** — 转写结果自动写入剪贴板并模拟 `Cmd+V` 粘贴
- **系统托盘** — 后台常驻，通过托盘图标管理窗口和查看状态
- **深色/浅色主题** — 支持跟随系统主题自动切换

## 截图

<!-- TODO: 添加应用截图 -->
<!--
建议添加：
- 主界面截图
- 录音指示器截图
- 设置页面截图
- 实际使用 GIF 动图
-->

## 快速开始

### 前置要求

| 依赖 | 说明 |
|------|------|
| [Rust](https://www.rust-lang.org/tools/install) | Rust 工具链（edition 2024） |
| [Node.js](https://nodejs.org/) | >= 18 |
| [Xcode Command Line Tools](https://developer.apple.com/xcode/) | macOS 开发环境 |
| [FLAC](https://xiph.org/flac/)（可选） | 安装后自动启用 FLAC 压缩，减小音频体积 |

安装 FLAC（可选）：

```bash
brew install flac
```

### 安装与运行

```bash
# 克隆仓库
git clone https://github.com/<your-username>/talkshow.git
cd talkshow

# 安装前端依赖
npm install

# 启动开发模式
npm run tauri dev
```

首次启动后，应用会出现在系统托盘中。点击托盘图标打开设置窗口。

## 使用方式

### 录音流程

1. **开始录音** — 按下录音快捷键（默认 `Ctrl+\`），屏幕底部出现录音指示器
2. **停止录音** — 再次按下快捷键停止录音，指示器切换为"处理中"
3. **自动粘贴** — 转写完成后，结果自动粘贴到当前前台应用

你也可以按 `Esc` 取消当前录音。

### 快捷键

| 功能 | 默认快捷键 | 说明 |
|------|-----------|------|
| 显示/隐藏主窗口 | `Ctrl+Shift+'` | 在任何应用中切换 TalkShow 窗口 |
| 开始/停止录音 | `Ctrl+\` | 开始或停止录音 |

快捷键可在设置页面自定义。

## AI Provider 配置

TalkShow 内置了三个 AI Provider，你也可以添加自定义 Provider。

### Vertex AI (Gemini)

使用 Google Cloud Vertex AI 的 Gemini 模型进行语音转写。

**前置条件：**

```bash
# 安装 Google Cloud CLI
# https://cloud.google.com/sdk/docs/install

# 登录并设置应用默认凭据
gcloud auth application-default login
```

需要设置以下环境变量：

| 环境变量 | 说明 | 示例 |
|----------|------|------|
| `GOOGLE_CLOUD_PROJECT` | GCP 项目 ID | `my-project-123` |
| `GOOGLE_CLOUD_LOCATION` | 区域 | `us-central1` |

默认模型：`gemini-2.0-flash`

### 阿里云 DashScope (Qwen)

使用阿里云 DashScope 的 Qwen 音频模型，兼容 OpenAI API 格式。

**配置步骤：**

1. 在 [阿里云 DashScope 控制台](https://dashscope.console.aliyun.com/) 获取 API Key
2. 在 TalkShow 的「模型」页面，找到「阿里云」Provider，填入 API Key

默认模型：`qwen2-audio-instruct`

### SenseVoice (本地模型)

完全离线的本地语音转写，基于 [SenseVoice-Small](https://huggingface.co/haixuantao/SenseVoiceSmall-onnx) ONNX 模型。

**特点：**

- 无需联网，无需 API Key
- 首次使用需下载模型（约 242 MB，从 HuggingFace 下载）
- 支持中文、英文、日文自动检测
- 使用 ONNX Runtime 本地推理

在 TalkShow 的「模型」页面，选择 SenseVoice Provider 并点击下载模型即可。

### 自定义 Provider

支持任何兼容 OpenAI API 格式的音频转写服务。在「模型」页面点击添加 Provider，填写：

- **名称** — 自定义显示名称
- **类型** — OpenAI Compatible
- **Endpoint** — API 地址（如 `https://api.example.com/v1`）
- **API Key** — 认证密钥
- **模型** — 模型名称

## Skills 后处理系统

Skills 是转写完成后的文本处理管线，可以对转写结果进行自动润色和修正。

### 内置 Skills

| Skill | 默认 | 说明 |
|-------|------|------|
| 语气词剔除 | 启用 | 去除「嗯」「啊」「那个」「就是」等口头语气词 |
| 错别字修正 | 启用 | 修正同音错误、输入法错误等 |
| 口语润色 | 关闭 | 保持口语化风格，使表达更流畅自然 |
| 书面格式化 | 关闭 | 口语转书面表达，适合邮件和文档场景 |

### 自定义 Skill

在「技能」页面可以添加自定义 Skill，填写名称、描述和 Prompt 即可。Skill 会感知当前前台应用信息，可以根据不同应用场景编写不同的处理规则。

> Skills 需要 LLM 服务支持。在「技能」页面选择用于 Skills 处理的 Provider 和模型。

## 技术架构

### 技术栈

| 层级 | 技术 |
|------|------|
| 前端框架 | SvelteKit 5 + Svelte 5 (Runes) |
| UI 组件 | bits-ui + Lucide Icons |
| 样式 | Tailwind CSS v4 |
| 桌面框架 | Tauri v2 |
| 后端语言 | Rust (Edition 2024) |
| 音频采集 | cpal + hound |
| AI 框架 | rig-core + rig-vertexai |
| 本地推理 | ONNX Runtime (ort) + SenseVoice-Small |
| 音频处理 | rubato (重采样) + kaldi-native-fbank (特征提取) |
| 剪贴板 | arboard |

### 架构概览

```
┌──────────────────────────────────────────────────────┐
│                     macOS Desktop                     │
│                                                      │
│  ┌──────────┐  全局快捷键   ┌─────────────────────┐  │
│  │ 系统托盘  │ ←─────────── │  Global Shortcut     │  │
│  │ (Tray)   │   窗口切换    │  Plugin              │  │
│  └──────────┘              └──────────┬──────────┘  │
│                                       │              │
│  ┌────────────────────┐               │              │
│  │ 录音指示器 (浮窗)    │◄── 事件 ──────┤              │
│  │ /recording         │               │              │
│  └────────────────────┘               │              │
│                                       ▼              │
│  ┌──────────────────────────────────────────────┐    │
│  │              Rust 后端 (lib.rs)               │    │
│  │                                               │    │
│  │  录音 → WAV/FLAC → AI转写 → Skills → 粘贴    │    │
│  │    │              │         │        │        │    │
│  │  recording     ai.rs   skills.rs  clipboard  │    │
│  │    │       ┌─────┴─────┐              │      │    │
│  │    │   Vertex AI  OpenAI              │      │    │
│  │    │   sensevoice.rs (本地)            │      │    │
│  │  config.rs · logger.rs                       │    │
│  └──────────────────────┬───────────────────────┘    │
│                         ▲ Tauri IPC                  │
│  ┌──────────────────────┴───────────────────────┐    │
│  │           SvelteKit 5 前端 (SPA)              │    │
│  │                                               │    │
│  │  /          首页                              │    │
│  │  /models    AI Provider / 模型管理            │    │
│  │  /skills    Skills 后处理配置                 │    │
│  │  /settings  快捷键设置                        │    │
│  │  /logs      运行日志                          │    │
│  │  /recording 录音指示器浮窗                    │    │
│  └───────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
```

## 开发指南

### 项目结构

```
talkshow/
├── src/                        # 前端源码 (SvelteKit)
│   ├── routes/                 # 页面路由
│   │   ├── +page.svelte        # 首页
│   │   ├── +layout.svelte      # 全局布局（侧边栏导航）
│   │   ├── models/             # 模型管理页
│   │   ├── skills/             # 技能设置页
│   │   ├── settings/           # 快捷键设置页
│   │   ├── logs/               # 日志查看页
│   │   └── recording/          # 录音指示器浮窗
│   └── lib/
│       ├── components/ui/      # UI 组件（Dialog、Select、ShortcutRecorder 等）
│       └── stores/             # Svelte Stores（config、theme）
├── src-tauri/                  # 后端源码 (Rust / Tauri)
│   ├── src/
│   │   ├── lib.rs              # 核心模块：初始化、托盘、快捷键、录音控制、AI 调度
│   │   ├── ai.rs               # AI Provider 请求（Vertex AI / OpenAI Compatible）
│   │   ├── config.rs           # 配置模型定义与持久化
│   │   ├── recording.rs        # 音频采集、WAV 写入、FLAC 编码
│   │   ├── sensevoice.rs       # SenseVoice 本地 ONNX 推理引擎
│   │   ├── skills.rs           # Skills 后处理管线
│   │   ├── clipboard.rs        # 剪贴板写入 + 模拟粘贴
│   │   └── logger.rs           # JSONL 格式日志
│   ├── capabilities/           # Tauri 权限配置
│   ├── icons/                  # 应用图标
│   └── tauri.conf.json         # Tauri 应用配置
├── docs/                       # 技术文档
├── static/                     # 静态资源
├── package.json
├── vite.config.js
├── svelte.config.js
└── tsconfig.json
```

### 开发命令

```bash
# 启动开发服务器（前端热更新 + Rust 增量编译）
npm run tauri dev

# TypeScript 类型检查
npm run check

# 监听模式类型检查
npm run check:watch

# 仅启动前端开发服务器
npm run dev
```

### 打包

```bash
# 构建生产版本（生成 .dmg 安装包）
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

## FAQ

### 首次启动提示需要麦克风权限？

TalkShow 需要麦克风权限进行录音。在系统弹窗中点击「允许」，或在「系统设置 → 隐私与安全性 → 麦克风」中手动开启。

### Vertex AI 提示认证失败？

确保已运行 `gcloud auth application-default login` 并设置了 `GOOGLE_CLOUD_PROJECT` 和 `GOOGLE_CLOUD_LOCATION` 环境变量。

### SenseVoice 模型下载失败？

模型托管在 HuggingFace（约 242 MB），请确保网络可以访问 HuggingFace。下载完成后，后续转写无需联网。

### 录音没有自动粘贴？

自动粘贴依赖模拟 `Cmd+V` 按键。部分以管理员权限运行的应用可能不接受模拟输入，请尝试以普通用户权限运行目标应用。

## License

[Apache License 2.0](LICENSE) &copy; 2026 Xia Jia
