import { applyGlassConfig, applyTheme } from "../shared/theme.js";
import { getIntlLocale, setLanguage } from "../shared/i18n.js";
import { createNotchRouter, PAGES } from "./router.js";
import { createMascotModule } from "./pages/mascot.js";
import { createMediaModule, fmtTime } from "./pages/media.js";
import { createLyricsModule } from "./pages/lyrics.js";
import { createPomoModule } from "./pages/pomo.js";
import { createWaterModule } from "./pages/water.js";
import { createShelfModule } from "./pages/shelf.js";
import { createTodoModule } from "./pages/todo.js";
import { createCalendarModule } from "./pages/calendar.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWin = window.__TAURI__.window.getCurrentWindow();

const mn = document.getElementById("mn");
const router = createNotchRouter(mn);
const mascot = createMascotModule(router);

let audioCtx = null;
let soundFlags = { pomo: true, waterTap: true, waterRemind: true };
let lastAppliedLang = "";

function getAudioCtx() {
  try {
    if (!audioCtx)
      audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    return audioCtx;
  } catch {
    return null;
  }
}

function playChime(type) {
  if (type === "pomo" && !soundFlags.pomo) return;
  if (type === "water_tap" && !soundFlags.waterTap) return;
  if (type === "water_remind" && !soundFlags.waterRemind) return;
  const ctx = getAudioCtx();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;
    const notes =
      type === "pomo"
        ? [523.25, 659.25, 783.99, 1046.5]
        : type === "water_remind"
          ? [587.33, 783.99, 987.77]
          : [587.33, 880.0];
    notes.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, now + idx * 0.11);
      gain.gain.setValueAtTime(0.001, now + idx * 0.11);
      gain.gain.exponentialRampToValueAtTime(0.14, now + idx * 0.11 + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + idx * 0.11 + 0.65);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now + idx * 0.11);
      osc.stop(now + idx * 0.11 + 0.7);
    });
  } catch {}
}

let media, lyrics, pomo, water, shelf, todo, calendar;

function syncWidgetData() {
  const wState = water.getState();
  invoke("sync_widget_data", {
    todos: todo.getItems(),
    waterCurrentMl: wState.waterCurrentMl,
    lastWaterDate: wState.lastWaterDate,
    shelf: shelf.getItems(),
  }).catch(() => {});
}

lyrics = createLyricsModule({ router, getMedia: () => media });
media = createMediaModule({
  router,
  mascot,
  onTrackChanged: (tr, ar, dur) => lyrics.onTrackChanged(tr, ar, dur),
  onTickSync: () => lyrics.syncScroll(),
});
pomo = createPomoModule({ router, mascot, playChime, getAudioCtx });
water = createWaterModule({ router, mascot, playChime, syncWidgetData });
shelf = createShelfModule({ router, syncWidgetData });
todo = createTodoModule({ syncWidgetData });
calendar = createCalendarModule({ router, mascot, playChime });

// Router lifecycle hooks
router.hooks.onHideInputs = (expanded, currentPage) => {
  if (todo.isInputOpen() && (!expanded || currentPage !== "todo"))
    todo.hideInput();
  if (calendar.isInputOpen() && (!expanded || currentPage !== "calendar"))
    calendar.hideInput();
};
router.hooks.onPageEnter = (next) => {
  if (next === "calendar") calendar.onEnter();
  if (next === "todo") todo.onEnter();
};
router.hooks.onStateChange = (next) => {
  lyrics.onStateChange(next);
};

// Navigasi tombol & dots
mn.addEventListener("click", (e) => {
  if (
    router.state.interactionMode === "click" &&
    !router.state.isExpandedByClick
  ) {
    e.stopPropagation();
    router.state.isExpandedByClick = true;
    router.clearPeek();
    media.syncVolumeFromSystem();
    router.renderActivePage();
  }
});

document.querySelectorAll(".js-prev-page").forEach((b) => {
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    router.prevPage();
  });
});
document.querySelectorAll(".js-next-page").forEach((b) => {
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    router.nextPage();
  });
});
document.getElementById("pageDots")?.addEventListener("click", (e) => {
  e.stopPropagation();
  const dot = e.target.closest("i[data-page]");
  if (dot) router.goToPage(parseInt(dot.dataset.page, 10));
});

// Wheel navigation & scroll
let wheelCooldown = 0;
window.addEventListener(
  "wheel",
  (e) => {
    if (
      !router.isNotchExpanded() ||
      document.activeElement?.tagName === "INPUT"
    )
      return;

    const inTodo = e.target.closest("#todoList");
    if (inTodo && todo.getItems().length > 2) {
      e.stopPropagation();
      if (
        todo.getStyle() === "horizontal" &&
        Math.abs(e.deltaY) > Math.abs(e.deltaX)
      ) {
        inTodo.scrollLeft += e.deltaY * 1.3;
        e.preventDefault();
      }
      return;
    }

    const inShelf = e.target.closest("#shelfItems");
    if (inShelf && shelf.getCount() > 4) {
      e.stopPropagation();
      if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
        inShelf.scrollLeft += e.deltaY * 1.2;
        e.preventDefault();
      }
      return;
    }

    if (e.target.closest("#volWrap")) {
      e.stopPropagation();
      e.preventDefault();
      media.handleVolumeWheel(e.deltaY);
      return;
    }

    if (router.state.isLyricsOpen) return;

    const now = performance.now();
    if (now < wheelCooldown) return;
    if (Math.abs(e.deltaX) > 20 || Math.abs(e.deltaY) > 20) {
      const dir = e.deltaX + e.deltaY > 0 ? 1 : -1;
      const next = router.state.pageIdx + dir;
      if (next >= 0 && next < PAGES.length) {
        wheelCooldown = now + 180;
        router.goToPage(next, dir);
      }
    }
  },
  { passive: false },
);

async function applyConfigData(cfg) {
  const resolvedLang = await setLanguage(cfg.language || "auto");
  const langChanged = resolvedLang !== lastAppliedLang;
  lastAppliedLang = resolvedLang;

  applyTheme(cfg.theme_preset, cfg);
  applyGlassConfig(cfg);

  router.state.interactionMode = cfg.interaction_mode || "hover";
  router.state.restingMode = cfg.resting_mode || "pet";
  router.state.dynamicIsland = !!cfg.dynamic_island;
  router.state.lyricsPinnedOpen = !!cfg.lyrics_pinned_open;
  router.state.peekDurationMs = cfg.peek_duration_ms || 2400;

  soundFlags.pomo = cfg.enable_sound !== false;
  soundFlags.waterTap = cfg.water_tap_sound !== false;
  soundFlags.waterRemind = cfg.water_reminder_sound !== false;

  mascot.render(cfg.mascot_style);
  media.onConfig(cfg);
  pomo.onConfig(cfg);
  water.onConfig(cfg);
  shelf.onConfig(cfg);
  todo.onConfig(cfg);
  calendar.onConfig(cfg);

  if (langChanged) {
    media.refreshLanguage();
    lyrics.refreshLanguage();
    pomo.refreshLanguage();
    shelf.refreshLanguage();
    todo.refreshLanguage();
    calendar.refreshLanguage();
    updateClock();
  }

  router.renderActivePage();
}

// === PER-MONITOR TARGETED EVENTS ===
appWin.listen("notch-hover", (ev) => {
  mascot.bumpActivity();
  router.state.isHovered = !!ev.payload;
  if (!router.state.isHovered) {
    router.state.isExpandedByClick = false;
  } else if (router.state.interactionMode === "hover") {
    router.clearPeek();
    media.syncVolumeFromSystem();
  }
  router.renderActivePage();
});

appWin.listen("notch-proximity", (ev) => {
  router.state.isObstructed = !!ev.payload;
  if (!router.isNotchExpanded()) router.renderActivePage();
});

appWin.listen("notch-hotkey-toggle", (ev) => {
  const open = !!ev.payload;
  router.state.isHovered = open;
  router.state.isExpandedByClick = open;
  if (open) media.syncVolumeFromSystem();
  router.renderActivePage();
});

appWin.listen("shelf-drag-enter", () => shelf.handleDragEnter());
appWin.listen("shelf-drag-leave", () => shelf.handleDragLeave());
appWin.listen("shelf-drag-position", (ev) =>
  shelf.handleDragPosition(ev.payload),
);
appWin.listen("shelf-drag-drop", (ev) => shelf.handleDropPaths(ev.payload));

if (typeof appWin.onDragDropEvent === "function") {
  appWin.onDragDropEvent((ev) => {
    const p = ev.payload;
    if (!p) return;
    if (p.type === "enter") {
      shelf.handleDragEnter();
    } else if (p.type === "over" && p.position) {
      const dpr = window.devicePixelRatio || 1;
      shelf.handleDragPosition({
        x: p.position.x / dpr,
        y: p.position.y / dpr,
      });
    } else if (p.type === "drop") {
      shelf.handleDropPaths(p.paths || []);
    } else if (p.type === "leave") {
      shelf.handleDragLeave();
    }
  });
}

// === GLOBAL BROADCAST EVENTS ===
listen("config-updated", (ev) => applyConfigData(ev.payload));
listen("calendar-updated", (ev) => calendar.loadJson(ev.payload));
listen("media-update", (ev) => media.applyMediaPayload(ev.payload));
listen("media-seek-sync", (ev) => media.syncSeekFromRemote(ev.payload));
listen("lyrics-data-sync", (ev) => lyrics.applySharedData(ev.payload));
listen("lyrics-ui-sync", (ev) => lyrics.applySharedUi(ev.payload));
listen("volume-sync", (ev) => {
  if (!media.isVolBusy())
    media.renderVolumeUI(ev.payload.volume, ev.payload.muted);
});

listen("audio-peak", (ev) => {
  const peak = Number(ev.payload) || 0;
  mascot.setAudioBeat(peak);
  document.querySelectorAll(".js-bars").forEach((barBox) => {
    const bars = barBox.querySelectorAll("i");
    if (peak > 0.02) {
      barBox.classList.remove("playing");
      const mults = [0.65, 1.0, 0.85, 0.95, 0.7];
      bars.forEach((b, i) => {
        b.style.height = `${Math.max(22, Math.min(100, Math.round(peak * mults[i] * 115)))}%`;
      });
    } else {
      bars.forEach((b) => {
        b.style.height = "";
      });
      barBox.classList.toggle("playing", router.state.isPlaying);
    }
  });
});

listen("system-peek", (ev) => {
  const p = ev.payload;
  if (p.kind === "battery_sync") {
    document.documentElement.style.setProperty("--bat", `${p.percent}%`);
    return;
  }
  mascot.bumpActivity();
  if (
    ["capslock", "numlock", "kb_layout", "clipboard", "battery_low"].includes(
      p.kind,
    )
  ) {
    mascot.setMood("surprised", 1800);
  } else if (
    ["charging", "bluetooth", "usb", "network"].includes(p.kind) &&
    p.active
  ) {
    mascot.setMood("happy", 2200);
  }

  if (p.kind === "volume") {
    const ico =
      !p.active || p.percent === 0
        ? `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:#ff7b7b;fill:none;stroke-width:2"><path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M22 9l-6 6M16 9l6 6"/></svg>`
        : `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c1);fill:none;stroke-width:2"><path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/></svg>`;
    router.showCustomPeek(
      ico,
      "Volume",
      p.label || "System Master Audio",
      p.active ? `${p.percent}%` : "MUTE",
      p.active ? "var(--c1)" : "#ff7b7b",
      p.percent,
    );
  } else if (p.kind === "brightness") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:#feca57;fill:none;stroke-width:2"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>`,
      "Brightness",
      p.label || "Display Backlight",
      `${p.percent}%`,
      "#feca57",
      p.percent,
    );
  } else if (p.kind === "bluetooth") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:#A2E4F5;fill:none;stroke-width:2"><path d="m6.5 6.5 11 11L12 23V1l5.5 5.5-11 11"/></svg>`,
      "Bluetooth",
      p.label || "Bluetooth",
      p.active ? (p.percent > 0 ? "LINKED" : "ON") : "OFF",
      p.active ? "#A2E4F5" : "rgba(255,255,255,.45)",
    );
  } else if (p.kind === "capslock") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c1);fill:none;stroke-width:2"><rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M8 15h8"/></svg>`,
      "Caps Lock",
      p.label || "Caps Lock",
      p.active ? "ON" : "OFF",
      p.active ? "var(--c1)" : "rgba(255,255,255,.45)",
    );
  } else if (p.kind === "numlock") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c2);fill:none;stroke-width:2"><rect x="3" y="3" width="18" height="18" rx="3"/><path d="M8 8h.01M12 8h.01M16 8h.01M8 12h.01M12 12h.01M16 12h.01M8 16h4M16 16h.01"/></svg>`,
      "Num Lock",
      p.label || "Numpad",
      p.active ? "ON" : "OFF",
      p.active ? "var(--c2)" : "rgba(255,255,255,.45)",
    );
  } else if (p.kind === "kb_layout") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c3);fill:none;stroke-width:2"><circle cx="12" cy="12" r="9"/><path d="M2 12h20M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/></svg>`,
      "Keyboard",
      "Input Language",
      p.label || "EN",
      "var(--c3)",
    );
  } else if (p.kind === "clipboard") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c1);fill:none;stroke-width:2"><rect x="8" y="2" width="8" height="4" rx="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/></svg>`,
      "Clipboard",
      p.label || "Copied",
      "COPIED",
      "var(--c1)",
    );
  } else if (p.kind === "usb") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:${p.active ? "#33e07a" : "#feca57"};fill:none;stroke-width:2"><path d="M12 2v15M8 6l4-4 4 4M6 12v2a6 6 0 0 0 6 6 6 6 0 0 0 6-6v-2"/><circle cx="6" cy="11" r="1.5"/><rect x="16.5" y="9.5" width="3" height="3" rx=".5"/></svg>`,
      p.active ? "Drive Connected" : "Drive Ejected",
      p.label || "Storage",
      p.active ? "READY" : "SAFE",
      p.active ? "#33e07a" : "#feca57",
    );
  } else if (p.kind === "network") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:${p.active ? "#33e07a" : "#ff7b7b"};fill:none;stroke-width:2"><path d="M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0"/><circle cx="12" cy="20" r="1"/></svg>`,
      "Network",
      p.label || "",
      p.active ? "ONLINE" : "OFFLINE",
      p.active ? "#33e07a" : "#ff7b7b",
    );
  } else if (p.kind === "battery_low") {
    const crit = !p.active;
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:${crit ? "#ff5b6b" : "#feca57"};fill:none;stroke-width:2"><rect x="2" y="7" width="18" height="10" rx="2"/><path d="M22 11v2M7 11v2"/></svg>`,
      crit ? "Battery Critical" : "Low Battery",
      p.label || "Plug in charger",
      `${p.percent}%`,
      crit ? "#ff5b6b" : "#feca57",
      p.percent,
    );
  } else if (p.kind === "charging") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;fill:${p.active ? "#33e07a" : "#feca57"}"><path d="M13 2L3 14h7l-1 8 10-12h-7z"/></svg>`,
      p.active ? "Charging" : "On Battery",
      p.label || "",
      `${p.percent}%`,
      p.active ? "#33e07a" : "#feca57",
      p.percent,
    );
  } else if (p.kind === "mic") {
    router.showCustomPeek(
      `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:${p.active ? "#33e07a" : "#ff7b7b"};fill:none;stroke-width:2"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2M12 19v4M8 23h8"/></svg>`,
      "Microphone",
      p.label || "",
      p.active ? "LIVE" : "MUTED",
      p.active ? "#33e07a" : "#ff7b7b",
    );
  }
});

// Live Pill Dynamic Island sync
let lastLiveKind = null;
setInterval(() => {
  const k = router.liveKind();
  const liveEl = document.getElementById("mmLive");
  if (router.state.dynamicIsland && k !== lastLiveKind) {
    lastLiveKind = k;
    if (!router.isNotchExpanded() && mn.dataset.s !== "peek")
      router.renderActivePage();
  }
  if (k && liveEl) {
    liveEl.dataset.k = k;
    if (k === "pomo") {
      const { remSec, totalSec } = pomo.getLiveState();
      document.getElementById("liveTime").textContent = fmtTime(remSec);
      document
        .getElementById("liveRing")
        .style.setProperty(
          "--lp",
          ((1 - remSec / Math.max(1, totalSec)) * 100).toFixed(1),
        );
    }
  }
}, 1000);

// Play/Pause icon pop morph
document.querySelectorAll(".js-play-icon").forEach((el) => {
  new MutationObserver(() => {
    el.classList.remove("pop");
    void el.getBoundingClientRect();
    el.classList.add("pop");
  }).observe(el, { subtree: true, attributes: true, attributeFilter: ["d"] });
});

// Pause animasi CSS saat hidden
document.addEventListener("visibilitychange", () => {
  document.documentElement.classList.toggle("mm-paused", document.hidden);
});

// Clock
function updateClock() {
  const d = new Date();
  const tm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  const dt = d.toLocaleDateString(getIntlLocale(), {
    weekday: "short",
    day: "numeric",
    month: "short",
    year: "numeric",
  });
  document.querySelectorAll(".js-time").forEach((e) => {
    e.textContent = tm;
  });
  document.querySelectorAll(".js-date").forEach((e) => {
    e.textContent = dt;
  });
  water.checkDailyAutoReset();
}
updateClock();
setInterval(updateClock, 5000);

// Opening animation
function startOpeningAnimation() {
  if (router.state.bootPhase !== "boot") return;
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      router.state.bootPhase = "intro";
      router.renderActivePage();
      const fill = document.getElementById("introBarFill");
      if (fill) fill.style.transform = "scaleX(1)";
      setTimeout(() => {
        router.state.bootPhase = "done";
        router.renderActivePage();
      }, 880);
    });
  });
}

appWin.listen("notch-revealed", () => setTimeout(startOpeningAnimation, 50));

// Init utama
async function boot() {
  const [cfg, curMedia] = await Promise.all([
    invoke("get_config"),
    invoke("get_current_media").catch(() => null),
  ]);
  await applyConfigData(cfg);
  if (curMedia && curMedia.has_media) media.applyMediaPayload(curMedia);
  await lyrics.hydrateFromShared();
  media.syncVolumeFromSystem();

  let sent = false;
  const sendReady = () => {
    if (sent) return;
    sent = true;
    invoke("notch_ready").catch(() => {});
    setTimeout(startOpeningAnimation, 650);
  };
  requestAnimationFrame(() => requestAnimationFrame(sendReady));
  setTimeout(sendReady, 300);
}

boot();
