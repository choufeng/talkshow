# T5: 导航集成 — 侧边栏与全局入口

## 所属项目
[Skills 文本处理系统](../../specs/2026-03-30-skills-system-design.md)

## 依赖
- T4: Skills UI 页面（需要 `/skills` 路由存在）

## 目标
在侧边栏导航中新增"技能"入口，并确保导航样式与现有项目保持一致。

## 任务详情

### 1. 修改 +layout.svelte

在现有侧边栏导航项中，在"模型"和"设置"之间新增"技能"导航项：

```
现有导航：
- 首页 (Home icon)
- 模型 (Bot icon)
- 设置 (Settings icon)
- 日志 (ScrollText icon)

新增后：
- 首页 (Home icon)
- 模型 (Bot icon)
- 技能 (Sparkles icon)    ← 新增
- 设置 (Settings icon)
- 日志 (ScrollText icon)
```

图标选择：使用 `lucide-svelte` 的 `Sparkles` 图标（与"技能"概念契合）。

### 2. 导航样式

复用现有导航项的样式模式：
- 使用 `$app/stores` 的 `page` store 判断当前路由，高亮激活项
- 路由路径为 `/skills`
- 支持亮暗主题

### 3. 路由配置

确保 `src/routes/skills/+page.svelte` 可正常通过侧边栏导航访问。无需额外路由配置（SvelteKit 文件路由自动处理）。

## 验收标准

- [ ] 侧边栏显示"技能"导航项，位于"模型"和"设置"之间
- [ ] 点击"技能"导航到 `/skills` 页面
- [ ] 当前页面为 `/skills` 时导航项高亮
- [ ] 图标和文字样式与其他导航项一致
- [ ] 亮暗主题适配正常
