# 子任务 2：前端 Onboarding Store 与向导框架

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 子任务 1（后端 command 已注册）  
**可并行**: 否（依赖后端 command）

---

## 目标

创建前端 onboarding store、OnboardingWizard 主容器组件（含步骤指示器、导航按钮）以及所有步骤组件的占位骨架。

---

## 任务清单

### T2.1: 创建 Onboarding Store

**文件**: `src/lib/stores/onboarding.ts`

**操作**: 创建 Svelte store 管理：
- `currentStep: number`（1-7）
- `completed: boolean`（引导是否已完成）
- `stepCompleted: (step: number) => boolean`（判断步骤是否可前进）
- `nextStep()` / `prevStep()` / `goToStep(n)` 方法

### T2.2: 创建 OnboardingWizard 主容器

**文件**: `src/lib/components/onboarding/OnboardingWizard.svelte`

**操作**: 创建居中卡片式布局：
- **顶部**：圆形步骤指示器（1-7），当前步骤高亮蓝色，已完成标勾，未来灰色
- **中间**：动态渲染当前步骤组件
- **底部**：「上一步」和「下一步/完成」按钮，条件不满足时「下一步」禁用
- 全屏覆盖，背景色与 app 主题一致

### T2.3: 创建所有步骤组件骨架

**文件**: `src/lib/components/onboarding/steps/` 下 7 个文件

每个组件包含基本结构和 props：
- `WelcomeStep.svelte` — 显示标题和副标题
- `DownloadModelStep.svelte` — 占位，显示「下载中...」
- `ProviderConfigStep.svelte` — 占位，显示「Provider 配置」
- `ShortcutsIntroStep.svelte` — 显示三个快捷键卡片
- `TryTranscriptionStep.svelte` — 占位，显示「试用转写」
- `TryTranslationStep.svelte` — 占位，显示「试用翻译」
- `CompletionStep.svelte` — 显示「完成」按钮

每个步骤组件通过 callback 或 store 通知父组件步骤是否完成。

---

## 验证

```bash
npm run check
npm run build
```

---

## 验收标准

- OnboardingWizard 渲染居中卡片式布局
- 步骤指示器正确显示 1-7 步
- 上一步/下一步按钮正常切换步骤
- 所有 7 个步骤组件存在并可渲染
- TypeScript 类型检查通过
