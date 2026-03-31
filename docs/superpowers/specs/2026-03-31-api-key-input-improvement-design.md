# API Key 输入框交互改进设计

## 概述

改进模型配置页中 API Key 输入框的交互体验和布局适配问题。

## 问题分析

### 当前实现
- `PasswordInput` 组件有两种状态：
  1. `visible = false`：显示不可编辑的 div（遮罩值）
  2. `visible = true`：显示可编辑的 input 框
- 用户必须点击眼睛图标才能切换到编辑模式

### 存在问题
1. **交互不直观**：用户不知道需要点击眼睛才能编辑
2. **布局问题**：输入框可能超出卡片宽度

## 设计方案

### 交互流程

#### 1. 默认状态
- 显示遮罩值（如 `sk-••••••••`）
- 眼睛图标在右侧，控制显示/隐藏明文
- 点击输入框区域进入编辑模式
- 输入框区域有轻微的 hover 效果，提示可点击

#### 2. 编辑模式
- 显示可编辑的输入框
- 输入框内显示当前值（明文或遮罩，取决于眼睛状态）
- 右侧显示 ✓ 确认按钮和 ✕ 取消按钮
- 点击 ✓ 保存修改，点击 ✕ 放弃修改
- 按 Esc 键等同于取消

#### 3. 视觉反馈
- 默认状态：cursor-pointer，hover 时背景色变化
- 编辑模式：输入框获得焦点，显示确认/取消按钮

### 组件结构

```svelte
<div class="flex items-center gap-1">
  {#if editing}
    <!-- 编辑模式 -->
    <input
      type={visible ? "text" : "password"}
      {value}
      {placeholder}
      oninput={...}
      onkeydown={handleKeyDown}
      class="..."
    />
    <button onclick={confirm}>✓</button>
    <button onclick={cancel}>✕</button>
  {:else}
    <!-- 显示模式 -->
    <div
      class="cursor-pointer hover:bg-muted ..."
      onclick={() => editing = true}
    >
      {visible ? value : mask(value)}
    </div>
    <button onclick={toggleVisibility}>
      {#if visible}<Eye />{:else}<EyeOff />{/if}
    </button>
  {/if}
</div>
```

### 布局适配

- 使用 `overflow-hidden` 和 `text-ellipsis` 防止内容溢出
- 输入框宽度限制为 `w-full`，父容器使用 `min-w-0` 允许收缩
- 卡片使用 `overflow-hidden` 包裹内容

## 实现步骤

1. 修改 `PasswordInput` 组件
   - 添加 `editing` 状态
   - 实现点击进入编辑模式
   - 添加确认/取消按钮
   - 实现 Esc 键取消

2. 修复布局适配
   - 添加 `overflow-hidden` 和 `text-ellipsis`
   - 确保输入框不超出卡片宽度

## 测试验证

1. 功能测试
   - 点击输入框区域进入编辑模式
   - 编辑后点击 ✓ 保存
   - 编辑后点击 ✕ 取消
   - 按 Esc 键取消
   - 眼睛图标切换显示/隐藏

2. 布局测试
   - 长 API Key 不超出卡片宽度
   - 不同窗口宽度下布局正常
