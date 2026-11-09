#!/usr/bin/env bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR/tauri_app/src-tauri"

echo "[HDMI-CEC Studio] Сборка нативного бинарного файла для Linux / SteamOS (x86_64)..."
cargo build --release

BIN_SRC="$ROOT_DIR/tauri_app/src-tauri/target/release/hdmi_cec_studio"
cp "$BIN_SRC" "$ROOT_DIR/hdmi_cec_studio_linux"
chmod +x "$ROOT_DIR/hdmi_cec_studio_linux"

DIST_DIR="$ROOT_DIR/HDMI_CEC_Studio_SteamOS_Linux_x64"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/bin/firmware" "$DIST_DIR/tv_profiles" "$DIST_DIR/scripts" "$DIST_DIR/logs"

cp "$ROOT_DIR/hdmi_cec_studio_linux" "$DIST_DIR/hdmi_cec_studio_linux"
chmod +x "$DIST_DIR/hdmi_cec_studio_linux"

if [ -d "$ROOT_DIR/tv_profiles" ]; then
    cp -r "$ROOT_DIR/tv_profiles/"* "$DIST_DIR/tv_profiles/" 2>/dev/null || true
fi
if [ -d "$ROOT_DIR/scripts" ]; then
    cp -r "$ROOT_DIR/scripts/"* "$DIST_DIR/scripts/" 2>/dev/null || true
fi
if [ -d "$ROOT_DIR/bin/firmware" ]; then
    cp -r "$ROOT_DIR/bin/firmware/"* "$DIST_DIR/bin/firmware/" 2>/dev/null || true
fi
if [ -f "$ROOT_DIR/install_steamos.sh" ]; then
    cp "$ROOT_DIR/install_steamos.sh" "$DIST_DIR/install_steamos.sh"
    chmod +x "$DIST_DIR/install_steamos.sh"
fi

echo "[HDMI-CEC Studio] Пакет для SteamOS / Linux готов в папке: $DIST_DIR"

if [ "$1" != "--build-only" ]; then
    echo "[HDMI-CEC Studio] Запуск $ROOT_DIR/hdmi_cec_studio_linux..."
    cd "$ROOT_DIR"
    ./hdmi_cec_studio_linux
fi
