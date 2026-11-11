// ==================== МОДУЛЬ ОБНОВЛЕНИЙ, ПРОФИЛЕЙ ТВ И ПРОШИВКИ ====================

let latestUpdateUrl = "https://github.com/mitrafun-creator/CecDesk/releases";
let latestUpdateInfo = null;
let lastUpdateCheckFailed = false;
const UPDATE_CHECK_THROTTLE_SECS = 43200; // 12 часов

function renderAppUpdateUiState() {
  const btn = document.getElementById("btn-check-update");
  const btnSettings = document.getElementById("btn-check-update-settings");
  const btnReleases = document.getElementById("btn-open-releases-settings");
  const lblStatus = document.getElementById("lbl-update-status");
  const lblVer = document.getElementById("lbl-app-version-val");
  const svgNS = "http://www.w3.org/2000/svg";

  if (lastUpdateCheckFailed) {
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.update_error") : "Ошибка проверки";
      lblStatus.style.color = "var(--apple-red)";
    }
    if (btnReleases) btnReleases.classList.remove("is-visible");
    return;
  }

  if (!latestUpdateInfo) return;
  const info = latestUpdateInfo;
  if (info.download_url) {
    latestUpdateUrl = info.download_url;
  }

  if (info.has_update) {
    if (btn) {
      btn.style.background = "var(--apple-blue)";
      btn.style.color = "#ffffff";
      btn.textContent = `v${info.latest_version}`;
      btn.dataset.updateUrl = latestUpdateUrl;
    }
    if (btnSettings) {
      btnSettings.innerHTML = "";
      const svg = document.createElementNS(svgNS, "svg");
      svg.setAttribute("class", "ico ico-13");
      svg.setAttribute("viewBox", "0 0 24 24");
      if (info.can_self_update) {
        svg.innerHTML = '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line>';
        const span = document.createElement("span");
        const installLabel = typeof t === "function" ? t("settings.install_update_btn", "Обновить до") : "Обновить до";
        span.textContent = `${installLabel} v${info.latest_version}`;
        btnSettings.appendChild(svg);
        btnSettings.appendChild(span);
        btnSettings.dataset.selfUpdate = "1";
        btnSettings.dataset.updateUrl = latestUpdateUrl;
        if (btnReleases) btnReleases.classList.add("is-visible");
      } else {
        const openText = typeof t === "function" ? t("settings.open_releases") : "Скачать";
        svg.innerHTML = '<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path><polyline points="15 3 21 3 21 9"></polyline><line x1="10" y1="14" x2="21" y2="3"></line>';
        const span = document.createElement("span");
        span.textContent = `${openText} v${info.latest_version}`;
        btnSettings.appendChild(svg);
        btnSettings.appendChild(span);
        delete btnSettings.dataset.selfUpdate;
        btnSettings.dataset.updateUrl = latestUpdateUrl;
        if (btnReleases) btnReleases.classList.remove("is-visible");
      }
    }
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.update_available") : "Доступна новая версия";
      lblStatus.style.color = "var(--apple-blue)";
    }
  } else {
    if (btn) btn.textContent = `v${info.current_version}`;
    if (btnSettings) {
      btnSettings.innerHTML = "";
      const svg = document.createElementNS(svgNS, "svg");
      svg.setAttribute("class", "ico ico-13");
      svg.setAttribute("viewBox", "0 0 24 24");
      svg.innerHTML = '<polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>';
      const span = document.createElement("span");
      span.setAttribute("data-i18n", "settings.check_update_btn");
      span.textContent = typeof t === "function" ? t("settings.check_update_btn") : "Проверить обновления";
      btnSettings.appendChild(svg);
      btnSettings.appendChild(span);
      delete btnSettings.dataset.selfUpdate;
      delete btnSettings.dataset.updateUrl;
    }
    if (btnReleases) btnReleases.classList.remove("is-visible");
    if (lblVer) lblVer.textContent = info.current_version;
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.up_to_date") : "Установлена последняя версия";
      lblStatus.style.color = "var(--text-secondary)";
    }
  }
}
window.renderAppUpdateUiState = renderAppUpdateUiState;

async function openReleasesPage() {
  if (invoke) {
    try {
      await invoke("open_browser_url", { url: latestUpdateUrl });
    } catch (_) {
      window.open(latestUpdateUrl, "_blank");
    }
  } else {
    window.open(latestUpdateUrl, "_blank");
  }
}

async function installAppUpdateUi() {
  if (!invoke) return;
  const btnSettings = document.getElementById("btn-check-update-settings");
  const btnReleases = document.getElementById("btn-open-releases-settings");
  const lblStatus = document.getElementById("lbl-update-status");
  const wrap = document.getElementById("app-update-progress-wrap");
  const bar = document.getElementById("app-update-progress-bar");
  const txt = document.getElementById("lbl-app-update-progress-text");

  if (btnSettings) btnSettings.disabled = true;
  if (btnReleases) btnReleases.disabled = true;
  if (wrap) wrap.classList.add("is-visible");
  if (bar) {
    bar.style.width = "6%";
    bar.style.background = "var(--apple-blue)";
  }
  if (txt) {
    txt.textContent = typeof t === "function" ? t("settings.installing_update", "Загрузка и проверка обновления...") : "Загрузка и проверка обновления...";
    txt.style.color = "var(--text-secondary)";
  }
  if (lblStatus) {
    lblStatus.textContent = typeof t === "function" ? t("settings.installing_update", "Загрузка и проверка обновления...") : "Загрузка и проверка обновления...";
    lblStatus.style.color = "var(--apple-blue)";
  }
  addLog("[OTA] Запуск автообновления приложения...", "log-ok");

  try {
    const resMsg = await invoke("install_app_update");
    if (bar) {
      bar.style.width = "100%";
      bar.style.background = "var(--apple-green)";
    }
    if (txt) {
      txt.textContent = resMsg || (typeof t === "function" ? t("settings.update_restarting", "Перезапуск в новую версию...") : "Перезапуск в новую версию...");
      txt.style.color = "var(--apple-green)";
    }
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.update_restarting", "Перезапуск в новую версию...") : "Перезапуск в новую версию...";
      lblStatus.style.color = "var(--apple-green)";
    }
    addLog(`[OTA] ${resMsg}`, "log-ok");
  } catch (err) {
    if (bar) bar.style.background = "var(--apple-red)";
    if (txt) {
      txt.textContent = String(err);
      txt.style.color = "var(--apple-red)";
    }
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.update_error", "Ошибка обновления") : "Ошибка обновления";
      lblStatus.style.color = "var(--apple-red)";
    }
    addLog(`[OTA Ошибка] ${err}`, "log-err");
    if (btnSettings) btnSettings.disabled = false;
    if (btnReleases) btnReleases.disabled = false;
  }
}

async function checkAppUpdatesUi(silent = false) {
  if (!invoke) return;
  if (silent && appConfig && appConfig.last_update_check_ts) {
    const nowSecs = Math.floor(Date.now() / 1000);
    if (nowSecs - Number(appConfig.last_update_check_ts) < UPDATE_CHECK_THROTTLE_SECS) {
      return;
    }
  }

  if (!silent) addLog("[OTA] Проверка обновлений приложения онлайн...", "log-ok");
  const btnSettings = document.getElementById("btn-check-update-settings");
  const lblStatus = document.getElementById("lbl-update-status");
  if (btnSettings && !silent) {
    btnSettings.disabled = true;
    if (lblStatus) {
      lblStatus.textContent = typeof t === "function" ? t("settings.checking_updates") : "Проверка...";
      lblStatus.style.color = "var(--text-tertiary)";
    }
  }

  try {
    const info = await invoke("check_app_updates");
    lastUpdateCheckFailed = false;
    latestUpdateInfo = info;
    if (appConfig) {
      appConfig.last_update_check_ts = Math.floor(Date.now() / 1000);
      invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }

    renderAppUpdateUiState();
    if (info.has_update) {
      addLog(`[OTA] Доступна новая версия: v${info.latest_version}! (${info.release_notes})`, "log-ok");
    } else if (!silent) {
      addLog(`[OTA] Установлена последняя версия v${info.current_version}`, "log-ok");
    }
  } catch (err) {
    lastUpdateCheckFailed = true;
    renderAppUpdateUiState();
    if (!silent) addLog(`[OTA] Ошибка проверки обновлений: ${err}`, "log-err");
  } finally {
    if (btnSettings) btnSettings.disabled = false;
  }
}

async function syncTvProfilesOnlineUi() {
  if (!invoke) return;
  addLog("[Каталог ТВ] Синхронизация профилей с онлайн-базой...", "log-ok");
  try {
    const res = await invoke("sync_online_tv_profiles");
    addLog(`[Каталог ТВ] ${res.message}`, res.success ? "log-ok" : "log-err");
    await loadProfiles();
  } catch (err) {
    addLog(`[Каталог ТВ] Ошибка синхронизации: ${err}`, "log-err");
  }
}

async function flashModuleFirmwareUi(useCustomFile = false) {
  if (!invoke) return;
  let customFile = null;
  if (useCustomFile) {
    try {
      customFile = await invoke("pick_firmware_file");
      if (!customFile) return;
    } catch (err) {
      addLog(`[Прошивка] Ошибка выбора файла: ${err}`, "log-err");
      return;
    }
  }

  const btnBundled = document.getElementById("btn-flash-bundled-fw");
  const btnCustom = document.getElementById("btn-flash-custom-fw");
  const wrap = document.getElementById("fw-progress-wrap");
  const bar = document.getElementById("fw-progress-bar");
  const txt = document.getElementById("lbl-fw-progress-text");

  if (btnBundled) btnBundled.disabled = true;
  if (btnCustom) btnCustom.disabled = true;
  if (wrap) {
    wrap.style.display = "";
    wrap.classList.add("is-visible");
  }
  if (bar) {
    bar.style.width = "8%";
    bar.style.background = "var(--apple-blue)";
  }
  if (txt) {
    txt.textContent = typeof t === "function" ? t("settings.fw_preparing", "Подготовка программатора...") : "Подготовка программатора...";
    txt.style.color = "var(--text-secondary)";
  }

  const targetPort = selectedPort || connectedPort || "";
  addLog(`[Прошивка] Старт обновления контроллера (${targetPort || "автопоиск"})...`, "log-ok");

  try {
    const resultMsg = await invoke("flash_module_firmware", {
      portName: targetPort,
      customFilePath: customFile
    });
    if (bar) {
      bar.style.width = "100%";
      bar.style.background = "var(--apple-green)";
    }
    if (txt) {
      txt.textContent = resultMsg;
      txt.style.color = "var(--apple-green)";
    }
    addLog(`[Прошивка] ${resultMsg}`, "log-ok");

    await refreshPorts();
    if (selectedPort) {
      await connectDeviceNow(true);
    }
  } catch (err) {
    if (bar) {
      bar.style.background = "var(--apple-red)";
    }
    if (txt) {
      txt.textContent = String(err);
      txt.style.color = "var(--apple-red)";
    }
    addLog(`[Ошибка прошивки] ${err}`, "log-err");
  } finally {
    if (btnBundled) btnBundled.disabled = false;
    if (btnCustom) btnCustom.disabled = false;
  }
}
