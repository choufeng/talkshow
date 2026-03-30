# T3: 录音流程集成 — 将 Skills 管线插入录音→转写→粘贴流程

## 所属项目
[Skills 文本处理系统](../../specs/2026-03-30-skills-system-design.md)

## 依赖
- T2: Skills 核心引擎（需要 `process_with_skills` 函数）

## 目标
修改 `lib.rs` 中的 `stop_recording` 流程，在转写完成后、剪贴板粘贴前插入 Skills 文本处理管线。

## 任务详情

### 1. 修改 lib.rs 的 stop_recording 流程

当前流程（简化）：
```rust
// 录音完成 → 转写 → 粘贴
let transcription = transcribe(&config, &providers, &recording_result).await?;
write_and_paste(&transcription)?;
```

改造后：
```rust
// 录音完成 → 转写 → Skills 处理 → 粘贴
let transcription = transcribe(&config, &providers, &recording_result).await?;

let final_text = if config.features.skills.enabled {
    match skills::process_with_skills(
        &ai_state,
        &config.features.skills,
        &config.ai.providers,
        &transcription,
    ).await {
        Ok(processed) => processed,
        Err(e) => {
            logger.log("skills", &format!("Skills 处理失败，使用原始文字: {}", e));
            transcription  // 降级：使用原始转写文字
        }
    }
} else {
    transcription  // Skills 全局关闭
};

write_and_paste(&final_text)?;
```

### 2. 错误处理策略

- Skills 处理错误**不阻塞**主流程，始终回退到原始转写文字
- 错误信息记录到日志系统
- 不弹出任何 UI 提示（不干扰用户）
- Skills 处理耗时应在托盘 tooltip 中有所体现（可选）

### 3. 状态管理

确保 `stop_recording` 能访问到 `SkillsConfig`。由于当前 `stop_recording` 已通过 Tauri State 访问 `AppConfig`，而 T1 已经将 `SkillsConfig` 嵌入 `FeaturesConfig`，因此无需额外状态管理。

## 验收标准

- [ ] 转写后、粘贴前正确调用 Skills 管线
- [ ] Skills 全局关闭时跳过处理，行为与旧版一致
- [ ] Skills 处理失败时回退到原始转写文字，粘贴正常执行
- [ ] 整个流程端到端无 panic
- [ ] 错误场景下日志正确记录
- [ ] 不影响现有转写和粘贴功能
