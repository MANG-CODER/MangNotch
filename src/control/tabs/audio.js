import { t } from "../../shared/i18n.js";
import {
  createCard,
  createRow,
  createSectionTitle,
  createSelect,
  createSlider,
  createSwitch,
} from "../ui.js";

const PEEK_TILES = [
  {
    key: "peek_volume",
    titleKey: "peek.vol",
    subKey: "peek.vol_sub",
    icon: `<path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>`,
  },
  {
    key: "peek_brightness",
    titleKey: "peek.br",
    subKey: "peek.br_sub",
    icon: `<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>`,
  },
  {
    key: "peek_mic",
    titleKey: "peek.mic",
    subKey: "peek.mic_sub",
    icon: `<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2M12 19v4M8 23h8"/>`,
  },
  {
    key: "peek_charging",
    titleKey: "peek.bat",
    subKey: "peek.bat_sub",
    icon: `<path d="M13 2L3 14h7l-1 8 10-12h-7z"/>`,
  },
  {
    key: "peek_bluetooth",
    titleKey: "peek.bt",
    subKey: "peek.bt_sub",
    icon: `<path d="m6.5 6.5 11 11L12 23V1l5.5 5.5-11 11"/>`,
  },
  {
    key: "peek_capslock",
    titleKey: "peek.caps",
    subKey: "peek.caps_sub",
    icon: `<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M8 15h8"/>`,
  },
  {
    key: "peek_numlock",
    titleKey: "peek.num",
    subKey: "peek.num_sub",
    icon: `<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M8 8h.01M12 8h.01M16 8h.01M8 12h.01M12 12h.01M16 12h.01M8 16h4M16 16h.01"/>`,
  },
  {
    key: "peek_keyboard_layout",
    titleKey: "peek.kb",
    subKey: "peek.kb_sub",
    icon: `<circle cx="12" cy="12" r="9"/><path d="M2 12h20M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>`,
  },
  {
    key: "peek_clipboard",
    titleKey: "peek.clip",
    subKey: "peek.clip_sub",
    icon: `<rect x="8" y="2" width="8" height="4" rx="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>`,
  },
  {
    key: "peek_usb",
    titleKey: "peek.usb",
    subKey: "peek.usb_sub",
    icon: `<path d="M12 2v15M8 6l4-4 4 4M6 12v2a6 6 0 0 0 6 6 6 6 0 0 0 6-6v-2"/><circle cx="6" cy="11" r="1.5"/><rect x="16.5" y="9.5" width="3" height="3" rx=".5"/>`,
  },
  {
    key: "peek_network",
    titleKey: "peek.net",
    subKey: "peek.net_sub",
    icon: `<path d="M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0"/><circle cx="12" cy="20" r="1"/>`,
  },
  {
    key: "peek_battery_low",
    titleKey: "peek.batlow",
    subKey: "peek.batlow_sub",
    icon: `<rect x="2" y="7" width="18" height="10" rx="2"/><path d="M22 11v2M7 11v2"/>`,
  },
];

export function renderAudioTab(ctx) {
  const { cfg, updateConfig, previewGlass } = ctx;
  const frag = document.createDocumentFragment();

  frag.appendChild(createSectionTitle(t("sec.audio_vis")));
  frag.appendChild(
    createCard([
      createRow(
        t("aud.vis"),
        t("aud.vis_desc"),
        createSwitch(cfg.enable_audio_visualizer, (val) =>
          updateConfig({ enable_audio_visualizer: val }),
        ),
      ),
      createRow(
        t("aud.rain"),
        t("aud.rain_desc"),
        createSlider({
          min: 10,
          max: 100,
          step: 5,
          value: cfg.ambient_volume ?? 55,
          format: (v) => `${Math.round(v)}%`,
          onInput: (val) => previewGlass({ ambient_volume: Math.round(val) }),
          onCommit: (val) => updateConfig({ ambient_volume: Math.round(val) }),
        }),
      ),
    ]),
  );

  frag.appendChild(createSectionTitle(t("sec.peeks")));
  frag.appendChild(
    createCard([
      createRow(
        t("aud.peeks"),
        t("aud.peeks_desc"),
        createSwitch(cfg.enable_peeks, (val) =>
          updateConfig({ enable_peeks: val }, { rerender: true }),
        ),
      ),
      createRow(
        t("aud.peek_dur"),
        t("aud.peek_dur_desc"),
        createSelect(
          [
            { value: "1600", label: t("aud.dur_short") },
            { value: "2400", label: t("aud.dur_norm") },
            { value: "3800", label: t("aud.dur_long") },
          ],
          String(cfg.peek_duration_ms || 2400),
          (val) => updateConfig({ peek_duration_ms: parseInt(val, 10) }),
          !cfg.enable_peeks,
        ),
      ),
    ]),
  );

  frag.appendChild(createSectionTitle(t("sec.peeks_filter")));
  const grid = document.createElement("div");
  grid.className = "tiles-grid";
  if (!cfg.enable_peeks) {
    grid.style.opacity = "0.42";
    grid.style.pointerEvents = "none";
  }

  PEEK_TILES.forEach((item) => {
    const isOn = cfg[item.key] !== false;
    const tile = document.createElement("div");
    tile.className = "peek-tile" + (isOn ? " on" : "");
    tile.innerHTML = `
      <span class="tile-orb"><svg viewBox="0 0 24 24">${item.icon}</svg></span>
      <span class="tile-info">
        <b>${t(item.titleKey)}</b>
        <small>${t(item.subKey)}</small>
      </span>
    `;
    tile.addEventListener("click", () => {
      const next = !(cfg[item.key] !== false);
      tile.classList.toggle("on", next);
      updateConfig({ [item.key]: next });
    });
    grid.appendChild(tile);
  });

  frag.appendChild(grid);
  return frag;
}
