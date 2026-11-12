# patternkit

Pick a garment, a fit and your measurements; get a print-ready, tiled pattern with the construction drawn on it.
The pattern is **generated**, not traced: every line is computed from your measurements.

Status: **T-shirt only** (short or long sleeve, fitted / regular / baggy). Shirt, trousers, joggers and hoodie are the planned next garments.

## Run it

```
cargo run --release -- serve            # http://127.0.0.1:8080  (the page calls the engine over a tiny local server)
cargo run --release -- tee --fit=baggy --chest=35.25 --out=out     # SVG sheet + A4 pages + report, no browser needed
cargo test --release
```

For a static site (GitHub Pages) the same engine is compiled to WebAssembly by `.github/workflows/pages.yml`
(`cargo build --release --lib --target wasm32-unknown-unknown`); `web/app.js` uses `patternkit.wasm` when it sits next to the page.

## How a pattern is designed

* Method: the numbered-points block from Cornelius Quiring's public video *How to Draft a Shirt Pattern* (half pieces on the fold, 1/2 in seam
  allowance in the cut line). The code is original; the video's steps and numbers are the reference (the test `reproduces_the_tutorial_example`
  checks the video's 37.5 in chest example).
* Fit: `src/params.rs::fit_table` opens the block up. `fitted` is the tutorial's own ease (+2 in chest). `baggy` adds 12 in chest, a 1¾ in lower
  armhole, a dropped shoulder and a flatter sleeve cap. Change the table to change what "baggy" means.
* Common complaints: each slider in the page is one fix from the videos (tight under the arms, drag lines, shoulder slope, balance...) as a number.
* Every pattern also reports self-checks: front/back side seams and shoulder seams equal, sleeve cap length versus the armhole it sews into, neck opening versus neck.
* The sheet is cut into pages with a 1/2 in overlap, registration crosses and a 1 in test square on page one.

## Not done / not verified

* Never sewn up: the geometry passes its own checks, but the first real test garment is the real test.
* The WebAssembly build is written but was not compiled where this was developed (no wasm target available there); CI builds it.
* Body mapping: "across shoulders" is the bone-to-bone width from a tailor's measurement sheet; the shirt block's own "half back" is derived from it.
* Seam allowances are fixed at 1/2 in, hems at 1 in.
