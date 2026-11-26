import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";

const { invoke } = window.__TAURI__.core;

const measureCanvas = document.createElement("canvas");
const measureCtx = measureCanvas.getContext("2d");

export function measureTextPx(text, fontSpec) {
  if (!text) return 0;
  measureCtx.font = fontSpec;
  return measureCtx.measureText(text).width;
}

export function fmtTime(sec) {
  sec = Math.max(0, Math.floor(sec));
  return `${String(Math.floor(sec / 60)).padStart(2, "0")}:${String(sec % 60).padStart(2, "0")}`;
}

export function applyMarqueeToElements(selector, text) {
  const clean = String(text || "");
  document.querySelectorAll(selector).forEach((el) => {
    if (el.dataset.rawText === clean) return;
    el.dataset.rawText = clean;
    const maxW = parseFloat(el.dataset.maxw || "252");
    const fontSpec = el.dataset.font || "600 14px Segoe UI";
    const textW = measureTextPx(clean, fontSpec);

    if (textW > maxW) {
      const dur = `${Math.max(6, (textW + 36) / 28).toFixed(1)}s`;
      el.classList.add("is-marquee");
      el.innerHTML = `<span class="mq-track" style="--mq-dur:${dur}"><span>${escHtml(clean)}</span><span class="mq-gap">•</span><span>${escHtml(clean)}</span><span class="mq-gap">•</span></span>`;
    } else {
      el.classList.remove("is-marquee");
      el.textContent = clean;
    }
  });
}

export function createMediaModule({
  router,
  mascot,
  onTrackChanged,
  onTickSync,
}) {
  let curTrackKey = "";
  let curPos = 0;
  let curDur = 0;
  let lastRawGsmtcPos = -1;
  let lastTickMs = performance.now();
  let albumArtGlow = true;
  let cachedAlbumGradient = "";

  // Volume state
  let sysVol = 50;
  let lastNonZeroVol = 50;
  let sysMuted = false;
  let volBusy = false;
  let pendingVol = null;
  let volTimer = null;

  function syncActiveMusicGlow() {
    document.documentElement.style.setProperty(
      "--lyr-glow",
      albumArtGlow && cachedAlbumGradient ? cachedAlbumGradient : "var(--glow)",
    );
  }

  function setAlbumBackdrop(dataUrl) {
    ["musicBd", "lyricsBd"].forEach((id) => {
      const bd = document.getElementById(id);
      if (!bd) return;
      if (dataUrl) {
        bd.style.backgroundImage = `linear-gradient(rgba(12,9,20,.28), rgba(12,9,20,.52)), url('${dataUrl}')`;
        bd.classList.add("on");
      } else {
        bd.classList.remove("on");
        bd.style.backgroundImage = "";
      }
    });
  }

  function extractAlbumGradient(dataUrl) {
    if (!dataUrl) {
      cachedAlbumGradient = "";
      syncActiveMusicGlow();
      setAlbumBackdrop("");
      return;
    }
    setAlbumBackdrop(dataUrl);
    const img = new Image();
    img.onload = () => {
      const cv = document.createElement("canvas");
      cv.width = 12;
      cv.height = 12;
      const ctx = cv.getContext("2d");
      ctx.drawImage(img, 0, 0, 12, 12);
      const d = ctx.getImageData(0, 0, 12, 12).data;
      let r = 0,
        g = 0,
        b = 0,
        cnt = 0;
      for (let i = 0; i < d.length; i += 4) {
        r += d[i];
        g += d[i + 1];
        b += d[i + 2];
        cnt++;
      }
      r = Math.round(r / cnt);
      g = Math.round(g / cnt);
      b = Math.round(b / cnt);
      const c1 = `rgb(${Math.min(255, r + 35)}, ${Math.min(255, g + 35)}, ${Math.min(255, b + 45)})`;
      const c2 = `rgb(${Math.max(16, r - 20)}, ${Math.max(16, g - 20)}, ${Math.max(24, b - 10)})`;
      cachedAlbumGradient = `linear-gradient(90deg, ${c2}, ${c1} 50%, ${c2})`;
      syncActiveMusicGlow();
    };
    img.src = dataUrl;
  }

  function getPlaybackPos() {
    return (
      curPos +
      (router.state.isPlaying ? (performance.now() - lastTickMs) / 1000 : 0)
    );
  }

  function updateMediaUI() {
    document.querySelectorAll(".js-pos").forEach((e) => {
      e.textContent = fmtTime(curPos);
    });
    const rem = curDur > 0 ? Math.max(0, curDur - curPos) : 0;
    document.querySelectorAll(".js-rem").forEach((e) => {
      e.textContent = `-${fmtTime(rem)}`;
    });
    const pct = curDur > 0 ? Math.min(100, (curPos / curDur) * 100) : 0;
    document.querySelectorAll(".js-prog").forEach((e) => {
      e.style.width = `${pct}%`;
    });
    onTickSync();
  }

  function seekToSec(sec) {
    curPos = Math.max(0, sec);
    lastRawGsmtcPos = curPos;
    lastTickMs = performance.now();
    invoke("media_seek", { positionSec: curPos }).catch(() => {});
    updateMediaUI();
  }

  function syncSeekFromRemote(sec) {
    if (typeof sec !== "number" || !Number.isFinite(sec)) return;
    curPos = Math.max(0, sec);
    lastRawGsmtcPos = curPos;
    lastTickMs = performance.now();
    updateMediaUI();
  }

  function applyMediaPayload(m) {
    if (!m) return;
    const wasPlaying = router.state.isPlaying;
    router.state.hasMedia = !!m.has_media;
    router.state.isPlaying = !!m.is_playing;

    if (router.state.isPlaying) {
      mascot.bumpActivity();
      if (!wasPlaying) mascot.setMood("happy", 3200);
    }

    document
      .querySelectorAll(".js-bars")
      .forEach((b) => b.classList.toggle("playing", router.state.isPlaying));
    const playPath = router.state.isPlaying
      ? "M7 5h4v14H7zM13 5h4v14h-4z"
      : "M8 5v14l11-7z";
    document
      .querySelectorAll(".js-play-icon path")
      .forEach((p) => p.setAttribute("d", playPath));

    if (!m.has_media) {
      refreshLanguage();
      return;
    }

    applyMarqueeToElements(".js-title", m.title);
    applyMarqueeToElements(".js-artist", m.artist);
    if (m.duration_sec > 1) curDur = m.duration_sec;

    const key = `${m.title}::${m.artist}`;
    if (key !== curTrackKey) {
      curTrackKey = key;
      curPos = m.position_sec || 0;
      lastRawGsmtcPos = curPos;
      lastTickMs = performance.now();
      if (m.thumb_changed && m.thumbnail_b64) {
        document.querySelectorAll(".js-art").forEach((el) => {
          el.style.backgroundImage = `url('${m.thumbnail_b64}')`;
        });
        extractAlbumGradient(m.thumbnail_b64);
      } else if (m.thumb_changed) {
        document.querySelectorAll(".js-art").forEach((el) => {
          el.style.backgroundImage = "";
        });
        extractAlbumGradient("");
      }
      onTrackChanged(m.title, m.artist, curDur);
    } else if (Math.abs((m.position_sec || 0) - lastRawGsmtcPos) > 0.5) {
      lastRawGsmtcPos = m.position_sec;
      curPos = m.position_sec;
      lastTickMs = performance.now();
    }

    updateMediaUI();
  }

  function pulseVolumeDrawer() {
    const wrap = document.getElementById("volWrap");
    if (!wrap) return;
    wrap.classList.add("active");
    clearTimeout(volTimer);
    volTimer = setTimeout(() => wrap.classList.remove("active"), 1300);
  }

  function renderVolumeUI(vol, muted) {
    sysVol = vol;
    if (vol > 0) lastNonZeroVol = vol;
    sysMuted = muted || vol === 0;
    const eff = sysMuted ? 0 : vol;
    const wrap = document.getElementById("volWrap");
    const slider = document.getElementById("volSlider");
    const icon = document.getElementById("volIcon");
    const pctTxt = document.getElementById("volPctTxt");

    if (wrap) {
      wrap.classList.toggle("muted", sysMuted);
      wrap.style.setProperty("--vol-pct", `${eff}%`);
    }
    if (slider) slider.value = String(eff);
    if (pctTxt) pctTxt.textContent = sysMuted ? "MUTE" : `${vol}%`;
    if (icon) {
      if (sysMuted) {
        icon.innerHTML = `<path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M22 9l-6 6M16 9l6 6"/>`;
      } else if (vol < 50) {
        icon.innerHTML = `<path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>`;
      } else {
        icon.innerHTML = `<path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>`;
      }
    }
  }

  async function onVolumeSlide(val) {
    const v = Math.max(0, Math.min(100, parseInt(val, 10) || 0));
    pulseVolumeDrawer();
    renderVolumeUI(v, v === 0);
    pendingVol = v;
    if (volBusy) return;
    volBusy = true;
    while (pendingVol !== null) {
      const target = pendingVol;
      pendingVol = null;
      try {
        await invoke("set_system_volume", {
          volume: target,
          mute: target === 0,
        });
      } catch {}
    }
    volBusy = false;
  }

  async function toggleMute() {
    pulseVolumeDrawer();
    try {
      const res = sysMuted
        ? await invoke("set_system_volume", {
            volume: sysVol > 0 ? sysVol : lastNonZeroVol || 35,
            mute: false,
          })
        : await invoke("set_system_volume", { volume: null, mute: true });
      renderVolumeUI(res.volume, res.muted);
    } catch {}
  }

  async function syncVolumeFromSystem() {
    try {
      const res = await invoke("get_system_volume");
      renderVolumeUI(res.volume, res.muted);
    } catch {}
  }

  function handleVolumeWheel(deltaY) {
    pulseVolumeDrawer();
    const step = deltaY < 0 ? 4 : -4;
    const base = sysMuted ? 0 : sysVol;
    onVolumeSlide(Math.max(0, Math.min(100, base + step)));
  }

  function refreshLanguage() {
    if (!router.state.hasMedia) {
      applyMarqueeToElements(".js-title", t("notch.no_media"));
      applyMarqueeToElements(".js-artist", t("notch.play_hint"));
    }
  }

  function onConfig(cfg) {
    albumArtGlow = cfg.album_art_glow !== false;
    syncActiveMusicGlow();
  }

  document.querySelectorAll(".js-media-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      invoke("media_action", { action: btn.dataset.act }).catch(() => {});
    });
  });

  document.querySelectorAll(".js-prog-bar").forEach((bar) => {
    bar.addEventListener("click", (e) => {
      e.stopPropagation();
      if (curDur <= 0) return;
      const rect = bar.getBoundingClientRect();
      const frac = Math.max(
        0,
        Math.min(1, (e.clientX - rect.left) / rect.width),
      );
      seekToSec(frac * curDur);
    });
  });

  document
    .getElementById("volWrap")
    ?.addEventListener("click", (e) => e.stopPropagation());
  document.getElementById("volMuteBtn")?.addEventListener("click", toggleMute);
  document
    .getElementById("volSlider")
    ?.addEventListener("input", (e) => onVolumeSlide(e.target.value));

  setInterval(() => {
    const now = performance.now();
    const dt = (now - lastTickMs) / 1000;
    lastTickMs = now;
    if (router.state.isPlaying && router.state.hasMedia) {
      curPos += dt;
      if (curDur > 0 && curPos > curDur) curPos = curDur;
      updateMediaUI();
    }
  }, 200);

  return {
    applyMediaPayload,
    syncVolumeFromSystem,
    renderVolumeUI,
    isVolBusy: () => volBusy,
    handleVolumeWheel,
    getPlaybackPos,
    getDuration: () => curDur,
    setDurationIfEmpty: (d) => {
      if (curDur <= 1 && d > 0) curDur = d;
    },
    getTrackTitle: () => (curTrackKey ? curTrackKey.split("::")[0] : ""),
    seekToSec,
    syncSeekFromRemote,
    onConfig,
    refreshLanguage,
  };
}
