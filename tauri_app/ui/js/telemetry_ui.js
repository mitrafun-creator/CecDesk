// ==================== ДИАГНОСТИКА, ОТЧЕТЫ ОБ ОШИБКАХ, ТЕЛЕМЕТРИЯ И XML ====================

async function checkPendingCrashOnStartup() {
  if (!invoke) return;
  try {
    const crash = await invoke("get_pending_crash_report");
    if (crash && crash.has_issue) {
      const banner = document.getElementById("crash-recovery-banner");
      const lbl = document.getElementById("lbl-crash-banner-text");
      if (lbl) {
        if (crash.has_panic_file) {
          lbl.textContent = typeof t === "function"
            ? t("crash.banner_panic", "Обнаружен отчёт о прошлом аварийном завершении. Рекомендуется собрать .zip отчёт об ошибке.")
            : "Обнаружен отчёт о прошлом аварийном завершении. Рекомендуется собрать .zip отчёт об ошибке.";
        } else {
          lbl.textContent = typeof t === "function"
            ? t("crash.banner_dirty", "Прошлый запуск завершился некорректно (возможен сбой). Вы можете сохранить диагностический отчёт.")
            : "Прошлый запуск завершился некорректно (возможен сбой). Вы можете сохранить диагностический отчёт.";
        }
      }
      if (banner) {
        banner.style.display = "";
        banner.classList.add("is-visible");
      }
    }
  } catch (_) {}
  refreshDiagnosticsSummary();
}

async function refreshDiagnosticsSummary() {
  if (!invoke) return;
  try {
    const snap = await invoke("get_diagnostics_snapshot");
    if (!snap) return;
    const lblStorage = document.getElementById("lbl-storage-mode");
    if (lblStorage) lblStorage.textContent = snap.storage_mode || "—";
    const lblCnt = document.getElementById("lbl-diag-counters");
    if (lblCnt) {
      lblCnt.textContent = `${snap.session_error_count || 0} / ${snap.session_warn_count || 0}`;
    }
  } catch (_) {}
}
window.refreshDiagnosticsSummary = refreshDiagnosticsSummary;

async function dismissCrashBanner() {
  const banner = document.getElementById("crash-recovery-banner");
  if (banner) {
    banner.classList.remove("is-visible");
    banner.style.display = "";
  }
  if (invoke) {
    await invoke("dismiss_crash_report").catch(() => {});
  }
}

async function onTelemetryOptInChanged() {
  if (!invoke || !appConfig) return;
  const chk = document.getElementById("chk-telemetry-opt-in");
  const statusEl = document.getElementById("lbl-telemetry-status");
  appConfig.telemetry_opt_in = chk ? chk.checked : false;
  try {
    await invoke("save_app_config", { newCfg: appConfig });
    if (statusEl) {
      statusEl.textContent = appConfig.telemetry_opt_in
        ? (typeof t === "function" ? t("diag.status_enabled", "Анонимная телеметрия включена.") : "Анонимная телеметрия включена.")
        : (typeof t === "function" ? t("diag.status_disabled", "Телеметрия отключена.") : "Телеметрия отключена.");
      statusEl.style.color = "var(--text-secondary)";
    }
  } catch (e) {
    if (statusEl) {
      statusEl.textContent = String(e);
      statusEl.style.color = "var(--apple-red)";
    }
  }
}

async function saveTelemetrySettings() {
  if (!invoke || !appConfig) return;
  const inp = document.getElementById("inp-telemetry-endpoint");
  const chk = document.getElementById("chk-telemetry-opt-in");
  const statusEl = document.getElementById("lbl-telemetry-status");
  const prevEndpoint = appConfig.telemetry_endpoint || "";
  appConfig.telemetry_endpoint = inp ? inp.value.trim() : "";
  if (chk) appConfig.telemetry_opt_in = chk.checked;
  try {
    await invoke("save_app_config", { newCfg: appConfig });
    if (statusEl) {
      statusEl.textContent = typeof t === "function" ? t("settings.saved_toast", "Настройки сохранены") : "Настройки сохранены";
      statusEl.style.color = "var(--apple-green)";
    }
  } catch (e) {
    appConfig.telemetry_endpoint = prevEndpoint;
    if (statusEl) {
      statusEl.textContent = String(e);
      statusEl.style.color = "var(--apple-red)";
    }
  }
}

function showBugModalResult(msg, isError = false) {
  const el = document.getElementById("bug-report-result");
  if (!el) return;
  el.textContent = msg;
  el.style.display = "";
  el.className = "bug-result-banner is-visible " + (isError ? "is-error" : "is-success");
}

function formatDiagnosticsSummaryText(snap) {
  if (!snap) return "";
  const lines = [
    `Version: ${snap.app_version} | OS: ${snap.os_platform} (${snap.os_arch}) | Session: ${snap.display_session}`,
    `Storage: ${snap.storage_mode} | Controller: ${snap.controller_type} (${snap.active_port})`,
    `TV EDID: ${snap.edid_pnp_id || "UNK"} (PA=${snap.edid_physical_address || "?"}) | Profile: ${snap.active_profile_id}`,
    `Session errors: ${snap.session_error_count || 0} | Warnings: ${snap.session_warn_count || 0} | Abnormal prev exit: ${!!(snap.dirty_shutdown_detected || snap.has_pending_panic)}`
  ];
  if (Array.isArray(snap.recent_errors) && snap.recent_errors.length > 0) {
    lines.push(`Last error: ${snap.recent_errors[snap.recent_errors.length - 1]}`);
  }
  return lines.join("\n");
}

async function openBugReportModal(fromCrash = false) {
  if (typeof closeAllCustomSelects === "function") closeAllCustomSelects();
  const modal = document.getElementById("bug-report-modal");
  const summaryBox = document.getElementById("bug-report-summary");
  const resultBox = document.getElementById("bug-report-result");
  const webhookBtn = document.getElementById("btn-modal-send-webhook");
  if (resultBox) {
    resultBox.style.display = "";
    resultBox.textContent = "";
    resultBox.className = "bug-result-banner";
  }
  if (webhookBtn) {
    const hasEndpoint = !!(appConfig && appConfig.telemetry_endpoint && appConfig.telemetry_endpoint.trim());
    webhookBtn.style.display = "";
    webhookBtn.classList.toggle("is-hidden", !hasEndpoint);
  }
  if (modal) {
    modal.style.display = "";
    modal.classList.add("is-open");
  }
  if (summaryBox) summaryBox.textContent = "...";

  if (invoke) {
    try {
      const snap = await invoke("get_diagnostics_snapshot");
      if (summaryBox) summaryBox.textContent = formatDiagnosticsSummaryText(snap);
      if (fromCrash) {
        await dismissCrashBanner();
      }
    } catch (e) {
      if (summaryBox) summaryBox.textContent = String(e);
    }
  }
}

function closeBugReportModal() {
  const modal = document.getElementById("bug-report-modal");
  if (modal) {
    modal.classList.remove("is-open");
    modal.style.display = "";
  }
}

async function createSupportBundleFromModal() {
  if (!invoke) return;
  const btn = document.getElementById("btn-modal-create-zip");
  const noteEl = document.getElementById("inp-bug-note");
  const note = noteEl && noteEl.value.trim() ? noteEl.value.trim() : null;
  if (btn) btn.disabled = true;
  try {
    const res = await invoke("create_support_bundle", { userNote: note });
    const sizeKb = Math.max(1, Math.round((res.size_bytes || 0) / 1024));
    showBugModalResult(`${res.filename} (${sizeKb} KB): ${res.zip_path}`, false);
    addLog(`[Support] Создан отчёт ${res.filename} (${res.zip_path})`, "log-ok");
  } catch (e) {
    showBugModalResult(String(e), true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function openReportsFolderUi() {
  if (invoke) {
    await invoke("open_support_bundle_folder").catch(() => {});
  }
}

async function sendBugReportWebhookFromModal() {
  if (!invoke) return;
  const btn = document.getElementById("btn-modal-send-webhook");
  const noteEl = document.getElementById("inp-bug-note");
  const note = noteEl && noteEl.value.trim() ? noteEl.value.trim() : null;
  if (btn) btn.disabled = true;
  try {
    const res = await invoke("send_bug_report", { userNote: note });
    showBugModalResult(res.message || "OK", !res.sent);
  } catch (e) {
    showBugModalResult(String(e), true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function openGithubBugIssueFromModal() {
  if (!invoke) return;
  const noteEl = document.getElementById("inp-bug-note");
  const note = noteEl && noteEl.value.trim() ? noteEl.value.trim() : null;
  try {
    const url = await invoke("build_github_issue_url", { userNote: note });
    await invoke("open_browser_url", { url });
  } catch (e) {
    showBugModalResult(String(e), true);
  }
}

async function openGithubIssueDirect() {
  if (!invoke) return;
  try {
    const url = await invoke("build_github_issue_url", { userNote: null });
    await invoke("open_browser_url", { url });
  } catch (e) {
    addLog(`[Support] Ошибка открытия GitHub Issue: ${e}`, "log-err");
  }
}

function escapeXmlAttr(val) {
  return String(val ?? "")
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

// ==================== ЗАГОТОВКИ ПОДДЕРЖКИ XML ПРОФИЛЕЙ ====================
function exportXmlProfileStub() {
  const profName = (currentProfile && currentProfile.name) || "Universal";
  const brand = (currentProfile && currentProfile.brand) || "Generic";
  const mappings = (appConfig && appConfig.key_mappings) || {};

  let xml = `<?xml version="1.0" encoding="UTF-8"?>\n<!-- CecDesk Profile XML (v1.0) -->\n`;
  xml += `<CecProfile brand="${escapeXmlAttr(brand)}" model="${escapeXmlAttr(profName)}" version="1.0">\n`;
  for (const [code, act] of Object.entries(mappings)) {
    if (act && act !== "NONE") {
      xml += `  <KeyMapping code="${escapeXmlAttr(code)}" action="${escapeXmlAttr(act)}" />\n`;
    }
  }
  xml += `</CecProfile>\n`;

  const blob = new Blob([xml], { type: "application/xml" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `cec_profile_${profName.replace(/[^a-zA-Z0-9_-]/g, "_")}.xml`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  addLog(`[XML] Профиль успешно экспортирован в XML (${profName})`, "log-ok");
}

function importXmlProfileStub() {
  const inp = document.createElement("input");
  inp.type = "file";
  inp.accept = ".xml,text/xml";
  inp.onchange = (e) => {
    const file = e.target.files && e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (evt) => {
      try {
        const text = evt.target.result;
        const parser = new DOMParser();
        const xmlDoc = parser.parseFromString(text, "application/xml");
        const parseError = xmlDoc.querySelector("parsererror");
        if (parseError) {
          throw new Error("Неверная XML-структура файла");
        }
        const mappings = xmlDoc.querySelectorAll("KeyMapping");
        let count = 0;
        if (!appConfig) appConfig = {};
        if (!appConfig.key_mappings) appConfig.key_mappings = {};
        mappings.forEach((m) => {
          const code = m.getAttribute("code");
          const act = m.getAttribute("action");
          if (code && act) {
            appConfig.key_mappings[code] = act;
            count++;
          }
        });
        if (typeof renderMapperTable === "function") renderMapperTable(true);
        if (typeof renderDynamicRemote === "function") renderDynamicRemote();
        saveMapperConfig();
        addLog(`[XML] Импортировано ${count} назначений клавиш из ${file.name}`, "log-ok");
      } catch (err) {
        addLog(`[XML Ошибка] ${err.message}`, "log-err");
      }
    };
    reader.readAsText(file);
  };
  inp.click();
}
