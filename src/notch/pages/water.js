import { t } from "../../shared/i18n.js";

function todayKey() {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function createWaterModule({
  router,
  mascot,
  playChime,
  syncWidgetData,
}) {
  let curr = 0;
  let goal = 1500;
  let cup = 250;
  let reminderEnabled = true;
  let intervalMin = 45;
  let lastDate = "";
  let secCounter = 0;

  function updateUI() {
    const pct = Math.min(100, Math.round((curr / Math.max(1, goal)) * 100));
    const totalCups = Math.max(
      1,
      Math.min(10, Math.round(goal / Math.max(1, cup))),
    );
    const drunkCups = Math.floor(curr / Math.max(1, cup));

    document.getElementById("waterCurr").textContent = (curr / 1000).toFixed(2);
    document.getElementById("waterGoalTxt").textContent =
      `/ ${(goal / 1000).toFixed(1)} L`;
    document.getElementById("waterGoalLbl").textContent = t("notch.water_goal");
    document.getElementById("waterSubTxt").textContent = t("notch.water_cups", {
      drunk: drunkCups,
      total: totalCups,
    });
    document.getElementById("drinkBtn").textContent = `${cup} ml`;
    document.getElementById("waterPctBig").textContent = `${pct}%`;
    document.getElementById("waterWave").style.height = `${pct}%`;

    let html = "";
    for (let i = 0; i < totalCups; i++) {
      html += `<span class="cup-dot ${i < drunkCups ? "filled" : ""}"></span>`;
    }
    document.getElementById("cupsRow").innerHTML = html;
  }

  function checkDailyAutoReset() {
    const today = todayKey();
    if (!lastDate) {
      lastDate = today;
      syncWidgetData();
    } else if (lastDate !== today) {
      lastDate = today;
      curr = 0;
      secCounter = 0;
      updateUI();
      syncWidgetData();
    }
  }

  function drink() {
    checkDailyAutoReset();
    curr += cup;
    secCounter = 0;
    playChime("water_tap");
    mascot.setMood("happy", 1500);
    updateUI();
    syncWidgetData();
  }

  function reset() {
    curr = 0;
    lastDate = todayKey();
    updateUI();
    syncWidgetData();
  }

  document.getElementById("drinkWaterBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    drink();
  });
  document.getElementById("resetWaterBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    reset();
  });

  setInterval(() => {
    if (reminderEnabled && intervalMin > 0 && curr < goal) {
      secCounter++;
      if (secCounter >= intervalMin * 60) {
        secCounter = 0;
        const pct = Math.min(100, Math.round((curr / Math.max(1, goal)) * 100));
        playChime("water_remind");
        mascot.setMood("surprised", 2000);
        router.showCustomPeek(
          `<svg viewBox="0 0 24 24" style="width:16px;height:16px;fill:#A2E4F5"><path d="M12 2.8s6 6.5 6 11.2a6 6 0 0 1-12 0c0-4.7 6-11.2 6-11.2z"/></svg>`,
          "Water Reminder",
          t("notch.water_rem_msg", { cup }),
          `${pct}%`,
          "#A2E4F5",
          pct,
        );
      }
    }
  }, 1000);

  function onConfig(cfg) {
    goal = cfg.water_goal_ml || 1500;
    cup = cfg.water_cup_ml || 250;
    curr = cfg.water_current_ml || 0;
    reminderEnabled = cfg.water_reminder_enabled !== false;
    intervalMin = cfg.water_interval_min || 45;
    lastDate = cfg.last_water_date || "";
    checkDailyAutoReset();
    updateUI();
  }

  return {
    onConfig,
    updateUI,
    checkDailyAutoReset,
    getState: () => ({
      waterCurrentMl: curr,
      lastWaterDate: lastDate || todayKey(),
    }),
  };
}
