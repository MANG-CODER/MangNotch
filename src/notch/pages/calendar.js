import { getActiveLang, getIntlLocale, t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";

const { invoke } = window.__TAURI__.core;

const CLOCK_SVG = `<svg class="ch-ico" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>`;
const BELL_SVG = `<svg class="ch-ico" viewBox="0 0 24 24"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg>`;
const CLOSE_SVG = `<svg viewBox="0 0 10 10"><path d="M2 2l6 6M8 2L2 8"/></svg>`;

const HR = (o) =>
  Object.fromEntries(
    o.split("|").map((x) => {
      const [d, n] = x.split("=");
      return [d, [n, n]];
    }),
  );
const HOLS = {
  ID: {
    "01-01": ["Tahun Baru Masehi", "New Year's Day"],
    "05-01": ["Hari Buruh Internasional", "Labour Day"],
    "06-01": ["Hari Lahir Pancasila", "Pancasila Day"],
    "08-17": ["Hari Kemerdekaan RI", "Indonesian Independence Day"],
    "12-25": ["Hari Raya Natal", "Christmas Day"],
  },
  US: HR(
    "01-01=New Year's Day|06-19=Juneteenth|07-04=Independence Day|11-11=Veterans Day|12-25=Christmas Day",
  ),
  GB: HR("01-01=New Year's Day|12-25=Christmas Day|12-26=Boxing Day"),
  CA: HR(
    "01-01=New Year's Day|07-01=Canada Day|12-25=Christmas Day|12-26=Boxing Day",
  ),
  JP: HR(
    "01-01=元日|02-11=建国記念の日|02-23=天皇誕生日|04-29=昭和の日|05-03=憲法記念日|05-04=みどりの日|05-05=こどもの日|11-03=文化の日|11-23=勤労感謝の日",
  ),
  KR: HR(
    "01-01=신정|03-01=삼일절|05-05=어린이날|06-06=현충일|08-15=광복절|10-03=개천절|10-09=한글날|12-25=성탄절",
  ),
  CN: HR("01-01=元旦|05-01=劳动节|10-01=国庆节"),
  TW: HR("01-01=元旦|02-28=和平紀念日|04-04=兒童節|10-10=國慶日"),
  SG: HR(
    "01-01=New Year's Day|05-01=Labour Day|08-09=National Day|12-25=Christmas Day",
  ),
  MY: HR(
    "01-01=Tahun Baru|05-01=Hari Pekerja|08-31=Hari Kebangsaan|09-16=Hari Malaysia|12-25=Hari Krismas",
  ),
  PH: HR(
    "01-01=Bagong Taon|04-09=Araw ng Kagitingan|05-01=Araw ng Paggawa|06-12=Araw ng Kalayaan|08-21=Ninoy Aquino Day|11-30=Bonifacio Day|12-25=Pasko|12-30=Rizal Day",
  ),
  TH: HR(
    "01-01=วันขึ้นปีใหม่|04-06=วันจักรี|04-13=วันสงกรานต์|05-01=วันแรงงาน|07-28=วันเฉลิมพระชนมพรรษา|08-12=วันแม่แห่งชาติ|12-05=วันพ่อแห่งชาติ|12-10=วันรัฐธรรมนูญ",
  ),
  VN: HR(
    "01-01=Tết Dương lịch|04-30=Ngày Giải phóng miền Nam|05-01=Quốc tế Lao động|09-02=Ngày Quốc khánh",
  ),
  IN: HR(
    "01-26=गणतंत्र दिवस|08-15=स्वतंत्रता दिवस|10-02=गांधी जयंती|12-25=क्रिसमस",
  ),
  AU: HR(
    "01-01=New Year's Day|01-26=Australia Day|04-25=Anzac Day|12-25=Christmas Day|12-26=Boxing Day",
  ),
  DE: HR(
    "01-01=Neujahr|05-01=Tag der Arbeit|10-03=Tag der Deutschen Einheit|12-25=1. Weihnachtstag|12-26=2. Weihnachtstag",
  ),
  FR: HR(
    "01-01=Jour de l'an|05-01=Fête du Travail|05-08=Victoire 1945|07-14=Fête nationale|08-15=Assomption|11-01=Toussaint|11-11=Armistice|12-25=Noël",
  ),
  ES: HR(
    "01-01=Año Nuevo|01-06=Epifanía|05-01=Día del Trabajador|08-15=Asunción|10-12=Fiesta Nacional|11-01=Todos los Santos|12-06=Constitución|12-25=Navidad",
  ),
  NL: HR(
    "01-01=Nieuwjaarsdag|04-27=Koningsdag|05-05=Bevrijdingsdag|12-25=Eerste kerstdag|12-26=Tweede kerstdag",
  ),
  RU: HR(
    "01-01=Новый год|01-07=Рождество|02-23=День защитника Отечества|03-08=Международный женский день|05-01=Праздник Весны и Труда|05-09=День Победы|06-12=День России|11-04=День народного единства",
  ),
  BR: HR(
    "01-01=Ano Novo|04-21=Tiradentes|05-01=Dia do Trabalho|09-07=Independência|10-12=Nossa Senhora Aparecida|11-02=Finados|11-15=Proclamação da República|12-25=Natal",
  ),
};

/* Pemetaan IANA Timezone OS -> Kode Negara Hari Libur (independen dari bahasa OS/App) */
const TZ_EXACT = {
  "Asia/Jakarta": "ID",
  "Asia/Pontianak": "ID",
  "Asia/Makassar": "ID",
  "Asia/Jayapura": "ID",
  "Asia/Ujung_Pandang": "ID",
  "Asia/Tokyo": "JP",
  "Asia/Seoul": "KR",
  "Asia/Taipei": "TW",
  "Asia/Shanghai": "CN",
  "Asia/Chongqing": "CN",
  "Asia/Harbin": "CN",
  "Asia/Urumqi": "CN",
  "Asia/Hong_Kong": "CN",
  "Asia/Singapore": "SG",
  "Asia/Kuala_Lumpur": "MY",
  "Asia/Kuching": "MY",
  "Asia/Manila": "PH",
  "Asia/Bangkok": "TH",
  "Asia/Ho_Chi_Minh": "VN",
  "Asia/Saigon": "VN",
  "Asia/Kolkata": "IN",
  "Asia/Calcutta": "IN",
  "Europe/London": "GB",
  "Europe/Belfast": "GB",
  "Europe/Berlin": "DE",
  "Europe/Busingen": "DE",
  "Europe/Paris": "FR",
  "Europe/Madrid": "ES",
  "Atlantic/Canary": "ES",
  "Africa/Ceuta": "ES",
  "Europe/Amsterdam": "NL",
  "Europe/Moscow": "RU",
  "Europe/Kaliningrad": "RU",
  "Europe/Samara": "RU",
  "Asia/Yekaterinburg": "RU",
  "Asia/Novosibirsk": "RU",
  "Asia/Krasnoyarsk": "RU",
  "Asia/Irkutsk": "RU",
  "Asia/Vladivostok": "RU",
  "America/Toronto": "CA",
  "America/Vancouver": "CA",
  "America/Edmonton": "CA",
  "America/Winnipeg": "CA",
  "America/Halifax": "CA",
  "America/Montreal": "CA",
  "America/Sao_Paulo": "BR",
  "America/Manaus": "BR",
  "America/Fortaleza": "BR",
  "America/Recife": "BR",
  "America/Bahia": "BR",
  "America/Belem": "BR",
};

const LANG_TO_COUNTRY = {
  id: "ID",
  en: "US",
  ja: "JP",
  ko: "KR",
  "zh-CN": "CN",
  "zh-TW": "TW",
  ms: "MY",
  fil: "PH",
  th: "TH",
  vi: "VN",
  hi: "IN",
  de: "DE",
  fr: "FR",
  es: "ES",
  nl: "NL",
  ru: "RU",
};

function detectCountryFromTimezone() {
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone || "";
    if (TZ_EXACT[tz]) return TZ_EXACT[tz];
    if (tz.startsWith("Australia/")) return "AU";
    if (
      tz.startsWith("US/") ||
      /^America\/(New_York|Chicago|Denver|Los_Angeles|Phoenix|Anchorage|Detroit|Indiana|Kentucky)/.test(
        tz,
      ) ||
      tz === "Pacific/Honolulu"
    )
      return "US";
    if (tz.startsWith("Canada/")) return "CA";
    if (tz.startsWith("Brazil/")) return "BR";
  } catch {}
  return LANG_TO_COUNTRY[getActiveLang()] || "US";
}

const DAYS_MAP = {
  senin: 1,
  mon: 1,
  monday: 1,
  selasa: 2,
  tue: 2,
  tuesday: 2,
  rabu: 3,
  wed: 3,
  wednesday: 3,
  kamis: 4,
  thu: 4,
  thursday: 4,
  jumat: 5,
  "jum'at": 5,
  fri: 5,
  friday: 5,
  sabtu: 6,
  sat: 6,
  saturday: 6,
  minggu: 0,
  sun: 0,
  sunday: 0,
};
const DW =
  "(senin|selasa|rabu|kamis|jum'at|jumat|sabtu|minggu|monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|wed|thu|fri|sat|sun)";

const pad = (n) => String(n).padStart(2, "0");
const keyOf = (d) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const mondayOf = (d) => {
  const x = new Date(d);
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7));
  return x;
};
const daysInMonth = (y, m) => new Date(y, m + 1, 0).getDate();
const dayDelta = (n) => {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() + n);
  return d;
};

function parseQuick(v) {
  let s = ` ${v} `,
    date = null,
    rep = "",
    tm = "",
    rm = null;
  const today = dayDelta(0);
  const take = (re, fn) => {
    const m = s.match(re);
    if (m) {
      s = s.replace(m[0], " ");
      fn(m);
    }
    return !!m;
  };
  const nextDow = (dow, incl) => {
    let add = (dow - today.getDay() + 7) % 7;
    if (!add && !incl) add = 7;
    return dayDelta(add);
  };

  take(/\br(\d{1,3})\b/i, (m) => {
    rm = +m[1];
  });
  take(new RegExp(`\\b(?:setiap|tiap|every)\\s+${DW}\\b`, "i"), (m) => {
    rep = "w";
    date = nextDow(DAYS_MAP[m[1].toLowerCase()], true);
  }) ||
    take(
      /\b(?:setiap|tiap|every)\s+(?:hari|day)\b|\b(?:harian|daily)\b/i,
      () => {
        rep = "d";
      },
    ) ||
    take(/\b(?:mingguan|weekly)\b/i, () => {
      rep = "w";
    });
  take(/(?:jam|at|@)?\s*(\d{1,2})[:.](\d{2})\s*(am|pm)?/i, (m) => {
    let h = +m[1];
    if (m[3]) h = (h % 12) + (/pm/i.test(m[3]) ? 12 : 0);
    if (h < 24 && +m[2] < 60) tm = `${pad(h)}:${m[2]}`;
  });
  if (!tm) {
    take(
      /\b(?:jam|at)\s+(\d{1,2})\b\s*(am|pm)?|\b(\d{1,2})\s*(am|pm)\b/i,
      (m) => {
        let h = +(m[1] || m[3]);
        const ap = m[2] || m[4];
        if (ap) h = (h % 12) + (/pm/i.test(ap) ? 12 : 0);
        if (h < 24) tm = `${pad(h)}:00`;
      },
    );
  }
  take(/\b(?:besok|tomorrow)\b/i, () => {
    date = date || dayDelta(1);
  });
  take(/\blusa\b/i, () => {
    date = date || dayDelta(2);
  });
  take(/\b(?:hari ini|today)\b/i, () => {
    date = date || dayDelta(0);
  });
  if (!date)
    take(new RegExp(`\\b${DW}\\b`, "i"), (m) => {
      date = nextDow(DAYS_MAP[m[1].toLowerCase()], false);
    });
  if (!date) {
    take(/\b(\d{1,2})[\/-](\d{1,2})(?:[\/-](\d{2,4}))?\b/, (m) => {
      let y = m[3] ? +m[3] : today.getFullYear();
      if (y < 100) y += 2000;
      const d = new Date(y, +m[2] - 1, +m[1]);
      if (!m[3] && d < today) d.setFullYear(y + 1);
      if (!Number.isNaN(d.getTime())) date = d;
    });
  }
  const x = s.replace(/\s+/g, " ").trim();
  return x ? { date, t: tm, x, rep, rm } : null;
}

export function createCalendarModule({ router, mascot, playChime }) {
  const $ = (id) => document.getElementById(id);
  const layer = $("calLayer");
  const inp = $("calInput");
  const addBtn = $("calAddBtn");

  let saveBtn = $("calSaveBtn");
  if (!saveBtn && layer) {
    saveBtn = document.createElement("span");
    saveBtn.id = "calSaveBtn";
    saveBtn.className = "a inline-save-btn";
    saveBtn.title = "Save (Enter)";
    saveBtn.innerHTML = `<svg viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5"/></svg>`;
    layer.appendChild(saveBtn);
  }

  let data = {};
  let lastJson = "";
  let saveTimer = null;
  let sel = new Date();
  let open = false;
  let shownAt = 0;
  let editing = null;
  let qs = { day: null, rep: "", rm: 10, tm: "" };
  let py = new Date().getFullYear();
  let holRegionPref = "auto";
  let remEnabled = true;
  let remLeadMin = 10;
  let remSound = true;
  const fired = new Set();

  function getHolidayMap() {
    if (holRegionPref === "none") return {};
    if (holRegionPref !== "auto") return HOLS[holRegionPref] || {};
    return HOLS[detectCountryFromTimezone()] || {};
  }

  const isDone = (e, k) => (e.rep ? (e.dn || []).includes(k) : !!e.d);

  function eventsFor(k) {
    const out = [];
    const dw = new Date(`${k}T00:00`).getDay();
    Object.keys(data).forEach((bk) => {
      if (bk > k) return;
      (data[bk] || []).forEach((e, i) => {
        if (
          bk === k ||
          e.rep === "d" ||
          (e.rep === "w" && new Date(`${bk}T00:00`).getDay() === dw)
        ) {
          out.push({ e, i, bk });
        }
      });
    });
    return out.sort((a, b) =>
      (a.e.t || "99:99").localeCompare(b.e.t || "99:99"),
    );
  }

  function save() {
    const j = JSON.stringify(data);
    lastJson = j;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      invoke("set_calendar_data", { json: j }).catch(() => {});
    }, 300);
  }

  function finalOf(q = {}) {
    return {
      date: q.date || (qs.day != null ? dayDelta(qs.day) : sel),
      t: q.t || qs.tm || "",
      rep: q.rep || qs.rep,
      rm: q.rm != null ? q.rm : qs.rm,
    };
  }

  function buildPreviewHtml(f) {
    const mn = t("notch.min");
    const when = f.date.toLocaleDateString(getIntlLocale(), {
      weekday: "short",
      day: "numeric",
      month: "short",
    });
    const parts = [`<span>${escHtml(when)}</span>`];
    parts.push(`<span>· ${escHtml(f.t || t("notch.cal_notime"))}</span>`);
    if (f.rep)
      parts.push(
        `<span>· ↻ ${escHtml(f.rep === "d" ? t("notch.cal_daily") : t("notch.cal_weekly"))}</span>`,
      );
    if (f.t)
      parts.push(
        `${BELL_SVG}<span>${escHtml(f.rm ? `${f.rm}${mn}` : t("notch.cal_ontime"))}</span>`,
      );
    return parts.join("");
  }

  function updatePreviewOnly() {
    const pv = $("calHelp")?.querySelector(".ch-prev");
    if (pv)
      pv.innerHTML = buildPreviewHtml(
        finalOf(parseQuick(inp.value.trim()) || {}),
      );
  }

  function stepTime(part, delta) {
    let [hh, mm] = (qs.tm || "09:00").split(":").map(Number);
    if (Number.isNaN(hh)) hh = 9;
    if (Number.isNaN(mm)) mm = 0;
    if (part === "h") hh = (hh + delta + 24) % 24;
    else mm = (mm + delta + 60) % 60;
    qs.tm = `${pad(hh)}:${pad(mm)}`;
    renderHelp();
  }

  function bindTimeInputs() {
    const hrIn = $("calHrIn");
    const mnIn = $("calMnIn");
    if (!hrIn || !mnIn) return;

    const commitDigits = () => {
      const h = Math.min(23, Math.max(0, parseInt(hrIn.value, 10) || 0));
      const m = Math.min(59, Math.max(0, parseInt(mnIn.value, 10) || 0));
      hrIn.value = pad(h);
      mnIn.value = pad(m);
      qs.tm = `${pad(h)}:${pad(m)}`;
      updatePreviewOnly();
    };

    [hrIn, mnIn].forEach((el, idx) => {
      el.addEventListener("focus", () => el.select());
      el.addEventListener("input", () => {
        el.value = el.value.replace(/\D/g, "").slice(0, 2);
        const h = Math.min(23, Math.max(0, parseInt(hrIn.value, 10) || 0));
        const m = Math.min(59, Math.max(0, parseInt(mnIn.value, 10) || 0));
        qs.tm = `${pad(h)}:${pad(m)}`;
        updatePreviewOnly();
        if (idx === 0 && el.value.length === 2) mnIn.focus();
      });
      el.addEventListener("blur", (e) => {
        commitDigits();
        if (performance.now() - shownAt < 380) return;
        if (e.relatedTarget && layer.contains(e.relatedTarget)) return;
        hideInput();
      });
      el.addEventListener("keydown", (e) => {
        if (e.key === "ArrowUp") {
          e.preventDefault();
          stepTime(idx === 0 ? "h" : "m", 1);
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          stepTime(idx === 0 ? "h" : "m", -1);
        } else if (e.key === "Enter") {
          commitDigits();
          commitEvent();
        } else if (e.key === "Escape") {
          hideInput();
        }
      });
    });
  }

  function renderHelp() {
    const h = $("calHelp");
    if (!h) return;
    const q = parseQuick(inp.value.trim());
    const f = finalOf(q || {});
    const mn = t("notch.min");
    const lab = {
      rep: [
        ["", t("notch.cal_once")],
        ["d", `↻ ${t("notch.cal_daily")}`],
        ["w", `↻ ${t("notch.cal_weekly")}`],
      ],
      rm: [
        ["0", t("notch.cal_ontime")],
        ["5", `5${mn}`],
        ["10", `10${mn}`],
        ["30", `30${mn}`],
      ],
    };
    const cur = { rep: f.rep, rm: String(f.rm) };

    const [hh, mm] = (f.t || "").split(":");
    const timeBarHtml = f.t
      ? `
        <div class="ch-time-row">
          <div class="ch-t-capsule">
            ${CLOCK_SVG}
            <span class="ch-t-btn" data-tp="h" data-d="-1">−</span>
            <input type="text" inputmode="numeric" maxlength="2" id="calHrIn" class="ch-t-in" data-tp="h" value="${escHtml(hh)}" title="Hour (type / scroll)">
            <span class="ch-t-btn" data-tp="h" data-d="1">+</span>
            <span class="ch-t-colon">:</span>
            <span class="ch-t-btn" data-tp="m" data-d="-5">−</span>
            <input type="text" inputmode="numeric" maxlength="2" id="calMnIn" class="ch-t-in" data-tp="m" value="${escHtml(mm)}" title="Minute (type / scroll)">
            <span class="ch-t-btn" data-tp="m" data-d="5">+</span>
          </div>
          <span class="ch-time-clr" id="calTimeClr" title="Clear">${CLOSE_SVG}</span>
        </div>`
      : `
        <div class="ch-time-row">
          <span class="ch-t-add" id="calTimeEnable">${CLOCK_SVG}<span>+ 09:00</span></span>
        </div>`;

    h.innerHTML = `
      <div class="ch-prev">${buildPreviewHtml(f)}</div>
      ${timeBarHtml}
      <div class="ch-row">${lab.rep
        .map(
          ([v, txt]) =>
            `<span class="ch${cur.rep === v ? " on" : ""}" data-g="rep" data-v="${v}">${escHtml(txt)}</span>`,
        )
        .join("")}</div>
      <div class="ch-row">${BELL_SVG}${lab.rm
        .map(
          ([v, txt]) =>
            `<span class="ch${cur.rm === v ? " on" : ""}" data-g="rm" data-v="${v}">${escHtml(txt)}</span>`,
        )
        .join("")}</div>
    `;

    bindTimeInputs();
  }

  function renderNext() {
    const el = $("calNext");
    const chip = $("calToday");
    if (!el) return;
    const now = new Date();
    const nk = keyOf(now);
    const sk = keyOf(sel);
    const far = sk !== nk;
    if (chip) {
      chip.classList.toggle("show", far);
      if (far)
        chip.querySelector("b").textContent =
          (sk > nk ? "‹ " : "") + t("notch.cal_today") + (sk > nk ? "" : " ›");
    }
    let txt = "";
    if (!far) {
      const nowT = `${pad(now.getHours())}:${pad(now.getMinutes())}`;
      const n = eventsFor(nk).find(
        (o) => o.e.t && !isDone(o.e, nk) && o.e.t >= nowT,
      );
      if (n) {
        const [hh, mm] = n.e.t.split(":").map(Number);
        const mins = hh * 60 + mm - (now.getHours() * 60 + now.getMinutes());
        txt =
          `${t("notch.cal_next")} ${n.e.t} · ` +
          (mins >= 60
            ? `${Math.floor(mins / 60)} ${t("notch.hour")}`
            : `${mins} ${t("notch.min")}`);
      }
    }
    el.textContent = txt;
  }

  function renderMonth(dir = 0) {
    const g = $("cxGrid");
    if (!g) return;
    const HOL = getHolidayMap();
    const y = sel.getFullYear(),
      m = sel.getMonth();
    const st = mondayOf(new Date(y, m, 1));
    const today = keyOf(new Date()),
      sk = keyOf(sel);
    const loc = getIntlLocale();

    $("cxWd").innerHTML = [...Array(7)]
      .map((_, i) => {
        const d = new Date(st);
        d.setDate(st.getDate() + i);
        return `<span>${d.toLocaleDateString(loc, { weekday: "narrow" })}</span>`;
      })
      .join("");
    $("cxTitle").textContent = sel.toLocaleDateString(loc, {
      month: "long",
      year: "numeric",
    });

    g.innerHTML = [...Array(42)]
      .map((_, i) => {
        const d = new Date(st);
        d.setDate(st.getDate() + i);
        const k = keyOf(d),
          h = HOL[k.slice(5)],
          has = eventsFor(k).length || h;
        const tip = h ? (getActiveLang() === "id" ? h[0] : h[1]) : "";
        return `<span class="c2${d.getMonth() !== m ? " out" : ""}${k === today ? " today" : ""}${k === sk ? " sel" : ""}${has ? " has" : ""}${h ? " hol" : ""}" data-d="${k}"${tip ? ` title="${escHtml(tip)}"` : ""}><b>${d.getDate()}</b><u></u></span>`;
      })
      .join("");

    if (dir) {
      g.style.setProperty("--wd", `${dir * 16}px`);
      g.classList.remove("wk");
      void g.offsetWidth;
      g.classList.add("wk");
    }
  }

  function render(dir = 0, doneTarget = null) {
    const strip = $("calStrip");
    if (!strip) return;
    const HOL = getHolidayMap();
    const today = keyOf(new Date()),
      st = mondayOf(sel),
      sk = keyOf(sel);
    const loc = getIntlLocale();

    strip.innerHTML = [...Array(7)]
      .map((_, i) => {
        const d = new Date(st);
        d.setDate(st.getDate() + i);
        const k = keyOf(d),
          h = HOL[k.slice(5)],
          has = eventsFor(k).length || h;
        const tip = h ? (getActiveLang() === "id" ? h[0] : h[1]) : "";
        return `<span class="cd${k === today ? " today" : ""}${k === sk ? " sel" : ""}${has ? " has" : ""}${h ? " hol" : ""}" style="--k:${i}" data-d="${k}"${tip ? ` title="${escHtml(tip)}"` : ""}><i>${d.toLocaleDateString(loc, { weekday: "narrow" })}</i><b>${d.getDate()}</b><u></u></span>`;
      })
      .join("");

    if (dir) {
      strip.style.setProperty("--wd", `${dir * 14}px`);
      strip.classList.remove("wk");
      void strip.offsetWidth;
      strip.classList.add("wk");
    }

    $("calDay").textContent = String(sel.getDate());
    $("calSub").textContent =
      `${sel.toLocaleDateString(loc, { weekday: "long" })} · ${sel.toLocaleDateString(loc, { month: "short", year: "numeric" })}`;
    $("calAddTxt").textContent = t("notch.cal_add");

    const hol = HOL[sk.slice(5)];
    const holText = hol ? hol[getActiveLang() === "id" ? 0 : 1] : null;
    const list = eventsFor(sk);
    $("calList").innerHTML =
      (holText
        ? `<div class="trow hol"><div class="trow-left"><span class="trow-txt">★ ${escHtml(holText)}</span></div></div>`
        : "") +
        list
          .map((o) => {
            const done = isDone(o.e, sk);
            const justDone = done && doneTarget === `${o.bk}|${o.i}`;
            return `
        <div class="trow${done ? " done" : ""}${justDone ? " t-done" : ""}" data-bk="${o.bk}" data-i="${o.i}">
          <div class="trow-left">
            <span class="chk"><svg viewBox="0 0 10 10"><path d="M2 5l2.2 2.2L8 2.5"/></svg></span>
            ${o.e.t ? `<span class="ctm">${escHtml(o.e.t)}</span>` : ""}
            ${o.e.rep ? `<span class="crp">↻</span>` : ""}
            <span class="trow-txt">${escHtml(o.e.x)}</span>
          </div>
          <span class="tdel" data-del="1"><svg viewBox="0 0 10 10"><path d="M2 2l6 6M8 2L2 8"/></svg></span>
        </div>`;
          })
          .join("") ||
      (holText ? "" : `<div class="cmt">${escHtml(t("notch.cal_none"))}</div>`);

    if (router.state.calExpanded) renderMonth(dir);
    renderNext();
  }

  function commitEvent() {
    if (!inp.value.trim()) {
      inp.classList.remove("mm-shake");
      void inp.offsetWidth;
      inp.classList.add("mm-shake");
      inp.focus();
      return;
    }
    const q = parseQuick(inp.value.trim());
    if (!q) return;
    const f = finalOf(q);
    if (editing) {
      const ev = (data[editing.bk] || [])[editing.i];
      if (ev) {
        ev.t = f.t;
        ev.x = q.x;
        if (f.rep) {
          ev.rep = f.rep;
          ev.dn = ev.dn || [];
        } else delete ev.rep;
        if (f.rm !== remLeadMin) ev.rm = f.rm;
        else delete ev.rm;
        const nk = keyOf(f.date);
        if ((q.date || qs.day != null) && nk !== editing.bk) {
          data[editing.bk].splice(editing.i, 1);
          if (!data[editing.bk].length) delete data[editing.bk];
          (data[nk] = data[nk] || []).push(ev);
          sel = f.date;
        }
      }
    } else {
      const ev = { t: f.t, x: q.x, d: false };
      if (f.rep) {
        ev.rep = f.rep;
        ev.dn = [];
      }
      if (f.rm !== remLeadMin) ev.rm = f.rm;
      const dk = keyOf(f.date);
      (data[dk] = data[dk] || []).push(ev);
      sel = new Date(`${dk}T00:00`);
    }
    save();
    hideInput();
    render(0);
  }

  async function showInput() {
    open = true;
    shownAt = performance.now();
    layer.classList.add("adding");
    qs = { day: null, rep: "", rm: remLeadMin, tm: "" };
    inp.value = "";
    inp.placeholder = t("notch.cal_add") + "...";
    saveBtn?.classList.remove("has-val");
    renderHelp();
    addBtn.style.display = "none";
    inp.style.display = "block";
    if (saveBtn) saveBtn.style.display = "flex";
    try {
      await invoke("set_notch_text_input", { active: true });
    } catch {}
    setTimeout(() => {
      if (open) inp.focus();
    }, 45);
  }

  function hideInput() {
    if (!open) return;
    open = false;
    editing = null;
    layer.classList.remove("adding");
    inp.style.display = "none";
    if (saveBtn) saveBtn.style.display = "none";
    addBtn.style.display = "flex";
    invoke("set_notch_text_input", { active: false }).catch(() => {});
  }

  function shiftMonth(delta) {
    const d = sel.getDate();
    const target = new Date(sel.getFullYear(), sel.getMonth() + delta, 1);
    target.setDate(
      Math.min(d, daysInMonth(target.getFullYear(), target.getMonth())),
    );
    sel = target;
    render(delta > 0 ? 1 : -1);
  }

  function renderPicker() {
    const loc = getIntlLocale();
    $("cxPick").innerHTML = `
      <div class="cx-py">
        <span class="cx-b" data-y="-1"><svg viewBox="0 0 10 10"><path d="M6.5 1.5 3 5l3.5 3.5"/></svg></span>
        <b>${py}</b>
        <span class="cx-b" data-y="1"><svg viewBox="0 0 10 10"><path d="M3.5 1.5 7 5l-3.5 3.5"/></svg></span>
      </div>
      <div class="cx-pm">
        ${[...Array(12)].map((_, i) => `<span class="cx-m${py === sel.getFullYear() && i === sel.getMonth() ? " on" : ""}" data-m="${i}">${new Date(2000, i, 1).toLocaleDateString(loc, { month: "short" })}</span>`).join("")}
      </div>`;
  }

  function checkReminders() {
    if (!remEnabled) return;
    const now = new Date();
    const k = keyOf(now);
    const curMin = now.getHours() * 60 + now.getMinutes();
    eventsFor(k).forEach((o) => {
      const e = o.e;
      if (!e.t || isDone(e, k)) return;
      const tm = +e.t.slice(0, 2) * 60 + +e.t.slice(3);
      const pre = e.rm != null ? e.rm : remLeadMin;
      [
        ["pre", tm - pre, pre > 0],
        ["at", tm, true],
      ].forEach(([kind, at, on]) => {
        const id = `${k}|${o.bk}|${o.i}|${kind}|${e.t}`;
        if (
          !on ||
          fired.has(id) ||
          curMin < at ||
          curMin - at > 5 ||
          (kind === "pre" && curMin >= tm)
        )
          return;
        if (remSound) playChime("pomo");
        mascot.setMood("surprised", 2000);
        router.showCustomPeek(
          `<svg viewBox="0 0 24 24" style="width:16px;height:16px;fill:#E2D1F9"><path d="M7 2v2H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2V2h-2v2H9V2zM5 9h14v10H5z"/></svg>`,
          kind === "pre" ? t("notch.cal_soon") : t("notch.cal_rem"),
          e.x,
          kind === "pre" ? `${pre} ${t("notch.min")}` : e.t,
          "#E2D1F9",
        );
        fired.add(id);
      });
    });
  }

  const goToday = (e) => {
    if (e) e.stopPropagation();
    const dir = keyOf(sel) > keyOf(new Date()) ? -1 : 1;
    sel = new Date();
    render(dir);
  };
  $("calDay").addEventListener("click", goToday);
  $("calToday").addEventListener("click", goToday);
  addBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    showInput();
  });

  saveBtn?.addEventListener("mousedown", (e) => e.preventDefault());
  saveBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    commitEvent();
  });

  $("calExpandBtn").addEventListener("click", (e) => {
    e.stopPropagation();
    router.setCalExpanded(true);
    render(0);
  });
  $("calCollapseBtn").addEventListener("click", (e) => {
    e.stopPropagation();
    $("cxPick").classList.remove("on");
    router.setCalExpanded(false);
    render(0);
  });
  $("cxPrevMonth").addEventListener("click", (e) => {
    e.stopPropagation();
    shiftMonth(-1);
  });
  $("cxNextMonth").addEventListener("click", (e) => {
    e.stopPropagation();
    shiftMonth(1);
  });
  $("cxTitle").addEventListener("click", (e) => {
    e.stopPropagation();
    const p = $("cxPick");
    if (p.classList.toggle("on")) {
      py = sel.getFullYear();
      renderPicker();
    }
  });

  $("cxPick").addEventListener("click", (e) => {
    e.stopPropagation();
    const yBtn = e.target.closest("[data-y]");
    const mBtn = e.target.closest("[data-m]");
    if (yBtn) {
      py += +yBtn.dataset.y;
      renderPicker();
    } else if (mBtn) {
      const mi = +mBtn.dataset.m;
      const target = new Date(py, mi, 1);
      target.setDate(Math.min(sel.getDate(), daysInMonth(py, mi)));
      const dir = target > sel ? 1 : -1;
      sel = target;
      $("cxPick").classList.remove("on");
      render(dir);
    }
  });

  $("calStrip").addEventListener("click", (e) => {
    const c = e.target.closest(".cd[data-d]");
    if (!c) return;
    e.stopPropagation();
    const [yy, mm, dd] = c.dataset.d.split("-").map(Number);
    sel = new Date(yy, mm - 1, dd);
    render(0);
  });
  $("calStrip").addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      e.stopPropagation();
      const dir = e.deltaY > 0 ? 1 : -1;
      sel.setDate(sel.getDate() + dir * 7);
      render(dir);
    },
    { passive: false },
  );

  $("cxGrid").addEventListener("click", (e) => {
    const c = e.target.closest(".c2[data-d]");
    if (!c) return;
    e.stopPropagation();
    const [yy, mm, dd] = c.dataset.d.split("-").map(Number);
    const next = new Date(yy, mm - 1, dd);
    const dir = next.getMonth() !== sel.getMonth() ? (next > sel ? 1 : -1) : 0;
    sel = next;
    render(dir);
  });
  let wheelTs = 0;
  $("cxGrid").addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      e.stopPropagation();
      const now = performance.now();
      if (now - wheelTs < 140) return;
      wheelTs = now;
      shiftMonth(e.deltaY > 0 ? 1 : -1);
    },
    { passive: false },
  );

  $("calList").addEventListener(
    "wheel",
    (e) => {
      const l = $("calList");
      if (l.scrollHeight > l.clientHeight + 1) e.stopPropagation();
    },
    { passive: true },
  );

  $("calList").addEventListener("click", (e) => {
    const row = e.target.closest(".trow[data-bk]");
    if (!row) return;
    e.stopPropagation();
    const bk = row.dataset.bk;
    const i = parseInt(row.dataset.i, 10);
    const ev = (data[bk] || [])[i];
    if (!ev) return;

    if (e.target.closest("[data-del]")) {
      row.classList.add("t-leave");
      setTimeout(() => {
        data[bk].splice(i, 1);
        if (!data[bk].length) delete data[bk];
        save();
        render(0);
      }, 170);
      return;
    }

    const sk = keyOf(sel);
    let nowDone = false;
    if (ev.rep) {
      ev.dn = ev.dn || [];
      const x = ev.dn.indexOf(sk);
      if (x >= 0) {
        ev.dn.splice(x, 1);
        nowDone = false;
      } else {
        ev.dn.push(sk);
        nowDone = true;
      }
    } else {
      ev.d = !ev.d;
      nowDone = ev.d;
    }
    save();
    render(0, nowDone ? `${bk}|${i}` : null);
  });

  $("calList").addEventListener("dblclick", (e) => {
    const row = e.target.closest(".trow[data-bk]");
    if (!row || e.target.closest("[data-del]")) return;
    e.stopPropagation();
    const bk = row.dataset.bk;
    const i = parseInt(row.dataset.i, 10);
    const ev = (data[bk] || [])[i];
    if (!ev) return;
    editing = { i, bk };
    showInput().then(() => {
      inp.value = ev.x || "";
      saveBtn?.classList.toggle("has-val", Boolean(inp.value.trim()));
      qs = {
        day: null,
        rep: ev.rep || "",
        rm: ev.rm != null ? ev.rm : remLeadMin,
        tm: ev.t || "",
      };
      renderHelp();
    });
  });

  inp.addEventListener("input", () => {
    saveBtn?.classList.toggle("has-val", Boolean(inp.value.trim()));
    updatePreviewOnly();
  });

  inp.addEventListener("keydown", (e) => {
    if (e.key === "Escape") return hideInput();
    if (e.key === "Enter") commitEvent();
  });

  inp.addEventListener("blur", (e) => {
    if (performance.now() - shownAt < 380) return;
    if (
      e.relatedTarget === saveBtn ||
      (e.relatedTarget && layer.contains(e.relatedTarget))
    )
      return;
    hideInput();
  });

  $("calHelp").addEventListener("mousedown", (e) => {
    if (!e.target.closest(".ch-t-in")) e.preventDefault();
  });

  $("calHelp").addEventListener("click", (e) => {
    e.stopPropagation();
    if (e.target.closest("#calTimeEnable")) {
      const now = new Date();
      const nextH = (now.getHours() + 1) % 24;
      qs.tm = `${pad(nextH)}:00`;
      renderHelp();
      return;
    }
    if (e.target.closest("#calTimeClr")) {
      qs.tm = "";
      renderHelp();
      inp.focus();
      return;
    }
    const step = e.target.closest(".ch-t-btn");
    if (step) {
      stepTime(step.dataset.tp, parseInt(step.dataset.d, 10));
      return;
    }
    const c = e.target.closest(".ch");
    if (!c) return;
    const { g, v } = c.dataset;
    if (g === "rep") qs.rep = v;
    else if (g === "rm") qs.rm = +v;
    renderHelp();
  });

  $("calHelp").addEventListener(
    "wheel",
    (e) => {
      const num = e.target.closest(".ch-t-in");
      if (!num) return;
      e.preventDefault();
      e.stopPropagation();
      stepTime(num.dataset.tp, e.deltaY < 0 ? 1 : -1);
    },
    { passive: false },
  );

  function loadJson(json) {
    if (!json || json === lastJson) return;
    try {
      const parsed = JSON.parse(json);
      if (parsed && typeof parsed === "object") {
        data = parsed;
        lastJson = json;
        render(0);
      }
    } catch {}
  }

  function onConfig(cfg) {
    holRegionPref = cfg.calendar_holiday_region || "auto";
    remEnabled = cfg.calendar_reminder_enabled !== false;
    remLeadMin = cfg.calendar_reminder_lead_min ?? 10;
    remSound = cfg.calendar_reminder_sound !== false;
    if (typeof cfg.calendar_json === "string" && cfg.calendar_json) {
      loadJson(cfg.calendar_json);
    }
    render(0);
  }

  setInterval(checkReminders, 15000);
  setInterval(renderNext, 30000);

  return {
    onConfig,
    loadJson,
    onEnter: () => {
      sel = new Date();
      render(0);
      const s = $("calStrip");
      if (s) {
        s.classList.remove("enter");
        void s.offsetWidth;
        s.classList.add("enter");
      }
    },
    refreshLanguage: () => render(0),
    isInputOpen: () => open,
    hideInput,
  };
}
