import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";

const { invoke } = window.__TAURI__.core;

export function createShelfModule({ router, syncWidgetData }) {
  const area = document.getElementById("shelfArea");
  const zone = document.getElementById("shelfZone");
  const itemsEl = document.getElementById("shelfItems");
  const clearBtn = document.getElementById("shelfClearBtn");
  const allBtn = document.getElementById("shelfAllBtn");

  // Celestial Optical Distortion Layer (SVG feDisplacementMap + Nebula + Waves + Starlight)
  const ndLayer = document.createElement("div");
  ndLayer.className = "nd-layer";
  ndLayer.innerHTML = `
    <svg class="nd-defs" width="0" height="0" aria-hidden="true">
      <defs>
        <filter id="celestialDistort" x="-25%" y="-25%" width="150%" height="150%" color-interpolation-filters="sRGB">
          <feTurbulence type="fractalNoise" baseFrequency="0.015 0.042" numOctaves="2" seed="11" result="noise"/>
          <feDisplacementMap in="SourceGraphic" in2="noise" scale="26" xChannelSelector="R" yChannelSelector="G"/>
        </filter>
      </defs>
    </svg>
    <div class="nd-warp-stage">
      <div class="nd-nebula"></div>
      <div class="nd-rays"></div>
      <div class="nd-wave w1"></div>
      <div class="nd-wave w2"></div>
      <div class="nd-wave w3"></div>
    </div>
    <div class="nd-corona"></div>
    <span class="nd-star" style="--sx:33%;--dx:-28px;--ss:7px;--st:2.5s;--sd:0s"></span>
    <span class="nd-star" style="--sx:42%;--dx:-14px;--ss:9px;--st:2.1s;--sd:-.7s"></span>
    <span class="nd-star" style="--sx:50%;--dx:4px;--ss:10px;--st:2.7s;--sd:-1.4s"></span>
    <span class="nd-star" style="--sx:58%;--dx:16px;--ss:8px;--st:2.2s;--sd:-.4s"></span>
    <span class="nd-star" style="--sx:67%;--dx:30px;--ss:7px;--st:2.6s;--sd:-1.8s"></span>
    <span class="nd-star" style="--sx:46%;--dx:10px;--ss:6px;--st:1.9s;--sd:-1.1s"></span>
  `;

  // Gravitational-lens refraction arcs (terpisah dari mix-blend-mode:screen agar backdrop-filter membias kartu)
  const lensLayer = document.createElement("div");
  lensLayer.className = "nd-lens-layer";
  lensLayer.innerHTML = `
    <div class="nd-lens l1"></div>
    <div class="nd-lens l2"></div>
  `;

  area.prepend(ndLayer);
  area.appendChild(lensLayer);

  const reduceMotion = window.matchMedia?.(
    "(prefers-reduced-motion: reduce)",
  )?.matches;

  let pointerHovering = false;
  let dragHovering = false;

  function syncFlowState() {
    if (reduceMotion) return;
    area.classList.toggle("nd-active", pointerHovering || dragHovering);
  }

  function triggerDropBurst() {
    if (reduceMotion || getComputedStyle(area).visibility === "hidden") return;
    const burst = document.createElement("div");
    burst.className = "nd-burst";
    area.appendChild(burst);
    setTimeout(() => burst.remove(), 880);

    const ar = area.getBoundingClientRect();
    area.querySelectorAll("#shelfZone, .sh-card").forEach((el) => {
      const r = el.getBoundingClientRect();
      const dy = Math.max(0, r.top + r.height / 2 - ar.top);
      el.style.setProperty("--bd", `${Math.round(dy * 1.8)}ms`);
      el.style.setProperty("--ba", "3.5px");
      el.classList.remove("nd-bump");
      void el.offsetWidth;
      el.classList.add("nd-bump");
    });
  }

  area.addEventListener(
    "pointerover",
    (e) => {
      if (e.target.closest?.(".sh-card, #shelfZone")) {
        pointerHovering = true;
        syncFlowState();
      }
    },
    { passive: true },
  );

  area.addEventListener(
    "pointerout",
    (e) => {
      if (!e.relatedTarget?.closest?.(".sh-card, #shelfZone")) {
        pointerHovering = false;
        syncFlowState();
      }
    },
    { passive: true },
  );

  area.addEventListener(
    "pointerleave",
    () => {
      pointerHovering = false;
      syncFlowState();
    },
    { passive: true },
  );

  window.addEventListener("dragenter", (e) => {
    e.preventDefault();
  });
  window.addEventListener("dragover", (e) => {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  });
  window.addEventListener("drop", (e) => {
    e.preventDefault();
  });

  let files = [];
  let dragIdx = -1;
  let dragStartX = 0;
  let dragStartY = 0;
  let draggingOut = false;
  let lastDragEnd = 0;
  let lastClickIdx = -1;
  let lastDropSig = "";
  let lastDropTime = 0;
  let lastRenderSig = "";

  itemsEl.addEventListener("dragstart", (e) => {
    e.preventDefault();
  });

  function updateAllBtn() {
    if (!allBtn) return;
    const allSel = files.length > 0 && files.every((f) => f.selected !== false);
    allBtn.classList.toggle("on", allSel);
  }

  function render(force = false) {
    const sig = JSON.stringify(
      files.map((f) => [f.path, f.size_str, Boolean(f.thumb_b64)]),
    );
    if (!force && sig === lastRenderSig) {
      refreshSelection();
      return;
    }
    lastRenderSig = sig;

    if (!files.length) {
      zone.style.display = "flex";
      itemsEl.style.display = "none";
      clearBtn.style.display = "none";
      allBtn.style.display = "none";
      return;
    }

    zone.style.display = "none";
    itemsEl.style.display = "flex";
    clearBtn.style.display = "flex";
    allBtn.style.display = "flex";

    const keepScroll = itemsEl.scrollLeft;
    itemsEl.innerHTML = files
      .map((f, i) => {
        const isSel = f.selected !== false;
        const tip = `${f.name}${f.size_str ? ` (${f.size_str})` : ""}`;
        return `
        <div class="sh-card sh-enter ${isSel ? "selected" : ""}" style="--k:${Math.min(i, 6)}" draggable="false" data-idx="${i}" title="${escHtml(tip)}">
          <span class="sh-sel"><svg viewBox="0 0 10 10"><path d="M2 5l2.2 2.2L8 2.5"/></svg></span>
          <span class="sh-del" data-del="${i}"><svg viewBox="0 0 10 10"><path d="M2 2l6 6M8 2L2 8"/></svg></span>
          ${
            f.thumb_b64
              ? `<img class="sh-thumb" src="${f.thumb_b64}" draggable="false" alt="">`
              : `<svg class="f-ico" viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/></svg>`
          }
          <span class="sh-name">${escHtml(f.name)}</span>
          <span class="sh-ext">${escHtml(f.size_str || f.ext || "FILE")}</span>
        </div>`;
      })
      .join("");
    itemsEl.scrollTo({ left: keepScroll, behavior: "instant" });
    updateAllBtn();
  }

  function refreshSelection() {
    Array.from(itemsEl.children).forEach((el, i) => {
      if (files[i])
        el.classList.toggle("selected", files[i].selected !== false);
    });
    updateAllBtn();
  }

  function addPath(fullPath) {
    if (!fullPath || typeof fullPath !== "string") return;
    const parts = fullPath.split(/[\\/]/);
    const name = parts[parts.length - 1] || fullPath;
    const extParts = name.split(".");
    const ext =
      extParts.length > 1 ? extParts.pop().slice(0, 5).toUpperCase() : "FILE";
    if (!files.some((x) => x.path === fullPath)) {
      files.push({
        name,
        path: fullPath,
        ext,
        size_str: "",
        thumb_b64: "",
        selected: true,
      });
    }
  }

  async function enrich(paths) {
    try {
      const enriched = await invoke("prepare_shelf_files", { paths });
      if (Array.isArray(enriched) && enriched.length) {
        enriched.forEach((item) => {
          const idx = files.findIndex((x) => x.path === item.path);
          if (idx >= 0)
            files[idx] = { ...item, selected: files[idx].selected !== false };
        });
        render(true);
        syncWidgetData();
      }
    } catch {}
  }

  function handleDropPaths(rawPayload) {
    clearDragVisual();
    const list = rawPayload?.paths ?? rawPayload;
    if (!Array.isArray(list) || !list.length) return;
    const paths = list
      .map((x) => (typeof x === "string" ? x : x.path))
      .filter(Boolean);
    if (!paths.length) return;

    const sig = paths.join("|");
    const now = performance.now();
    if (sig === lastDropSig && now - lastDropTime < 350) return;
    lastDropSig = sig;
    lastDropTime = now;

    router.state.isHovered = true;
    router.state.isExpandedByClick = true;
    paths.forEach(addPath);
    router.goToPage(3);
    render(true);
    triggerDropBurst();
    syncWidgetData();
    enrich(paths);
  }

  function handleDragEnter() {
    router.state.isHovered = true;
    router.state.isExpandedByClick = true;
    if (router.state.pageIdx !== 3) router.goToPage(3);
    else router.renderActivePage();
    zone.classList.add("dragover");
    dragHovering = true;
    syncFlowState();
  }

  function handleDragPosition(p) {
    if (!p || !Number.isFinite(p.x) || !Number.isFinite(p.y)) return;
    zone.classList.add("file-drag-active");
    if (!dragHovering) {
      dragHovering = true;
      syncFlowState();
    }
    const r = zone.getBoundingClientRect();
    const x = Math.max(0, Math.min(r.width, p.x - r.left));
    const y = Math.max(0, Math.min(r.height, p.y - r.top));
    zone.style.setProperty("--drag-x", `${x}px`);
    zone.style.setProperty("--drag-y", `${y}px`);
  }

  function clearDragVisual() {
    dragHovering = false;
    syncFlowState();
    zone.classList.remove("dragover", "file-drag-active");
    zone.style.removeProperty("--drag-x");
    zone.style.removeProperty("--drag-y");
  }

  zone.addEventListener("click", async (e) => {
    e.stopPropagation();
    try {
      const picked = await invoke("pick_shelf_files");
      if (Array.isArray(picked) && picked.length) handleDropPaths(picked);
    } catch {}
  });

  allBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    const allSel = files.every((f) => f.selected !== false);
    files.forEach((f) => {
      f.selected = !allSel;
    });
    refreshSelection();
  });

  clearBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    files = [];
    render(true);
    syncWidgetData();
  });

  itemsEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || e.target.closest(".sh-del, .sh-sel")) return;
    const card = e.target.closest(".sh-card[data-idx]");
    if (!card) return;
    e.preventDefault();
    e.stopPropagation();
    dragIdx = parseInt(card.dataset.idx, 10);
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    draggingOut = false;
  });

  window.addEventListener("mousemove", (e) => {
    if (dragIdx < 0 || draggingOut) return;
    if (Math.hypot(e.clientX - dragStartX, e.clientY - dragStartY) > 5) {
      draggingOut = true;
      const card = files[dragIdx];
      dragIdx = -1;
      if (card) {
        let targets =
          card.selected !== false
            ? files
                .filter((f) => f.selected !== false && f.path)
                .map((f) => f.path)
            : [];
        if (!targets.length) targets = [card.path];
        invoke("start_file_drag", { paths: targets }).finally(() => {
          lastDragEnd = performance.now();
          draggingOut = false;
        });
      } else {
        draggingOut = false;
      }
    }
  });

  window.addEventListener("mouseup", () => {
    dragIdx = -1;
    draggingOut = false;
  });

  itemsEl.addEventListener("click", (e) => {
    const del = e.target.closest(".sh-del[data-del]");
    if (del) {
      e.stopPropagation();
      files.splice(parseInt(del.dataset.del, 10), 1);
      render(true);
      syncWidgetData();
      return;
    }
    const card = e.target.closest(".sh-card[data-idx]");
    if (!card) return;
    e.stopPropagation();
    if (performance.now() - lastDragEnd < 350) return;
    const idx = parseInt(card.dataset.idx, 10);
    if (e.shiftKey && lastClickIdx >= 0 && lastClickIdx !== idx) {
      const [a, b] = [Math.min(lastClickIdx, idx), Math.max(lastClickIdx, idx)];
      files.forEach((f, i) => {
        if (i >= a && i <= b) f.selected = true;
      });
    } else if (files[idx]) {
      files[idx].selected = !(files[idx].selected !== false);
    }
    lastClickIdx = idx;
    refreshSelection();
  });

  itemsEl.addEventListener("dblclick", (e) => {
    if (e.target.closest(".sh-del, .sh-sel")) return;
    const card = e.target.closest(".sh-card[data-idx]");
    if (!card) return;
    e.stopPropagation();
    const item = files[parseInt(card.dataset.idx, 10)];
    if (item) invoke("open_shelf_file", { path: item.path }).catch(() => {});
  });

  itemsEl.addEventListener("contextmenu", (e) => {
    const card = e.target.closest(".sh-card[data-idx]");
    if (!card) return;
    e.preventDefault();
    e.stopPropagation();
    const item = files[parseInt(card.dataset.idx, 10)];
    if (item) invoke("reveal_shelf_file", { path: item.path }).catch(() => {});
  });

  function refreshLanguage() {
    document.getElementById("shelfDropTxt").textContent = t("notch.shelf_drop");
    document.getElementById("shelfHintTxt").textContent = t("notch.shelf_hint");
  }

  function onConfig(cfg) {
    const prevSel = new Map(files.map((f) => [f.path, f.selected !== false]));
    files = (cfg.shelf || []).map((f) => ({
      ...f,
      selected: prevSel.has(f.path) ? prevSel.get(f.path) : true,
    }));
    render(false);
  }

  return {
    onConfig,
    refreshLanguage,
    handleDragEnter,
    handleDragLeave: clearDragVisual,
    handleDragPosition,
    handleDropPaths,
    getItems: () => files,
    getCount: () => files.length,
  };
}
