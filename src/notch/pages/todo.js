import { t } from "../../shared/i18n.js";
import { escHtml } from "../../shared/theme.js";
import { measureTextPx } from "./media.js";

const { invoke } = window.__TAURI__.core;

export function createTodoModule({ syncWidgetData }) {
  const listEl = document.getElementById("todoList");
  const countEl = document.getElementById("todoCount");
  const addBtn = document.getElementById("addTodoBtn");
  const inputEl = document.getElementById("todoInlineInput");

  let saveBtn = document.getElementById("todoSaveBtn");
  if (!saveBtn && inputEl?.parentElement) {
    saveBtn = document.createElement("span");
    saveBtn.id = "todoSaveBtn";
    saveBtn.className = "a inline-save-btn";
    saveBtn.title = "Save (Enter)";
    saveBtn.innerHTML = `<svg viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5"/></svg>`;
    inputEl.parentElement.appendChild(saveBtn);
  }

  let todos = [];
  let todoStyle = "horizontal";
  let inputOpen = false;
  let shownAt = 0;
  let prevStateMap = null;
  let lastSig = "";

  const reduceMotion = window.matchMedia?.(
    "(prefers-reduced-motion: reduce)",
  )?.matches;

  function todoKeys() {
    const seen = {};
    return todos.map((item) => {
      const n = (seen[item.text] = (seen[item.text] || 0) + 1);
      return `${item.text}|${n}`;
    });
  }

  function playWave() {
    if (reduceMotion) return;
    listEl.querySelectorAll(".trow").forEach((r, i) => {
      r.style.setProperty("--k", String(Math.min(i, 8)));
      r.classList.remove("t-wave");
      void r.offsetWidth;
      r.classList.add("t-wave");
    });
  }

  function render(force = false, explicitDoneIdx = -1) {
    const done = todos.filter((x) => x.done).length;
    const cntTxt = `${done}/${todos.length}`;
    if (countEl.textContent !== cntTxt) {
      countEl.textContent = cntTxt;
      if (prevStateMap && !reduceMotion) {
        countEl.classList.remove("cnt-pop");
        void countEl.offsetWidth;
        countEl.classList.add("cnt-pop");
      }
    }

    const sig =
      JSON.stringify(todos.map((x) => [x.text, !!x.done])) + todoStyle;
    if (!force && sig === lastSig && listEl.children.length) return;
    lastSig = sig;

    listEl.classList.toggle("horiz", todoStyle === "horizontal");

    const oldRects = new Map();
    if (prevStateMap && !reduceMotion) {
      listEl.querySelectorAll(".trow[data-k]").forEach((el) => {
        const r = el.getBoundingClientRect();
        if (r.width) oldRects.set(el.dataset.k, r);
      });
    }
    const sTop = listEl.scrollTop;
    const sLeft = listEl.scrollLeft;

    if (!todos.length) {
      listEl.innerHTML = `<div class="dim" style="font-size:11px;margin:auto 0;text-align:center;grid-column:1/-1">${escHtml(t("notch.todo_empty"))}</div>`;
      prevStateMap = new Map();
      return;
    }

    const keys = todoKeys();
    const hadPrev = !!prevStateMap;

    listEl.innerHTML = todos
      .map((item, i) => {
        const prev = prevStateMap?.get(keys[i]);
        let cls = item.done ? "done" : "";
        if (explicitDoneIdx === i && item.done) {
          cls += " t-done";
        } else if (hadPrev) {
          if (!prev) cls += " t-new";
          else if (prev.done !== item.done && item.done) cls += " t-done";
        }
        return `
        <div class="trow ${cls}" data-idx="${i}" data-k="${escHtml(keys[i])}">
          <div class="trow-left">
            <span class="chk"><svg viewBox="0 0 10 10"><path d="M2 5l2.2 2.2L8 2.5"/></svg></span>
            <span class="trow-txt mq-box js-todo-txt">${escHtml(item.text)}</span>
          </div>
          <span class="tdel" data-del="${i}"><svg viewBox="0 0 10 10"><path d="M2 2l6 6M8 2L2 8"/></svg></span>
        </div>`;
      })
      .join("");

    prevStateMap = new Map(keys.map((k, i) => [k, { done: !!todos[i].done }]));
    listEl.scrollTop = sTop;
    listEl.scrollLeft = sLeft;

    if (!reduceMotion && oldRects.size) {
      listEl.querySelectorAll(".trow[data-k]").forEach((el) => {
        const o = oldRects.get(el.dataset.k);
        if (!o) return;
        const n = el.getBoundingClientRect();
        const dx = o.left - n.left;
        const dy = o.top - n.top;
        if (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5) {
          el.animate([{ translate: `${dx}px ${dy}px` }, { translate: "0 0" }], {
            duration: 320,
            easing: "cubic-bezier(.22, 1, .36, 1)",
          });
        }
      });
    }

    listEl.querySelectorAll(".js-todo-txt").forEach((el, idx) => {
      const raw = todos[idx]?.text || "";
      const w = measureTextPx(raw, "400 11.5px Segoe UI");
      if (w > 132) {
        const dur = `${Math.max(5.5, (w + 32) / 26).toFixed(1)}s`;
        el.classList.add("is-marquee");
        el.innerHTML = `<span class="mq-track" style="--mq-dur:${dur}"><span>${escHtml(raw)}</span><span class="mq-gap">•</span><span>${escHtml(raw)}</span><span class="mq-gap">•</span></span>`;
      }
    });
  }

  function commitTodo() {
    if (!inputEl.value.trim()) {
      inputEl.classList.remove("mm-shake");
      void inputEl.offsetWidth;
      inputEl.classList.add("mm-shake");
      inputEl.focus();
      return;
    }
    todos.push({ text: inputEl.value.trim(), done: false });
    render(true);
    syncWidgetData();
    hideInput();
  }

  async function showInput() {
    addBtn.style.display = "none";
    inputEl.style.display = "block";
    if (saveBtn) saveBtn.style.display = "flex";
    inputEl.value = "";
    inputOpen = true;
    shownAt = performance.now();
    try {
      await invoke("set_notch_text_input", { active: true });
    } catch {}
    setTimeout(() => {
      if (inputOpen) inputEl.focus();
    }, 45);
    saveBtn?.classList.remove("has-val");
  }

  function hideInput() {
    if (!inputOpen) return;
    inputOpen = false;
    inputEl.style.display = "none";
    if (saveBtn) saveBtn.style.display = "none";
    addBtn.style.display = "flex";
    invoke("set_notch_text_input", { active: false }).catch(() => {});
  }

  addBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    showInput();
  });
  saveBtn?.addEventListener("mousedown", (e) => e.preventDefault());
  saveBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    commitTodo();
  });

  inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter") commitTodo();
    else if (e.key === "Escape") hideInput();
  });

  inputEl.addEventListener("input", () => {
    saveBtn?.classList.toggle("has-val", Boolean(inputEl.value.trim()));
  });

  inputEl.addEventListener("blur", (e) => {
    if (performance.now() - shownAt < 380) return;
    if (e.relatedTarget === saveBtn) return;
    hideInput();
  });

  listEl.addEventListener("click", (e) => {
    const del = e.target.closest(".tdel[data-del]");
    if (del) {
      e.stopPropagation();
      const idx = parseInt(del.dataset.del, 10);
      const row = del.closest(".trow");
      const removeNow = () => {
        todos.splice(idx, 1);
        render(true);
        syncWidgetData();
      };
      if (row && !reduceMotion) {
        row.classList.add("t-leave");
        setTimeout(removeNow, 180);
      } else {
        removeNow();
      }
      return;
    }

    const row = e.target.closest(".trow[data-idx]");
    if (!row) return;
    e.stopPropagation();
    const idx = parseInt(row.dataset.idx, 10);
    if (todos[idx]) {
      todos[idx].done = !todos[idx].done;
      render(true, todos[idx].done ? idx : -1);
      syncWidgetData();
    }
  });

  function refreshLanguage() {
    document.getElementById("todoModeSub").textContent = t("notch.todo_done");
    document.getElementById("addTodoTxt").textContent = t("notch.todo_add");
    inputEl.placeholder = t("notch.todo_ph");
    if (!todos.length) render(true);
  }

  function onConfig(cfg) {
    todoStyle = cfg.todo_style || "horizontal";
    todos = (cfg.todos || []).map((x) => ({ ...x }));
    render(false);
  }

  return {
    onConfig,
    onEnter: playWave,
    refreshLanguage,
    isInputOpen: () => inputOpen,
    hideInput,
    getItems: () => todos,
    getStyle: () => todoStyle,
  };
}
