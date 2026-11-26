const MOODS = ["happy", "surprised", "party", "sleepy"];

export function createMascotModule(router) {
  let style = "neko";
  let currentMood = "";
  let moodTimer = null;
  let lastActivity = performance.now();

  const stage = () =>
    document.querySelector("#nekoStage, #mochiStage, #kitsuneStage");

  function render(nextStyle = style) {
    style = nextStyle === "visor" ? "mochi" : nextStyle || "neko";
    const box = document.getElementById("mascotBox");
    if (!box) return;

    if (style === "mochi") {
      box.innerHTML = `
        <div class="mochi-stage" id="mochiStage">
          <div class="mochi-body">
            <div class="mochi-face">
              <i class="mochi-eye l"></i><i class="mochi-eye r"></i>
              <i class="mochi-blush l"></i><i class="mochi-blush r"></i>
              <i class="mochi-mouth"></i>
            </div>
          </div>
        </div>`;
    } else if (style === "kitsune") {
      box.innerHTML = `<div class="kitsune-wrap" id="kitsuneStage"><i class="kit-eye"></i><i class="kit-eye"></i></div>`;
    } else {
      box.innerHTML = `
        <div class="neko-stage" id="nekoStage">
          <span class="neko-ear left"></span>
          <div class="neko-eye-wrap"><i class="neko-eye left"></i><i class="neko-eye right"></i></div>
          <span class="neko-ear right"></span>
        </div>`;
    }
    if (currentMood) stage()?.classList.add(currentMood);
  }

  function setMood(mood, ms) {
    clearTimeout(moodTimer);
    moodTimer = null;
    currentMood = mood || "";
    const st = stage();
    if (st) {
      st.classList.remove(...MOODS);
      if (mood) st.classList.add(mood);
    }
    if (mood && ms) moodTimer = setTimeout(() => setMood(""), ms);
  }

  function bumpActivity() {
    lastActivity = performance.now();
    if (currentMood === "sleepy") setMood("");
  }

  function setAudioBeat(peak) {
    const mochiBody = document.querySelector("#mochiStage .mochi-body");
    if (mochiBody)
      mochiBody.style.setProperty("--beat", Math.min(1, peak * 1.6).toFixed(2));
  }

  setInterval(() => {
    if (router.state.isPlaying && !currentMood && Math.random() < 0.55) {
      setMood("happy", 1800);
    }
  }, 5400);

  setInterval(() => {
    if (
      router.state.bootPhase === "done" &&
      !router.state.isPlaying &&
      !router.state.isHovered &&
      !currentMood &&
      !router.state.pomoRunning &&
      performance.now() - lastActivity > 120000
    ) {
      setMood("sleepy");
    }
  }, 10000);

  render("neko");
  return { render, setMood, bumpActivity, setAudioBeat };
}
