# 子任务 7：错误处理与边界情况完善

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 子任务 3、4、5、6（所有功能步骤已完成）  
**可并行**: 否

---

## 目标

统一处理所有错误场景和边界情况，确保引导流程健壮。

---

## 任务清单

### T7.1: 下载失败处理

**文件**: `src/lib/components/onboarding/steps/DownloadModelStep.svelte`

**操作**:
- 下载错误时显示具体错误信息（网络失败、磁盘空间不足等）
- 显示「重试」按钮，点击后重新调用 `download_sensevoice_model`
- 不允许跳过此步骤

### T7.2: Provider 连通性测试失败处理

**文件**: `src/lib/components/onboarding/steps/ProviderConfigStep.svelte`

**操作**:
- 连通性测试失败时显示具体错误（API Key 无效、网络超时、权限不足等）
- 允许重新输入或更换 Provider
- 不允许跳过此步骤

### T7.3: 试用步骤超时处理

**文件**: `TryTranscriptionStep.svelte`、`TryTranslationStep.svelte`

**操作**:
- 30 秒无响应时显示提示：「未检测到操作，请重试或跳过」
- 「重试」按钮：重置计时器，重新等待
- 「跳过」按钮：标注「稍后可在设置中测试」，跳过不影响引导完成
- 跳过后 `onboarding_completed` 仍设为 `true`

### T7.4: 中途关闭应用处理

**操作**:
- 确认 `onboarding_completed` 字段在引导未完成时保持 `false`
- 下次启动从头开始引导
- 无需保存中间进度

### T7.5: 已有配置自动跳过逻辑

**文件**: `OnboardingWizard.svelte`

**操作**:
- 引导启动时检测：
  - SenseVoice 已下载？→ 跳过步骤 2
  - 至少一个 Provider 已配置？→ 跳过步骤 3
  - 步骤 5/6 不自动跳过（体验需要）
- 跳过时在步骤指示器中标记为已完成（勾选）
- 如果所有可跳过步骤都已满足，直接定位到第一个不可跳过的步骤

---

## 验证

```bash
npm run check
npm run dev
```

手动验证场景：
1. 断网下载模型 → 确认显示错误和重试
2. 输入无效 API Key → 确认显示具体错误
3. 试用步骤等待 30 秒不操作 → 确认显示超时提示
4. 中途关闭应用再打开 → 确认从头开始
5. 已有模型和 Provider → 确认自动跳过相关步骤

---

## 验收标准

- 所有错误场景有明确的用户反馈
- 下载和 Provider 配置不允许跳过
- 试用步骤超时可跳过
- 中途关闭后重启从头开始
- 已有配置时自动跳过对应步骤
