import { t } from "../../shared/i18n.js";
import { createCard, createSectionTitle, mascotMarkup } from "../ui.js";

const MASCOTS = [
  { id: "neko", name: "MangCat", subKey: "mascot.neko_sub" },
  { id: "mochi", name: "Mochi", subKey: "mascot.mochi_sub" },
  { id: "kitsune", name: "Kitsune", subKey: "mascot.kitsune_sub" },
];

export function renderMascotTab(ctx) {
  const { cfg, updateConfig } = ctx;
  const frag = document.createDocumentFragment();

  frag.appendChild(createSectionTitle(t("sec.mascot_pick")));

  const desc = document.createElement("p");
  desc.style.cssText =
    "margin:0 0 12px;font-size:11.5px;color:var(--mist);line-height:1.45";
  desc.textContent = t("mascot.hero_desc");

  const grid = document.createElement("div");
  grid.className = "mascot-grid";

  MASCOTS.forEach((m) => {
    const card = document.createElement("div");
    card.className = "m-card" + (cfg.mascot_style === m.id ? " active" : "");
    card.innerHTML = `
      <div class="m-preview">${mascotMarkup(m.id)}</div>
      <b>${m.name}</b>
      <span>${t(m.subKey)}</span>
    `;
    card.addEventListener("click", () =>
      updateConfig({ mascot_style: m.id }, { rerender: true }),
    );
    grid.appendChild(card);
  });

  frag.appendChild(createCard([desc, grid], true));
  return frag;
}
