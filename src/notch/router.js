const { invoke } = window.__TAURI__.core;

export const PAGES = ["music", "pomo", "water", "shelf", "todo", "calendar"];

const PAGE_SELECTORS = {
  music: ".l-music",
  pomo: ".l-pomo",
  water: ".l-water",
  shelf: ".l-shelf",
  todo: ".l-todo",
  calendar: ".l-calendar",
};

const SLIDE_CLASSES = [
  "ly-in-next",
  "ly-in-prev",
  "ly-out-next",
  "ly-out-prev",
];

export function createNotchRouter(mn) {
  const state = {
    bootPhase: "boot", // "boot" | "intro" | "done"
    pageIdx: 0,
    isLyricsOpen: false,
    lyricsPinnedOpen: false,
    calExpanded: (() => {
      try {
        return localStorage.getItem("mmCalX") === "1";
      } catch {
        return false;
      }
    })(),
    dynamicIsland: false,
    interactionMode: "hover",
    restingMode: "pet",
    peekDurationMs: 2400,
    isHovered: false,
    isExpandedByClick: false,
    isObstructed: false,
    hasMedia: false,
    isPlaying: false,
    pomoRunning: false,
    dynLyricsW: 520,
    dynIdleLyricsW: 340,
  };

  let peekTimer = null;
  let slideTimer = null;
  let pendingSlideDir = 0; // 1 = next (geser kiri), -1 = prev (geser kanan), 0 = tanpa slide

  const hooks = {
    onHideInputs: () => {},
    onPageEnter: () => {},
    onStateChange: () => {},
  };

  function clearSlideClasses() {
    clearTimeout(slideTimer);
    slideTimer = null;
    mn.querySelectorAll(".ly").forEach((el) =>
      el.classList.remove(...SLIDE_CLASSES),
    );
  }

  function runPageSlide(prevName, nextName, dir) {
    clearSlideClasses();
    if (!dir || prevName === nextName) return;
    const outSel = PAGE_SELECTORS[prevName];
    const inSel = PAGE_SELECTORS[nextName];
    if (!outSel || !inSel) return;

    const outEl = mn.querySelector(outSel);
    const inEl = mn.querySelector(inSel);
    if (!outEl || !inEl) return;

    const isNext = dir > 0;
    outEl.classList.add(isNext ? "ly-out-next" : "ly-out-prev");
    inEl.classList.add(isNext ? "ly-in-next" : "ly-in-prev");

    slideTimer = setTimeout(clearSlideClasses, 360);
  }

  function liveKind() {
    if (state.pomoRunning) return "pomo";
    if (state.hasMedia && state.isPlaying) return "music";
    return null;
  }

  function isNotchExpanded() {
    return state.interactionMode === "click"
      ? state.isHovered && state.isExpandedByClick
      : state.isHovered;
  }

  function applyNotchState(next) {
    const prev = mn.dataset.s;
    const slideDir = pendingSlideDir;
    pendingSlideDir = 0;

    if (slideDir && PAGE_SELECTORS[prev] && PAGE_SELECTORS[next]) {
      runPageSlide(prev, next, slideDir);
    } else if (prev !== next) {
      clearSlideClasses();
    }

    mn.dataset.s = next;
    mn.dataset.island = state.dynamicIsland ? "1" : "0";
    mn.classList.toggle("mm-open", state.lyricsPinnedOpen);
    mn.classList.toggle("mm-cal-x", state.calExpanded);
    mn.classList.toggle(
      "click-ready",
      state.interactionMode === "click" &&
        state.isHovered &&
        !state.isExpandedByClick,
    );

    document
      .querySelectorAll("#pageDots i")
      .forEach((d, i) => d.classList.toggle("on", i === state.pageIdx));

    let w = 155,
      h = 30;
    if (next === "boot") {
      w = 44;
      h = 4;
    } else if (next === "intro") {
      w = 284;
      h = 64;
    } else if (next === "minimal") {
      w = 130;
      h = 6;
    } else if (next === "live") {
      w = 164;
      h = 34;
    } else if (next === "idle-lyrics") {
      w = state.dynIdleLyricsW;
      h = 32;
    } else if (next === "peek") {
      w = 320;
      h = 62;
    } else if (next === "lyrics") {
      w = state.lyricsPinnedOpen ? 600 : state.dynLyricsW;
      h = 140;
    } else if (next === "calendar" && state.calExpanded) {
      w = 520;
      h = 170;
    } else if (!["pet", "idle"].includes(next)) {
      w = 420;
      h = 140;
    }

    const restingW =
      next === "live"
        ? 164
        : state.isLyricsOpen && state.hasMedia
          ? state.lyricsPinnedOpen
            ? 600
            : state.dynIdleLyricsW
          : state.restingMode === "minimal"
            ? 130
            : 155;

    invoke("update_notch_hitbox", {
      width: w,
      height: h + (state.dynamicIsland ? 8 : 0),
      restingWidth: restingW,
    }).catch(() => {});

    if (prev !== next) {
      hooks.onPageEnter(next);
      hooks.onStateChange(next);
    }
  }

  function renderActivePage() {
    if (state.bootPhase !== "done") {
      applyNotchState(state.bootPhase);
      return;
    }

    const expanded = isNotchExpanded();
    if (peekTimer && !expanded) return;

    const currentPage = PAGES[state.pageIdx];
    hooks.onHideInputs(expanded, currentPage);

    if (!expanded) {
      if (
        state.isObstructed &&
        !(state.interactionMode === "click" && state.isHovered)
      ) {
        applyNotchState("minimal");
      } else if (state.isLyricsOpen && state.hasMedia) {
        applyNotchState(state.lyricsPinnedOpen ? "lyrics" : "idle-lyrics");
      } else if (state.dynamicIsland && liveKind()) {
        applyNotchState("live");
      } else if (
        state.restingMode === "minimal" &&
        state.interactionMode === "click" &&
        state.isHovered
      ) {
        applyNotchState("pet");
      } else {
        applyNotchState(state.restingMode);
      }
      return;
    }

    if (currentPage === "music" && state.isLyricsOpen) {
      applyNotchState("lyrics");
    } else {
      applyNotchState(currentPage);
    }
  }

  function goToPage(idx, explicitDir = 0) {
    const normalized = ((idx % PAGES.length) + PAGES.length) % PAGES.length;
    if (normalized === state.pageIdx && !state.isLyricsOpen) return;

    const dir = explicitDir || (normalized > state.pageIdx ? 1 : -1);
    pendingSlideDir = isNotchExpanded() ? dir : 0;
    state.isLyricsOpen = false;
    state.pageIdx = normalized;
    renderActivePage();
  }

  function nextPage() {
    goToPage(state.pageIdx + 1, 1);
  }
  function prevPage() {
    goToPage(state.pageIdx - 1, -1);
  }

  function clearPeek() {
    if (peekTimer) {
      clearTimeout(peekTimer);
      peekTimer = null;
    }
  }

  function showCustomPeek(
    svgHtml,
    titleTxt,
    subTxt,
    rightTxt,
    rightColor,
    barPct = null,
  ) {
    if (isNotchExpanded() || state.bootPhase !== "done") return;
    document.getElementById("peekIcon").innerHTML = svgHtml;
    document.getElementById("peekTitle").textContent = titleTxt;
    document.getElementById("peekSub").textContent = subTxt;
    const right = document.getElementById("peekRight");
    right.textContent = rightTxt;
    right.style.color = rightColor;

    const barWrap = document.getElementById("peekBarWrap");
    const barFill = document.getElementById("peekBarFill");
    if (barPct !== null && barPct >= 0) {
      barWrap.style.display = "block";
      barFill.style.width = `${Math.min(100, Math.max(0, barPct))}%`;
      barFill.style.background = rightColor;
    } else {
      barWrap.style.display = "none";
    }

    applyNotchState("peek");
    clearPeek();
    peekTimer = setTimeout(() => {
      peekTimer = null;
      renderActivePage();
    }, state.peekDurationMs);
  }

  function setDynamicWidths(expW, idleW) {
    state.dynLyricsW = state.lyricsPinnedOpen ? 600 : expW;
    state.dynIdleLyricsW = idleW;
    document.documentElement.style.setProperty(
      "--dyn-lyr-w",
      `${state.dynLyricsW}px`,
    );
    document.documentElement.style.setProperty(
      "--dyn-idle-lyr-w",
      `${state.dynIdleLyricsW}px`,
    );
    if (mn.dataset.s === "lyrics" || mn.dataset.s === "idle-lyrics") {
      applyNotchState(mn.dataset.s);
    }
  }

  function setCalExpanded(val) {
    state.calExpanded = !!val;
    try {
      localStorage.setItem("mmCalX", state.calExpanded ? "1" : "0");
    } catch {}
    renderActivePage();
  }

  return {
    state,
    hooks,
    liveKind,
    isNotchExpanded,
    applyNotchState,
    renderActivePage,
    goToPage,
    nextPage,
    prevPage,
    clearPeek,
    showCustomPeek,
    setDynamicWidths,
    setCalExpanded,
  };
}
