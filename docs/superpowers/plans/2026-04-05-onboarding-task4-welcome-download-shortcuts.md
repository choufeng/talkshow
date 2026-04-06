# 子任务 4：步骤 1 欢迎 & 步骤 2 下载模型 & 步骤 4 快捷键介绍

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 子任务 2（向导框架）  
**可并行**: 是（与子任务 5 并行）

---

## 目标

实现三个无复杂逻辑的步骤：欢迎页、模型下载、快捷键介绍。

---

## 任务清单

### T4.1: WelcomeStep（步骤 1）

**文件**: `src/lib/components/onboarding/steps/WelcomeStep.svelte`

**操作**:
- 标题：「欢迎使用 TalkShow」
- 副标题：「按快捷键，语音即转文字。让我们一起完成初始配置。」
- 无阻塞条件，渲染后立即通知父组件步骤可前进

### T4.2: DownloadModelStep（步骤 2）

**文件**: `src/lib/components/onboarding/steps/DownloadModelStep.svelte`

**操作**:
- 组件挂载时检查 `invoke('get_sensevoice_status')`：
  - 已下载 → 自动通知步骤完成
  - 未下载 → 调用 `invoke('download_sensevoice_model')` 开始下载
- 监听 Tauri 事件 `sensevoice-download-progress` 显示进度条
- 显示已下载大小 / 总大小
- 下载完成后通知步骤可前进
- 下载失败时显示错误信息 + 「重试」按钮

**复用**: 参考 `src/routes/models/+page.svelte` 中 SenseVoice 下载相关的逻辑和事件监听。

### T4.3: ShortcutsIntroStep（步骤 4）

**文件**: `src/lib/components/onboarding/steps/ShortcutsIntroStep.svelte`

**操作**:
- 三个快捷键卡片，每张卡片显示：
  - 快捷键组合（键盘按键样式）
  - 功能描述
- 快捷键列表：
  - `Ctrl+Shift+'` — 窗口切换
  - `Ctrl+\` — 开始/停止录音
  - `Ctrl+Shift+T` — AI 翻译
- 无阻塞条件，渲染后立即通知步骤可前进

---

## 验证

```bash
npm run check
npm run dev
```

手动验证：
1. 启动应用进入向导，确认欢迎页正常显示
2. 进入下载步骤，确认进度条和下载逻辑正常
3. 如果模型已存在，确认自动跳过
4. 进入快捷键步骤，确认三个快捷键卡片正确显示

---

## 验收标准

- 欢迎页文字正确显示
- 模型下载有进度反馈，完成后可前进
- 模型已存在时自动跳过
- 下载失败有重试按钮
- 快捷键卡片正确展示三个快捷键
