import {
  applyGlassConfig,
  applyTheme,
  GLASS_DEFAULTS,
} from "../shared/theme.js";
import { setLanguage, t } from "../shared/i18n.js";
import { createRouter } from "./router.js";
import { mascotMarkup, showToast } from "./ui.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWin = window.__TAURI__.window.getCurrentWindow();

const VERSION_MANIFEST_URL =
  "https://raw.githubusercontent.com/MANG-CODER/MangNotch/main/version.json";

let currentCfg = null;
let monitors = [];
let appVersion = "2.1.0";
let logoDataUrl = "";
let selfSaveTimer = null;
let ignoreNextExternal = 0;

function updateStaticChrome() {
  document.getElementById("settingsSearch").placeholder = t("app.search");
  document.getElementById("clearShelfTxt").textContent = t("app.clear_shelf");
  document.getElementById("clearShelfBtn").title = t("app.clear_shelf_tip");
  document.getElementById("closeBtn").title = t("app.close");
  document.getElementById("heroSubtitle").textContent = t("app.subtitle");
  document.getElementById("forcedTitle").textContent = t("app.force_title");
  document.getElementById("forcedUpdateBtn").textContent = t("app.force_btn");
}

function updateHeroPreview() {
  if (!currentCfg) return;
  const npNotch = document.getElementById("npNotch");
  const npMascot = document.getElementById("npMascot");
  if (npNotch) npNotch.classList.toggle("island", !!currentCfg.dynamic_island);
  if (npMascot) npMascot.innerHTML = mascotMarkup(currentCfg.mascot_style);
}

function persistConfig(silent = false) {
  if (!currentCfg) return;
  ignoreNextExternal = Date.now() + 400;
  invoke("save_full_config", { newCfg: currentCfg })
    .then(() => {
      if (!silent) showToast(t("app.saved"));
    })
    .catch(() => {});
}

function previewGlass(patch) {
  if (!currentCfg) return;
  Object.assign(currentCfg, patch);
  applyTheme(currentCfg.theme_preset, currentCfg);
  applyGlassConfig(currentCfg);
  clearTimeout(selfSaveTimer);
  selfSaveTimer = setTimeout(() => persistConfig(true), 60);
}

function updateConfig(patch, opts = {}) {
  if (!currentCfg) return;
  Object.assign(currentCfg, patch);
  applyTheme(currentCfg.theme_preset, currentCfg);
  applyGlassConfig(currentCfg);
  updateHeroPreview();
  persistConfig(false);
  if (opts.rerender) router.renderAll(false);
}

async function onLanguageChange(lang) {
  if (!currentCfg) return;
  currentCfg.language = lang;
  await setLanguage(lang);
  updateStaticChrome();
  router.renderAll(false);
  persistConfig(false);
}

async function resetDefaultConfig() {
  if (!currentCfg) return;
  const keepTodos = currentCfg.todos || [];
  const keepShelf = currentCfg.shelf || [];
  const keepCal = currentCfg.calendar_json || "";
  currentCfg = {
    ...currentCfg,
    language: "auto",
    monitor_mode: "primary",
    selected_monitors: [0],
    use_primary_monitor: true,
    selected_monitor_index: 0,
    hide_on_fullscreen: true,
    smart_proximity: true,
    interaction_mode: "hover",
    hover_close_speed: "normal",
    enable_hotkey: true,
    resting_mode: "pet",
    mascot_style: "neko",
    theme_preset: "dusk",
    custom_c1: "#FFB7C5",
    custom_c2: "#E2D1F9",
    custom_c3: "#A2E4F5",
    dynamic_island: false,
    album_art_glow: true,
    lyrics_pinned_open: false,
    ...GLASS_DEFAULTS,
    enable_audio_visualizer: true,
    ambient_volume: 55,
    enable_peeks: true,
    peek_duration_ms: 2400,
    peek_volume: true,
    peek_brightness: true,
    peek_mic: true,
    peek_charging: true,
    peek_bluetooth: true,
    peek_capslock: true,
    enable_sound: true,
    pomo_auto_break: false,
    pomo_minutes: 25,
    pomo_break_minutes: 5,
    water_reminder_enabled: true,
    water_tap_sound: true,
    water_reminder_sound: true,
    water_goal_ml: 1500,
    water_cup_ml: 250,
    water_interval_min: 45,
    todo_style: "horizontal",
    calendar_holiday_region: "auto",
    calendar_reminder_enabled: true,
    calendar_reminder_lead_min: 10,
    calendar_reminder_sound: true,
    calendar_json: keepCal,
    todos: keepTodos,
    shelf: keepShelf,
  };
  await setLanguage(currentCfg.language);
  applyTheme(currentCfg.theme_preset, currentCfg);
  applyGlassConfig(currentCfg);
  updateStaticChrome();
  updateHeroPreview();
  router.renderAll(false);
  persistConfig(false);
}

const router = createRouter(() => ({
  cfg: currentCfg,
  monitors,
  appVersion,
  logoDataUrl,
  updateConfig,
  previewGlass,
  onLanguageChange,
  openUrl: (url) => invoke("open_external_url", { url }),
  openConfigDir: () => invoke("open_config_folder"),
  resetDefaultConfig,
}));

function compareSemver(v1, v2) {
  const a = String(v1).split(".").map(Number);
  const b = String(v2).split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((a[i] || 0) > (b[i] || 0)) return 1;
    if ((a[i] || 0) < (b[i] || 0)) return -1;
  }
  return 0;
}

async function checkAppUpdate() {
  try {
    const channel = await invoke("get_install_channel");
    const res = await fetch(`${VERSION_MANIFEST_URL}?t=${Date.now()}`, {
      cache: "no-store",
    });
    if (!res.ok) return;
    const data = await res.json();

    const targetUrl =
      channel === "msix"
        ? data.store_url || "ms-windows-store://pdp/?ProductId=9PNWT253Z5RL"
        : data.github_download_url ||
          "https://github.com/MANG-CODER/MangNotch/releases/latest";
    const minVer =
      channel === "msix"
        ? data.store_min_supported_version || "1.0.0"
        : data.min_supported_version || "1.0.0";
    const latestVer =
      channel === "msix"
        ? data.store_latest_version || appVersion
        : data.latest_version || appVersion;

    if (minVer && compareSemver(appVersion, minVer) < 0) {
      document.getElementById("forcedNotes").textContent =
        data.release_notes || t("app.force_desc");
      document.getElementById("forcedUpdateBtn").onclick = () =>
        invoke("launch_update_and_exit", { url: targetUrl });
      document.getElementById("forcedUpdateModal").style.display = "grid";
      invoke("trigger_update_window", { isForced: true });
      return;
    }

    const banner = document.getElementById("updateBanner");
    if (latestVer && compareSemver(appVersion, latestVer) < 0) {
      document.getElementById("updateBannerTitle").textContent =
        `MangNotch v${latestVer} — ${t("app.update_avail")}`;
      document.getElementById("updateBannerSub").textContent =
        data.release_notes || t("app.update_sub");
      const btn = document.getElementById("updateBannerBtn");
      btn.textContent = t("app.update_now");
      btn.onclick = () => invoke("launch_update_and_exit", { url: targetUrl });
      banner.style.display = "flex";
    } else {
      banner.style.display = "none";
    }
  } catch {}
}

async function init() {
  try {
    appVersion = await invoke("get_app_version");
  } catch {}
  document.getElementById("hdrVerPill").textContent = `v${appVersion}`;

  try {
    logoDataUrl = await invoke("get_app_logo");
    document.querySelectorAll(".js-app-logo").forEach((img) => {
      img.src = logoDataUrl;
    });
  } catch {}

  monitors = await invoke("get_monitors");
  currentCfg = await invoke("get_config");

  await setLanguage(currentCfg.language || "auto");
  applyTheme(currentCfg.theme_preset, currentCfg);
  applyGlassConfig(currentCfg);
  updateStaticChrome();
  updateHeroPreview();
  router.renderAll(false);

  checkAppUpdate();
}

document.getElementById("settingsSearch").addEventListener("input", (e) => {
  router.setSearch(e.target.value);
});
document.getElementById("clearShelfBtn").addEventListener("click", () => {
  updateConfig({ shelf: [] });
});
document.getElementById("closeBtn").addEventListener("click", () => {
  appWin.close();
});

const startDrag = (e) => {
  if (
    e.buttons !== 1 ||
    e.target.closest(
      "button, input, label, .csel-wrap, .nav-item, .seg-bar, .peek-tile, .lg-slider, .color-pill",
    )
  )
    return;
  appWin.startDragging();
};
document
  .getElementById("topbarDragArea")
  .addEventListener("mousedown", startDrag);
document
  .getElementById("sidebarDragArea")
  .addEventListener("mousedown", startDrag);

listen("config-updated", async (ev) => {
  if (Date.now() < ignoreNextExternal) return;
  const prevLang = currentCfg?.language;
  const prevTodos = JSON.stringify(currentCfg?.todos || []);
  currentCfg = ev.payload;

  if (currentCfg.language !== prevLang) {
    await setLanguage(currentCfg.language || "auto");
    updateStaticChrome();
    router.renderAll(false);
  } else if (
    router.getActiveId() === "productivity" &&
    JSON.stringify(currentCfg.todos || []) !== prevTodos
  ) {
    router.renderAll(false);
  }

  applyTheme(currentCfg.theme_preset, currentCfg);
  applyGlassConfig(currentCfg);
  updateHeroPreview();
});

init();
