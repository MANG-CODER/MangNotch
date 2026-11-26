import { t } from "../../shared/i18n.js";
import { fmtTime } from "./media.js";

const THUNDER_FIRST_MS = [10000, 20000];
const THUNDER_GAP_MS = [30000, 90000];
const THUNDER_VOLUME = 0.7;
const WIND_LEVEL = 0.12;

const RAIN_WORKLET_SRC = `
class RainProc extends AudioWorkletProcessor {
  constructor(opts) {
    super();
    const o = opts.processorOptions;
    this.tickP = o.tickRate / sampleRate;
    this.roofP = o.roofRate / sampleRate;
    this.bed = o.bedLevel;
    this.windLevel = o.windLevel;
    this.aHi = 1 - Math.exp(-2 * Math.PI * 6500 / sampleRate);
    this.aLo = 1 - Math.exp(-2 * Math.PI * 1500 / sampleRate);
    this.hiL = 0; this.hiR = 0; this.loL = 0; this.loR = 0;
    this.wA1 = 1 - Math.exp(-2 * Math.PI * 400 / sampleRate);
    this.wA2 = 1 - Math.exp(-2 * Math.PI * 90 / sampleRate);
    this.wSm = 1 - Math.exp(-1 / (1.8 * sampleRate));
    this.wa = [0, 0]; this.wb = [0, 0]; this.wl = [0, 0];
    this.wCur = 0.3; this.wTarget = 0.3; this.wCount = 0;
    this.drops = [];
    this.alive = true;
    this.port.onmessage = (e) => { if (e.data === 'stop') this.alive = false; };
  }
  spawn(roof) {
    if (this.drops.length > 48) return;
    let freq, decMs, q, amp, click;
    if (roof) {
      freq = 300 + Math.random() * 700;
      decMs = 8 + Math.random() * 14;
      q = 0.4 + Math.random() * 0.3;
      amp = 0.05 + Math.random() * 0.10;
      click = 0.8;
    } else {
      freq = 2500 + Math.random() * 4500;
      decMs = 1.5 + Math.random() * 3.5;
      q = 1.0 + Math.random() * 0.9;
      amp = 0.02 + Math.random() * 0.07;
      click = 0;
    }
    const pan = Math.random() * 1.8 - 0.9;
    this.drops.push({
      env: 1,
      decay: Math.exp(-1 / (decMs * 0.001 * sampleRate)),
      f: Math.min(0.9, 2 * Math.sin(Math.PI * freq / sampleRate)),
      q, click, lp: 0, bp: 0,
      gL: amp * (1 - pan) * 0.5,
      gR: amp * (1 + pan) * 0.5
    });
  }
  process(inputs, outputs) {
    if (!this.alive) return false;
    const L = outputs[0][0], R = outputs[0][1];
    const drops = this.drops;
    for (let i = 0; i < L.length; i++) {
      if (Math.random() < this.tickP) this.spawn(false);
      if (Math.random() < this.roofP) this.spawn(true);
      const wL = Math.random() * 2 - 1, wR = Math.random() * 2 - 1;
      this.hiL += (wL - this.hiL) * this.aHi; this.loL += (wL - this.loL) * this.aLo;
      this.hiR += (wR - this.hiR) * this.aHi; this.loR += (wR - this.loR) * this.aLo;
      let sL = (this.hiL - this.loL) * this.bed * 3;
      let sR = (this.hiR - this.loR) * this.bed * 3;
      if (this.windLevel > 0) {
        if (--this.wCount <= 0) {
          this.wTarget = 0.15 + Math.pow(Math.random(), 2) * 0.85;
          this.wCount = (3 + Math.random() * 5) * sampleRate;
        }
        this.wCur += (this.wTarget - this.wCur) * this.wSm;
        const gain = this.windLevel * this.wCur * 6;
        for (let c = 0; c < 2; c++) {
          const x = Math.random() * 2 - 1;
          this.wa[c] += (x - this.wa[c]) * this.wA1;
          this.wb[c] += (this.wa[c] - this.wb[c]) * this.wA1;
          this.wl[c] += (x - this.wl[c]) * this.wA2;
          const w = (this.wb[c] - this.wl[c]) * gain;
          if (c === 0) sL += w; else sR += w;
        }
      }
      const white = Math.random() * 2 - 1;
      for (let d = drops.length - 1; d >= 0; d--) {
        const dr = drops[d];
        dr.env *= dr.decay;
        if (dr.env < 0.003) { drops.splice(d, 1); continue; }
        const ex = (white + (dr.env > 0.85 ? dr.click * 2 : 0)) * dr.env;
        dr.lp += dr.f * dr.bp;
        const hp = ex - dr.lp - dr.q * dr.bp;
        dr.bp += dr.f * hp;
        const sig = dr.bp * dr.q;
        sL += sig * dr.gL;
        sR += sig * dr.gR;
      }
      L[i] = Math.tanh(sL);
      R[i] = Math.tanh(sR);
    }
    return true;
  }
}
registerProcessor('rain-proc', RainProc);
`;

function makeRainIR(ctx, seconds = 1.0, curve = 3.0) {
  const len = Math.floor(ctx.sampleRate * seconds);
  const ir = ctx.createBuffer(2, len, ctx.sampleRate);
  for (let c = 0; c < 2; c++) {
    const d = ir.getChannelData(c);
    for (let i = 0; i < len; i++)
      d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / len, curve);
  }
  return ir;
}

function makeThunderBuffer(ctx, d) {
  const sr = ctx.sampleRate;
  const dur = 5 + d * 6;
  const len = Math.floor(sr * dur);
  const buf = ctx.createBuffer(2, len, sr);
  const env = new Float32Array(len);
  const bumps = 6 + Math.floor(Math.random() * 5);
  for (let k = 0; k < bumps; k++) {
    const t0 = k === 0 ? 0 : Math.random() * dur * 0.65;
    const amp = k === 0 ? 1 : 0.2 + Math.random() * 0.5;
    const atk = k === 0 ? 0.02 + d * 0.25 : 0.05 + Math.random() * 0.2;
    const dec = 0.6 + Math.random() * 1.4 + d * 0.8;
    const i0 = Math.floor(t0 * sr);
    const n1 = Math.min(len, i0 + Math.floor((atk + dec * 5) * sr));
    for (let n = i0; n < n1; n++) {
      const tm = (n - i0) / sr;
      env[n] += amp * (tm < atk ? tm / atk : Math.exp(-(tm - atk) / dec));
    }
  }
  for (let c = 0; c < 2; c++) {
    const out = buf.getChannelData(c);
    let b = 0,
      y1 = 0,
      y2 = 0,
      peak = 0;
    for (let n = 0; n < len; n++) {
      const p = n / len;
      b = (b + 0.02 * (Math.random() * 2 - 1)) / 1.02;
      const fc = 70 + (900 - d * 600) * Math.exp(-2.5 * p);
      const a = 1 - Math.exp((-2 * Math.PI * fc) / sr);
      y1 += (b - y1) * a;
      y2 += (y1 - y2) * a;
      const v = y2 * env[n] * Math.pow(1 - p, 1.5);
      out[n] = v;
      const av = Math.abs(v);
      if (av > peak) peak = av;
    }
    const norm = peak > 0 ? 1 / peak : 1;
    for (let n = 0; n < len; n++) out[n] *= norm;
  }
  return buf;
}

const rand = (a, b) => a + Math.random() * (b - a);

export function createPomoModule({ router, mascot, playChime, getAudioCtx }) {
  let defaultMin = 25;
  let breakMin = 5;
  let totalSec = 25 * 60;
  let remSec = 25 * 60;
  let currentMode = "focus";
  let autoBreak = false;
  let ambientVolPct = 55;
  let ambientPlaying = false;
  let ambientStop = null;
  let activeMasterGain = null;
  let workletReady = false;

  function updateRing() {
    document.getElementById("pomoTime").textContent = fmtTime(remSec);
    const frac = totalSec > 0 ? 1 - remSec / totalSec : 0;
    document.getElementById("pomoRingArc").style.strokeDashoffset = (
      201 * frac
    ).toFixed(1);
  }

  function refreshLanguage() {
    const lbl = document.getElementById("pomoLabel");
    if (lbl)
      lbl.textContent =
        currentMode === "break"
          ? t("notch.pomo_short_break")
          : t("notch.pomo_focus_session");
    const brTxt = document.getElementById("pomoBreakTxt");
    if (brTxt) brTxt.textContent = t("notch.pomo_break");
    const fcTxt = document.getElementById("pomoFocusTxt");
    if (fcTxt) fcTxt.textContent = t("notch.pomo_focus");
    const ambTxt = document.getElementById("ambientTxt");
    if (ambTxt)
      ambTxt.textContent = ambientPlaying
        ? t("notch.rain_on")
        : t("notch.rain");
  }

  function setMode(mins, mode) {
    router.state.pomoRunning = false;
    currentMode = mode;
    totalSec = mins * 60;
    remSec = totalSec;
    document.getElementById("pomoPlaySvg").innerHTML =
      `<path d="M8 5v14l11-7z"/>`;
    refreshLanguage();
    updateRing();
  }

  function togglePomo() {
    router.state.pomoRunning = !router.state.pomoRunning;
    document.getElementById("pomoPlaySvg").innerHTML = router.state.pomoRunning
      ? `<path d="M7 5h4v14H7zM13 5h4v14h-4z"/>`
      : `<path d="M8 5v14l11-7z"/>`;
  }

  async function toggleRain(e) {
    if (e) e.stopPropagation();
    const audioCtx = getAudioCtx();
    if (!audioCtx) return;
    if (audioCtx.state === "suspended") await audioCtx.resume();

    if (ambientPlaying) {
      if (ambientStop) {
        ambientStop();
        ambientStop = null;
      }
      activeMasterGain = null;
      ambientPlaying = false;
    } else {
      if (!workletReady) {
        const url = URL.createObjectURL(
          new Blob([RAIN_WORKLET_SRC], { type: "application/javascript" }),
        );
        await audioCtx.audioWorklet.addModule(url);
        URL.revokeObjectURL(url);
        workletReady = true;
      }

      const VOLUME = Math.max(0.1, (ambientVolPct || 55) / 100);
      const now = audioCtx.currentTime;
      const master = audioCtx.createGain();
      master.gain.setValueAtTime(0.0001, now);
      master.gain.exponentialRampToValueAtTime(VOLUME, now + 1.0);
      master.connect(audioCtx.destination);
      activeMasterGain = master;

      const node = new AudioWorkletNode(audioCtx, "rain-proc", {
        numberOfInputs: 0,
        outputChannelCount: [2],
        processorOptions: {
          tickRate: 50,
          roofRate: 6,
          bedLevel: 0.005,
          windLevel: WIND_LEVEL,
        },
      });

      const hp = audioCtx.createBiquadFilter();
      hp.type = "highpass";
      hp.frequency.value = 70;
      hp.Q.value = 0.7;
      const lp = audioCtx.createBiquadFilter();
      lp.type = "lowpass";
      lp.frequency.value = 9000;
      lp.Q.value = 0.5;

      const dry = audioCtx.createGain();
      dry.gain.value = 0.85;
      const wet = audioCtx.createGain();
      wet.gain.value = 0.28;
      const verb = audioCtx.createConvolver();
      verb.buffer = makeRainIR(audioCtx);

      node.connect(hp);
      hp.connect(lp);
      lp.connect(dry);
      dry.connect(master);
      lp.connect(verb);
      verb.connect(wet);
      wet.connect(master);

      const timers = [];
      const liveOscs = new Set();
      let running = true;

      const later = (fn, ms) => timers.push(setTimeout(fn, ms));
      function loop(fn, [a, b], first) {
        later(
          () => {
            if (!running) return;
            fn();
            loop(fn, [a, b]);
          },
          first ?? rand(a, b),
        );
      }

      function fireThunder() {
        if (!running) return;
        const d = 0.5 + Math.random() * 0.5;
        const buf = makeThunderBuffer(audioCtx, d);
        const delayMs = 300 + d * 4500;
        later(() => {
          if (!running) return;
          const src = audioCtx.createBufferSource();
          src.buffer = buf;
          const g = audioCtx.createGain();
          g.gain.value = THUNDER_VOLUME * (1 - d * 0.55);
          src.connect(g);
          g.connect(master);
          g.connect(verb);
          liveOscs.add(src);
          src.onended = () => {
            liveOscs.delete(src);
            try {
              g.disconnect();
            } catch {}
          };
          src.start();
        }, delayMs);
      }
      loop(fireThunder, THUNDER_GAP_MS, rand(...THUNDER_FIRST_MS));

      ambientStop = () => {
        running = false;
        timers.forEach(clearTimeout);
        liveOscs.forEach((s) => {
          try {
            s.stop();
          } catch {}
        });
        try {
          const tm = audioCtx.currentTime;
          master.gain.setValueAtTime(Math.max(master.gain.value, 0.0001), tm);
          master.gain.exponentialRampToValueAtTime(0.0001, tm + 0.5);
          setTimeout(() => {
            try {
              node.port.postMessage("stop");
              node.disconnect();
              hp.disconnect();
              lp.disconnect();
              dry.disconnect();
              wet.disconnect();
              verb.disconnect();
              master.disconnect();
            } catch {}
          }, 550);
        } catch {}
      };
      ambientPlaying = true;
    }

    const btn = document.getElementById("ambientBtn");
    if (btn) btn.classList.toggle("on", ambientPlaying);
    refreshLanguage();
  }

  document.getElementById("pomoToggleBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    togglePomo();
  });
  document.getElementById("pomoBreakBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    setMode(breakMin, "break");
  });
  document.getElementById("pomoFocusBtn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    setMode(defaultMin, "focus");
  });
  document.getElementById("ambientBtn")?.addEventListener("click", toggleRain);

  setInterval(() => {
    if (router.state.pomoRunning && remSec > 0) {
      remSec--;
      updateRing();
      if (remSec === 0) {
        router.state.pomoRunning = false;
        document.getElementById("pomoPlaySvg").innerHTML =
          `<path d="M8 5v14l11-7z"/>`;
        playChime("pomo");
        mascot.setMood("party", 4200);
        router.showCustomPeek(
          `<svg viewBox="0 0 24 24" style="width:16px;height:16px;stroke:var(--c1);fill:none;stroke-width:2"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/></svg>`,
          t("notch.pomo_done_t"),
          t("notch.pomo_done_s"),
          "DONE",
          "#FFB7C5",
          100,
        );
        if (autoBreak && currentMode === "focus") {
          setMode(breakMin, "break");
          togglePomo();
        }
      }
    }
  }, 1000);

  function onConfig(cfg) {
    const prevDef = defaultMin;
    const prevBr = breakMin;
    defaultMin = cfg.pomo_minutes || 25;
    breakMin = cfg.pomo_break_minutes || 5;
    autoBreak = !!cfg.pomo_auto_break;
    ambientVolPct = cfg.ambient_volume || 55;
    if (activeMasterGain && ambientPlaying) {
      try {
        activeMasterGain.gain.value = Math.max(0.1, ambientVolPct / 100);
      } catch {}
    }
    if (
      !router.state.pomoRunning ||
      prevDef !== defaultMin ||
      prevBr !== breakMin
    ) {
      setMode(defaultMin, "focus");
    } else {
      refreshLanguage();
    }
  }

  return {
    onConfig,
    refreshLanguage,
    getLiveState: () => ({ remSec, totalSec }),
  };
}
