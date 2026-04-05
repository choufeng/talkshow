# 子任务 5：步骤 3 AI Provider 配置

**所属设计**: `2026-04-05-onboarding-wizard-design.md`  
**依赖**: 子任务 2（向导框架）  
**可并行**: 是（与子任务 4 并行）

---

## 目标

实现 Provider 配置步骤：自动检测 Vertex 环境，引导用户配置至少一个可用的 AI Provider。

---

## 任务清单

### T5.1: ProviderConfigStep 组件

**文件**: `src/lib/components/onboarding/steps/ProviderConfigStep.svelte`

**操作**:

#### 环境检测
- 组件挂载时调用 `invoke('get_vertex_env_info')` 检测 Vertex 环境变量
- 调用 `invoke('get_config')` 获取当前配置，检查是否已有 Provider

#### Vertex 环境存在
- 显示 Vertex 详细说明卡片：
  - 工作原理：通过 Google Cloud Vertex AI API 调用
  - 能力：文本翻译、润色、AI 处理
  - 已知限制：目前不支持直接音频输入（使用文本测试规避）
  - 自动检测到的项目 ID 和 Location
- 自动将 Vertex AI 设为默认 Provider
- 运行 `invoke('test_model_connectivity')` 验证连通性
- 连通性测试成功 → 步骤完成

#### Vertex 环境不存在
- 显示 Provider 选择界面（复用 models 页面的 Provider 配置 UI）
- 可选项：阿里云、自定义 OpenAI Compatible Provider
- 需要填写 API Key 等必要信息
- 点击「测试连接」验证 Provider 可用性
- 至少一个 Provider 测试成功 → 步骤完成

#### Provider 已配置（已有至少一个有效 Provider）
- 自动跳过此步骤

**复用**: 从 `src/routes/models/+page.svelte` 中提取 Provider 配置相关逻辑为可复用组件。

---

## 验证

```bash
npm run check
npm run dev
```

手动验证：
1. 设置 Vertex 环境变量后启动，确认自动检测并显示说明
2. 不设置 Vertex 环境变量，确认显示 Provider 选择界面
3. 配置一个 Provider 并测试连通性
4. 已有 Provider 配置时确认自动跳过

---

## 验收标准

- Vertex 环境自动检测并显示详细说明
- 无 Vertex 时引导配置其他 Provider
- 至少一个 Provider 连通性测试通过才能前进
- 已有配置时自动跳过
- 复用 models 页面的 Provider 配置 UI
