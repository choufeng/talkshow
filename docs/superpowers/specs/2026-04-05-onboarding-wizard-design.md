# Onboarding 引导向导设计

## 目标

让新用户尽快到达「按快捷键就能录音转文字」的状态。首次启动时触发全屏向导，通过端到端验证确保用户完成引导后即可正常使用。

## 流程步骤

线性串行向导，共 7 步：

1. **欢迎页** — 一句话介绍 TalkShow：「按快捷键，语音即转文字。让我们一起完成初始配置。」无阻塞条件，点「开始」进入下一步。

2. **下载本地模型** — 触发 SenseVoice 下载（~242MB），阻塞式等待。显示进度条 + 已下载/总大小。「下一步」按钮禁用直到下载完成。如果模型已存在，自动跳过此步骤。

3. **AI Provider 配置** — 检测 Vertex 环境变量（`get_vertex_env_info`）：
   - **有 Vertex 环境**：显示 Vertex 详细说明（工作原理、能力、已知限制如音频不支持），自动配置为默认 Provider，运行连通性测试。
   - **无 Vertex 环境**：显示 Provider 选择界面（复用 models 页面组件），引导配置阿里云或自定义 OpenAI Compatible Provider，需填写 API Key。
   - 至少一个 Provider 配置成功 + 通过 `test_model_connectivity` 才能继续。

4. **快捷键介绍** — 展示三个核心快捷键卡片：
   - `Ctrl+Shift+'` — 窗口切换
   - `Ctrl+\` — 开始/停止录音
   - `Ctrl+Shift+T` — AI 翻译
   - 无阻塞条件。

5. **试用转写** — 提示「按 `Ctrl+\` 录一段话，然后再次按下停止」。显示录音状态指示，等待转写结果返回并展示。收到结果后「下一步」可用。

6. **试用翻译** — 提示「选中上一步的转写文本，按 `Ctrl+Shift+T`」。等待翻译结果返回并展示。收到结果后「下一步」可用。

7. **完成** — 显示「配置完成！TalkShow 已准备就绪。」按钮「开始使用」调用 `set_onboarding_completed(true)` 进入主界面。

## UI 设计

居中卡片式布局：
- 顶部：圆形步骤指示器（1-7），当前步骤高亮，已完成步骤标勾，未来步骤灰色
- 中间：步骤内容区域，居中显示
- 底部：「上一步」和「下一步/完成」按钮，条件不满足时「下一步」禁用

## 状态管理与触发机制

### 首次启动判断
- `config.rs` 新增 `onboarding_completed: bool` 字段，默认 `false`
- 后端提供两个 Tauri command：`get_onboarding_status` 和 `set_onboarding_completed`
- 引导完成后置为 `true`

### 重新触发
- 设置页底部添加「重新运行引导」按钮
- 调用 `set_onboarding_completed(false)` 后刷新页面

### 引导进度状态（前端）
- `src/lib/stores/onboarding.ts` 管理 `currentStep`（1-7）
- 步骤完成条件：
  - 步骤 2：`get_sensevoice_status` 返回 `Downloaded`
  - 步骤 3：至少一个 Provider 配置完成 + 连通性测试通过
  - 步骤 5：成功收到一次转写结果
  - 步骤 6：成功收到一次翻译结果
  - 其他步骤：无阻塞条件

### 路由层面
- `+layout.svelte` 的 `onMount` 中检查 `onboarding_completed`
- `false`：渲染全屏向导组件（覆盖侧边栏布局）
- `true`：渲染正常主界面

## 错误处理与边界情况

### 下载失败（步骤 2）
- 显示错误信息 + 「重试」按钮，不自动跳过
- 网络问题提示：「请检查网络连接后重试」

### Provider 连通性测试失败（步骤 3）
- 显示具体错误（API Key 无效、网络超时等）
- 允许重新输入或更换 Provider
- 不允许跳过

### 试用超时（步骤 5/6）
- 转写/翻译等待 30 秒无响应，显示「未检测到操作，请重试」
- 提供「跳过」选项（标注「稍后可在设置中测试」），跳过不影响引导完成

### 用户中途关闭应用
- `onboarding_completed` 仍为 `false`，下次启动从头开始
- 不保存中间进度

### 已有配置自动跳过
- 步骤 2 模型已存在：自动跳过
- 步骤 3 所有 Provider 已配置：自动跳过
- 步骤 5/6：不自动跳过（体验需要）

## 组件结构

### 新增文件
- `src/lib/components/onboarding/OnboardingWizard.svelte` — 向导主容器（步骤切换、指示器、导航按钮）
- `src/lib/components/onboarding/steps/WelcomeStep.svelte`
- `src/lib/components/onboarding/steps/DownloadModelStep.svelte`
- `src/lib/components/onboarding/steps/ProviderConfigStep.svelte`
- `src/lib/components/onboarding/steps/ShortcutsIntroStep.svelte`
- `src/lib/components/onboarding/steps/TryTranscriptionStep.svelte`
- `src/lib/components/onboarding/steps/TryTranslationStep.svelte`
- `src/lib/components/onboarding/steps/CompletionStep.svelte`
- `src/lib/stores/onboarding.ts` — 引导进度状态管理

### 修改文件
- `src/routes/+layout.svelte` — 添加 onboarding 检查，条件渲染
- `src-tauri/src/config.rs` — 添加 `onboarding_completed` 字段
- `src-tauri/src/lib.rs` — 添加 `get_onboarding_status` / `set_onboarding_completed` 命令
- `src/routes/settings/+page.svelte` — 添加「重新运行引导」按钮

### 复用现有组件
- Provider 配置步骤复用 `models/+page.svelte` 中的 Provider 配置 UI
- 下载步骤复用 SenseVoice 下载进度逻辑
- 转写/翻译试用步骤复用现有 Tauri invoke 调用
