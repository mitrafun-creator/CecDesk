#!/usr/bin/env bash
set -e

REPO_URL="https://github.com/mitrafun-creator/CecDesk.git"
RELEASE_BASE_URL="https://github.com/mitrafun-creator/CecDesk/releases/latest/download"
RELEASE_BIN_URL="$RELEASE_BASE_URL/cecdesk_linux"
INSTALL_DIR="$HOME/.local/share/cecdesk"
BIN_PATH="$INSTALL_DIR/cecdesk_linux"

echo "================================================================"
echo "       Установка CecDesk для SteamOS (Steam Deck) и Linux       "
echo "================================================================"

mkdir -p "$INSTALL_DIR/bin/firmware" "$INSTALL_DIR/tv_profiles" "$INSTALL_DIR/scripts" "$INSTALL_DIR/logs"

# Безопасный перенос старых данных при обновлении с HDMI-CEC Studio
LEGACY_INSTALL_DIR="$HOME/.local/share/hdmi_cec_studio"
LEGACY_DATA_DIR="$HOME/.local/share/hdmi-cec-studio"
for old_dir in "$LEGACY_INSTALL_DIR" "$LEGACY_DATA_DIR"; do
    if [ -d "$old_dir" ]; then
        if [ -f "$old_dir/cec_studio_config.json" ] && [ ! -f "$INSTALL_DIR/cecdesk_config.json" ]; then
            cp "$old_dir/cec_studio_config.json" "$INSTALL_DIR/cecdesk_config.json" 2>/dev/null || true
        fi
        if [ -d "$old_dir/scripts" ]; then
            cp -rn "$old_dir/scripts/"* "$INSTALL_DIR/scripts/" 2>/dev/null || true
        fi
    fi
done

SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

# Если скрипт запущен напрямую из интернета (curl ... | bash) или вне папки репозитория
if [ -z "$SCRIPT_DIR" ] || [ ! -d "$SCRIPT_DIR/tv_profiles" ]; then
    echo "[*] Загрузка файлов репозитория CecDesk..."
    TMP_SRC="$HOME/.cache/cecdesk_src"
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

if [ -f "$APP_DIR/version.json" ] && command -v python3 >/dev/null 2>&1; then
    VER_TAG="$(python3 -c 'import json; print(json.load(open("'"$APP_DIR/version.json"'")).get("version", ""))' 2>/dev/null || true)"
    if [ -n "$VER_TAG" ]; then
        RELEASE_BASE_URL="https://github.com/mitrafun-creator/CecDesk/releases/download/v$VER_TAG"
        RELEASE_BIN_URL="$RELEASE_BASE_URL/cecdesk_linux"
    fi
fi

systemctl --user stop cecdesk.service 2>/dev/null || true

if [ "$APP_DIR" != "$INSTALL_DIR" ]; then
    rm -f "$BIN_PATH"
    if [ -f "$APP_DIR/cecdesk_linux" ]; then
        echo "[*] Найден локальный бинарный файл cecdesk_linux..."
        cp "$APP_DIR/cecdesk_linux" "$BIN_PATH"
        chmod +x "$BIN_PATH"
    elif command -v curl >/dev/null 2>&1 && curl -fsSLI "$RELEASE_BIN_URL" >/dev/null 2>&1; then
        echo "[*] Скачивание готовой сборки для SteamOS / Linux x86_64 из GitHub Releases..."
        curl -fL "$RELEASE_BIN_URL" -o "$BIN_PATH"
        chmod +x "$BIN_PATH"
    elif [ -f "$APP_DIR/build_and_run_linux.sh" ]; then
        echo "[*] Готовый бинарный файл в релизах еще собирается. Запускаем локальную сборку..."
        chmod +x "$APP_DIR/build_and_run_linux.sh"
        (cd "$APP_DIR" && ./build_and_run_linux.sh --build-only)
        cp "$APP_DIR/cecdesk_linux" "$BIN_PATH"
        chmod +x "$BIN_PATH"
    else
        echo "[!] Не удалось получить бинарный файл cecdesk_linux."
        exit 1
    fi
fi

if [ "$APP_DIR" != "$INSTALL_DIR" ]; then
    cp -r "$APP_DIR/tv_profiles/"* "$INSTALL_DIR/tv_profiles/" 2>/dev/null || true
    cp -r "$APP_DIR/scripts/"* "$INSTALL_DIR/scripts/" 2>/dev/null || true
    cp -r "$APP_DIR/bin/firmware/"* "$INSTALL_DIR/bin/firmware/" 2>/dev/null || true
fi
if command -v curl >/dev/null 2>&1; then
    for fw_file in rp2040_zero_firmware.uf2 esp32_s3_firmware.bin; do
        if [ ! -f "$INSTALL_DIR/bin/firmware/$fw_file" ]; then
            curl -fL "$RELEASE_BASE_URL/$fw_file" -o "$INSTALL_DIR/bin/firmware/$fw_file" 2>/dev/null || rm -f "$INSTALL_DIR/bin/firmware/$fw_file"
        fi
    done
fi
if [ -f "$APP_DIR/cecdesk_config.default.json" ] && [ ! -f "$INSTALL_DIR/cecdesk_config.json" ]; then
    cp "$APP_DIR/cecdesk_config.default.json" "$INSTALL_DIR/cecdesk_config.json" 2>/dev/null || true
fi
if [ -f "$APP_DIR/icon.png" ]; then
    cp "$APP_DIR/icon.png" "$INSTALL_DIR/icon.png" 2>/dev/null || true
elif [ -f "$APP_DIR/tauri_app/src-tauri/icons/icon.png" ]; then
    cp "$APP_DIR/tauri_app/src-tauri/icons/icon.png" "$INSTALL_DIR/icon.png" 2>/dev/null || true
fi

echo "[1/4] Настройка правил udev (доступ к портам ESP32-S3 / RP2040-Zero и виртуальному вводу uinput через uaccess без world-writable прав)..."
UDEV_RULE_CONTENT='# CecDesk Dongle: ESP32-S3 (303a/10c4/1a86) и RP2040-Zero (2e8a) + USB Remote Wakeup + uinput для Gamescope / Wayland
SUBSYSTEM=="tty", ATTRS{idVendor}=="303a", MODE="0660", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="tty", ATTRS{idVendor}=="10c4", MODE="0660", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", MODE="0660", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="tty", ATTRS{idVendor}=="2e8a", MODE="0660", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1"
SUBSYSTEM=="usb", ATTRS{idVendor}=="2e8a", MODE="0660", TAG+="uaccess"
SUBSYSTEM=="usb", ATTRS{idVendor}=="303a", MODE="0660", TAG+="uaccess"
ACTION=="add|change", SUBSYSTEM=="usb", ATTRS{idVendor}=="2e8a", TEST=="power/wakeup", ATTR{power/wakeup}="enabled"
ACTION=="add|change", SUBSYSTEM=="usb", ATTRS{idVendor}=="303a", TEST=="power/wakeup", ATTR{power/wakeup}="enabled"
KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0660", GROUP="input", TAG+="uaccess", OPTIONS+="static_node=uinput"
'

if command -v sudo >/dev/null 2>&1; then
    echo "$UDEV_RULE_CONTENT" | sudo tee /etc/udev/rules.d/99-cecdesk.rules >/dev/null || true
    sudo rm -f /etc/udev/rules.d/99-hdmi-cec-studio.rules 2>/dev/null || true
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
cat > "$HOME/.local/share/applications/cecdesk.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=CecDesk
Comment=Управление SteamOS / ПК с пульта телевизора по HDMI-CEC
Exec=$BIN_PATH
Path=$INSTALL_DIR
${ICON_LINE}
Terminal=false
Categories=Utility;HardwareSettings;Game;
StartupNotify=false
EOF
chmod +x "$HOME/.local/share/applications/cecdesk.desktop"
rm -f "$HOME/.local/share/applications/hdmi-cec-studio.desktop" 2>/dev/null || true

echo "[3/4] Настройка непрерывного фонового сервиса systemd (не закрывается при переходе в/из Big Picture)..."
systemctl --user disable --now hdmi-cec-studio.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/hdmi-cec-studio.service" 2>/dev/null || true
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/cecdesk.service" <<EOF
[Unit]
Description=CecDesk Background Daemon (SteamOS Gaming & Desktop Mode)
After=pipewire.service graphical-session.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
Environment=DISPLAY=:0
Environment=GDK_BACKEND=x11
ExecStart=$BIN_PATH --minimized
KillMode=process
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable --now cecdesk.service 2>/dev/null || true

echo "[4/4] Готово!"
echo "----------------------------------------------------------------"
echo "Приложение установлено в: $INSTALL_DIR"
echo "Ярлык в меню приложений: CecDesk"
echo "Фоновая служба SteamOS:  systemctl --user status cecdesk.service"
echo "----------------------------------------------------------------"
