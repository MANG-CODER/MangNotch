export const THEMES = {
  dusk: { name: "MangNime Dusk", colors: ["#FFB7C5", "#E2D1F9", "#A2E4F5"] },
  emerald: {
    name: "Matcha Emerald",
    colors: ["#10b981", "#059669", "#34d399"],
  },
  racing: { name: "Racing Red", colors: ["#ff1e42", "#ff8800", "#1e3cff"] },
  sunset: { name: "Sunset", colors: ["#ff6b6b", "#feca57", "#ff9ff3"] },
  cyber: { name: "Ice Blue", colors: ["#19b8ff", "#2d5bff", "#7b4bff"] },
  sakura: { name: "Sakura", colors: ["#ff8fab", "#ffc2d1", "#b8f2e6"] },
  grape: { name: "Grape", colors: ["#c084fc", "#818cf8", "#f0abfc"] },
  mint: { name: "Mint", colors: ["#6ee7b7", "#67e8f9", "#bef264"] },
  amber: { name: "Amber", colors: ["#fbbf24", "#fb923c", "#fde68a"] },
};

export const GLASS_DEFAULTS = {
  bg_backdrop: "acrylic",
  bg_opacity: 0.42,
  notch_bg_opacity: 0.94,
  comp_opacity: 0.06,
  comp_blur: 14,
  btn_opacity: 0.09,
  btn_blur: 12,
  glass_rim: 0.16,
};

export function getThemeColors(preset, cfg = null) {
  if (preset === "custom" && cfg) {
    return [
      cfg.custom_c1 || "#FFB7C5",
      cfg.custom_c2 || "#E2D1F9",
      cfg.custom_c3 || "#A2E4F5",
    ];
  }
  return (THEMES[preset] || THEMES.dusk).colors;
}

export function applyTheme(preset, cfg = null) {
  const [c1, c2, c3] = getThemeColors(preset, cfg);
  const rs = document.documentElement.style;
  rs.setProperty("--c1", c1);
  rs.setProperty("--c2", c2);
  rs.setProperty("--c3", c3);
}

export function applyGlassConfig(cfg = {}) {
  const rs = document.documentElement.style;
  const num = (v, def) =>
    typeof v === "number" && Number.isFinite(v) ? v : def;

  rs.setProperty(
    "--bg-opacity",
    num(cfg.bg_opacity, GLASS_DEFAULTS.bg_opacity).toFixed(3),
  );
  rs.setProperty(
    "--notch-bg-opacity",
    num(cfg.notch_bg_opacity, GLASS_DEFAULTS.notch_bg_opacity).toFixed(3),
  );
  rs.setProperty(
    "--comp-opacity",
    num(cfg.comp_opacity, GLASS_DEFAULTS.comp_opacity).toFixed(3),
  );
  rs.setProperty(
    "--comp-blur",
    `${Math.round(num(cfg.comp_blur, GLASS_DEFAULTS.comp_blur))}px`,
  );
  rs.setProperty(
    "--btn-opacity",
    num(cfg.btn_opacity, GLASS_DEFAULTS.btn_opacity).toFixed(3),
  );
  rs.setProperty(
    "--btn-blur",
    `${Math.round(num(cfg.btn_blur, GLASS_DEFAULTS.btn_blur))}px`,
  );
  rs.setProperty(
    "--glass-rim",
    num(cfg.glass_rim, GLASS_DEFAULTS.glass_rim).toFixed(3),
  );
}

export function escHtml(s) {
  return String(s ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[c],
  );
}
