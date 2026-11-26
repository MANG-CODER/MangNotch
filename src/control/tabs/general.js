import { LANGUAGE_OPTIONS, t } from "../../shared/i18n.js";
import {
  createCard,
  createRow,
  createSectionTitle,
  createSegmented,
  createSelect,
  createSwitch,
} from "../ui.js";

export function renderGeneralTab(ctx) {
  const { cfg, monitors, updateConfig, onLanguageChange } = ctx;
  const frag = document.createDocumentFragment();

  // Section 1: Perilaku Layar & Interaksi
  frag.appendChild(createSectionTitle(t("sec.behavior")));
  frag.appendChild(
    createCard([
      createRow(
        t("gen.lang"),
        t("gen.lang_desc"),
        createSelect(
          LANGUAGE_OPTIONS.map((l) => ({ value: l.code, label: l.label })),
          cfg.language || "auto",
          (val) => onLanguageChange(val),
        ),
      ),
      createRow(
        t("gen.inter"),
        t("gen.inter_desc"),
        createSegmented(
          [
            { value: "hover", label: t("gen.hover") },
            { value: "click", label: t("gen.click") },
          ],
          cfg.interaction_mode,
          (val) => updateConfig({ interaction_mode: val }),
        ),
      ),
      createRow(
        t("gen.close_speed"),
        t("gen.close_speed_desc"),
        createSelect(
          [
            { value: "fast", label: t("gen.speed_fast") },
            { value: "normal", label: t("gen.speed_normal") },
            { value: "relaxed", label: t("gen.speed_relaxed") },
          ],
          cfg.hover_close_speed,
          (val) => updateConfig({ hover_close_speed: val }),
        ),
      ),
      createRow(
        t("gen.hotkey"),
        t("gen.hotkey_desc"),
        createSwitch(cfg.enable_hotkey, (val) =>
          updateConfig({ enable_hotkey: val }),
        ),
      ),
      createRow(
        t("gen.autostart"),
        t("gen.autostart_desc"),
        createSwitch(cfg.start_with_windows, (val) =>
          updateConfig({ start_with_windows: val }),
        ),
      ),
    ]),
  );

  // Section 2: Multi-Monitor & Deteksi Jendela
  frag.appendChild(createSectionTitle(t("sec.monitors")));

  const monRows = [
    createRow(
      t("mon.mode"),
      t("mon.mode_desc"),
      createSelect(
        [
          { value: "primary", label: t("mon.primary") },
          { value: "all", label: t("mon.all") },
          { value: "custom", label: t("mon.custom") },
        ],
        cfg.monitor_mode || "primary",
        (val) => {
          updateConfig(
            {
              monitor_mode: val,
              use_primary_monitor: val === "primary",
            },
            { rerender: true },
          );
        },
      ),
    ),
  ];

  // Picker multi-monitor interaktif saat mode === "custom"
  if ((cfg.monitor_mode || "primary") === "custom") {
    const listWrap = document.createElement("div");
    listWrap.className = "mon-list";
    const selectedSet = new Set(
      cfg.selected_monitors && cfg.selected_monitors.length
        ? cfg.selected_monitors
        : [0],
    );

    monitors.forEach((m) => {
      const pill = document.createElement("button");
      pill.type = "button";
      const isSel = selectedSet.has(m.index);
      pill.className = "mon-pill" + (isSel ? " selected" : "");
      const cleanName = m.name.replace(" - Primary", "");
      pill.innerHTML = `
        <span class="mon-dot"></span>
        <span>${cleanName}${m.is_primary ? ` (${t("mon.primary_tag")})` : ""}</span>
        <small>${m.width}×${m.height}</small>
      `;
      pill.addEventListener("click", () => {
        if (selectedSet.has(m.index)) {
          if (selectedSet.size === 1) return; // Minimal 1 monitor aktif
          selectedSet.delete(m.index);
        } else {
          selectedSet.add(m.index);
        }
        const nextList = Array.from(selectedSet).sort((a, b) => a - b);
        updateConfig(
          {
            selected_monitors: nextList,
            selected_monitor_index: nextList[0] ?? 0,
          },
          { rerender: true },
        );
      });
      listWrap.appendChild(pill);
    });

    monRows.push(createRow(t("mon.pick"), t("mon.pick_desc"), listWrap, true));
  }

  monRows.push(
    createRow(
      t("mon.fs"),
      t("mon.fs_desc"),
      createSwitch(cfg.hide_on_fullscreen, (val) =>
        updateConfig({ hide_on_fullscreen: val }),
      ),
    ),
    createRow(
      t("mon.prox"),
      t("mon.prox_desc"),
      createSwitch(cfg.smart_proximity, (val) =>
        updateConfig({ smart_proximity: val }),
      ),
    ),
  );

  frag.appendChild(createCard(monRows));
  return frag;
}
