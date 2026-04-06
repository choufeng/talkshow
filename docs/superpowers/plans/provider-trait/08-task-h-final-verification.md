# 子任务 H: 最终集成验证

> **依赖**: A-G (所有子任务完成) | **阶段**: Phase 4 | **复杂度**: 低

## 目标

验证所有子任务的集成正确性：后端编译、测试通过、前端构建成功。

## 步骤

- [ ] **Step 1: Rust 编译**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

预期：`Finished` 无错误。

**常见问题排查：**
- 如有 `rig_core` 或 `rig_vertexai` 残留引用 → 搜索 `use rig` 并移除
- 如有 `.provider_type` 或 `.endpoint` 残留引用 → 替换为 `.id`
- 如有 `VertexClientCache` 残留引用 → 替换为 `ProviderContext`

- [ ] **Step 2: Rust 测试**

```bash
cd src-tauri && cargo test 2>&1 | tail -30
```

预期：所有测试通过。

**常见问题排查：**
- MockLlmClient 参数数量不匹配 → 检查是否移除了 endpoint 参数
- ProviderConfig 构造缺少字段 → 检查是否遗漏了 type/endpoint 的移除
- ProviderConfig 多余字段 → 检查是否添加了 type/endpoint

- [ ] **Step 3: 前端构建**

```bash
npm run build 2>&1 | tail -20
```

预期：无错误。

**常见问题排查：**
- TypeScript 类型错误 `provider.type` → 替换为 `provider.id`
- TypeScript 类型错误 `provider.endpoint` → 移除该属性引用
- Svelte 组件引用已删除的函数/状态 → 移除对应引用

- [ ] **Step 4: 检查是否有遗留的 rig 引用**

```bash
grep -r "rig_core\|rig_vertexai\|rig-core\|rig-vertexai" src-tauri/src/ src-tauri/Cargo.toml
grep -r "provider_type\|provider\.type\|provider_type:" src-tauri/src/
grep -r "\.endpoint\|provider\.endpoint" src-tauri/src/ src/lib/ src/routes/
```

预期：无结果（所有 rig 和旧字段引用已清除）。

- [ ] **Step 5: 修复集成问题（如有）**

如果上述步骤发现问题，逐一修复并提交：

```bash
git add -A
git commit -m "fix: resolve integration issues from provider trait migration"
```

## 完成标准

- [x] `cargo build` 成功
- [x] `cargo test` 全部通过
- [x] `npm run build` 成功
- [x] 无 rig 相关残留引用
- [x] 无 `provider_type` / `endpoint` 残留引用
