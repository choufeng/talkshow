# TalkShow 系统图标实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 生成 TalkShow 全套系统图标（App Icon + Tray Icon），替换 src-tauri/icons/ 下所有默认图标。

**Architecture:** 使用 SVG 作为源文件定义矢量图形，通过 ImageMagick（convert/magick）生成各尺寸 PNG，通过 iconutil 生成 macOS icns，通过 ImageMagick 生成 Windows ico。托盘图标使用独立的简化 SVG 源文件。

**Tech Stack:** SVG + ImageMagick 7 + iconutil（macOS 内置）+ Bash

---

## File Structure

```
src-tauri/icons/
├── icon.png                  # 1024×1024 主图标（重新生成）
├── 32x32.png                 # 重新生成
├── 128x128.png               # 重新生成
├── 128x128@2x.png            # 重新生成（256×256）
├── icon.icns                 # 重新生成（macOS 图标集）
├── icon.ico                  # 重新生成（Windows 图标集）
├── recording.png             # 重新生成（录音状态托盘图标）
├── StoreLogo.png             # 重新生成（50×50）
├── Square30x30Logo.png       # 重新生成
├── Square44x44Logo.png       # 重新生成
├── Square71x71Logo.png       # 重新生成
├── Square89x89Logo.png       # 重新生成
├── Square107x107Logo.png     # 重新生成
├── Square142x142Logo.png     # 重新生成
├── Square150x150Logo.png     # 重新生成
├── Square284x284Logo.png     # 重新生成
├── Square310x310Logo.png     # 重新生成

scripts/
└── generate-icons.sh         # 图标生成脚本（新建）

src-tauri/icons-src/          # SVG 源文件（新建）
├── icon.svg                  # 主图标矢量源
└── tray-idle.svg             # 托盘空闲状态矢量源
```

---

### Task 1: 创建主图标 SVG 源文件

**Files:**
- Create: `src-tauri/icons-src/icon.svg`

- [ ] **Step 1: 创建 icons-src 目录**

```bash
mkdir -p src-tauri/icons-src
```

- [ ] **Step 2: 编写主图标 SVG（1024×1024 画布）**

基于设计规格，在 1024×1024 画布上按比例绘制。所有尺寸放大 6.4 倍（160→1024）。

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%" gradientTransform="rotate(160)">
      <stop offset="0%" stop-color="#30D158"/>
      <stop offset="100%" stop-color="#28B44D"/>
    </linearGradient>
  </defs>
  <!-- 圆角矩形底板 (squircle 近似) -->
  <rect width="1024" height="1024" rx="224" fill="url(#bg)"/>
  <!-- 麦克风头部 -->
  <rect x="332.8" y="115.2" width="358.4" height="448" rx="179.2" fill="white" opacity="0.95"/>
  <!-- 麦克风支架弧线 -->
  <path d="M268.8 524.8 C268.8 736, 755.2 736, 755.2 524.8" stroke="white" stroke-width="44.8" fill="none" opacity="0.95" stroke-linecap="round"/>
  <!-- 底部竖线 -->
  <line x1="512" y1="691.2" x2="512" y2="864" stroke="white" stroke-width="44.8" opacity="0.95" stroke-linecap="round"/>
  <!-- 底部横线 -->
  <line x1="396.8" y1="864" x2="627.2" y2="864" stroke="white" stroke-width="44.8" opacity="0.95" stroke-linecap="round"/>
  <!-- 左侧粒子 -->
  <circle cx="153.6" cy="435.2" r="44.8" fill="white" opacity="0.55"/>
  <circle cx="76.8" cy="524.8" r="32" fill="white" opacity="0.35"/>
  <circle cx="51.2" cy="384" r="25.6" fill="white" opacity="0.22"/>
  <!-- 右侧粒子 -->
  <circle cx="870.4" cy="435.2" r="44.8" fill="white" opacity="0.55"/>
  <circle cx="947.2" cy="524.8" r="32" fill="white" opacity="0.35"/>
  <circle cx="972.8" cy="384" r="25.6" fill="white" opacity="0.22"/>
</svg>
```

保存到 `src-tauri/icons-src/icon.svg`。

- [ ] **Step 3: 验证 SVG 可被 ImageMagick 渲染**

```bash
magick src-tauri/icons-src/icon.svg -resize 64x64 /tmp/icon-test.png
file /tmp/icon-test.png
```

预期输出: `/tmp/icon-test.png: PNG image data, 64 x 64`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/icons-src/icon.svg
git commit -m "feat(icons): add main icon SVG source file"
```

---

### Task 2: 创建托盘图标 SVG 源文件

**Files:**
- Create: `src-tauri/icons-src/tray-idle.svg`

- [ ] **Step 1: 编写托盘空闲图标 SVG（64×64 画布）**

仅保留麦克风核心轮廓剪影，去除粒子，纯黑色，适配 macOS 托盘。

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <!-- 麦克风头部 -->
  <rect x="21" y="6" width="22" height="28" rx="11" fill="black"/>
  <!-- 支架弧线 -->
  <path d="M15 32 C15 48, 49 48, 49 32" stroke="black" stroke-width="3" fill="none" stroke-linecap="round"/>
  <!-- 底部竖线 -->
  <line x1="32" y1="45" x2="32" y2="56" stroke="black" stroke-width="3" stroke-linecap="round"/>
  <!-- 底部横线 -->
  <line x1="24" y1="56" x2="40" y2="56" stroke="black" stroke-width="3" stroke-linecap="round"/>
</svg>
```

保存到 `src-tauri/icons-src/tray-idle.svg`。

- [ ] **Step 2: 验证 SVG 渲染**

```bash
magick src-tauri/icons-src/tray-idle.svg -resize 22x22 /tmp/tray-test.png
file /tmp/tray-test.png
```

预期: `PNG image data, 22 x 22`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/icons-src/tray-idle.svg
git commit -m "feat(icons): add tray icon SVG source file"
```

---

### Task 3: 编写图标生成脚本

**Files:**
- Create: `scripts/generate-icons.sh`

- [ ] **Step 1: 创建 scripts 目录**

```bash
mkdir -p scripts
```

- [ ] **Step 2: 编写生成脚本**

脚本功能：
1. 从 `icon.svg` 生成所有尺寸的 PNG（主图标 + Windows 磁贴）
2. 从 `icon.svg` 生成 macOS iconset 再转换为 icns
3. 从 `icon.svg` 生成 Windows ico
4. 从 `tray-idle.svg` 生成托盘空闲图标
5. 生成录音状态托盘图标（空闲图标 + 红色圆点）

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ICONS_SRC="$PROJECT_DIR/src-tauri/icons-src"
ICONS_DIR="$PROJECT_DIR/src-tauri/icons"
ICONSET_DIR="$PROJECT_DIR/src-tauri/icons/icon.iconset"

mkdir -p "$ICONS_DIR"

echo "=== Generating app icons from SVG ==="

MAIN_SVG="$ICONS_SRC/icon.svg"
TRAY_SVG="$ICONS_SRC/tray-idle.svg"

generate_png() {
  local size="$1"
  local output="$ICONS_DIR/$2"
  magick "$MAIN_SVG" -resize "${size}x${size}" -density 300 -quality 100 "$output"
  echo "Generated: $output (${size}x${size})"
}

generate_png 1024 "icon.png"
generate_png 32 "32x32.png"
generate_png 128 "128x128.png"
generate_png 256 "128x128@2x.png"
generate_png 50 "StoreLogo.png"
generate_png 30 "Square30x30Logo.png"
generate_png 44 "Square44x44Logo.png"
generate_png 71 "Square71x71Logo.png"
generate_png 89 "Square89x89Logo.png"
generate_png 107 "Square107x107Logo.png"
generate_png 142 "Square142x142Logo.png"
generate_png 150 "Square150x150Logo.png"
generate_png 284 "Square284x284Logo.png"
generate_png 310 "Square310x310Logo.png"

echo ""
echo "=== Generating macOS iconset ==="

mkdir -p "$ICONSET_DIR"
magick "$MAIN_SVG" -resize 16x16 "$ICONSET_DIR/icon_16x16.png"
magick "$MAIN_SVG" -resize 32x32 "$ICONSET_DIR/icon_16x16@2x.png"
magick "$MAIN_SVG" -resize 32x32 "$ICONSET_DIR/icon_32x32.png"
magick "$MAIN_SVG" -resize 64x64 "$ICONSET_DIR/icon_32x32@2x.png"
magick "$MAIN_SVG" -resize 128x128 "$ICONSET_DIR/icon_128x128.png"
magick "$MAIN_SVG" -resize 256x256 "$ICONSET_DIR/icon_128x128@2x.png"
magick "$MAIN_SVG" -resize 256x256 "$ICONSET_DIR/icon_256x256.png"
magick "$MAIN_SVG" -resize 512x512 "$ICONSET_DIR/icon_256x256@2x.png"
magick "$MAIN_SVG" -resize 512x512 "$ICONSET_DIR/icon_512x512.png"
magick "$MAIN_SVG" -resize 1024x1024 "$ICONSET_DIR/icon_512x512@2x.png"

rm -f "$ICONS_DIR/icon.icns"
iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/icon.icns"
rm -rf "$ICONSET_DIR"
echo "Generated: icon.icns"

echo ""
echo "=== Generating Windows ICO ==="

magick "$MAIN_SVG" \
  \( -clone 0 -resize 16x16 \) \
  \( -clone 0 -resize 32x32 \) \
  \( -clone 0 -resize 48x48 \) \
  \( -clone 0 -resize 64x64 \) \
  \( -clone 0 -resize 128x128 \) \
  \( -clone 0 -resize 256x256 \) \
  -delete 0 \
  "$ICONS_DIR/icon.ico"
echo "Generated: icon.ico"

echo ""
echo "=== Generating tray icons ==="

magick "$TRAY_SVG" -resize 32x32 "$ICONS_DIR/icon-tray-idle-1x.png"
magick "$TRAY_SVG" -resize 64x64 "$ICONS_DIR/icon-tray-idle-2x.png"
echo "Generated: tray idle icons (1x + 2x)"

magick "$TRAY_SVG" -resize 32x32 \
  \( -clone 0 -fill "#FF3B30" -draw "circle 26,6 26,3" \) \
  -flatten "$ICONS_DIR/recording.png"
magick "$TRAY_SVG" -resize 64x64 \
  \( -clone 0 -fill "#FF3B30" -draw "circle 52,12 52,6" \) \
  -flatten "$ICONS_DIR/recording@2x.png"
echo "Generated: recording tray icons (1x + 2x)"

echo ""
echo "=== Done! All icons generated. ==="
ls -la "$ICONS_DIR"/*.png "$ICONS_DIR"/*.icns "$ICONS_DIR"/*.ico
```

保存到 `scripts/generate-icons.sh`。

- [ ] **Step 3: 设置可执行权限**

```bash
chmod +x scripts/generate-icons.sh
```

- [ ] **Step 4: Commit**

```bash
git add scripts/generate-icons.sh
git commit -m "feat(icons): add icon generation script"
```

---

### Task 4: 运行生成脚本并验证

**Files:**
- Modify: `src-tauri/icons/` (所有图标文件)

- [ ] **Step 1: 备份旧图标**

```bash
cp -r src-tauri/icons src-tauri/icons-backup
```

- [ ] **Step 2: 运行生成脚本**

```bash
bash scripts/generate-icons.sh
```

预期: 所有文件生成成功，无报错。

- [ ] **Step 3: 验证所有文件存在且尺寸正确**

```bash
for f in src-tauri/icons/*.png src-tauri/icons/*.icns src-tauri/icons/*.ico; do
  echo "$(basename $f): $(file -b "$f" | head -1)"
done
```

预期输出应包含所有 17 个图标文件，PNG 文件尺寸与文件名匹配。

- [ ] **Step 4: 验证主图标视觉正确性**

```bash
magick src-tauri/icons/icon.png -resize 64x64 /tmp/icon-verify.png
magick identify src-tauri/icons/icon.png
```

预期: `icon.png PNG 1024x1024`

- [ ] **Step 5: 验证 icns 包含所有尺寸**

```bash
iconutil -c iconset src-tauri/icons/icon.icns -o /tmp/iconset-verify
ls -la /tmp/iconset-verify/
rm -rf /tmp/iconset-verify
```

预期: 包含 10 个 PNG（16 到 512@2x）。

- [ ] **Step 6: 验证 ico 包含多尺寸**

```bash
magick identify src-tauri/icons/icon.ico
```

预期: 显示多个分辨率层级（16, 32, 48, 64, 128, 256）。

- [ ] **Step 7: Commit 所有图标文件**

```bash
git add src-tauri/icons/
git commit -m "feat(icons): replace default icons with TalkShow design"
```

---

### Task 5: 更新图标预览页面

**Files:**
- Modify: `icons-preview.html`

- [ ] **Step 1: 更新预览页面以反映新设计**

在现有 `icons-preview.html` 中，标题和描述已匹配。需确认所有文件路径仍然正确（文件名未变），预览页面无需修改。

```bash
grep -c 'src="src-tauri/icons/' icons-preview.html
```

预期: 大于 10（所有图标引用都存在）。

如果需要添加新生成的 `recording@2x.png` 和 `icon-tray-idle-*.png` 的预览，追加到对应 section。

- [ ] **Step 2: 在浏览器中打开验证**

```bash
open icons-preview.html
```

确认所有图标显示正确，无空白/缺失。

- [ ] **Step 3: Commit（如有修改）**

```bash
git add icons-preview.html
git commit -m "feat(icons): update preview page for new icons"
```

---

### Task 6: 清理与最终验证

- [ ] **Step 1: 删除备份目录**

```bash
rm -rf src-tauri/icons-backup
```

- [ ] **Step 2: 验证 tauri.conf.json 图标配置匹配**

```bash
grep -A10 '"icon"' src-tauri/tauri.conf.json
```

确认引用的文件（32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico）均存在。

- [ ] **Step 3: 构建验证**

```bash
npm run build
```

确认构建成功，无图标相关错误。

- [ ] **Step 4: 最终 Commit**

如有未提交的变更：
```bash
git add -A
git commit -m "chore(icons): finalize icon system"
```
