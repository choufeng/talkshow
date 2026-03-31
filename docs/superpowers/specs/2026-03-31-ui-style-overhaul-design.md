# TalkShow UI 样式调整设计

## 目标

解决当前 UI 的两个核心问题：字体偏小/不够大气 + 色系需匹配新的绿色系 logo。

## 色系方案：沉稳墨绿 (Hue ~160)

低饱和度绿色，专业工具感，不喧宾夺主。

### 亮色模式

| Token | 当前 | 调整后 |
|-------|------|--------|
| background | `oklch(100% 0 0)` | 不变 |
| background-alt | `oklch(97% 0 0)` | `oklch(97.5% 0.003 160)` |
| foreground | `oklch(20% 0 0)` | `oklch(18% 0 0)` |
| foreground-alt | `oklch(37% 0.01 67.558)` | `oklch(38% 0.01 160)` |
| muted | `oklch(92% 0 0)` | `oklch(93% 0.005 160)` |
| muted-foreground | `oklch(42% 0.095 57.708)` | `oklch(44% 0.02 160)` |
| border | `oklch(85% 0 0 / 0.2)` | `oklch(88% 0.01 160 / 0.25)` |
| border-input | `oklch(75% 0 0 / 0.35)` | `oklch(78% 0.01 160 / 0.35)` |
| accent | `oklch(85% 0.199 91.936)` (黄) | `oklch(84% 0.12 160)` (绿) |
| accent-foreground | `oklch(42% 0.095 57.708)` (棕) | `oklch(30% 0.08 160)` (深绿) |
| destructive | 不变 | 不变 |

### 暗色模式

| Token | 当前 | 调整后 |
|-------|------|--------|
| background | `oklch(14.076% 0.004 285.822)` | `oklch(14% 0.005 160)` |
| background-alt | `oklch(20.219% 0.004 308.229)` | `oklch(20% 0.006 160)` |
| foreground | `oklch(75.687% 0.123 76.89)` (暖金) | `oklch(80% 0.01 160)` |
| foreground-alt | `oklch(93.203% 0.089 90.861)` | `oklch(93% 0.01 160)` |
| muted | `oklch(23.219% 0.004 308.229)` | `oklch(23% 0.006 160)` |
| muted-foreground | `oklch(85.516% 0.012 261.069)` | `oklch(65% 0.015 160)` |
| border | `oklch(36.674% 0.051 338.825 / 0.25)` | `oklch(30% 0.02 160 / 0.3)` |
| border-input | `oklch(36.674% 0.051 338.825 / 0.4)` | `oklch(30% 0.02 160 / 0.45)` |
| accent | `oklch(36.674% 0.051 338.825)` (暗粉) | `oklch(40% 0.08 160)` (暗绿) |
| accent-foreground | `oklch(87.334% 0.01 338.825)` | `oklch(90% 0.04 160)` |
| destructive | 不变 | 不变 |

## 字体调整

| 元素 | 当前 | 调整后 |
|------|------|--------|
| body 基础字号 | 未设置 (默认16px) | 15px |
| body 行高 | 默认 | 1.6 |
| 导航项文字 | text-sm (14px) | text-[15px] |
| Logo 文字 | text-sm | text-base |
| 页面标题 (h2) | text-xl (20px) | text-2xl (24px) |
| Section label | text-[11px] | text-xs (12px) |
| 卡片标题 | text-[13px] / text-sm | text-[15px] / text-base |
| 卡片描述/副标题 | text-[11px] | text-sm (14px) |
| 标签文字 | text-[10px] | text-[11px] |
| 模型标签 tag | text-[10px] | text-[11px] |
| 输入框文字 | text-xs (12px) | text-sm (14px) |
| 输入框高度 | h-7 / h-8 | h-9 / h-10 |
| 按钮 | text-xs | text-sm |
| 导航图标 | size={18} | size={20} |

## 留白 & 间距调整

| 元素 | 当前 | 调整后 |
|------|------|--------|
| 侧边栏宽度 | w-40 (160px) | w-52 (208px) |
| 主内容内边距 | p-6 (24px) | p-8 (32px) |
| Logo 区内边距 | px-5 py-4 | px-6 py-5 |
| 导航项内边距 | px-5 py-2.5 | px-6 py-3 |
| 导航项间距 gap | gap-2 | gap-3 |
| 页面标题下间距 | mb-6 (24px) | mb-8 (32px) |
| Section 间距 | mb-7 (28px) | mb-10 (40px) |
| Section label 下间距 | mb-2.5 | mb-3 |
| 卡片内边距 | p-3.5 (14px) | p-5 (20px) |
| 卡片圆角 | rounded-lg | rounded-xl |
| 卡片间距 (grid) | gap-3 (12px) | gap-4 (16px) |
| 表单项间距 | mb-2.5 | mb-3 |
| Label 下间距 | mb-1 | mb-1.5 |

## 涉及文件

1. `src/app.css` — 色系变量 + body 基础样式
2. `src/routes/+layout.svelte` — 侧边栏 + 主内容区
3. `src/routes/+page.svelte` — 首页
4. `src/routes/settings/+page.svelte` — 设置页
5. `src/routes/models/+page.svelte` — 模型页
6. `src/routes/skills/+page.svelte` — 技能页
7. `src/routes/logs/+page.svelte` — 日志页
8. `src/lib/components/ui/dialog/index.svelte` — 弹窗组件
9. `src/lib/components/ui/select/index.svelte` — 选择器组件
10. `src/lib/components/ui/shortcut-recorder/index.svelte` — 快捷键录制
11. `src/lib/components/ui/password-input/index.svelte` — 密码输入

## 不调整的部分

- `src/routes/recording/+page.svelte` — 浮动录音指示器，独立样式，不参与此次调整
- `src/lib/components/ui/tag-input/index.svelte` — 未在页面中直接使用
