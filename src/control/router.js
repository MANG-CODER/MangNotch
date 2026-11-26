import { t } from "../shared/i18n.js";
import { renderGeneralTab } from "./tabs/general.js";
import { renderAppearanceTab } from "./tabs/appearance.js";
import { renderMascotTab } from "./tabs/mascot.js";
import { renderAudioTab } from "./tabs/audio.js";
import { renderProductivityTab } from "./tabs/productivity.js";
import { renderAboutTab } from "./tabs/about.js";

export const ROUTES = [
  {
    id: "general",
    titleKey: "nav.general",
    icon: `<circle cx="7" cy="7" r="2.2"/><circle cx="17" cy="7" r="2.2"/><circle cx="7" cy="17" r="2.2"/><circle cx="17" cy="17" r="2.2"/>`,
    render: renderGeneralTab,
  },
  {
    id: "appearance",
    titleKey: "nav.appearance",
    icon: `<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 1 9 9c0 1.66-1.34 3-3 3h-1.5a1.5 1.5 0 0 0-1.5 1.5c0 .83.67 1.5 1.5 1.5H17"/>`,
    render: renderAppearanceTab,
  },
  {
    id: "mascot",
    titleKey: "nav.mascot",
    icon: `<path d="M12 3c4.97 0 9 3.58 9 8 0 3.2-2.1 5.96-5.15 7.24L15 21l-3-1.5L9 21l-.85-2.76C5.1 16.96 3 14.2 3 11c0-4.42 4.03-8 9-8z"/><circle cx="9" cy="11" r="1"/><circle cx="15" cy="11" r="1"/>`,
    render: renderMascotTab,
  },
  {
    id: "audio",
    titleKey: "nav.audio",
    icon: `<path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>`,
    render: renderAudioTab,
  },
  {
    id: "productivity",
    titleKey: "nav.productivity",
    icon: `<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/>`,
    render: renderProductivityTab,
  },
  {
    id: "about",
    titleKey: "nav.about",
    icon: `<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>`,
    render: renderAboutTab,
  },
];

export function createRouter(getContext) {
  let activeId = "general";
  let lastIdx = 0;
  let searchQuery = "";
  const paneMap = new Map();

  const navEl = document.getElementById("ctrlNav");
  const pillEl = document.getElementById("navPill");
  const mountEl = document.getElementById("tabMount");
  const contentEl = document.getElementById("ctrlContent");
  const titleEl = document.getElementById("activePaneTitle");

  function updateNavHighlight() {
    ROUTES.forEach((route, idx) => {
      const btn = navEl.querySelector(`.nav-item[data-id="${route.id}"]`);
      if (btn) btn.classList.toggle("active", route.id === activeId);
      if (route.id === activeId && pillEl) {
        pillEl.style.transform = `translateY(${idx * 40}px)`;
      }
    });
    const activeRoute = ROUTES.find((r) => r.id === activeId) || ROUTES[0];
    if (titleEl) titleEl.textContent = t(activeRoute.titleKey);
  }

  function applyVisibility(animate = false) {
    const q = searchQuery.trim().toLowerCase();
    paneMap.forEach((pane, id) => {
      const show = q ? true : id === activeId;
      pane.classList.remove("tab-animate");
      pane.classList.toggle("active", show);

      if (q) {
        pane
          .querySelectorAll(".sec-title")
          .forEach((el) => el.classList.add("search-hidden"));
        pane.querySelectorAll(".card").forEach((card) => {
          const rows = card.querySelectorAll(".row");
          if (!rows.length) {
            card.classList.toggle(
              "search-hidden",
              !card.textContent.toLowerCase().includes(q),
            );
            return;
          }
          let any = false;
          rows.forEach((r) => {
            const match = r.textContent.toLowerCase().includes(q);
            r.classList.toggle("search-hidden", !match);
            if (match) any = true;
          });
          card.classList.toggle("search-hidden", !any);
        });
        pane.querySelectorAll(".tiles-grid").forEach((grid) => {
          grid.classList.toggle(
            "search-hidden",
            !grid.textContent.toLowerCase().includes(q),
          );
        });
      } else {
        pane
          .querySelectorAll(".search-hidden")
          .forEach((el) => el.classList.remove("search-hidden"));
        if (show && animate) {
          void pane.offsetWidth;
          pane.classList.add("tab-animate");
        }
      }
    });
  }

  function renderNav() {
    navEl.querySelectorAll(".nav-item").forEach((n) => n.remove());
    ROUTES.forEach((route, idx) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "nav-item" + (route.id === activeId ? " active" : "");
      btn.dataset.id = route.id;
      btn.innerHTML = `
        <span class="nav-ico"><svg viewBox="0 0 24 24">${route.icon}</svg></span>
        <span>${t(route.titleKey)}</span>
      `;
      btn.addEventListener("click", () => {
        if (route.id === activeId && !searchQuery) return;
        const searchInp = document.getElementById("settingsSearch");
        if (searchInp && searchInp.value) {
          searchInp.value = "";
          searchQuery = "";
        }
        const dist = Math.abs(idx - lastIdx);
        if (contentEl) contentEl.dataset.dir = idx >= lastIdx ? "down" : "up";
        lastIdx = idx;
        activeId = route.id;

        if (pillEl && dist > 0) {
          navEl.classList.add("pressing");
          setTimeout(() => navEl.classList.remove("pressing"), 220);
          if (pillEl.animate) {
            const sy = 1 + Math.min(0.14, 0.045 * dist);
            const sx = 1 - Math.min(0.03, 0.01 * dist);
            pillEl.animate(
              [
                { scale: "1 1" },
                { scale: `${sx} ${sy}`, offset: 0.38 },
                { scale: "1.01 .99", offset: 0.76 },
                { scale: "1 1" },
              ],
              { duration: 340, easing: "cubic-bezier(.22, 1, .36, 1)" },
            );
          }
        }
        updateNavHighlight();
        applyVisibility(true);
      });
      navEl.appendChild(btn);
    });
    updateNavHighlight();
  }

  function renderAll(animate = false) {
    const ctx = getContext();
    renderNav();
    mountEl.innerHTML = "";
    paneMap.clear();
    ROUTES.forEach((route) => {
      const pane = document.createElement("div");
      pane.className = "tab-pane";
      pane.appendChild(route.render(ctx));
      paneMap.set(route.id, pane);
      mountEl.appendChild(pane);
    });
    applyVisibility(animate);
  }

  function setSearch(q) {
    searchQuery = q;
    applyVisibility(false);
  }

  return { renderAll, setSearch, getActiveId: () => activeId };
}
