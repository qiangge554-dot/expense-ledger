#!/usr/bin/env bash
# 生成 Windows 安装包（NSIS）
#
# 为什么需要这个脚本：MinGW 的 windres / dlltool 无法处理中文路径，
# 项目源码在 E:\服务\expense-ledger，直接跑 `tauri build` 会在资源编译阶段失败。
# 解决办法是把源码镜像到 ASCII 路径 E:\expense-ledger，在镜像里构建。
#
# 用法：bash scripts/build-installer.sh
set -u

SRC="E:/服务/expense-ledger"
MIRROR="E:/expense-ledger"
NSIS_DIR="E:/tools-dl/nsis-3.10"
TARGET_DIR="E:/tauri_build"

export PATH="$NSIS_DIR:/e/tools-dl/w64devkit/bin:$PATH"
# 本机默认工具链是 msvc（没装 VS Build Tools，无法链接），
# 显式切到我们装好的 GNU 工具链，否则 tauri 会报 target not installed
export RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu
export CARGO_TARGET_DIR="$TARGET_DIR"
export TMPDIR="E:/tauri_tmp"
export CARGO_TMPDIR="E:/tauri_tmp"
export LINKER=gcc AR=gcc-ar CC=gcc WINDRES=windres
mkdir -p "$TMPDIR"

echo "==> 1/4 同步源码到 ASCII 镜像 $MIRROR"
mkdir -p "$MIRROR"
for f in package.json package-lock.json index.html vite.config.ts vite.config.js \
         tailwind.config.js postcss.config.js tsconfig.json tsconfig.node.json; do
  cp -f "$SRC/$f" "$MIRROR/" 2>/dev/null
done
# 旧目录一律用 mv 改名而不是 rm 删除：本环境的删除操作走安全回收站，
# 批量删除会被拦截（trash-failed / bulk confirm）。改名后的旧副本留在 .old_* 目录里，可随时手动清。
# dist 必须换干净的——Tauri 会把整个 dist 目录打进安装包，残留旧 hash 文件会让包体变大
stamp=$(date +%s)
for d in dist src src-tauri/src src-tauri/icons src-tauri/nsis-lang; do
  if [ -d "$MIRROR/$d" ]; then
    mv "$MIRROR/$d" "$MIRROR/.old_${stamp}_$(echo "$d" | tr '/' '_')" 2>/dev/null
  fi
done
cp -rf "$SRC/src" "$MIRROR/src"
cp -rf "$SRC/dist" "$MIRROR/dist"
mkdir -p "$MIRROR/src-tauri"
cp -rf "$SRC/src-tauri/src" "$MIRROR/src-tauri/src"
cp -rf "$SRC/src-tauri/icons" "$MIRROR/src-tauri/icons"
mkdir -p "$MIRROR/src-tauri"
cp -rf "$SRC/src-tauri/nsis-lang" "$MIRROR/src-tauri/nsis-lang"
cp -f "$SRC/src-tauri/Cargo.toml" "$SRC/src-tauri/Cargo.lock" \
      "$SRC/src-tauri/tauri.conf.json" "$SRC/src-tauri/build.rs" "$MIRROR/src-tauri/" 2>/dev/null

if [ ! -e "$MIRROR/node_modules" ]; then
  cmd //c "mklink /J $(echo "$MIRROR" | sed 's|/|\\|g')\\node_modules $(echo "$SRC" | sed 's|/|\\|g')\\node_modules" >/dev/null 2>&1
fi

echo "==> 2/4 校验 NSIS"
"$NSIS_DIR/makensis.exe" //DVERSION >/dev/null 2>&1
ls "$NSIS_DIR/makensis.exe" >/dev/null || { echo "找不到 makensis，请先解压 NSIS 到 $NSIS_DIR"; exit 1; }

echo "==> 3/4 构建 + 打包（前端 → Rust → NSIS）"
cd "$MIRROR" || exit 1
node "$MIRROR/node_modules/@tauri-apps/cli/tauri.js" build --target x86_64-pc-windows-gnu
BUNDLE_RC=$?

echo "==> 4/4 产物"
find "$TARGET_DIR/x86_64-pc-windows-gnu/release/bundle" -type f \( -name "*.exe" -o -name "*.msi" \) 2>/dev/null
exit $BUNDLE_RC
