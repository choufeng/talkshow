# 设定页面设计规范

## 概述

在 TalkShow 主界面中添加一个设定页面，采用经典侧边栏布局，支持快捷键设置功能。

## 目标

1. 提供用户友好的快捷键设置界面
2. 支持两个快捷键的配置：窗口切换和录音控制
3. 采用点击录制模式，简化快捷键设置流程
4. 为后续扩展更多设置项预留接口

## 页面布局

### 左侧菜单栏

- 宽度：160px
- 背景色：#f5f5f5
- 菜单项：
  - 首页（🏠）：显示空白内容
  - 设置（⚙️）：显示快捷键设置界面
- 选中状态：背景色 #e8e8e8，左侧蓝色边框 (#396cd8)

### 右侧主内容区

- 自适应宽度
- 内边距：24px
- 根据左侧菜单选择显示对应内容

## 功能规格

### 首页

- 显示空白内容或欢迎信息
- 为后续功能预留空间

### 设置页面

#### 快捷键设置

支持设置两个快捷键：

1. **窗口切换快捷键**
   - 功能：显示或隐藏主窗口
   - 默认值：Control+Shift+Quote (⌃ ⇧ ')
   - 描述：显示或隐藏主窗口

2. **录音控制快捷键**
   - 功能：开始或结束录音
   - 默认值：Control+Shift+KeyR (⌃ ⇧ R)
   - 描述：开始或结束录音

#### 交互流程

1. 用户点击"修改"按钮
2. 按钮变为"取消"，输入区域显示"请按下快捷键..."
3. 用户按下键盘组合键
4. 系统记录按键组合并更新显示
5. 自动保存到配置文件
6. 重新注册全局快捷键

**取消操作：**
- 按 Esc 键取消录制
- 恢复原快捷键显示

## 技术实现

### 前端 (SvelteKit)

#### 路由结构

```
src/routes/
├── +page.svelte        # 首页
├── settings/
│   └── +page.svelte    # 设置页面
```

#### 组件设计

1. **Layout 组件**
   - 左右栏布局
   - 菜单导航

2. **ShortcutRecorder 组件**
   - 快捷键录制交互
   - 按键事件监听
   - 状态管理

#### 状态管理

使用 Svelte 5 的 runes 语法：

```typescript
// 菜单选中状态
let activeMenu = $state('home');

// 快捷键录制状态
let isRecording = $state(false);
let recordingTarget = $state<string | null>(null);
```

### 后端 (Tauri/Rust)

#### Tauri 命令

1. **get_config()** - 获取当前配置
   - 返回：AppConfig

2. **save_config(config: AppConfig)** - 保存配置
   - 参数：AppConfig
   - 返回：Result<(), String>

3. **update_shortcut(shortcut_type: String, shortcut: String)** - 更新快捷键
   - 参数：
     - shortcut_type: "toggle" | "recording"
     - shortcut: 快捷键字符串
   - 返回：Result<(), String>
   - 功能：更新配置并重新注册快捷键

#### 配置更新流程

1. 接收新的快捷键字符串
2. 调用 `config::save_config()` 保存到文件
3. 调用 `app.global_shortcut().unregister()` 注销旧快捷键
4. 调用 `app.global_shortcut().register()` 注册新快捷键
5. 返回操作结果

## 数据结构

### AppConfig

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub shortcut: String,           // 窗口切换快捷键
    pub recording_shortcut: String, // 录音控制快捷键
}
```

### 快捷键字符串格式

格式：`Modifier1+Modifier2+Key`

修饰键：
- Control
- Shift
- Alt
- Command (macOS) / Super (Linux/Windows)

按键：
- 字母：KeyA, KeyB, ...
- 数字：Digit1, Digit2, ...
- 特殊：Space, Quote, Escape, ...

## 用户体验

### 视觉反馈

1. **默认状态**
   - 快捷键显示在灰色背景框中
   - "修改"按钮可点击

2. **录制状态**
   - 输入框变为蓝色边框
   - 显示"请按下快捷键..."
   - "修改"按钮变为"取消"

3. **成功保存**
   - 显示新的快捷键组合
   - 可选：显示成功提示

### 错误处理

1. **快捷键冲突**
   - 检测是否与其他应用冲突
   - 显示错误提示

2. **无效快捷键**
   - 检测是否为有效组合
   - 显示错误提示

3. **保存失败**
   - 显示错误提示
   - 保持原快捷键

## 扩展性

### 未来可添加的设置项

1. **通用设置**
   - 开机自启动
   - 语言选择
   - 主题切换

2. **录音设置**
   - 音频输入设备选择
   - 录音质量设置
   - 自动保存路径

3. **高级设置**
   - 日志级别
   - 调试模式
   - 数据清理

## 测试要求

### 单元测试

1. 快捷键字符串解析测试
2. 配置文件读写测试
3. 快捷键格式验证测试

### 集成测试

1. 快捷键录制流程测试
2. 快捷键注册/注销测试
3. 配置持久化测试

### 用户测试

1. 快捷键设置流程测试
2. 不同操作系统兼容性测试
3. 快捷键冲突检测测试
