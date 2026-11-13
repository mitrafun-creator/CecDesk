const listen = window.__TAURI__ ? window.__TAURI__.event.listen : null;
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;

let hudErrCount = 0;
function forwardHudError(msg, stack) {
  if (!invoke || hudErrCount >= 10) return;
  hudErrCount++;
  invoke("report_client_error", {
    source: "hud",
    message: String(msg || "Unknown HUD error"),
    stack: stack ? String(stack) : null
  }).catch(() => {});
}
window.addEventListener("error", (ev) => {
  forwardHudError(ev.message, ev.error && ev.error.stack ? ev.error.stack : `${ev.filename || ""}:${ev.lineno || 0}`);
});
window.addEventListener("unhandledrejection", (ev) => {
  const r = ev.reason;
  forwardHudError(r && r.message ? r.message : String(r), r && r.stack ? r.stack : null);
});
window.addEventListener("securitypolicyviolation", (ev) => {
  forwardHudError(
    `CSP violation (${ev.violatedDirective || ev.effectiveDirective || "unknown"}): ${ev.blockedURI || "inline"}`,
    `${ev.sourceFile || ""}:${ev.lineNumber || 0}:${ev.columnNumber || 0}`
  );
});

let hideTimer = null;
let isMouseActive = false;
let activeSector = 5;
let hudLang = "ru";

const HUD_I18N = {
  ru: {
    action_done: "Действие выполнено",
    mouse_mode: "Режим мыши",
    mouse_active: "АКТИВЕН",
    sectors_title: "Секторы (1-9)",
    lmb: "ЛКМ",
    menu_btn: "Меню",
    rmb: "ПКМ",
    dbl_click: "2x Клик",
    scroll: "Скролл",
    red_btn: "Красная",
    green_btn: "Зеленая",
    yellow_btn: "Желтая",
    footer_hint: "0: Центр • Стрелки: Курсор • 1-9: Прыжок в сектор",
    sector_center: "Сектор 5 (Центр)",
    sector_n: "Сектор"
  },
  en: {
    action_done: "Action completed",
    mouse_mode: "Mouse Mode",
    mouse_active: "ACTIVE",
    sectors_title: "Sectors (1-9)",
    lmb: "LMB",
    menu_btn: "Menu",
    rmb: "RMB",
    dbl_click: "2x Click",
    scroll: "Scroll",
    red_btn: "Red",
    green_btn: "Green",
    yellow_btn: "Yellow",
    footer_hint: "0: Center • Arrows: Cursor • 1-9: Jump to sector",
    sector_center: "Sector 5 (Center)",
    sector_n: "Sector"
  }
};

function applyHudLang(lang) {
  hudLang = lang === "en" ? "en" : "ru";
  document.documentElement.lang = hudLang;
  const dict = HUD_I18N[hudLang];
  document.querySelectorAll("[data-hud-i18n]").forEach((el) => {
    const key = el.getAttribute("data-hud-i18n");
    if (dict[key]) el.textContent = dict[key];
  });
  const lbl = document.getElementById("cockpit-sector-label");
  if (lbl) {
    lbl.textContent = (activeSector === 5 || activeSector === 0)
      ? dict.sector_center
      : `${dict.sector_n} ${activeSector}`;
  }
}

const HUD_SVGS = {
  "mouse": '<svg viewBox="0 0 24 24"><rect x="6" y="3" width="12" height="18" rx="6"></rect><line x1="12" y1="7" x2="12" y2="11"></line></svg>',
  "warning": '<svg viewBox="0 0 24 24"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>',
  "volume-up": '<svg viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>',
  "volume-down": '<svg viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>',
  "volume-mute": '<svg viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><line x1="23" y1="9" x2="17" y2="15"></line><line x1="17" y1="9" x2="23" y2="15"></line></svg>',
  "play-pause": '<svg viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>',
  "stop": '<svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"></rect></svg>',
  "next": '<svg viewBox="0 0 24 24"><polygon points="5 4 15 12 5 20 5 4"></polygon><line x1="19" y1="5" x2="19" y2="19"></line></svg>',
  "prev": '<svg viewBox="0 0 24 24"><polygon points="19 20 9 12 19 4 19 20"></polygon><line x1="5" y1="19" x2="5" y2="5"></line></svg>',
  "gamepad": '<svg viewBox="0 0 24 24"><line x1="6" y1="12" x2="10" y2="12"></line><line x1="8" y1="10" x2="8" y2="14"></line><line x1="15" y1="13" x2="15.01" y2="13"></line><line x1="18" y1="11" x2="18.01" y2="11"></line><rect x="2" y="6" width="20" height="12" rx="6"></rect></svg>',
  "browser": '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>',
  "numpad": '<svg viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"></rect><circle cx="7.5" cy="7.5" r="1"></circle><circle cx="12" cy="7.5" r="1"></circle><circle cx="16.5" cy="7.5" r="1"></circle><circle cx="7.5" cy="12" r="1"></circle><circle cx="12" cy="12" r="1"></circle><circle cx="16.5" cy="12" r="1"></circle><circle cx="12" cy="16.5" r="1"></circle></svg>',
  "spark": '<svg viewBox="0 0 24 24"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon></svg>',
  "command": '<svg viewBox="0 0 24 24"><path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"></path></svg>'
};

async function initHud() {
  if (listen) {
    await Promise.all([
      listen("mouse-mode-changed", (ev) => {
        isMouseActive = !!ev.payload;
        if (isMouseActive) {
          showMouseCockpit();
        } else {
          hideMouseCockpit();
        }
      }),
      listen("mouse-sector-jump", (ev) => {
        const s = (ev.payload && typeof ev.payload.sector === "number") ? ev.payload.sector : 5;
        highlightSector(s);
      }),
      listen("show-hud", (ev) => {
        handleShowHudPayload(ev.payload);
      }),
      listen("language-changed", (ev) => {
        if (ev && ev.payload) applyHudLang(String(ev.payload));
      })
    ]);
  }

  if (invoke) {
    try {
      const cfg = await invoke("get_app_config");
      if (cfg && cfg.language) applyHudLang(cfg.language);
    } catch (_) {}
    try {
      const st = await invoke("get_initial_hud_state");
      if (st) {
        if (st.mouse_active) {
          isMouseActive = true;
          if (st.last_sector && typeof st.last_sector.sector === "number") {
            highlightSector(st.last_sector.sector);
          } else {
            showMouseCockpit();
          }
        } else if (st.last_show) {
          handleShowHudPayload(st.last_show);
        }
      }
    } catch (_) {}
  }
}

function handleShowHudPayload(p) {
  if (!p) return;
  if (p.mouse_active === true) {
    isMouseActive = true;
    showMouseCockpit();
    return;
  }
  if (p.mouse_active === false) {
    isMouseActive = false;
    hideMouseCockpit();
    return;
  }
  if (!isMouseActive) {
    showOverlay(p.icon || "command", p.title || "HDMI-CEC", p.desc || "", p.progress);
  } else {
    const lbl = document.getElementById("cockpit-sector-label");
    if (lbl && p.title) {
      lbl.textContent = p.title;
      setTimeout(() => {
        if (isMouseActive) {
          lbl.textContent = (activeSector === 5 || activeSector === 0) ? "Сектор 5 (Центр)" : `Сектор ${activeSector}`;
        }
      }, 1400);
    }
  }
}

function showMouseCockpit() {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  const mouseCard = document.getElementById("mouse-hud");
  const stdCard = document.getElementById("hud-card");
  if (stdCard) stdCard.classList.remove("visible");
  if (mouseCard) mouseCard.classList.add("visible");

  if (invoke) {
    invoke("show_hud_window").catch(() => {});
  }
}

function hideMouseCockpit() {
  const mouseCard = document.getElementById("mouse-hud");
  if (mouseCard) mouseCard.classList.remove("visible");

  setTimeout(() => {
    if (invoke && !isMouseActive) {
      invoke("hide_hud_window").catch(() => {});
    }
  }, 240);
}

function highlightSector(sector) {
  activeSector = sector;
  const targetSector = (sector === 0) ? 5 : sector;

  document.querySelectorAll(".minimap-cell").forEach(c => {
    c.classList.remove("jump-active");
  });

  const cell = document.getElementById(`sector-cell-${targetSector}`);
  if (cell) {
    cell.classList.add("jump-active");
    setTimeout(() => {
      cell.classList.remove("jump-active");
    }, 650);
  }

  const lbl = document.getElementById("cockpit-sector-label");
  if (lbl) {
    const dict = HUD_I18N[hudLang] || HUD_I18N.ru;
    if (sector === 5 || sector === 0) {
      lbl.textContent = dict.sector_center;
    } else {
      lbl.textContent = `${dict.sector_n} ${sector}`;
    }
  }

  showMouseCockpit();
}

function showOverlay(iconId, title, desc, progress) {
  const box = document.getElementById("hud-icon");
  if (HUD_SVGS[iconId]) {
    box.innerHTML = HUD_SVGS[iconId];
  } else if (typeof iconId === "string" && iconId.startsWith("<svg")) {
    box.innerHTML = iconId;
  } else {
    box.innerHTML = HUD_SVGS["command"];
  }

  document.getElementById("hud-title").textContent = title;
  document.getElementById("hud-sub").textContent = desc;

  const barBg = document.getElementById("hud-bar-bg");
  const barFill = document.getElementById("hud-bar-fill");
  if (typeof progress === "number" && progress >= 0) {
    barBg.style.display = "block";
    barFill.style.width = Math.min(100, Math.max(0, progress)) + "%";
  } else {
    barBg.style.display = "none";
  }

  const card = document.getElementById("hud-card");
  const mouseCard = document.getElementById("mouse-hud");
  if (mouseCard) mouseCard.classList.remove("visible");
  card.classList.add("visible");

  if (invoke) {
    invoke("show_hud_window").catch(() => {});
  }

  if (hideTimer) clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    card.classList.remove("visible");
    setTimeout(() => {
      if (invoke && !isMouseActive) invoke("hide_hud_window").catch(() => {});
    }, 240);
  }, 1600);
}

window.addEventListener("DOMContentLoaded", initHud);
