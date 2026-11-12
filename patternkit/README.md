# patternkit

Pick a garment, a fit and your measurements; get a print-ready, tiled pattern with the construction drawn on it.
The pattern is **generated**, not traced: every line is computed from your measurements.

Garments (each in fitted / regular / baggy):

| Garment | What you get |
|---|---|
| T-shirt | front, back, sleeve (short or long), neck band |
| Shirt | the tee block with a cut-on button band, collar stand, collar, and cuffs for long sleeves |
| Hoodie | the tee block with a hood, rib cuffs and waistband, and a kangaroo pocket |
| Trousers | front, back (with dart and easing) and waistband; high-waisted or standard rise |
| Knitted pullover | drop-shoulder crew neck: schematic with stitch counts, plus written instructions from your gauge |

Planned: joggers, blazer, women's clothing.

## Run it

```
cargo run --release -- serve            # http://127.0.0.1:8080  (the page calls the engine over a tiny local server)
cargo run --release -- make --garment=trousers --fit=baggy --waist=34 --seat=41 --out=out   # SVG sheet + A4 pages + report, no browser needed
                                                                                 # garments: tshirt shirt hoodie trousers knit (`tee` is an alias of `make`)
cargo test --release
```

For a static site (GitHub Pages) the same engine is compiled to WebAssembly by `.github/workflows/pages.yml`
(`cargo build --release --lib --target wasm32-unknown-unknown`); `web/app.js` uses `patternkit.wasm` when it sits next to the page.

## How a pattern is designed

* Trousers: the numbered-points block from his men's pant video: four panels, the seat line a quarter of the rise above the crotch line, the front
  crotch line at seat/8 - 1/2 in, widths of 1/4 seat + 1 in (front) and + 1 1/4 in (back), 1/4 waist + 1 in / + 1 3/4 in at the waist, the back dart
  (1 in wide, 4 1/2 in long), outside and inside seams equalised, then the back crotch seam "eased" by pivoting the upper back about the outside seat
  point. The fit table adds seat ease and a wider leg; "high-waisted" adds 1 1/2 in of rise and a 2 1/2 in waistband.
* Shirt and hoodie reuse the tee block with their own neck sizes and extra pieces; the hood height and width are proportional to your neck (no head
  measurement is taken, so check the hood on a test garment).
* Knitting: finished sizes come from the same measurements and fit presets; your gauge (stitches and rows per 4 in) converts them to counts, with
  stitch totals checked (shoulders + neck = body, shaping fits the available rows). Shapes are simple and flat (drop shoulder, round neck, rib).
* Method (tops): the numbered-points block from Cornelius Quiring's public video *How to Draft a Shirt Pattern* (half pieces on the fold, 1/2 in seam
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
* Seam allowances are fixed at 1/2 in, hems at 1 in (trousers 1 1/2 in).
* The trouser back crotch curve is shorter than the one in the video (about 10 in against 11 3/4 in), so if you enter your own crotch length the
  tool asks for more easing than the video's example; leave it blank to use the fit's default (2 1/2 to 3 in) and adjust after a test pair.
* Shirt: no pleats, yoke or curved hem; button holes are not marked. Hoodie: no drawstring channel. Knitting: no swatch-size sanity check beyond the
  gauge range, and the neck and sleeve shaping are the simplest that work.
* Joggers, blazer and women's clothing are not covered yet.

## Credits and inspiration

PatternKit is original code, but the ideas and techniques behind it come from people who share their craft publicly. Thank you to:

* **Cornelius Quiring**: the drafting method (tops and trousers) (numbered points, half pieces on the fold, sleeve cap matched to the armhole) and the "common complaints" alterations come from his pattern-drafting and fitting videos.
* **Firmé Atelier**: how to take body measurements for bespoke work (natural-waist cord, straight-line shoulder and back widths, snug circumferences), from their measurement video.
* **Glory Allan**: the loose, true-to-size retail fit that inspired the proportions of the "baggy" setting; her tee's published ease and armhole depth were used as a sanity check, not copied.
* **Bernadette Banner** ([YouTube](https://www.youtube.com/@bernadettebanner)): general inspiration. Her channel covers costume analysis and fashion history, which encourages treating a garment as something you can take apart and explain.

None of them have endorsed or reviewed this project; any mistakes in how their ideas are applied are mine.
