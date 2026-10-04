# 校对浮窗 · 智能候选按钮 — 设计文档

日期：2026-10-04
分支：`review-intercept`（基于已上线的 review 拦截浮窗迭代）

## 背景

review 浮窗 v1 将四个去向（粘贴/翻译/问询/Hook）平铺为四宫格按钮。问题：

1. 大多数录音意图明确，平铺全量按钮制造选择噪音
2. 语义路由其实已判定意图，浮窗没有利用这个信息
3. 用户无法在浮窗内用语音编辑（新录音会再次触发拦截，覆盖待确认内容）

## 目标

- 按钮 = 推荐路径 +（仅歧义时）次选；其余路径收进「其他去向…」折叠
- 歧义信号由语义路由 LLM 在现有合并调用中自报（`alternatives` 字段）
- 回车 = 当前主按钮，所见即所得
- 浮窗编辑态下，新录音一律纯转写插入（选区替换），不再进交互流程
- 发送键可配置（Enter / ⌘Enter）

## 1. 语义路由扩展（intent.rs）

意图 JSON schema 增加 `alternatives`：

```json
{"intent":"transcribe","text":"…","alternatives":["hook"]}
```

- prompt 规则新增：「开头无明确指令、但句意可能指向记录/问询时，intent 判为 transcribe，并把该意图列入 alternatives 数组」
- `alternatives` 只接受 `"hook"` / `"ask"`（无参数意图）；`translate` 需要目标语言参数，作为 alternative 出现时忽略
- 未输出 / 解析失败 → 空数组 = 高置信，行为与现状一致
- `IntentDecision` 增加 `alternatives: Vec<String>`（kind 字符串，不解析为枚举——translate 剔除后仅剩无参意图）

## 2. 数据流（pipeline → ReviewRequest）

```
IntentDecision { intent, text, result, alternatives }
    ↓ 拦截组装(ask/hook/汇合三个拦截点)
ReviewRequest {
  primary: String,            // 模型首选: "paste"|"ask"|"hook";翻译模式固定 "paste"(译文已就绪)
  alternatives: Vec<String>,  // 歧义次选 kinds,组装时过滤:
                              //   去重(与 primary 相同的剔除)
                              //   hook 未配置(hooks.enabled=false 或 url 空)时剔除 "hook"
  hook: Option<HookPreview>,  // 不变
  …其余字段不变
}
```

- 移除 `action` 与 `can_hook` 字段：`action` 由 `primary` 取代；`can_hook` 语义并入「其他去向…」面板的禁用判断（前端拉配置或后端下发可用项——取后者，见 §3）
- hook 为 primary 但 Hook 全局未启用：沿用现有降级（pipeline 已将 intent 改回 Transcribe），primary = "paste"
- 新增 `available: Vec<String>`：全量去向中当前配置可用的 kinds（前端禁用判断不再自行推断）

## 3. 前端浮窗（/review）

### 按钮布局

- **高置信**（alternatives 空）：单一主按钮，文案按 primary：
  - paste → `粘贴到 {target_app}`
  - hook → `发送到 Hook`，按钮下方副标题显示 hook URL 域名 + body 预览
  - ask → `转入对话`
  - 按钮右上标注回车图标（或配置的发送键）
- **歧义**（alternatives 非空）：主按钮（模型首选，回车）+ 次按钮（alternative，`⌘2`）纵向排列；顶部提示条「⚠ 语义路由不确定，请选择去向」
- 右下角「其他去向…」小链接常驻：点击展开内联面板，列全量四项（粘贴/翻译/问询/Hook），`available` 之外的禁用；面板项标注 `⌘1..⌘4`；选中即收起，所选项成为新主按钮

### 键盘

- 发送键（可配置，默认 Enter）：确认当前主按钮；textarea 内按发送键 = 发送，另一变体（⌘Enter 或 Enter）= 换行，Shift+Enter 恒为换行
- `⌘1..⌘4`：恒定映射固定四项 [粘贴, 翻译, 问询, Hook]，歧义双选与「其他去向」面板共用同一映射（与按钮标注一致）；不在 `available` 的项忽略
- Esc = 取消（不变）

### 编辑模式 · 语音插入

- 监听 `review:insert` 事件（`emit_to("review")`）
- 收到 `{text}`：textarea 有选区 → 替换选区；无选区 → 插入光标处
- 插入不改变 primary/alternatives 与主按钮状态（只是编辑文本）

## 4. pipeline 编辑模式分支

转写成功后、语义路由调用前插入检查：

```rust
if review_enabled && review::is_active(h) {
    // 编辑模式:纯转写,跳过润色/路由/拦截
    emit_to("review", "review:insert", { text });
    indicator::show_result(h, "已插入");
    restore_tray_only(h); cleanup_esc(h);
    return;
}
```

- 浮窗活跃的判定 = `ReviewState.current` 有请求（与拦截共享状态，取消/确认后自动失效）
- 翻译快捷键录音在浮窗活跃时同样走此分支（编辑态一切语音都是编辑素材）
- 日志记 `review` 域 `编辑模式语音插入`

## 5. 设置

- 设置页「语音指令」区「发送前人工确认」卡片内新增：
  - 「浮窗发送键」单选：`Enter`（默认）/ `⌘Enter`
- 持久化：`FeaturesConfig.review.send_key: String`（"enter" | "cmd_enter"，serde default "enter"）
- 候选键 ⌘1..⌘4 固定，不进设置

## 6. 不变的部分

- 全局开关「发送前人工确认」默认关；开关关闭时编辑模式分支同样不生效（review_enabled 前置）
- 确认执行路径（review_confirm 四动作分发）、失败放回重试、superseded 检查
- 指示条「待确认」态、取消收尾、历史记录写入时机（确认时）
- 翻译快捷键模式：primary 固定 = 粘贴译文，alternatives 不参与

## 7. 边界与错误处理

| 场景 | 行为 |
|---|---|
| LLM 输出 translate 进 alternatives | 解析时忽略 |
| alternatives 含 primary 同 kind | 去重剔除 |
| hook 候选但 Hook 未配置 | 组装时剔除；available 不含 hook |
| ask 候选选中确认 | 走现有 confirm ask 分发（chat 窗） |
| 编辑模式时浮窗已被用户隐藏但 state 残留 | dismiss 时清 state → is_active=false，不残留 |
| 发送键冲突（textarea 输入法确认） | Enter 默认下输入法回车上屏行为与发送冲突的极端场景接受（语音短句为主），⌘Enter 可选规避 |

## 8. 测试

- Rust：`parse_llm_output` alternatives 解析（输出/缺失/含 translate 剔除/含未知 kind 忽略）；ReviewRequest 组装过滤逻辑
- 前端：浮窗高置信单按钮渲染；歧义双选渲染与 ⌘2；「其他去向」展开选择更新主按钮；review:insert 选区替换/光标插入；发送键 Enter/⌘Enter 两配置
- 已有 97 前端 + 155 Rust 测试不回退

## 9. 决策记录

| 决策 | 选择 | 理由 |
|---|---|---|
| 不相关路径处理 | 折叠进「其他去向…」 | 极简同时保留逃生口 |
| 歧义信号来源 | LLM 自报 alternatives | 语义歧义规则抓不全；零额外往返 |
| 歧义时回车 | 模型首选 | 回车=主按钮一致；分裂的快捷键更危险 |
| translate 进候选 | 不进 | 缺语言参数；「其他去向」面板仍可选 |
| 候选键可配 | 否（固定 ⌘1-4） | 候选数随场景变，可配维度过多 |
