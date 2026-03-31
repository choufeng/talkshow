# 模型配置页输入框交互改进设计

## 概述

改进模型配置页中 API Key 和 Endpoint 输入框的交互体验和布局适配问题。

## 问题分析

### 当前实现
- `PasswordInput` 组件有两种状态：
  1. `visible = false`：显示不可编辑的 div（遮罩值）
  2. `visible = true`：显示可编辑的 input 框
- 用户必须点击眼睛图标才能切换到编辑模式
- Endpoint 输入框始终可编辑，无保护机制

### 存在问题
1. **交互不直观**：用户不知道需要点击眼睛才能编辑 API Key
2. **布局问题**：输入框可能超出卡片宽度
3. **一致性问题**：API Key 和 Endpoint 的编辑方式不一致

## 设计方案

### 组件重命名

将 `PasswordInput` 重命名为 `EditableField`，支持两种模式：
- **密码模式**：API Key 输入，有眼睛图标控制显示/隐藏
- **文本模式**：Endpoint 输入，直接显示文本

### 交互流程

#### 1. 默认状态
- 显示当前值（密码模式显示遮罩值，文本模式显示完整值）
- 密码模式：眼睛图标在右侧，控制显示/隐藏明文
- 点击输入框区域进入编辑模式
- 输入框区域有轻微的 hover 效果，提示可点击

#### 2. 编辑模式
- 显示可编辑的输入框
- 右侧显示 ✓ 确认按钮和 ✕ 取消按钮
- 点击 ✓ 保存修改，点击 ✕ 放弃修改
- 按 Esc 键等同于取消

#### 3. 视觉反馈
- 默认状态：cursor-pointer，hover 时背景色变化
- 编辑模式：输入框获得焦点，显示确认/取消按钮

### 组件结构

```svelte
<!-- EditableField 组件 -->
<script lang="ts">
  interface Props {
    value: string;
    placeholder?: string;
    mode: 'password' | 'text';  // 新增：支持密码和文本两种模式
    onChange: (value: string) => void;
  }

  let { value, placeholder = '', mode = 'password', onChange }: Props = $props();
  let visible = $state(false);
  let editing = $state(false);
  let editValue = $state(value);

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }

  function startEdit() {
    editValue = value;
    editing = true;
  }

  function confirm() {
    onChange(editValue);
    editing = false;
  }

  function cancel() {
    editValue = value;
    editing = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel();
    if (e.key === 'Enter') confirm();
  }
</script>

<div class="flex items-center gap-1">
  {#if editing}
    <!-- 编辑模式 -->
    <input
      type={mode === 'password' && !visible ? 'password' : 'text'}
      bind:value={editValue}
      {placeholder}
      onkeydown={handleKeyDown}
      class="..."
    />
    <button onclick={confirm}>✓</button>
    <button onclick={cancel}>✕</button>
  {:else}
    <!-- 显示模式 -->
    <div
      class="cursor-pointer hover:bg-muted ..."
      onclick={startEdit}
    >
      {#if mode === 'password'}
        {visible ? value : mask(value)}
      {:else}
        {value}
      {/if}
    </div>
    {#if mode === 'password'}
      <button onclick={() => visible = !visible}>
        {#if visible}<Eye />{:else}<EyeOff />{/if}
      </button>
    {/if}
  {/if}
</div>
```

### 布局适配

- 使用 `overflow-hidden` 和 `text-ellipsis` 防止内容溢出
- 输入框宽度限制为 `w-full`，父容器使用 `min-w-0` 允许收缩
- 卡片使用 `overflow-hidden` 包裹内容

## 实现步骤

1. 重命名并改进组件
   - 将 `PasswordInput` 重命名为 `EditableField`
   - 添加 `mode` 属性支持 'password' 和 'text' 两种模式
   - 添加 `editing` 状态
   - 实现点击进入编辑模式
   - 添加确认/取消按钮
   - 实现 Esc 键取消、Enter 键确认

2. 更新 models 页面
   - API Key 使用 `EditableField` 的 password 模式
   - Endpoint 使用 `EditableField` 的 text 模式

3. 修复布局适配
   - 添加 `overflow-hidden` 和 `text-ellipsis`
   - 确保输入框不超出卡片宽度

## 测试验证

1. 功能测试
   - API Key 输入框
     - 点击输入框区域进入编辑模式
     - 编辑后点击 ✓ 保存
     - 编辑后点击 ✕ 取消
     - 按 Esc 键取消
     - 按 Enter 键确认
     - 眼睛图标切换显示/隐藏
   
   - Endpoint 输入框
     - 点击输入框区域进入编辑模式
     - 编辑后点击 ✓ 保存
     - 编辑后点击 ✕ 取消
     - 按 Esc 键取消
     - 按 Enter 键确认

2. 布局测试
   - 长 API Key 不超出卡片宽度
   - 长 Endpoint 不超出卡片宽度
   - 不同窗口宽度下布局正常
