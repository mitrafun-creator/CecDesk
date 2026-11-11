# Third-Party Notices and Component Licensing

HDMI-CEC Studio & Hardware Emulator (`tauri_app/`, host scripts, documentation, and TV profiles) is licensed under the **PolyForm Noncommercial License 1.0.0** (see [`LICENSE`](LICENSE)), except for the separate third-party and firmware components listed below, which remain subject to their respective license terms.

---

## 1. Microcontroller Firmware (`firmware/`) — GNU GPL v2.0

The embedded microcontroller firmware in `firmware/` (compiled into standalone `.uf2` / `.bin` binaries under `bin/firmware/` and communicating with the desktop host application exclusively over USB CDC Serial via the text/JSON CBP v1.0 protocol) links against:

- **CEC-master (`firmware/lib/CEC-master`)**
  - Description: Arduino library for HDMI CEC wire-protocol communication.
  - Original Authors / Attribution: `(c) Phil Burr and Andrew N. Carr` (`https://code.google.com/archive/p/cec-arduino/`), adapted via `https://github.com/floe/CEC`.
  - License: **GNU General Public License, Version 2 (GPLv2)** — full text included at [`firmware/lib/CEC-master/LICENSE`](firmware/lib/CEC-master/LICENSE).
  - Scope & Obligations: Any distribution or modification of `firmware/` or compiled `.uf2` / `.bin` firmware images is governed by GNU GPL v2.0, including the requirement to make corresponding firmware source code available under GPL v2.0 when distributing modified firmware binaries.

### PlatformIO Board Toolchains & Cores
Building `firmware/` via `firmware/platformio.ini` retrieves board packages under their respective open-source licenses:
- **espressif32 (`6.9.0`) / ESP32 Arduino Core** — Apache-2.0 / LGPL-2.1 (per upstream Espressif manifests).
- **arduino-pico (`earlephilhower` RP2040 core via `platform-raspberrypi`)** — LGPL-2.1 / MIT / BSD (per upstream core manifests).

---

## 2. Host Application Rust Dependencies (`tauri_app/src-tauri/`)

The desktop application uses Rust crates declared in `tauri_app/src-tauri/Cargo.toml` and pinned in `tauri_app/src-tauri/Cargo.lock`, including:

- **Tauri (`tauri` 2.12.5, `tauri-build` 2.x, `wry` 0.57.1, `tao` 0.37.2)** — Apache-2.0 / MIT
- **Serde (`serde` 1.0.229, `serde_json` 1.0.154)** — Apache-2.0 / MIT
- **serialport (`4.10.1`) / `libudev` (`0.3.0`) / `mio` (`1.2.4`) / `nix` (`0.26.4`)** — MPL-2.0 / MIT / Apache-2.0
- **flate2 (`1.1.10`)** — Apache-2.0 / MIT
- **crc32fast (`1.5.3`)** — Apache-2.0 / MIT
- **windows-sys (`0.59.0` / `0.61.2`) / `winreg` (`0.52.0`)** — Apache-2.0 / MIT
- **libc (`0.2.191`)** — Apache-2.0 / MIT

Each crate remains governed by its respective upstream license as distributed on crates.io.

---

## 3. Windows Runtime & Helper Binaries

- **`WebView2Loader.dll`**
  - Component: Microsoft Edge WebView2 SDK loader library (bundled alongside `HDMI_CEC_Studio.exe` in Windows distribution archives).
  - License: Subject to the Microsoft Edge WebView2 SDK redistribution terms.
- **`bin/AudioTool.exe`**
  - Component: Project-bundled .NET Framework (`v4.0.30319`) Windows CoreAudio / WASAPI helper binary used to enumerate and switch default audio output endpoints.
  - License: Covered by the project [`LICENSE`](LICENSE) terms for distribution as part of HDMI-CEC Studio.
