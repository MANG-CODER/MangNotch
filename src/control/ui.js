import { escHtml } from "../shared/theme.js";

let toastTimer = null;
export function showToast(text) {
  const el = document.getElementById("toast");
  if (!el) return;
  el.textContent = text;
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), 1100);
}

export function shakeEl(el) {
  if (!el) return;
  el.classList.remove("mm-shake");
  void el.offsetWidth;
  el.classList.add("mm-shake");
}

export function spawnRipple(target, clientX, clientY) {
  if (!target) return;
  const r = target.getBoundingClientRect();
  const size = Math.max(r.width, r.height) * 0.9;
  const dot = document.createElement("span");
  dot.className = "mm-ripple";
  dot.style.width = `${size}px`;
  dot.style.height = `${size}px`;
  dot.style.left = `${clientX - r.left}px`;
  dot.style.top = `${clientY - r.top}px`;
  target.appendChild(dot);
  setTimeout(() => dot.remove(), 560);
}

export function createSectionTitle(text, rightEl = null) {
  const div = document.createElement("div");
  div.className = "sec-title";
  const span = document.createElement("span");
  span.textContent = text;
  div.appendChild(span);
  if (rightEl) div.appendChild(rightEl);
  return div;
}

export function createCard(rows = [], padded = false) {
  const card = document.createElement("div");
  card.className = padded ? "card padded" : "card";
  rows.forEach((r) => r && card.appendChild(r));
  return card;
}

export function createRow(title, desc, controlEl, stack = false) {
  const row = document.createElement("div");
  row.className = stack ? "row stack" : "row";
  const wrap = document.createElement("div");
  wrap.className = "lbl-wrap";
  wrap.innerHTML = `<b>${escHtml(title)}</b>${desc ? `<small>${escHtml(desc)}</small>` : ""}`;
  row.appendChild(wrap);
  if (controlEl) row.appendChild(controlEl);
  return row;
}

export function createSwitch(checked, onChange) {
  const label = document.createElement("label");
  label.className = "switch";
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = !!checked;
  input.addEventListener("change", () => onChange(input.checked));
  const track = document.createElement("span");
  track.className = "track";
  label.append(input, track);
  label._input = input;
  return label;
}

/* iOS 26 Magnifying Glass Segmented Bar (klik biasa ATAU hold & geser!) */
export function createSegmented(options, value, onChange) {
  const bar = document.createElement("div");
  bar.className = "seg-bar";
  const thumb = document.createElement("div");
  thumb.className = "seg-thumb";
  bar.appendChild(thumb);

  let current = value;
  const valLeft = options[0].value;
  const valRight = options[1].value;

  const buttons = options.map((opt) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "seg-btn" + (opt.value === value ? " active" : "");
    btn.dataset.val = opt.value;
    btn.textContent = opt.label;
    bar.appendChild(btn);
    return btn;
  });

  function paint(val) {
    const isRight = val === valRight;
    thumb.style.transform = `translateX(${isRight ? "100%" : "0%"})`;
    buttons.forEach((b) => b.classList.toggle("active", b.dataset.val === val));
  }
  paint(current);

  let dragging = false;
  let startX = 0;
  let startIdx = 0;

  bar.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    dragging = true;
    bar.setPointerCapture(e.pointerId);
    bar.classList.add("magnifying");
    startX = e.clientX;
    startIdx = current === valRight ? 1 : 0;
  });

  bar.addEventListener("pointermove", (e) => {
    if (!dragging) return;
    const dx = e.clientX - startX;
    if (Math.abs(dx) > 3) bar.classList.add("dragging");
    const slotW = (bar.clientWidth - 6) / 2;
    let nextPx = startIdx * slotW + dx;
    if (nextPx < 0) nextPx *= 0.22;
    if (nextPx > slotW) nextPx = slotW + (nextPx - slotW) * 0.22;
    const pct = (nextPx / Math.max(1, slotW)) * 100;
    thumb.style.transform = `translateX(${pct.toFixed(1)}%)`;
    const hoverVal = nextPx > slotW * 0.5 ? valRight : valLeft;
    buttons.forEach((b) =>
      b.classList.toggle("active", b.dataset.val === hoverVal),
    );
  });

  const finish = (e) => {
    if (!dragging) return;
    dragging = false;
    bar.classList.remove("magnifying", "dragging");
    const rect = bar.getBoundingClientRect();
    const chosen = e.clientX - rect.left > rect.width / 2 ? valRight : valLeft;
    const changed = chosen !== current;
    current = chosen;
    paint(current);
    if (changed) onChange(current);
  };

  bar.addEventListener("pointerup", finish);
  bar.addEventListener("pointercancel", finish);
  return bar;
}

export function createSelect(options, value, onChange, disabled = false) {
  const wrap = document.createElement("div");
  wrap.className = "csel-wrap" + (disabled ? " disabled" : "");
  let current = String(value);

  const render = () => {
    const selected =
      options.find((o) => String(o.value) === current) || options[0];
    wrap.innerHTML = `
      <div class="csel-trigger">
        <span class="csel-label">${escHtml(selected ? selected.label : "")}</span>
        <svg class="csel-arrow" viewBox="0 0 24 24"><path d="m6 9 6 6 6-6"/></svg>
      </div>
      <div class="csel-menu">
        ${options
          .map(
            (o) => `
          <div class="csel-opt ${String(o.value) === current ? "selected" : ""}" data-val="${escHtml(o.value)}">
            <span>${escHtml(o.label)}</span>
            <svg class="csel-check" viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5"/></svg>
          </div>
        `,
          )
          .join("")}
      </div>
    `;

    wrap.querySelector(".csel-trigger").addEventListener("click", (e) => {
      e.stopPropagation();
      if (wrap.classList.contains("disabled")) return;
      const isOpen = wrap.classList.contains("open");
      document
        .querySelectorAll(".csel-wrap.open")
        .forEach((w) => w.classList.remove("open"));
      if (!isOpen) {
        const sc = document
          .getElementById("ctrlContent")
          .getBoundingClientRect();
        const r = wrap.getBoundingClientRect();
        const menuH = Math.min(210, 10 + options.length * 32);
        wrap.classList.toggle(
          "up",
          sc.bottom - r.bottom < menuH + 12 &&
            r.top - sc.top > sc.bottom - r.bottom,
        );
        wrap.classList.add("open");
      }
    });

    wrap.querySelectorAll(".csel-opt").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const val = el.dataset.val;
        wrap.classList.remove("open");
        if (val !== current) {
          current = val;
          render();
          onChange(val);
        }
      });
    });
  };

  render();
  return wrap;
}

export function createSlider({
  min,
  max,
  step = 1,
  value,
  format,
  onInput,
  onCommit,
}) {
  const group = document.createElement("div");
  group.className = "slider-group";

  const wrap = document.createElement("div");
  wrap.className = "lg-slider";
  wrap.innerHTML = `
    <div class="lg-slider-track"><div class="lg-slider-fill"></div></div>
    <div class="lg-slider-thumb"></div>
  `;
  const valEl = document.createElement("span");
  valEl.className = "slider-val";
  group.append(wrap, valEl);

  const fill = wrap.querySelector(".lg-slider-fill");
  const thumb = wrap.querySelector(".lg-slider-thumb");
  const thumbW = 30;
  const dec = (String(step).split(".")[1] || "").length;
  let cur = Math.min(max, Math.max(min, Number(value)));
  let dragging = false;

  const paint = () => {
    const pct = (cur - min) / (max - min || 1);
    const trackW = wrap.clientWidth || 152;
    const maxLeft = Math.max(0, trackW - thumbW);
    thumb.style.translate = `${(pct * maxLeft).toFixed(1)}px -50%`;
    fill.style.width = `${(pct * 100).toFixed(2)}%`;
    valEl.textContent = format ? format(cur) : String(cur);
  };

  const fromClientX = (cx) => {
    const rect = wrap.getBoundingClientRect();
    const usable = Math.max(1, rect.width - thumbW);
    const pct = Math.max(
      0,
      Math.min(1, (cx - rect.left - thumbW / 2) / usable),
    );
    let v = min + pct * (max - min);
    v = Math.round(v / step) * step;
    v = Math.min(max, Math.max(min, +v.toFixed(dec)));
    if (v !== cur) {
      cur = v;
      paint();
      if (onInput) onInput(cur);
    }
  };

  wrap.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    dragging = true;
    wrap.setPointerCapture(e.pointerId);
    wrap.classList.add("dragging");
    fromClientX(e.clientX);
  });
  wrap.addEventListener("pointermove", (e) => {
    if (dragging) fromClientX(e.clientX);
  });
  const end = () => {
    if (!dragging) return;
    dragging = false;
    wrap.classList.remove("dragging");
    if (onCommit) onCommit(cur);
  };
  wrap.addEventListener("pointerup", end);
  wrap.addEventListener("pointercancel", end);
  wrap.addEventListener("lostpointercapture", end);

  requestAnimationFrame(paint);
  paint();

  group.setValue = (v) => {
    cur = Math.min(max, Math.max(min, Number(v)));
    paint();
  };
  return group;
}

export function mascotMarkup(style) {
  if (style === "mochi") {
    return `
      <div class="mochi-stage">
        <div class="mochi-body">
          <div class="mochi-face">
            <i class="mochi-eye l"></i><i class="mochi-eye r"></i>
            <i class="mochi-blush l"></i><i class="mochi-blush r"></i>
            <i class="mochi-mouth"></i>
          </div>
        </div>
      </div>`;
  }
  if (style === "kitsune") {
    return `<div class="kitsune-wrap"><i class="kit-eye"></i><i class="kit-eye"></i></div>`;
  }
  return `
    <div class="neko-stage">
      <span class="neko-ear left"></span>
      <div class="neko-eye-wrap"><i class="neko-eye left"></i><i class="neko-eye right"></i></div>
      <span class="neko-ear right"></span>
    </div>`;
}

// Tutup dropdown & pasang pointer specular + click ripple global
window.addEventListener("click", () => {
  document
    .querySelectorAll(".csel-wrap.open")
    .forEach((w) => w.classList.remove("open"));
});

document.addEventListener(
  "pointermove",
  (e) => {
    const el = e.target.closest?.(".g-btn, .csel-trigger, .chip, .mon-pill");
    if (!el) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty("--mx", `${(e.clientX - r.left).toFixed(0)}px`);
    el.style.setProperty("--my", `${(e.clientY - r.top).toFixed(0)}px`);
  },
  { passive: true },
);

document.addEventListener(
  "pointerdown",
  (e) => {
    if (e.button !== 0) return;
    const target = e.target.closest?.(
      ".g-btn, .peek-tile, .m-card, .chip, .mon-pill",
    );
    if (target) spawnRipple(target, e.clientX, e.clientY);
  },
  { passive: true },
);
