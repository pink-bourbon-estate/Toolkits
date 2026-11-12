"use strict";
// --- measurement fields (inches internally; shown in the chosen unit): key -> [label, default, optional] ---
const M = {
  chest: ["Chest (around, arms up)", 35.25], neck: ["Neck (around the base)", 16.25], shoulder: ["Across shoulders (bone to bone)", 19.75],
  bicep: ["Bicep (widest)", 11.625], wrist: ["Wrist", 7.5], cb_to_waist: ["Centre-back neck to waist", 15.25],
  sleeve_to_wrist: ["Shoulder point to wrist", 24.0], short_sleeve: ["Short-sleeve length", 8.5],
  waist: ["Waist (where the waistband sits)", 34.0], seat: ["Seat (fullest part, around)", 41.0], rise: ["Body rise (seated, seat to waistband top)", 10.5],
  inseam: ["Inseam (floor to crotch)", 32.0], knee_height: ["Floor to middle of knee", 20.0],
  hem_width: ["Leg opening around (0 = fit default)", 0, true], crotch_length: ["Crotch length: back waist, under, to front waist (0 = fit default)", 0, true],
};
const TOP = ["chest", "neck", "shoulder", "bicep", "wrist", "cb_to_waist", "sleeve_to_wrist", "short_sleeve"];
const TROUSERS = ["waist", "seat", "rise", "inseam", "knee_height", "hem_width", "crotch_length"];
const PRESETS = {
  demo: { chest: 35.25, neck: 16.25, shoulder: 19.75, bicep: 11.625, wrist: 7.5, cb_to_waist: 15.25, sleeve_to_wrist: 24.0, short_sleeve: 8.5, waist: 34.0, seat: 41.0, rise: 10.5, inseam: 32.0, knee_height: 20.0, hem_width: 0, crotch_length: 0 },
  corn: { chest: 37.5, neck: 15.0, shoulder: 17.0, bicep: 10.5, wrist: 6.5, cb_to_waist: 19.0, sleeve_to_wrist: 24.0, short_sleeve: 8.0, waist: 33.5, seat: 40.5, rise: 10.0, inseam: 33.0, knee_height: 20.0, hem_width: 14.5, crotch_length: 28.0 },
};
// --- common complaints: [param, label, hint, min, max, step] (inches) ---
const ALTS_TOP = [
  ["armhole_drop_adj", "Tight or pulling under the arms", "lowers the underarm", -1.5, 2.5, 0.125],
  ["chest_ease_adj", "Too tight (+) or too loose (−) across the torso", "changes the finished chest", -6, 6, 0.25],
  ["shoulder_slope_adj", "Folds at the neck (+) / pull at the back shoulder (−)", "steeper or flatter shoulder line", -1, 1, 0.125],
  ["shoulder_seam_shift", "Shoulder seam too far back (+) or forward (−)", "moves the seam, wedge front↔back", -1, 1, 0.125],
  ["sleeve_cap_adj", "Drag lines in the sleeve: taller cap (+)", "sleeve cap height", -1.5, 1.5, 0.125],
  ["sleeve_shift", "Sleeve drags to the front (+) or back (−)", "moves the sleeve centre line", -1, 1, 0.125],
  ["neck_adj", "Neck hole too tight (+) or loose (−)", "total neck circumference", -2, 3, 0.25],
  ["balance", "Hem rides up at the back / hangs back (+)", "lengthens back, shortens front", -2, 2, 0.125],
  ["length_adj", "Body too short (+) or long (−)", "", -6, 8, 0.25],
  ["sleeve_length_adj", "Sleeve too short (+) or long (−)", "", -3, 4, 0.25],
  ["waist_taper", "Too boxy: take in at the waist (each side seam)", "per quarter", 0, 2, 0.125],
  ["hem_flare", "Add flare at the hem (each side seam)", "per quarter", 0, 2, 0.125],
];
const ALTS_TR = [
  ["crotch_ease_adj", "V-shaped drag lines under the seat (+ more easing) / saggy seat (−)", "the back-crotch easing pivot", -2, 3, 0.125],
  ["seat_adj", "Too tight (+) or too loose (−) at the seat", "total seat ease", -4, 8, 0.25],
  ["thigh_adj", "Too tight (+) or too loose (−) in the thighs", "total, moves outside and inside seams", -4, 6, 0.25],
  ["rise_adj", "Waistband sits too low (+) or too high (−)", "rise", -3, 3, 0.125],
  ["length_adj", "Too short (+) or long (−)", "inseam", -6, 8, 0.25],
];
const ALTS_KNIT = [
  ["chest_ease_adj", "Too tight (+) or too loose (−) across the torso", "changes the finished chest", -8, 8, 0.5],
  ["armhole_drop_adj", "Sleeves too tight at the top (+ deeper armhole)", "armhole depth, sets the sleeve-top width", -2, 3, 0.25],
  ["neck_adj", "Neck hole too tight (+) or loose (−)", "total neck circumference", -2, 3, 0.25],
  ["length_adj", "Body too short (+) or long (−)", "", -6, 8, 0.25],
  ["sleeve_length_adj", "Sleeve too short (+) or long (−)", "", -3, 4, 0.25],
];
const GARMENTS = {
  tshirt: { meas: TOP, alts: ALTS_TOP, sleeve: true }, shirt: { meas: TOP, alts: ALTS_TOP, sleeve: true }, hoodie: { meas: TOP, alts: ALTS_TOP, sleeve: false },
  trousers: { meas: TROUSERS, alts: ALTS_TR, trousers: true }, knit: { meas: TOP.filter((k) => k !== "short_sleeve" && k !== "bicep"), alts: ALTS_KNIT, knit: true },
};
const $ = (id) => document.getElementById(id);
let state = { ...PRESETS.demo, gauge_sts: 20, gauge_rows: 28 }; let alts = {}; let cm = false;
const G = () => GARMENTS[$("garment").value]; const ALTS = () => G().alts;
const toDisp = (v) => cm ? v * 2.54 : v; const fromDisp = (v) => cm ? v / 2.54 : v;
const r2 = (v) => Math.round(v * 100) / 100;

function build() {
  for (const id of ["garment", "fit", "sleeve", "length", "page", "rise_style"]) $(id).addEventListener("change", () => { if (id === "garment") layout(); schedule(); });
  for (const id of ["show_construction", "show_ruler", "show_sew_line"]) $(id).addEventListener("change", schedule);
  $("units").addEventListener("change", () => { cm = $("units").value === "cm"; fillMeas(); for (const [k] of ALTS()) showAlt(k); $("unitnote").textContent = cm ? "(centimetres)" : "(inches)"; schedule(); });
  $("p-demo").onclick = () => { Object.assign(state, PRESETS.demo); fillMeas(); schedule(); };
  $("p-corn").onclick = () => { Object.assign(state, PRESETS.corn); fillMeas(); schedule(); };
  $("reset-alts").onclick = () => { for (const [k] of ALTS()) { alts[k] = 0; $("a_" + k).value = 0; showAlt(k); } schedule(); };
  document.querySelectorAll(".tabs button").forEach((b) => b.onclick = () => {
    document.querySelectorAll(".tabs button").forEach((x) => x.classList.toggle("on", x === b));
    document.querySelectorAll(".pane").forEach((p) => p.classList.toggle("on", p.id === "pane-" + b.dataset.t));
  });
  $("print").onclick = () => window.print();
  $("dl").onclick = download;
  for (const k of ["gauge_sts", "gauge_rows"]) $(k).addEventListener("input", () => { state[k] = parseFloat($(k).value) || 0; schedule(); });
  layout();
}
// rebuild the measurement fields and sliders for the chosen garment
function layout() {
  const g = G();
  $("meas").innerHTML = g.meas.map((k) => `<label>${M[k][0]}<input type="number" step="0.05" min="0" id="m_${k}"></label>`).join("");
  $("alts").innerHTML = g.alts.map(([k, l, h, mn, mx, st]) => `<div class="sl"><b>${l}</b><span class="val" id="v_${k}">0</span><input type="range" id="a_${k}" min="${mn}" max="${mx}" step="${st}" value="${alts[k] || 0}"><small class="note">${h}</small></div>`).join("");
  for (const k of g.meas) $("m_" + k).addEventListener("input", () => { state[k] = fromDisp(parseFloat($("m_" + k).value) || 0); schedule(); });
  for (const [k] of g.alts) $("a_" + k).addEventListener("input", () => { alts[k] = parseFloat($("a_" + k).value); showAlt(k); schedule(); });
  $("row-sleeve").style.display = g.sleeve ? "" : "none"; $("row-rise").style.display = g.trousers ? "" : "none";
  $("row-length").style.display = g.trousers ? "none" : ""; $("fs-gauge").style.display = g.knit ? "" : "none";
  $("tab-instr").style.display = g.knit ? "" : "none";
  if (!g.knit && $("pane-instr").classList.contains("on")) document.querySelector('.tabs button[data-t="sheet"]').click();
  fillMeas(); for (const [k] of g.alts) showAlt(k);
}
function fillMeas() { for (const k of G().meas) $("m_" + k).value = r2(toDisp(state[k] || 0)); $("gauge_sts").value = state.gauge_sts; $("gauge_rows").value = state.gauge_rows; }
function showAlt(k) { const v = alts[k] || 0; $("v_" + k).textContent = (v > 0 ? "+" : "") + (cm ? r2(v * 2.54) + " cm" : r2(v) + " in"); }

function params() {
  const p = { ...state, ...alts };
  for (const id of ["garment", "fit", "sleeve", "length", "page", "rise_style"]) p[id] = $(id).value;
  p.units = $("units").value;
  for (const id of ["show_construction", "show_ruler", "show_sew_line"]) p[id] = $(id).checked;
  for (const k of ["hem_width", "crotch_length"]) if (!(p[k] > 0)) p[k] = null;
  // only the sliders of this garment apply; stale values for other garments are dropped
  const keep = new Set(G().alts.map((a) => a[0]));
  for (const [k] of [...ALTS_TOP, ...ALTS_TR, ...ALTS_KNIT]) if (!keep.has(k)) p[k] = 0;
  return p;
}

// --- backends: WebAssembly if patternkit.wasm sits next to this page, else the local server's /api/render ---
let wasm = null;
async function initBackend() {
  try {
    const r = await fetch("patternkit.wasm");
    if (r.ok && (r.headers.get("content-type") || "").includes("wasm")) {
      const { instance } = await WebAssembly.instantiate(await r.arrayBuffer(), {});
      wasm = instance.exports; $("sheetinfo").textContent = "(engine: WebAssembly, runs in your browser)";
    }
  } catch (e) { /* fall through to the API */ }
  if (!wasm) $("sheetinfo").textContent = "(engine: local server)";
}
async function render(p) {
  const json = JSON.stringify(p);
  if (wasm) {
    const bytes = new TextEncoder().encode(json);
    const ptr = wasm.pk_alloc(bytes.length);
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
    const res = wasm.pk_render(ptr, bytes.length);
    const op = Number(res >> 32n), ol = Number(res & 0xffffffffn);
    const out = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, op, ol).slice());
    wasm.pk_free(ptr, bytes.length); wasm.pk_free(op, ol);
    return JSON.parse(out);
  }
  const r = await fetch("/api/render", { method: "POST", body: json });
  return r.json();
}

let timer = null, last = null;
function schedule() { clearTimeout(timer); timer = setTimeout(draw, 120); }
async function draw() {
  const p = params();
  location.hash = encodeURIComponent(JSON.stringify({ ...state, ...alts, garment: p.garment, fit: p.fit, sleeve: p.sleeve, length: p.length, units: p.units, rise_style: p.rise_style }));
  let res;
  try { res = await render(p); } catch (e) { $("err").textContent = "Could not run the pattern engine: " + e; return; }
  if (!res.ok) { $("err").textContent = res.error; return; }
  $("err").textContent = ""; last = res;
  const W = res.sheet_w, H = res.sheet_h;
  $("sheetdef").innerHTML = res.sheet;
  $("sheetwrap").innerHTML = `<svg viewBox="0 0 ${W} ${H}" style="width:${Math.max(900, W * 28)}px;height:auto;background:#fff"><rect width="${W}" height="${H}" fill="#fff"/>${res.sheet}</svg>`;
  $("sheetinfo").textContent += "";
  const pw = res.paper_w, ph = res.paper_h;
  $("pages").innerHTML = res.pages.map((g) => `<div class="pgbox"><div class="pg"><svg style="--pw:${pw}in;--ph:${ph}in" viewBox="${g.x - 0.25} ${g.y - 0.25} ${pw} ${ph}" xmlns="http://www.w3.org/2000/svg"><svg x="${g.x}" y="${g.y}" width="${g.w}" height="${g.h}" viewBox="${g.x} ${g.y} ${g.w} ${g.h}" overflow="hidden"><use href="#sheetdef"/></svg>${g.marks}</svg></div><div class="cap note">row ${g.row}, column ${g.col}</div></div>`).join("");
  $("pageinfo").textContent = `${res.pages.length} pages (${res.pages.at(-1).rows} rows × ${res.pages.at(-1).cols} columns of the grid; blank ones skipped)`;
  $("warnings").innerHTML = res.warnings.map((w) => `<div class="warn">⚠ ${w}</div>`).join("");
  $("checks").innerHTML = res.checks.map((c) => `<div class="check ${c.ok ? "ok" : "bad"}"><i>${c.ok ? "✓" : "✗"}</i><span><b>${c.label}</b> — ${c.value}</span></div>`).join("");
  $("explain").innerHTML = res.explain.map((e) => `<li>${e}</li>`).join("");
  $("instr").innerHTML = (res.instructions || []).map((e) => `<li>${e}</li>`).join("");
  let style = document.getElementById("pagesize"); if (!style) { style = document.createElement("style"); style.id = "pagesize"; document.head.appendChild(style); }
  style.textContent = `@page { size: ${pw}in ${ph}in; margin: 0 }`;
}
function download() {
  if (!last) return;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${last.sheet_w}in" height="${last.sheet_h}in" viewBox="0 0 ${last.sheet_w} ${last.sheet_h}"><rect width="100%" height="100%" fill="white"/>${last.sheet}</svg>`;
  const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" })); a.download = "pattern-full-size.svg"; a.click();
}

(async function () {
  build();
  try {
    const h = location.hash.slice(1);
    if (h) {
      const o = JSON.parse(decodeURIComponent(h));
      for (const id of ["garment", "fit", "sleeve", "length", "units", "rise_style"]) if (o[id]) $(id).value = o[id];
      for (const k of Object.keys(M)) if (k in o) state[k] = o[k];
      for (const k of ["gauge_sts", "gauge_rows"]) if (k in o) state[k] = o[k];
      for (const [k] of [...ALTS_TOP, ...ALTS_TR, ...ALTS_KNIT]) if (k in o) alts[k] = o[k];
      cm = $("units").value === "cm"; layout();
    }
  } catch (e) {}
  await initBackend(); draw();
})();
