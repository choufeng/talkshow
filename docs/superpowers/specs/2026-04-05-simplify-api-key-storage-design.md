# API Key 存储简化设计

**日期**: 2026-04-05  
**状态**: 设计中  
**分支**: `feature/simplify-api-key-storage`

---

## 1. 背景与目标

### 1.1 问题

当前 API Key 存储方案存在以下问题：

1. **Bug 导致密钥无法正常保存**：使用 `"..."` 字符串检测掩码值，但前端 `mask()` 函数产生的是 `sk-••••••••`（圆点符号），导致掩码检测失效，密钥可能被误存为掩码值。

2. **流程过于复杂**：keyring + config.json 双存储，涉及 6 个步骤、3 个转换环节，可维护性差，调试困难。

3. **空字符串覆盖失败**：`merge_api_keys_into_config` 不覆盖已存在的空字符串，导致 keyring 中的值无法覆盖 config 中的默认值。

### 1.2 目标

- 简化存储架构：废弃 keyring，API Key 直接存储在 `config.json` 中
- 修复现有 bug
- 降低代码复杂度，提高可维护性
- 保持用户体验不变（掩码显示）

---

## 2. 设计决策

### 2.1 存储方案

**决策**：API Key 直接以明文存储在 `config.json` 中。

**理由**：

| 方案 | 优点 | 缺点 |
|------|------|------|
| keyring + config 双存储 | OS Keychain 理论更安全 | 复杂度高、平台差异、Bug 多发 |
| **config.json 明文存储** | 简单可靠、易调试、易维护 | 理论上安全性略低 |

对于桌面应用而言，config.json 在 `~/.local/share/` 下受 OS 文件权限保护，keyring 的额外安全优势几乎为零。

### 2.2 UI 显示策略

**决策**：保持掩码显示。

- UI 显示 `sk-••••••••`
- 用户点击编辑，输入明文
- 确认后保存到 config.json（含明文）

### 2.3 空值处理

**决策**：空字符串 `""` 或 `null` 表示未配置，测试时返回认证错误。

- 用户清空输入框 → 保存 `""` → config 中 `api_key: ""`
- 测试时不执行 keyring 删除逻辑，直接使用空值
- API 返回认证错误（语义正确）

---

## 3. 架构变更

### 3.1 删除的组件

| 文件 | 内容 |
|------|------|
| `src-tauri/src/keyring_store.rs` | 整个文件删除 |
| `src-tauri/Cargo.toml` | 删除 `keyring = "3"` 依赖 |
| `src-tauri/src/config.rs` | 删除 `mask_api_keys`、`strip_api_keys`、`merge_api_keys_into_config` 函数 |
| `src-tauri/src/lib.rs` | 删除 keyring 相关调用 |

### 3.2 简化的函数

**config.rs**：
- 保留 `load_config`、`save_config`、`validate_config`
- 保留 `ProviderConfig.api_key: Option<String>`
- 删除所有密钥相关辅助函数

**lib.rs**：
- `get_config`：直接返回 config，无需 keyring 合并和掩码
- `save_config_cmd`：直接保存 config，无需 keyring 存储
- `test_model_connectivity`：直接使用 config 中的 key

### 3.3 前端变更

**models/+page.svelte**：
- `handleApiKeyChange`：保持不变，流程不变
- 无需关注后端存储细节

**EditableField**：
- 保持不变，掩码显示逻辑不变

---

## 4. 数据流

### 4.1 保存流程（简化后）

```
用户编辑 API Key → handleApiKeyChange(providerId, value)
  → config.save(updatedConfig)
    → invoke('save_config_cmd', { config })
      → Rust:
        1. validate_config() — 校验合法性
        2. save_config() — 直接写入 config.json（含明文 key）
      → 完成
```

### 4.2 读取流程（简化后）

```
前端 config.load()
  → invoke('get_config')
    → Rust: load_config() — 直接返回含 key 的 config
    → 前端收到明文 key
      → 显示时通过 mask() 函数掩码
```

### 4.3 测试流程（简化后）

```
testModel(providerId, modelName)
  → invoke('test_model_connectivity', { providerId, modelName })
    → Rust:
      1. load_config() — 读取含 key 的配置
      2. 查找 provider 和 model
      3. 直接使用 provider.api_key（无需 keyring）
    → 返回测试结果
```

---

## 5. 安全考量

### 5.1 文件权限

- `config.json` 位于 `~/.local/share/talkshow/config.json`（Linux）或等效路径（macOS/Windows）
- 桌面应用运行在用户级别，config 文件受 OS 文件权限保护
- 其他用户无法读取当前用户的 config 文件

### 5.2 已知限制

- **本地安全**：同一台电脑的其他应用/用户理论上可读取 config 文件（但受 OS 权限限制）
- **传输安全**：API Key 通过 Tauri IPC（JSON over localhost）传输，短暂存在于内存

对于桌面应用场景，以上限制在实践中可接受。

---

## 6. 测试策略

### 6.1 单元测试

- `config.rs`：删除 keyring 相关测试，保留 config 读写测试
- 无需 mock keyring

### 6.2 集成测试

- API Key 输入、保存、读取流程
- 掩码显示正确
- 测试连通性功能正常

---

## 7. 实现计划

详见 `docs/superpowers/plans/2026-04-05-simplify-api-key-storage-plan.md`

---

## 8. 附录：变更文件清单

### 删除

- `src-tauri/src/keyring_store.rs`

### 修改

| 文件 | 变更内容 |
|------|---------|
| `src-tauri/Cargo.toml` | 删除 `keyring = "3"` |
| `src-tauri/src/config.rs` | 删除 keyring 相关函数和测试 |
| `src-tauri/src/lib.rs` | 简化 `get_config`、`save_config_cmd`、`test_model_connectivity` |
| `src/routes/models/+page.svelte` | 无变更（流程不变） |

### 新增

无
