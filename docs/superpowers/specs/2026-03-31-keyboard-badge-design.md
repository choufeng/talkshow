# 快捷键徽章组件设计

## 背景

设置页面中的快捷键展示区域当前使用单一灰色等宽字体容器显示所有按键符号（如 `⌃ ⇧ '`），缺乏层次感和视觉吸引力。需要改为 macOS 系统风格的独立按键徽章，提升美观度和原生感。

## 设计方案

### 风格：macOS 系统风格分离式按键徽章

每个按键独立显示为一个带渐变和阴影的小方块，模拟物理键盘按键的外观。

### 核心变更

1. **新建 `KeyBadge` 组件** (`src/lib/components/ui/key-badge/index.svelte`)
   - 接收单个按键字符串作为 prop
   - 渲染为独立的按键徽章元素
   - 支持 light/dark 模式自动切换

2. **改造 `ShortcutRecorder` 组件** (`src/lib/components/ui/shortcut-recorder/index.svelte`)
   - 将快捷键字符串拆分为单个按键
   - 每个按键用 `KeyBadge` 渲染
   - 替换当前的单一灰色容器

3. **新增平台感知的按键格式化逻辑**
   - macOS: 修饰键显示符号 `⌃⇧⌥⌘`
   - 其他平台: 修饰键显示全名 `Ctrl/Shift/Alt/Cmd`

### KeyBadge 组件规格

**Props:**
- `key: string` — 要显示的按键文本

**样式规格:**

Light 模式:
- 背景: `linear-gradient(180deg, #fafafa 0%, #e8e8e8 100%)`
- 边框: `1px solid #c0c0c0`, 底部 `2px`
- 阴影: `0 1px 2px rgba(0,0,0,0.12), inset 0 1px 0 rgba(255,255,255,0.9)`
- 文字: `#333`
- 尺寸: `min-width: 40px`, `height: 40px`, `padding: 0 12px`
- 圆角: `6px`

Dark 模式:
- 背景: `linear-gradient(180deg, #4a4a4a 0%, #3a3a3a 100%)`
- 边框: `1px solid #555`, 底部 `2px`
- 阴影: `0 1px 3px rgba(0,0,0,0.3), inset 0 1px 0 rgba(255,255,255,0.08)`
- 文字: `#ddd`

### 快捷键拆分逻辑

将 `Control+Shift+Quote` 格式的字符串拆分为 `['Control', 'Shift', 'Quote']`，然后通过格式化函数映射为展示文本。

修饰键映射（macOS）:
- `Control` → `⌃`
- `Shift` → `⇧`
- `Alt` → `⌥`
- `Command` → `⌘`

修饰键映射（其他平台）:
- `Control` → `Ctrl`
- `Shift` → `Shift`
- `Alt` → `Alt`
- `Command` → `Cmd`

普通键映射:
- `Quote` → `'`
- `Backslash` → `\`
- `Space` → `Space`
- `Key*` → 去掉 `Key` 前缀（`KeyA` → `A`）
- `Digit*` → 去掉 `Digit` 前缀（`Digit1` → `1`）

### 布局

按键徽章之间使用 `8px` 间距，无连接符号。按键组与"修改"按钮之间保持 `12px` 间距。

### 影响范围

- 新增文件: `src/lib/components/ui/key-badge/index.svelte`
- 修改文件: `src/lib/components/ui/shortcut-recorder/index.svelte`
- 不涉及后端变更，纯前端展示层改造
