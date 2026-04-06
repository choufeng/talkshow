# 子任务 3：Layout 集成 — 首次启动判断

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 子任务 1（后端 command）、子任务 2（向导组件）  
**可并行**: 否

---

## 目标

在 `+layout.svelte` 中集成 onboarding 判断逻辑：首次启动时渲染全屏向导，完成后渲染正常主界面。

---

## 任务清单

### T3.1: 修改 +layout.svelte

**文件**: `src/routes/+layout.svelte`

**操作**:
- `onMount` 中调用 `invoke('get_onboarding_status')` 获取引导状态
- 根据返回值条件渲染：
  - `false` → 渲染 `<OnboardingWizard />`（全屏覆盖，不显示侧边栏）
  - `true` → 渲染现有侧边栏布局
- 向导完成时（`set_onboarding_completed(true)` 被调用后）切换到主界面

### T3.2: 设置页添加「重新运行引导」按钮

**文件**: `src/routes/settings/+page.svelte`

**操作**:
- 在设置页底部新增区域
- 添加「重新运行引导」按钮
- 点击后调用 `invoke('set_onboarding_completed', { completed: false })` 并刷新页面

---

## 验证

```bash
npm run check
npm run dev
```

手动验证：
1. 删除 config.json 中的 `onboarding_completed` 字段（或设为 false）
2. 启动应用，确认显示向导而非主界面
3. 完成引导后，确认进入主界面
4. 进入设置页，点击「重新运行引导」，确认回到向导

---

## 验收标准

- `onboarding_completed = false` 时显示全屏向导
- `onboarding_completed = true` 时显示正常主界面
- 设置页有「重新运行引导」按钮且功能正常
- 侧边栏在向导模式下不显示
