import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";
import {
  createCard,
  createRow,
  createSectionTitle,
  createSegmented,
  createSelect,
  createSwitch,
} from "../ui.js";

const HOLIDAY_COUNTRIES = [
  { value: "ID", label: "Indonesia" },
  { value: "US", label: "United States" },
  { value: "GB", label: "United Kingdom" },
  { value: "CA", label: "Canada" },
  { value: "JP", label: "Japan" },
  { value: "KR", label: "South Korea" },
  { value: "SG", label: "Singapore" },
  { value: "MY", label: "Malaysia" },
  { value: "PH", label: "Philippines" },
  { value: "IN", label: "India" },
  { value: "AU", label: "Australia" },
  { value: "DE", label: "Germany" },
  { value: "FR", label: "France" },
  { value: "BR", label: "Brazil" },
];

function numInput(val, min, max, step, onChange, disabled = false) {
  const inp = document.createElement("input");
  inp.type = "number";
  inp.value = String(val);
  if (min != null) inp.min = String(min);
  if (max != null) inp.max = String(max);
  if (step != null) inp.step = String(step);
  inp.disabled = disabled;
  inp.addEventListener("change", () => {
    const n = parseInt(inp.value, 10);
    if (!Number.isNaN(n)) onChange(n);
  });
  return inp;
}

export function renderProductivityTab(ctx) {
  const { cfg, updateConfig } = ctx;
  const frag = document.createDocumentFragment();

  // 1. POMODORO
  const chips = document.createElement("div");
  chips.className = "preset-chips";
  [15, 25, 45, 60].forEach((m) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "chip" + (cfg.pomo_minutes === m ? " active" : "");
    b.textContent = `${m}m`;
    b.addEventListener("click", () =>
      updateConfig({ pomo_minutes: m }, { rerender: true }),
    );
    chips.appendChild(b);
  });
  frag.appendChild(createSectionTitle(t("sec.pomo"), chips));

  const pomoPair = document.createElement("div");
  pomoPair.className = "num-pair";
  pomoPair.append(
    numInput(cfg.pomo_minutes, 1, 180, 1, (v) =>
      updateConfig({ pomo_minutes: Math.max(1, v) }),
    ),
    numInput(cfg.pomo_break_minutes, 1, 60, 1, (v) =>
      updateConfig({ pomo_break_minutes: Math.max(1, v) }),
    ),
  );

  frag.appendChild(
    createCard([
      createRow(t("prod.pomo_dur"), t("prod.pomo_dur_desc"), pomoPair),
      createRow(
        t("prod.pomo_auto"),
        t("prod.pomo_auto_desc"),
        createSwitch(cfg.pomo_auto_break, (val) =>
          updateConfig({ pomo_auto_break: val }),
        ),
      ),
      createRow(
        t("prod.pomo_snd"),
        t("prod.pomo_snd_desc"),
        createSwitch(cfg.enable_sound, (val) =>
          updateConfig({ enable_sound: val }),
        ),
      ),
    ]),
  );

  // 2. WATER TRACKER
  frag.appendChild(createSectionTitle(t("sec.water")));
  const waterPair = document.createElement("div");
  waterPair.className = "num-pair";
  waterPair.append(
    numInput(cfg.water_goal_ml, 250, 10000, 100, (v) =>
      updateConfig({ water_goal_ml: Math.max(250, v) }),
    ),
    numInput(cfg.water_cup_ml, 50, 2000, 50, (v) =>
      updateConfig({ water_cup_ml: Math.max(50, v) }),
    ),
  );

  frag.appendChild(
    createCard([
      createRow(t("prod.water_goal"), t("prod.water_goal_desc"), waterPair),
      createRow(
        t("prod.water_rem"),
        t("prod.water_rem_desc"),
        createSwitch(cfg.water_reminder_enabled, (val) =>
          updateConfig({ water_reminder_enabled: val }, { rerender: true }),
        ),
      ),
      createRow(
        t("prod.water_int"),
        t("prod.water_int_desc"),
        numInput(
          cfg.water_interval_min,
          1,
          360,
          5,
          (v) => updateConfig({ water_interval_min: Math.max(1, v) }),
          !cfg.water_reminder_enabled,
        ),
      ),
      createRow(
        t("prod.water_tap"),
        t("prod.water_tap_desc"),
        createSwitch(cfg.water_tap_sound, (val) =>
          updateConfig({ water_tap_sound: val }),
        ),
      ),
      createRow(
        t("prod.water_rsnd"),
        t("prod.water_rsnd_desc"),
        createSwitch(cfg.water_reminder_sound, (val) =>
          updateConfig({ water_reminder_sound: val }),
        ),
      ),
    ]),
  );

  // 3. KALENDER & PENGINGAT (dipindah dari Maskot & Tema)
  frag.appendChild(createSectionTitle(t("sec.calendar")));
  const holOpts = [
    { value: "auto", label: t("cal.hol_auto") },
    ...HOLIDAY_COUNTRIES,
    { value: "none", label: t("cal.hol_none") },
  ];
  const leadOpts = [
    { value: "0", label: t("cal.lead_0") },
    { value: "5", label: t("cal.lead_5") },
    { value: "10", label: t("cal.lead_10") },
    { value: "15", label: t("cal.lead_15") },
    { value: "30", label: t("cal.lead_30") },
    { value: "60", label: t("cal.lead_60") },
  ];

  frag.appendChild(
    createCard([
      createRow(
        t("cal.holidays"),
        t("cal.holidays_desc"),
        createSelect(holOpts, cfg.calendar_holiday_region || "auto", (val) =>
          updateConfig({ calendar_holiday_region: val }),
        ),
      ),
      createRow(
        t("cal.rem_on"),
        t("cal.rem_on_desc"),
        createSwitch(cfg.calendar_reminder_enabled !== false, (val) =>
          updateConfig({ calendar_reminder_enabled: val }, { rerender: true }),
        ),
      ),
      createRow(
        t("cal.rem_lead"),
        t("cal.rem_lead_desc"),
        createSelect(
          leadOpts,
          String(cfg.calendar_reminder_lead_min ?? 10),
          (val) =>
            updateConfig({ calendar_reminder_lead_min: parseInt(val, 10) }),
          cfg.calendar_reminder_enabled === false,
        ),
      ),
      createRow(
        t("cal.rem_snd"),
        t("cal.rem_snd_desc"),
        createSwitch(cfg.calendar_reminder_sound !== false, (val) =>
          updateConfig({ calendar_reminder_sound: val }),
        ),
      ),
    ]),
  );

  // 4. TO-DO LIST
  frag.appendChild(createSectionTitle(t("sec.todo")));
  const todoBar = document.createElement("div");
  todoBar.className = "todo-bar";
  const todoInput = document.createElement("input");
  todoInput.type = "text";
  todoInput.className = "ctrl-input";
  todoInput.placeholder = t("todo.placeholder");

  const addBtn = document.createElement("button");
  addBtn.type = "button";
  addBtn.className = "g-btn accent";
  addBtn.innerHTML = `<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg><span>${escHtml(t("todo.add"))}</span>`;

  const commitTodo = () => {
    const txt = todoInput.value.trim();
    if (!txt) return;
    const next = [...(cfg.todos || []), { text: txt, done: false }];
    todoInput.value = "";
    updateConfig({ todos: next }, { rerender: true });
  };
  addBtn.addEventListener("click", commitTodo);
  todoInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") commitTodo();
  });
  todoBar.append(todoInput, addBtn);

  const listBox = document.createElement("div");
  listBox.className = "todo-items";
  const todos = cfg.todos || [];
  if (!todos.length) {
    listBox.innerHTML = `<div style="font-size:11.5px;color:var(--mist);padding:6px 0">${escHtml(t("todo.empty"))}</div>`;
  } else {
    todos.forEach((item, idx) => {
      const row = document.createElement("div");
      row.className = "t-item" + (item.done ? " done" : "");
      row.innerHTML = `
        <div class="t-left">
          <span class="custom-chk"><svg viewBox="0 0 10 10"><path d="M2 5l2.2 2.2L8 2.5"/></svg></span>
          <span class="t-txt">${escHtml(item.text)}</span>
        </div>
        <button type="button" class="icon-del" title="${escHtml(t("todo.delete"))}">
          <svg viewBox="0 0 24 24"><path d="M3 6h18M8 6V4h8v2M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>
        </button>
      `;
      row.querySelector(".t-left").addEventListener("click", () => {
        const next = todos.map((x, i) =>
          i === idx ? { ...x, done: !x.done } : x,
        );
        updateConfig({ todos: next }, { rerender: true });
      });
      row.querySelector(".icon-del").addEventListener("click", () => {
        const next = todos.filter((_, i) => i !== idx);
        updateConfig({ todos: next }, { rerender: true });
      });
      listBox.appendChild(row);
    });
  }

  const todoCard = createCard([
    createRow(
      t("todo.layout"),
      t("todo.layout_desc"),
      createSegmented(
        [
          { value: "horizontal", label: t("todo.horiz") },
          { value: "vertical", label: t("todo.vert") },
        ],
        cfg.todo_style,
        (val) => updateConfig({ todo_style: val }),
      ),
    ),
  ]);
  todoCard.style.paddingBottom = "12px";
  todoCard.append(todoBar, listBox);
  frag.appendChild(todoCard);

  return frag;
}
