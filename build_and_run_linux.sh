#!/usr/bin/env bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        echo "[HDMI-CEC Studio] Установка компилятора Rust (rustup)..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
fi

if ! command -v pkg-config >/dev/null 2>&1 || ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    echo "[HDMI-CEC Studio] Проверка системных библиотек для сборки..."
    if command -v pacman >/dev/null 2>&1; then
        sudo steamos-readonly disable 2>/dev/null || true
        sudo pacman-key --init 2>/dev/null || true
        sudo pacman-key --populate archlinux holo 2>/dev/null || true
        sudo pacman -Sy --needed --noconfirm base-devel pkgconf webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg systemd-libs 2>/dev/null || true
    elif command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libudev-dev pkg-config build-essential curl file
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libayatana-appindicator-gtk3-devel librsvg2-devel systemd-devel pkgconf-pkg-config @development-tools
    fi
fi

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
