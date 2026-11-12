"use strict";
// --- measurement fields (inches internally; shown in the chosen unit) ---
const MEAS = [
  ["chest", "Chest (around, arms up)", 35.25], ["neck", "Neck (around the base)", 16.25], ["shoulder", "Across shoulders (bone to bone)", 19.75],
  ["bicep", "Bicep (widest)", 11.625], ["wrist", "Wrist", 7.5], ["cb_to_waist", "Centre-back neck to waist", 15.25],
  ["sleeve_to_wrist", "Shoulder point to wrist", 24.0], ["short_sleeve", "Short-sleeve length", 8.5],
];
const PRESETS = {
  demo: { chest: 35.25, neck: 16.25, shoulder: 19.75, bicep: 11.625, wrist: 7.5, cb_to_waist: 15.25, sleeve_to_wrist: 24.0, short_sleeve: 8.5 },
  corn: { chest: 37.5, neck: 15.0, shoulder: 17.0, bicep: 10.5, wrist: 6.5, cb_to_waist: 19.0, sleeve_to_wrist: 24.0, short_sleeve: 8.0 },
};
// --- common complaints: [param, label, hint, min, max, step] (inches) ---
const ALTS = [
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
const $ = (id) => document.getElementById(id);
let state = { ...PRESETS.demo }; let alts = {}; let cm = false;
const toDisp = (v) => cm ? v * 2.54 : v; const fromDisp = (v) => cm ? v / 2.54 : v;
const r2 = (v) => Math.round(v * 100) / 100;

function build() {
  $("meas").innerHTML = MEAS.map(([k, l]) => `<label>${l}<input type="number" step="0.05" min="1" id="m_${k}"></label>`).join("");
  $("alts").innerHTML = ALTS.map(([k, l, h, mn, mx, st]) => `<div class="sl"><b>${l}</b><span class="val" id="v_${k}">0</span><input type="range" id="a_${k}" min="${mn}" max="${mx}" step="${st}" value="0"><small class="note">${h}</small></div>`).join("");
  for (const [k] of MEAS) $("m_" + k).addEventListener("input", () => { state[k] = fromDisp(parseFloat($("m_" + k).value) || 0); schedule(); });
  for (const [k, , , , , st] of ALTS) $("a_" + k).addEventListener("input", () => { alts[k] = parseFloat($("a_" + k).value); showAlt(k); schedule(); });
  for (const id of ["garment", "fit", "sleeve", "length", "page"]) $(id).addEventListener("change", schedule);
  for (const id of ["show_construction", "show_ruler", "show_sew_line"]) $(id).addEventListener("change", schedule);
  $("units").addEventListener("change", () => { cm = $("units").value === "cm"; fillMeas(); for (const [k] of ALTS) showAlt(k); $("unitnote").textContent = cm ? "(centimetres)" : "(inches)"; schedule(); });
  $("p-demo").onclick = () => { state = { ...PRESETS.demo }; fillMeas(); schedule(); };
  $("p-corn").onclick = () => { state = { ...PRESETS.corn }; fillMeas(); schedule(); };
  $("reset-alts").onclick = () => { for (const [k] of ALTS) { alts[k] = 0; $("a_" + k).value = 0; showAlt(k); } schedule(); };
  document.querySelectorAll(".tabs button").forEach((b) => b.onclick = () => {
    document.querySelectorAll(".tabs button").forEach((x) => x.classList.toggle("on", x === b));
    document.querySelectorAll(".pane").forEach((p) => p.classList.toggle("on", p.id === "pane-" + b.dataset.t));
  });
  $("print").onclick = () => window.print();
  $("dl").onclick = download;
  fillMeas(); for (const [k] of ALTS) showAlt(k);
}
function fillMeas() { for (const [k] of MEAS) $("m_" + k).value = r2(toDisp(state[k])); }
function showAlt(k) { const v = alts[k] || 0; $("v_" + k).textContent = (v > 0 ? "+" : "") + (cm ? r2(v * 2.54) + " cm" : r2(v) + " in"); }

function params() {
  const p = { ...state, ...alts };
  for (const id of ["garment", "fit", "sleeve", "length", "page"]) p[id] = $(id).value;
  p.units = $("units").value;
  for (const id of ["show_construction", "show_ruler", "show_sew_line"]) p[id] = $(id).checked;
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
  location.hash = encodeURIComponent(JSON.stringify({ ...state, ...alts, fit: p.fit, sleeve: p.sleeve, length: p.length, units: p.units }));
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
  try { const h = location.hash.slice(1); if (h) { const o = JSON.parse(decodeURIComponent(h)); for (const [k] of MEAS) if (k in o) state[k] = o[k]; for (const [k] of ALTS) if (k in o) { alts[k] = o[k]; $("a_" + k).value = o[k]; showAlt(k); } for (const id of ["fit", "sleeve", "length", "units"]) if (o[id]) $(id).value = o[id]; cm = $("units").value === "cm"; fillMeas(); } } catch (e) {}
  await initBackend(); draw();
})();
