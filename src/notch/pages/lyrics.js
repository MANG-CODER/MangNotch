import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";
import { applyMarqueeToElements, measureTextPx } from "./media.js";

const { invoke } = window.__TAURI__.core;

const NOTE_SVG = `<svg class="music-note-svg" viewBox="0 0 24 24"><path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/></svg>`;
const YL_HOSTS = [
  "https://lyricsplus.prjktla.my.id",
  "https://lyricsplus.binimum.org",
];
const PX_HOST = "https://lyrics.paxsenix.org";

async function httpJson(url, ms = 7500, preferNative = false) {
  const tryNative = async () => {
    const res = await Promise.race([
      invoke("lyrics_http_get", { url }),
      new Promise((_, rej) => setTimeout(() => rej(new Error("timeout")), ms)),
    ]);
    if (res && res.status >= 200 && res.status < 300 && res.body) {
      return JSON.parse(res.body);
    }
    throw new Error(String(res?.status || "native_fail"));
  };

  const tryFetch = async () => {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), ms);
    try {
      const r = await fetch(url, { signal: ctrl.signal });
      if (!r.ok) throw new Error(String(r.status));
      return await r.json();
    } finally {
      clearTimeout(timer);
    }
  };

  if (preferNative) {
    try {
      return await tryNative();
    } catch {
      return await tryFetch();
    }
  }
  try {
    return await tryFetch();
  } catch {
    return await tryNative();
  }
}

function makeWordRow(ws) {
  const clean = ws.filter((w) => w.x !== "");
  const text = clean
    .map((w) => w.x)
    .join("")
    .replace(/\s+/g, " ")
    .trim();
  if (!clean.length || !text) return null;
  return {
    time: clean[0].t,
    text,
    isNote: false,
    words: clean.map((w) => ({ ...w, held: w.e - w.t >= 1 })),
  };
}

function spacer(list, getText, getPart) {
  const spaced = list.slice(0, -1).some((p) => /\s$/.test(getText(p)));
  return (p, i) =>
    getText(p) +
    (!spaced && i < list.length - 1 && !getPart(p) && !/\s$/.test(getText(p))
      ? " "
      : "");
}

function parseLrc(txt) {
  const out = [];
  String(txt || "")
    .replace(/\r/g, "")
    .split("\n")
    .forEach((raw) => {
      const line = raw.trim();
      const m = line.match(/^((?:\[\d+:\d+(?:\.\d+)?\])+)(.*)$/);
      if (!m) return;
      const text = m[2].trim();
      if (!text || /^(作词|作曲|编曲|作詞|編曲|词|曲)\s*[:：]/.test(text))
        return;
      [...m[1].matchAll(/\[(\d+):(\d+(?:\.\d+)?)\]/g)].forEach((tm) => {
        out.push({ time: +tm[1] * 60 + +tm[2], text, isNote: false });
      });
    });
  return out.sort((a, b) => a.time - b.time);
}

function detectScript(str) {
  const s = String(str || "");
  if (/[\u3040-\u30ff]/.test(s)) return "JA";
  if (/[\uac00-\ud7af]/.test(s)) return "KO";
  if (/[\u4e00-\u9fff]/.test(s)) return "ZH";
  return "LAT";
}

function isScriptCompatible(rows, rawTitle, rawArtist) {
  const sample = rows
    .slice(0, 14)
    .map((r) => r.text)
    .join("");
  const lyricScr = detectScript(sample);
  const trackScr = detectScript(rawTitle + " " + rawArtist);
  if (trackScr === "JA") return lyricScr === "JA" || lyricScr === "LAT";
  if (trackScr === "KO") return lyricScr === "KO" || lyricScr === "LAT";
  if (lyricScr === "ZH") return trackScr === "ZH";
  return true;
}

function isTitleRoughMatch(candidateName, targetTitle) {
  const norm = (s) =>
    String(s || "")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, "");
  const a = norm(candidateName);
  const b = norm(targetTitle);
  if (!a || !b) return true;
  return a.includes(b) || b.includes(a);
}

function normalizeTrack(title, artist) {
  let tr = String(title || "")
    .replace(
      /\s*[([{【][^)\]}】]*(lirik|lyrics?|official|video|audio|visuali[sz]er|m\/v|\bmv\b|\bhd\b|4k|remaster|feat|ft\.|live)[^)\]}】]*[)\]}】]/gi,
      "",
    )
    .replace(/\s*【[^】]*】/g, "")
    .replace(/\s*[~|｜].*$/, "")
    .replace(/\s+(official|lyrics?|lirik)(\s+(video|audio|lagu))?\s*$/i, "")
    .trim();
  let ar = String(artist || "")
    .replace(/\s*-\s*topic$/i, "")
    .replace(/vevo$/i, "")
    .replace(/ちゃんねる$/i, "")
    .replace(/\b(official|channel|music)\b/gi, "")
    .split(/,|&|\/|;| feat\.?| x /i)[0]
    .trim();
  const m = tr.match(/^(.+?)\s+[-–—]\s+(.+)$/);
  if (
    m &&
    !/remaster|live|version|mono|stereo|edit|mix|acoustic|from|feat|ft\./i.test(
      m[2],
    )
  ) {
    ar = m[1].trim();
    tr = m[2].trim();
  }
  return [tr || String(title || "").trim(), ar];
}

async function fetchYouLy(tr, ar, dur) {
  const base =
    "/v2/lyrics/get?title=" +
    encodeURIComponent(tr) +
    "&artist=" +
    encodeURIComponent(ar);
  const queries =
    dur > 1 ? [base + "&duration=" + Math.round(dur), base] : [base];
  for (const q of queries) {
    try {
      const j = await Promise.any(
        YL_HOSTS.map((h) => httpJson(h + q, 6500, true)),
      );
      const rows = (j.lyrics || [])
        .map((it) => {
          const sy = (it.syllabus || []).filter(
            (s) => !s.isBackground && s.text != null,
          );
          if (!sy.length) return null;
          const f = spacer(
            sy,
            (s) => s.text,
            () => false,
          );
          return makeWordRow(
            sy.map((s, i) => ({
              x: f(s, i),
              t: s.time / 1000,
              e: (s.time + (s.duration || 0)) / 1000,
            })),
          );
        })
        .filter(Boolean);
      if (rows.length > 2) return Object.assign(rows, { kind: "word" });
    } catch {}
  }
  return null;
}

async function fetchPax(tr, ar, dur) {
  let hit = null;
  const queries = ar ? [tr + " " + ar, tr] : [tr];
  for (const q of queries) {
    const list = await httpJson(
      PX_HOST + "/apple-music/search?q=" + encodeURIComponent(q),
      6500,
      true,
    ).catch(() => []);
    if (!Array.isArray(list) || !list.length) continue;
    const ranked = list
      .filter((r) => isTitleRoughMatch(r.songName || r.name || "", tr))
      .map((r) => ({ r, diff: Math.abs((r.duration || 0) - dur * 1000) }))
      .sort((a, b) => a.diff - b.diff);
    hit = ranked.find((x) => dur <= 1 || x.diff <= 6000) || ranked[0];
    if (hit) break;
  }
  if (!hit) return null;
  const j = await httpJson(
    PX_HOST + "/apple-music/lyrics?id=" + encodeURIComponent(hit.r.id),
    6500,
    true,
  ).catch(() => null);
  if (!j) return null;
  const rows = (j.content || [])
    .filter((c) => !c.background && (c.text || []).length)
    .map((c) => {
      const f = spacer(
        c.text,
        (p) => p.text,
        (p) => p.part,
      );
      return makeWordRow(
        c.text.map((p, i) => ({
          x: f(p, i),
          t: p.timestamp / 1000,
          e: p.endtime / 1000,
        })),
      );
    })
    .filter(Boolean);
  return rows.length > 2 ? Object.assign(rows, { kind: "word" }) : null;
}

async function fetchNetEase(tr, ar, dur, _onDur, rawTitle, rawArtist) {
  const q = encodeURIComponent(ar ? tr + " " + ar : tr);
  const s = await httpJson(
    "https://music.163.com/api/search/get?s=" + q + "&type=1&limit=10",
    6500,
    true,
  ).catch(() => null);
  if (!s) return null;

  const songs = (s.result && s.result.songs) || [];
  const candidates = songs
    .filter((x) => isTitleRoughMatch(x.name, tr))
    .map((x) => ({ x, diff: Math.abs((x.duration || 0) - dur * 1000) }))
    .filter((o) => dur <= 1 || o.diff <= 4500)
    .sort((a, b) => a.diff - b.diff);

  const hit = candidates[0];
  if (!hit) return null;

  const j = await httpJson(
    "https://music.163.com/api/song/lyric?id=" + hit.x.id + "&lv=1&yv=1",
    6500,
    true,
  ).catch(() => null);
  if (!j) return null;

  const yrc = ((j.yrc && j.yrc.lyric) || "")
    .split("\n")
    .map((raw) => {
      const m = raw.trim().match(/^\[(\d+),\d+\](.*)$/);
      if (!m) return null;
      return makeWordRow(
        [
          ...m[2].matchAll(
            /\((\d+),(\d+),-?\d+\)(.*?)(?=\(\d+,\d+,-?\d+\)|$)/g,
          ),
        ].map((w) => ({
          x: w[3],
          t: +w[1] / 1000,
          e: (+w[1] + +w[2]) / 1000,
        })),
      );
    })
    .filter(Boolean);

  if (yrc.length > 2 && isScriptCompatible(yrc, rawTitle, rawArtist)) {
    return Object.assign(yrc, { kind: "word" });
  }
  const l = parseLrc(j.lrc && j.lrc.lyric);
  if (l.length > 2 && isScriptCompatible(l, rawTitle, rawArtist)) {
    return Object.assign(l, { kind: "line" });
  }
  return null;
}

async function fetchLrcLib(tr, ar, dur, onDur) {
  const tries = [];
  if (tr && ar) {
    tries.push(
      "track_name=" +
        encodeURIComponent(tr) +
        "&artist_name=" +
        encodeURIComponent(ar),
    );
    tries.push("q=" + encodeURIComponent(tr + " " + ar));
  }
  if (tr) tries.push("q=" + encodeURIComponent(tr));

  for (const qs of tries) {
    const list = await httpJson(
      "https://lrclib.net/api/search?" + qs,
      6500,
      false,
    ).catch(() => null);
    if (!Array.isArray(list) || !list.length) continue;

    const synced = list.filter((x) => x && x.syncedLyrics);
    if (synced.length) {
      if (dur > 5)
        synced.sort(
          (a, b) =>
            Math.abs((a.duration || 0) - dur) -
            Math.abs((b.duration || 0) - dur),
        );
      const best = synced[0];
      if (best.duration > 0) onDur(best.duration);
      const rows = parseLrc(best.syncedLyrics);
      if (rows.length > 1) return Object.assign(rows, { kind: "line" });
    }

    const plain = list.find((x) => x && x.plainLyrics);
    if (plain) {
      const effDur = dur > 10 ? dur : plain.duration || 180;
      const lines = plain.plainLyrics
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      const step = effDur / Math.max(1, lines.length + 1);
      const rows = lines.map((text, idx) => ({
        time: (idx + 1) * step,
        text,
        isNote: false,
      }));
      if (rows.length > 1) return Object.assign(rows, { kind: "line" });
    }
  }
  return null;
}

const WORD_PROVIDERS = [
  { id: "youly", name: "YouLyPlus", rankWord: 4, run: fetchYouLy },
  { id: "pax", name: "PaxSenix", rankWord: 3.5, run: fetchPax },
  { id: "ne", name: "NetEase", rankWord: 3, run: fetchNetEase },
];

const PROVIDERS = [
  {
    id: "lrclib",
    name: "LrcLib",
    rankWord: 3,
    rankLine: 2.5,
    run: fetchLrcLib,
  },
  { id: "youly", name: "YouLyPlus", rankWord: 4, rankLine: 1, run: fetchYouLy },
  { id: "pax", name: "PaxSenix", rankWord: 3.5, rankLine: 1, run: fetchPax },
  { id: "ne", name: "NetEase", rankWord: 3, rankLine: 1, run: fetchNetEase },
];

export function createLyricsModule({ router, getMedia }) {
  let parsedLyrics = [];
  let lastActiveIdx = -1;
  let lastAppliedSig = "";
  let fetchToken = 0;
  let trackKey = "";
  let offsetSec = 0;
  let provState = {};
  let userForcedId = null;
  let rafId = 0;

  const offMap = () => {
    try {
      return JSON.parse(localStorage.getItem("mmLyrOff") || "{}");
    } catch {
      return {};
    }
  };
  const lyrBox = () => document.querySelector(".l-lyrics .lyr");

  function setSkeleton(on) {
    lyrBox()?.classList.toggle("mm-skel", !!on);
  }

  function updateOffsetUI() {
    const el = document.getElementById("mmOffV");
    if (!el) return;
    el.textContent = `${offsetSec > 0 ? "+" : ""}${offsetSec.toFixed(1)}s`;
    el.parentElement?.classList.toggle("on", !!offsetSec);
  }

  function adjustOffset(delta) {
    offsetSec = delta ? Math.round((offsetSec + delta) * 10) / 10 : 0;
    const m = offMap();
    if (offsetSec) m[trackKey] = offsetSec;
    else delete m[trackKey];
    const keys = Object.keys(m);
    if (keys.length > 120) delete m[keys[0]];
    try {
      localStorage.setItem("mmLyrOff", JSON.stringify(m));
    } catch {}
    updateOffsetUI();
    lastActiveIdx = -1;
    syncScroll();
    invoke("sync_lyrics_ui", { isOpen: null, offsetSec }).catch(() => {});
  }

  function setIdleText(text) {
    const el = document.getElementById("idleLyrText");
    if (!el) return;
    const clean = String(text || "");
    const w = measureTextPx(
      clean,
      '700 12px "Segoe UI Variable Text", "Segoe UI", sans-serif',
    );
    if (203 + w + 42 > 960) {
      delete el.dataset.rawText;
      el.dataset.maxw = "720";
      applyMarqueeToElements("#idleLyrText", clean);
    } else {
      delete el.dataset.rawText;
      el.classList.remove("is-marquee");
      el.textContent = clean;
    }
  }

  function updateDynamicWidths(activeText) {
    const clean = String(activeText || "").trim();
    if (!clean || clean === "♪" || clean === "Instrumental...") {
      router.setDynamicWidths(520, 340);
      return;
    }
    const expPx = measureTextPx(
      clean,
      '700 14.5px "Segoe UI Variable Text", "Segoe UI", sans-serif',
    );
    const idlePx = measureTextPx(
      clean,
      '700 12px "Segoe UI Variable Text", "Segoe UI", sans-serif',
    );
    const expW = Math.max(520, Math.min(1060, Math.ceil(282 + expPx + 54)));
    const idleW = Math.max(340, Math.min(960, Math.ceil(203 + idlePx + 42)));
    router.setDynamicWidths(expW, idleW);
  }

  function renderEmpty(customMsg = null, searching = false) {
    setSkeleton(searching);
    const list = document.getElementById("lyrList");
    const msg =
      customMsg ||
      (router.state.hasMedia ? t("notch.no_lyrics") : t("notch.wait_lyrics"));
    if (list) {
      list.style.transition = "none";
      list.style.transform = "translate3d(0, 24px, 0)";
      list.innerHTML = `<li class="on">${NOTE_SVG}</li><li class="on" style="opacity:.82">${escHtml(msg)}</li>`;
    }
    updateDynamicWidths("");
    const title = getMedia().getTrackTitle();
    if (router.state.hasMedia && title && !searching) {
      const el = document.getElementById("idleLyrText");
      if (el) delete el.dataset.rawText;
      applyMarqueeToElements("#idleLyrText", title);
    } else {
      setIdleText(msg);
    }
    stopKaraokeLoop();
  }

  function setSourceLabel(text) {
    const el = document.getElementById("mmSrc");
    if (el) el.textContent = text;
  }

  function renderProviderSheet() {
    const box = document.getElementById("mmProv");
    if (!box) return;
    box.innerHTML = PROVIDERS.map((p) => {
      const s = provState[p.id] || { state: "none" };
      const tag = s.used
        ? t("notch.prov_used")
        : s.state === "found"
          ? s.kind === "word"
            ? t("notch.prov_word")
            : t("notch.prov_line")
          : s.state === "pending"
            ? t("notch.prov_wait")
            : t("notch.prov_none");
      return `<div class="mp ${s.state}${s.used ? " used" : ""}" data-pid="${p.id}"><i></i><b>${escHtml(p.name)}</b><span>${escHtml(tag)}</span></div>`;
    }).join("");
  }

  function renderListDom() {
    const list = document.getElementById("lyrList");
    if (!list || !parsedLyrics.length) return;
    list.innerHTML = parsedLyrics
      .map(
        (l, i) =>
          `<li data-idx="${i}">${l.isNote ? NOTE_SVG : l.words ? l.words.map((w) => `<span class="w">${escHtml(w.x)}</span>`).join("") : escHtml(l.text)}</li>`,
      )
      .join("");
  }

  function applyRows(id, rows, name, kind) {
    setSkeleton(false);
    Object.values(provState).forEach((s) => {
      s.used = false;
    });
    if (provState[id]) {
      provState[id].state = "found";
      provState[id].kind = kind;
      provState[id].rows = rows;
      provState[id].used = true;
    }

    parsedLyrics = [{ time: 0, text: "♪", isNote: true }, ...rows];
    lastAppliedSig = `${trackKey}|${id}|${rows.length}`;
    renderListDom();
    lastActiveIdx = -1;
    setSourceLabel(
      (kind === "word" ? t("notch.word_sync") : t("notch.line_sync")) + name,
    );
    renderProviderSheet();
    syncScroll();
  }

  function driveWords(kids, words, pos) {
    for (let i = 0; i < words.length; i++) {
      const el = kids[i];
      if (!el) continue;
      const w = words[i];
      const p = Math.min(
        1,
        Math.max(0, (pos - w.t) / Math.max(0.14, w.e - w.t)),
      );
      const eased = +(1 - Math.pow(1 - p, 3)).toFixed(3);
      if (el._p !== eased) {
        el._p = eased;
        el.style.setProperty("--p", eased);
      }
      const glow = w.held ? +Math.min(1, p * 5, (1 - p) * 5).toFixed(2) : 0;
      if (el._g !== glow) {
        el._g = glow;
        el.style.setProperty("--g", glow);
      }
    }
  }

  function driveCurrentWordLine() {
    const line = parsedLyrics[lastActiveIdx];
    if (!line || !line.words) return false;
    const pos = getMedia().getPlaybackPos() + offsetSec;

    const li = document.querySelector(
      `#lyrList li[data-idx="${lastActiveIdx}"]`,
    );
    if (li) driveWords(li.children, line.words, pos);

    const idleEl = document.getElementById("idleLyrText");
    if (idleEl) {
      let trk = idleEl.firstElementChild;
      if (
        idleEl._mi !== lastActiveIdx ||
        !trk ||
        !trk.classList.contains("mm-trk")
      ) {
        idleEl._mi = lastActiveIdx;
        idleEl._x = 0;
        idleEl.classList.remove("is-marquee");
        idleEl.innerHTML = `<span class="mm-trk">${line.words.map((w) => `<span class="w">${escHtml(w.x)}</span>`).join("")}</span>`;
        trk = idleEl.firstElementChild;
      }
      driveWords(trk.children, line.words, pos);
      const over = trk.offsetWidth - idleEl.clientWidth;
      if (over > 3) {
        let k = 0;
        line.words.forEach((w, i) => {
          if (pos >= w.t) k = i;
        });
        const tx = Math.max(
          0,
          Math.min(
            over,
            trk.children[k].offsetLeft - idleEl.clientWidth * 0.35,
          ),
        );
        idleEl._x += (tx - idleEl._x) * 0.12;
        trk.style.translate = `${(-idleEl._x).toFixed(1)}px 0`;
      } else if (idleEl._x) {
        idleEl._x = 0;
        trk.style.translate = "0 0";
      }
    }
    return true;
  }

  function startKaraokeLoop() {
    if (rafId) return;
    const tick = () => {
      const st = document.getElementById("mn")?.dataset.s;
      const needed =
        !document.hidden &&
        router.state.isPlaying &&
        (st === "lyrics" || st === "idle-lyrics");
      if (!needed || !driveCurrentWordLine()) {
        rafId = 0;
        return;
      }
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  }

  function stopKaraokeLoop() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
  }

  function syncScroll() {
    if (!parsedLyrics.length) return;
    const list = document.getElementById("lyrList");
    if (list && list.children.length !== parsedLyrics.length) {
      renderListDom();
      lastActiveIdx = -1;
    }

    const pos = getMedia().getPlaybackPos() + offsetSec;
    let activeIdx = 0;
    for (let i = 0; i < parsedLyrics.length; i++) {
      if (pos >= parsedLyrics[i].time) activeIdx = i;
      else break;
    }

    if (activeIdx === lastActiveIdx) {
      if (parsedLyrics[activeIdx]?.words) startKaraokeLoop();
      return;
    }
    lastActiveIdx = activeIdx;

    const line = parsedLyrics[activeIdx];
    const rawTxt = line.isNote ? "" : line.text;
    updateDynamicWidths(rawTxt);

    const maxPx = router.state.lyricsPinnedOpen ? 292 : 754;
    const measured = rawTxt
      ? measureTextPx(
          rawTxt,
          '700 14.5px "Segoe UI Variable Text", "Segoe UI", sans-serif',
        )
      : 0;
    const fitSize =
      measured > maxPx
        ? `${Math.max(11, 14.5 * (maxPx / measured)).toFixed(1)}px`
        : "";

    const offsetY = 52 - activeIdx * 28 - 14;
    if (list) {
      list.style.transition = "transform .42s cubic-bezier(.22, 1, .36, 1)";
      list.style.transform = `translate3d(0, ${offsetY}px, 0)`;
    }

    const items = document.querySelectorAll("#lyrList li");
    items.forEach((li, i) => {
      const isCur = i === activeIdx;
      li.classList.toggle("on", isCur);
      li.dataset.d = String(Math.min(Math.abs(i - activeIdx), 3));
      li.style.fontSize = isCur ? fitSize : "";
    });

    if (line.words) {
      driveCurrentWordLine();
      startKaraokeLoop();
    } else {
      stopKaraokeLoop();
      setIdleText(line.isNote ? "Instrumental..." : line.text);
    }
  }

  function applySharedData(data) {
    if (!data) return;
    if (data.track_key && trackKey && data.track_key !== trackKey) return;
    if (data.track_key && !trackKey) trackKey = data.track_key;
    if (data.empty) {
      if (!parsedLyrics.length) renderEmpty(t("notch.no_lyrics"), false);
      return;
    }
    if (Array.isArray(data.rows) && data.rows.length) {
      const sig = `${trackKey}|${data.id}|${data.rows.length}`;
      const listCount = document.querySelectorAll("#lyrList li").length;
      if (sig === lastAppliedSig && listCount === data.rows.length + 1) return;
      if (!provState[data.id]) {
        provState = Object.fromEntries(
          PROVIDERS.map((p) => [
            p.id,
            { state: "none", kind: null, rows: null, used: false },
          ]),
        );
      }
      applyRows(data.id, data.rows, data.name || data.id, data.kind || "line");
    }
  }

  function applySharedUi(ui) {
    if (!ui) return;
    if (typeof ui.offset_sec === "number" && ui.offset_sec !== offsetSec) {
      offsetSec = ui.offset_sec;
      updateOffsetUI();
      lastActiveIdx = -1;
      syncScroll();
    }
  }

  async function hydrateFromShared() {
    try {
      const shared = await invoke("get_shared_lyrics");
      if (!shared) return;
      if (shared.track_key) trackKey = shared.track_key;
      if (typeof shared.offset_sec === "number") {
        offsetSec = shared.offset_sec;
        updateOffsetUI();
      }
      if (shared.payload) applySharedData(shared.payload);
    } catch {}
  }

  async function onTrackChanged(title, artist, dur) {
    const tok = ++fetchToken;
    parsedLyrics = [];
    lastAppliedSig = "";
    lastActiveIdx = -1;
    userForcedId = null;
    stopKaraokeLoop();
    setSourceLabel("");

    const [tr, ar] = normalizeTrack(title, artist);
    const nextKey = `${tr}|${ar}`;
    trackKey = nextKey;
    offsetSec = offMap()[trackKey] || 0;
    updateOffsetUI();

    renderEmpty(t("notch.search_lyrics"), true);
    provState = Object.fromEntries(
      PROVIDERS.map((p) => [
        p.id,
        { state: "pending", kind: null, rows: null, used: false },
      ]),
    );
    renderProviderSheet();

    let role = "fetch";
    try {
      role = await invoke("claim_lyrics_fetch", { trackKey: nextKey });
    } catch {}
    if (tok !== fetchToken) return;

    if (role === "cached") {
      const shared = await invoke("get_shared_lyrics").catch(() => null);
      if (tok === fetchToken && shared?.payload)
        applySharedData(shared.payload);
      return;
    }
    if (role === "wait") return;

    let bestRank = 0;
    let baselineScript = detectScript(title + " " + artist);

    const lrcPromise = fetchLrcLib(tr, ar, dur, (d) =>
      getMedia().setDurationIfEmpty(d),
    )
      .then((rows) => {
        if (tok !== fetchToken) return;
        if (rows && rows.length > 1) {
          baselineScript = detectScript(
            rows
              .slice(0, 12)
              .map((r) => r.text)
              .join(""),
          );
          provState.lrclib = {
            state: "found",
            kind: "line",
            rows,
            used: false,
          };
          if (!userForcedId && 2.5 > bestRank) {
            bestRank = 2.5;
            const payload = {
              track_key: nextKey,
              id: "lrclib",
              name: "LrcLib",
              kind: "line",
              rows,
            };
            applyRows("lrclib", rows, "LrcLib", "line");
            invoke("publish_lyrics_data", {
              trackKey: nextKey,
              payload,
              done: false,
            }).catch(() => {});
          }
        } else {
          provState.lrclib = {
            state: "none",
            kind: null,
            rows: null,
            used: false,
          };
        }
        renderProviderSheet();
      })
      .catch(() => {
        if (tok === fetchToken) {
          provState.lrclib = {
            state: "none",
            kind: null,
            rows: null,
            used: false,
          };
          renderProviderSheet();
        }
      });

    const wordTasks = WORD_PROVIDERS.map(async (p) => {
      try {
        const rows = await p.run(
          tr,
          ar,
          dur,
          (d) => getMedia().setDurationIfEmpty(d),
          title,
          artist,
        );
        await lrcPromise;
        if (tok !== fetchToken) return;
        if (rows && rows.length > 1) {
          const kind = rows.kind || (rows[0]?.words ? "word" : "line");
          const candScript = detectScript(
            rows
              .slice(0, 12)
              .map((r) => r.text)
              .join(""),
          );
          const scriptOk =
            isScriptCompatible(rows, title, artist) &&
            (baselineScript === "LAT"
              ? candScript !== "ZH"
              : candScript === baselineScript || candScript === "LAT");

          if (!scriptOk) {
            provState[p.id] = {
              state: "none",
              kind: null,
              rows: null,
              used: false,
            };
            renderProviderSheet();
            return;
          }

          const rank = kind === "word" ? p.rankWord : 1;
          provState[p.id] = { state: "found", kind, rows, used: false };
          if (!userForcedId && rank > bestRank) {
            bestRank = rank;
            const payload = {
              track_key: nextKey,
              id: p.id,
              name: p.name,
              kind,
              rows,
            };
            applyRows(p.id, rows, p.name, kind);
            invoke("publish_lyrics_data", {
              trackKey: nextKey,
              payload,
              done: false,
            }).catch(() => {});
          }
        } else {
          provState[p.id] = {
            state: "none",
            kind: null,
            rows: null,
            used: false,
          };
        }
      } catch {
        if (tok !== fetchToken) return;
        provState[p.id] = {
          state: "none",
          kind: null,
          rows: null,
          used: false,
        };
      }
      renderProviderSheet();
    });

    await Promise.allSettled([lrcPromise, ...wordTasks]);
    if (tok !== fetchToken) return;

    if (bestRank === 0) {
      renderEmpty(t("notch.no_lyrics"), false);
    }
    invoke("publish_lyrics_data", {
      trackKey: nextKey,
      payload: null,
      done: true,
    }).catch(() => {});
  }

  function refreshLanguage() {
    const pin = document.getElementById("mmPin");
    if (pin)
      pin.dataset.tip = router.state.lyricsPinnedOpen
        ? t("notch.lyr_unpin")
        : t("notch.lyr_pin");
    const provBtn = document.getElementById("mmProvBtn");
    if (provBtn) provBtn.dataset.tip = t("notch.lyr_sources");
    if (!parsedLyrics.length) renderEmpty(null, false);
    renderProviderSheet();
  }

  document
    .getElementById("openLyricsBtn")
    ?.addEventListener("click", async (e) => {
      e.stopPropagation();
      router.state.isLyricsOpen = true;
      router.renderActivePage();
      if (!parsedLyrics.length) {
        await hydrateFromShared();
      }
      lastActiveIdx = -1;
      if (parsedLyrics.length) syncScroll();
      else renderEmpty(null, false);
    });

  document.getElementById("closeLyricsBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    router.state.isLyricsOpen = false;
    stopKaraokeLoop();
    router.renderActivePage();
  });

  document.getElementById("mmPin")?.addEventListener("click", (e) => {
    e.stopPropagation();
    router.state.lyricsPinnedOpen = !router.state.lyricsPinnedOpen;
    refreshLanguage();
    router.renderActivePage();
    invoke("set_lyrics_pinned", {
      pinned: router.state.lyricsPinnedOpen,
    }).catch(() => {});
  });

  const toggleProv = (e) => {
    e.stopPropagation();
    document.getElementById("lyricsLayer")?.classList.toggle("mm-prov-open");
    renderProviderSheet();
  };
  document.getElementById("mmProvBtn")?.addEventListener("click", toggleProv);
  document.getElementById("mmSrc")?.addEventListener("click", toggleProv);

  document.getElementById("mmProv")?.addEventListener("click", (e) => {
    e.stopPropagation();
    const row = e.target.closest(".mp[data-pid]");
    if (!row) {
      toggleProv(e);
      return;
    }
    const pid = row.dataset.pid;
    const p = PROVIDERS.find((x) => x.id === pid);
    const st = provState[pid];
    if (p && st && st.rows) {
      userForcedId = pid;
      const payload = {
        track_key: trackKey,
        id: pid,
        name: p.name,
        kind: st.kind,
        rows: st.rows,
      };
      applyRows(pid, st.rows, p.name, st.kind);
      invoke("publish_lyrics_data", { trackKey, payload, done: true }).catch(
        () => {},
      );
    }
  });

  document.getElementById("lyrList")?.addEventListener("click", (e) => {
    const li = e.target.closest("li[data-idx]");
    if (!li) return;
    e.stopPropagation();
    const item = parsedLyrics[parseInt(li.dataset.idx, 10)];
    if (item) getMedia().seekToSec(item.time);
  });

  document.getElementById("mmOffMinus")?.addEventListener("click", (e) => {
    e.stopPropagation();
    adjustOffset(-0.5);
  });
  document.getElementById("mmOffV")?.addEventListener("click", (e) => {
    e.stopPropagation();
    adjustOffset(0);
  });
  document.getElementById("mmOffPlus")?.addEventListener("click", (e) => {
    e.stopPropagation();
    adjustOffset(0.5);
  });

  return {
    onTrackChanged,
    applySharedData,
    applySharedUi,
    hydrateFromShared,
    syncScroll,
    renderEmpty,
    refreshLanguage,
    onStateChange: (next) => {
      if (next === "lyrics" || next === "idle-lyrics") {
        if (parsedLyrics[lastActiveIdx]?.words) startKaraokeLoop();
      } else {
        stopKaraokeLoop();
      }
    },
  };
}
