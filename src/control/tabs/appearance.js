import { t } from "../../shared/i18n.js";
import { GLASS_DEFAULTS, THEMES } from "../../shared/theme.js";
import {
  createCard,
  createRow,
  createSectionTitle,
  createSelect,
  createSlider,
  createSwitch,
} from "../ui.js";

function createColorPill(labelText, hexVal, onInput, onCommit) {
  const pill = document.createElement("label");
  pill.className = "color-pill";

  const dot = document.createElement("span");
  dot.className = "color-dot";
  dot.style.background = hexVal;

  const info = document.createElement("span");
  info.className = "color-info";
  const lbl = document.createElement("b");
  lbl.textContent = labelText;
  const code = document.createElement("small");
  code.textContent = hexVal.toUpperCase();
  info.append(lbl, code);

  const inp = document.createElement("input");
  inp.type = "color";
  inp.value = hexVal;
  inp.addEventListener("input", () => {
    dot.style.background = inp.value;
    code.textContent = inp.value.toUpperCase();
    onInput(inp.value);
  });
  inp.addEventListener("change", () => onCommit(inp.value));

  pill.append(dot, info, inp);
  return pill;
}

export function renderAppearanceTab(ctx) {
  const { cfg, updateConfig, previewGlass } = ctx;
  const frag = document.createDocumentFragment();

  // Section 1: Palet Warna + Custom Color Picker
  frag.appendChild(createSectionTitle(t("sec.palette")));
  const swatchRow = document.createElement("div");
  swatchRow.className = "swatch-row";

  Object.entries(THEMES).forEach(([key, info]) => {
    const dot = document.createElement("span");
    dot.className = "swatch" + (cfg.theme_preset === key ? " active" : "");
    dot.title = info.name;
    dot.style.background = `linear-gradient(135deg, ${info.colors.join(",")})`;
    dot.addEventListener("click", () => {
      const wasCustom = cfg.theme_preset === "custom";
      swatchRow
        .querySelectorAll(".swatch")
        .forEach((s) => s.classList.remove("active"));
      dot.classList.add("active");
      updateConfig({ theme_preset: key }, { rerender: wasCustom });
    });
    swatchRow.appendChild(dot);
  });

  const customDot = document.createElement("span");
  customDot.className =
    "swatch swatch-custom" + (cfg.theme_preset === "custom" ? " active" : "");
  customDot.title = "Custom Colors";
  customDot.style.background =
    "conic-gradient(from 0deg, #ff7eb3, #feca57, #6ee7b7, #67e8f9, #c084fc, #ff7eb3)";
  customDot.addEventListener("click", () => {
    if (cfg.theme_preset === "custom") return;
    updateConfig({ theme_preset: "custom" }, { rerender: true });
  });
  swatchRow.appendChild(customDot);

  const paletteRows = [
    createRow(t("app.palette"), t("app.palette_desc"), swatchRow, true),
  ];

  if (cfg.theme_preset === "custom") {
    const pickerRow = document.createElement("div");
    pickerRow.className = "custom-colors-row";
    pickerRow.append(
      createColorPill(
        "Primary",
        cfg.custom_c1 || "#FFB7C5",
        (v) => previewGlass({ theme_preset: "custom", custom_c1: v }),
        (v) => updateConfig({ theme_preset: "custom", custom_c1: v }),
      ),
      createColorPill(
        "Secondary",
        cfg.custom_c2 || "#E2D1F9",
        (v) => previewGlass({ theme_preset: "custom", custom_c2: v }),
        (v) => updateConfig({ theme_preset: "custom", custom_c2: v }),
      ),
      createColorPill(
        "Accent",
        cfg.custom_c3 || "#A2E4F5",
        (v) => previewGlass({ theme_preset: "custom", custom_c3: v }),
        (v) => updateConfig({ theme_preset: "custom", custom_c3: v }),
      ),
    );
    paletteRows.push(
      createRow(
        "Custom Palette",
        "Primary · Secondary · Accent",
        pickerRow,
        true,
      ),
    );
  }

  frag.appendChild(createCard(paletteRows));

  // Section 2: Gaya Notch & Lirik
  frag.appendChild(createSectionTitle(t("sec.notch_style")));
  frag.appendChild(
    createCard([
      createRow(
        t("app.island"),
        t("app.island_desc"),
        createSwitch(cfg.dynamic_island, (val) =>
          updateConfig({ dynamic_island: val }),
        ),
      ),
      createRow(
        t("app.idle"),
        t("app.idle_desc"),
        createSelect(
          [
            { value: "pet", label: t("app.idle_pet") },
            { value: "idle", label: t("app.idle_clock") },
            { value: "minimal", label: t("app.idle_min") },
          ],
          cfg.resting_mode,
          (val) => updateConfig({ resting_mode: val }),
        ),
      ),
      createRow(
        t("app.album_glow"),
        t("app.album_glow_desc"),
        createSwitch(cfg.album_art_glow, (val) =>
          updateConfig({ album_art_glow: val }),
        ),
      ),
      createRow(
        t("app.keep_lyr"),
        t("app.keep_lyr_desc"),
        createSwitch(cfg.lyrics_pinned_open, (val) =>
          updateConfig({ lyrics_pinned_open: val }),
        ),
      ),
    ]),
  );

  // Section 3: HyperOS 4 Soft Light Glass Customizer (Hanya kontrol yang berefek nyata)
  frag.appendChild(createSectionTitle(t("sec.soft_glass")));

  const pct = (v) => `${Math.round(v)}%`;

  const glassSlider = (field, { min, max, step, scale = 1 }) =>
    createSlider({
      min,
      max,
      step,
      value: (cfg[field] ?? GLASS_DEFAULTS[field]) * scale,
      format: pct,
      onInput: (val) => previewGlass({ [field]: val / scale }),
      onCommit: (val) => updateConfig({ [field]: val / scale }),
    });

  const resetBtn = document.createElement("button");
  resetBtn.type = "button";
  resetBtn.className = "g-btn accent";
  resetBtn.textContent = t("glass.reset_btn");
  resetBtn.addEventListener("click", () => {
    updateConfig({ ...GLASS_DEFAULTS }, { rerender: true });
  });

  frag.appendChild(
    createCard([
      createRow(
        t("glass.backdrop"),
        t("glass.backdrop_desc"),
        createSelect(
          [
            { value: "acrylic", label: t("glass.bd_acrylic") },
            { value: "mica", label: t("glass.bd_mica") },
            { value: "mica_alt", label: t("glass.bd_mica_alt") },
            { value: "none", label: t("glass.bd_none") },
          ],
          cfg.bg_backdrop || "acrylic",
          (val) => updateConfig({ bg_backdrop: val }),
        ),
      ),
      createRow(
        t("glass.bg_op"),
        t("glass.bg_op_desc"),
        glassSlider("bg_opacity", { min: 0, max: 96, step: 1, scale: 100 }),
      ),
      createRow(
        t("glass.notch_op"),
        t("glass.notch_op_desc"),
        glassSlider("notch_bg_opacity", {
          min: 35,
          max: 100,
          step: 1,
          scale: 100,
        }),
      ),
      createRow(
        t("glass.comp_op"),
        t("glass.comp_op_desc"),
        glassSlider("comp_opacity", { min: 0, max: 40, step: 1, scale: 100 }),
      ),
      createRow(
        t("glass.btn_op"),
        t("glass.btn_op_desc"),
        glassSlider("btn_opacity", { min: 0, max: 40, step: 1, scale: 100 }),
      ),
      createRow(
        t("glass.rim"),
        t("glass.rim_desc"),
        glassSlider("glass_rim", { min: 0, max: 45, step: 1, scale: 100 }),
      ),
      createRow(t("glass.reset"), t("glass.reset_desc"), resetBtn),
    ]),
  );

  return frag;
}
