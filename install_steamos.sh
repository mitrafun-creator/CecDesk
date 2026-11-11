#!/usr/bin/env bash
set -e

REPO_URL="https://github.com/mitrafun-creator/HDMI_CEC_Emulator.git"
RELEASE_BIN_URL="https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases/latest/download/hdmi_cec_studio_linux"
INSTALL_DIR="$HOME/.local/share/hdmi_cec_studio"
BIN_PATH="$INSTALL_DIR/hdmi_cec_studio_linux"

echo "================================================================"
echo "  Установка HDMI-CEC Studio для SteamOS (Steam Deck) и Linux    "
echo "================================================================"

mkdir -p "$INSTALL_DIR/bin/firmware" "$INSTALL_DIR/tv_profiles" "$INSTALL_DIR/scripts" "$INSTALL_DIR/logs"

SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

# Если скрипт запущен напрямую из интернета (curl ... | bash) или вне папки репозитория
if [ -z "$SCRIPT_DIR" ] || [ ! -d "$SCRIPT_DIR/tv_profiles" ]; then
    echo "[*] Загрузка файлов репозитория HDMI_CEC_Emulator..."
    TMP_SRC="$HOME/.cache/hdmi_cec_studio_src"
    rm -rf "$TMP_SRC"
    if command -v git >/dev/null 2>&1; then
        git clone --depth 1 "$REPO_URL" "$TMP_SRC"
        APP_DIR="$TMP_SRC"
    else
        echo "[!] Утилита git не найдена. Установите git или скачайте архив репозитория."
        exit 1
    fi
else
    APP_DIR="$SCRIPT_DIR"
fi

if [ -f "$APP_DIR/hdmi_cec_studio_linux" ]; then
    echo "[*] Найден локальный бинарный файл hdmi_cec_studio_linux..."
    cp "$APP_DIR/hdmi_cec_studio_linux" "$BIN_PATH"
    chmod +x "$BIN_PATH"
elif command -v curl >/dev/null 2>&1 && curl -fsSLI "$RELEASE_BIN_URL" >/dev/null 2>&1; then
    echo "[*] Скачивание готовой сборки для SteamOS / Linux x86_64 из GitHub Releases..."
    curl -fL "$RELEASE_BIN_URL" -o "$BIN_PATH"
    chmod +x "$BIN_PATH"
elif [ -f "$APP_DIR/build_and_run_linux.sh" ]; then
    echo "[*] Готовый бинарный файл в релизах еще собирается. Запускаем локальную сборку..."
    chmod +x "$APP_DIR/build_and_run_linux.sh"
    (cd "$APP_DIR" && ./build_and_run_linux.sh --build-only)
    cp "$APP_DIR/hdmi_cec_studio_linux" "$BIN_PATH"
    chmod +x "$BIN_PATH"
else
    echo "[!] Не удалось получить бинарный файл hdmi_cec_studio_linux."
    exit 1
fi

cp -r "$APP_DIR/tv_profiles/"* "$INSTALL_DIR/tv_profiles/" 2>/dev/null || true
cp -r "$APP_DIR/scripts/"* "$INSTALL_DIR/scripts/" 2>/dev/null || true
cp -r "$APP_DIR/bin/firmware/"* "$INSTALL_DIR/bin/firmware/" 2>/dev/null || true
if [ -f "$APP_DIR/cec_studio_config.default.json" ] && [ ! -f "$INSTALL_DIR/cec_studio_config.json" ]; then
    cp "$APP_DIR/cec_studio_config.default.json" "$INSTALL_DIR/cec_studio_config.json" 2>/dev/null || true
fi
if [ -f "$APP_DIR/icon.png" ]; then
    cp "$APP_DIR/icon.png" "$INSTALL_DIR/icon.png" 2>/dev/null || true
elif [ -f "$APP_DIR/tauri_app/src-tauri/icons/icon.png" ]; then
    cp "$APP_DIR/tauri_app/src-tauri/icons/icon.png" "$INSTALL_DIR/icon.png" 2>/dev/null || true
fi

echo "[1/4] Настройка правил udev (доступ к портам ESP32-S3 / RP2040-Zero и виртуальному вводу uinput через uaccess без world-writable прав)..."
UDEV_RULE_CONTENT='# HDMI-CEC Studio: ESP32-S3 (303a) и RP2040-Zero (2e8a) + uinput для Gamescope / Wayland
SUBSYSTEM=="tty", ATTRS{idVendor}=="303a", MODE="0660", GROUP="uucp", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="tty", ATTRS{idVendor}=="2e8a", MODE="0660", GROUP="uucp", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="usb", ATTRS{idVendor}=="2e8a", MODE="0660", GROUP="uucp", TAG+="uaccess"
KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0660", GROUP="input", TAG+="uaccess", OPTIONS+="static_node=uinput"
'

if command -v sudo >/dev/null 2>&1; then
    echo "$UDEV_RULE_CONTENT" | sudo tee /etc/udev/rules.d/99-hdmi-cec-studio.rules >/dev/null || true
    sudo udevadm control --reload-rules 2>/dev/null || true
    sudo udevadm trigger 2>/dev/null || true
    sudo usermod -aG uucp,dialout,input "$USER" 2>/dev/null || true
fi

echo "[2/4] Создание ярлыка приложения для Desktop Mode (KDE Plasma) и добавления в Steam..."
mkdir -p "$HOME/.local/share/applications"
ICON_LINE=""
if [ -f "$INSTALL_DIR/icon.png" ]; then
    ICON_LINE="Icon=$INSTALL_DIR/icon.png"
fi
cat > "$HOME/.local/share/applications/hdmi-cec-studio.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=HDMI-CEC Studio
Comment=Управление SteamOS / ПК с пульта телевизора по HDMI-CEC
Exec=$BIN_PATH
Path=$INSTALL_DIR
${ICON_LINE}
Terminal=false
Categories=Utility;HardwareSettings;Game;
StartupNotify=false
EOF
chmod +x "$HOME/.local/share/applications/hdmi-cec-studio.desktop"

echo "[3/4] Настройка непрерывного фонового сервиса systemd (не закрывается при переходе в/из Big Picture)..."
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/hdmi-cec-studio.service" <<EOF
[Unit]
Description=HDMI-CEC Studio Background Daemon (SteamOS Gaming & Desktop Mode)
After=pipewire.service

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
Environment=DISPLAY=:0
Environment=GDK_BACKEND=x11
ExecStart=$BIN_PATH --minimized
KillMode=process
Restart=always
RestartSec=2

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable --now hdmi-cec-studio.service 2>/dev/null || true

echo "[4/4] Готово!"
echo "----------------------------------------------------------------"
echo "Приложение установлено в: $INSTALL_DIR"
echo "Ярлык в меню приложений: HDMI-CEC Studio"
echo "Фоновая служба SteamOS:  systemctl --user status hdmi-cec-studio.service"
echo "----------------------------------------------------------------"
