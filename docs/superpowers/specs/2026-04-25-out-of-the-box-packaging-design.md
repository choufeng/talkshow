# TalkShow 开箱即用打包方案设计文档

## 1. 背景与目标
目前 TalkShow 应用依赖于系统环境中预装的 `onnxruntime`（用于语音识别）和 `ffmpeg`（用于音频转码）。在打包分发后，由于缺乏这些运行时环境，最终用户无法直接运行应用。本方案的目标是实现“全包装”，将所有外部依赖内置于应用包中，确保用户安装即用。

## 2. 核心组件
- **ONNX Runtime (ort)**: 作为 Tauri 资源 (Resource) 捆绑。
- **FFmpeg**: 作为 Tauri 侧载二进制 (Sidecar) 捆绑。

## 3. 技术架构

### 3.1 资源打包配置 (`tauri.conf.json`)
利用 Tauri 的 `bundle` 功能，在构建时自动包含外部文件：
- `src-tauri/resources/libonnxruntime.dylib` (macOS) / `onnxruntime.dll` (Windows)
- `src-tauri/binaries/ffmpeg-<target-triple>`

### 3.2 代码逻辑调整
- **动态库加载**: 修改 `src-tauri/src/sensevoice/engine.rs`。
    - 使用 Tauri 的路径解析 API 定位内置的动态库文件。
    - 调用 `ort::init_from(&path)` 进行初始化，彻底废弃对 Homebrew 和 Python 的依赖。
- **音频转码**: 修改 `load_audio` 函数。
    - 通过 Tauri 的 Sidecar 接口调用内置的 `ffmpeg`。
    - 处理不同平台下的 Sidecar 命名差异。

### 3.3 目录结构
```text
src-tauri/
├── binaries/          # 存放 ffmpeg 侧载二进制
│   ├── ffmpeg-aarch64-apple-darwin
│   └── ffmpeg-x86_64-apple-darwin
├── resources/         # 存放动态链接库
│   └── libonnxruntime.dylib
└── tauri.conf.json    # 配置资源路径
```

## 4. 初始化与错误处理
- 应用启动阶段将验证核心组件的存在性与可执行性。
- 若资源缺失，应用将弹出清晰的错误对话框提示用户重新安装，而非静默失败。

## 5. 后续实施建议
1. 下载各平台对应的 `onnxruntime` 和 `ffmpeg` 二进制文件。
2. 按照架构要求放入对应目录并重命名。
3. 逐步替换 `engine.rs` 中的路径硬编码逻辑。
