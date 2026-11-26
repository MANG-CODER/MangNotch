import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";
import { createCard, createRow, createSectionTitle } from "../ui.js";

export function renderAboutTab(ctx) {
  const {
    appVersion,
    logoDataUrl,
    openUrl,
    openConfigDir,
    resetDefaultConfig,
  } = ctx;
  const frag = document.createDocumentFragment();

  const heroCard = document.createElement("div");
  heroCard.className = "card padded";
  heroCard.innerHTML = `
    <div class="about-hero">
      <div class="about-logo"><img src="${logoDataUrl}" alt="MangNotch"></div>
      <div class="about-title">
        <h2>MangNotch <span style="font-size:12px;color:var(--c1);font-weight:600">v${escHtml(appVersion)}</span></h2>
        <p>${escHtml(t("about.tagline"))}</p>
        <div class="tag-list">
          <span class="tag">Rust Backend</span>
          <span class="tag">Tauri v2</span>
          <span class="tag">Soft Light Glass</span>
          <span class="tag">Multi-Monitor</span>
        </div>
      </div>
    </div>
    <div class="feat-grid">
      <div class="feat-box"><b>${escHtml(t("about.f1_t"))}</b><span>${escHtml(t("about.f1_d"))}</span></div>
      <div class="feat-box"><b>${escHtml(t("about.f2_t"))}</b><span>${escHtml(t("about.f2_d"))}</span></div>
      <div class="feat-box"><b>${escHtml(t("about.f3_t"))}</b><span>${escHtml(t("about.f3_d"))}</span></div>
      <div class="feat-box"><b>${escHtml(t("about.f4_t"))}</b><span>${escHtml(t("about.f4_d"))}</span></div>
    </div>
  `;
  frag.appendChild(heroCard);

  frag.appendChild(createSectionTitle(t("sec.dev")));

  const devBtns = document.createElement("div");
  devBtns.style.cssText = "display:flex;gap:6px";
  const webBtn = document.createElement("button");
  webBtn.type = "button";
  webBtn.className = "g-btn";
  webBtn.textContent = t("about.web");
  webBtn.addEventListener("click", () =>
    openUrl("https://mangnotch.vercel.app/"),
  );

  const ghBtn = document.createElement("button");
  ghBtn.type = "button";
  ghBtn.className = "g-btn accent";
  ghBtn.textContent = t("about.gh");
  ghBtn.addEventListener("click", () =>
    openUrl("https://github.com/MANG-CODER/MangNotch"),
  );
  devBtns.append(webBtn, ghBtn);

  const dataBtns = document.createElement("div");
  dataBtns.style.cssText = "display:flex;gap:6px";
  const dirBtn = document.createElement("button");
  dirBtn.type = "button";
  dirBtn.className = "g-btn";
  dirBtn.textContent = t("about.open_dir");
  dirBtn.addEventListener("click", openConfigDir);

  const resetBtn = document.createElement("button");
  resetBtn.type = "button";
  resetBtn.className = "g-btn danger";
  resetBtn.textContent = t("about.reset_all");
  resetBtn.addEventListener("click", resetDefaultConfig);
  dataBtns.append(dirBtn, resetBtn);

  frag.appendChild(
    createCard([
      createRow(t("about.dev"), t("about.dev_desc"), devBtns),
      createRow(t("about.data"), t("about.data_desc"), dataBtns),
    ]),
  );

  return frag;
}
