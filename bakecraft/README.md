# BakeCraft

A bake planner that works the way a bakery production sheet does. Written in Rust with egui, the same toolkit as ChessCraft and PianoCraft. Local-first: no account, no tracking, and your plans stay on your computer.

- Pick a **recipe**, **how many**, **how big** and **when**, and get **every gram** to weigh, with the levain or poolish build split out.
- Choose **"ready by"** or **"start at"** and get a **clock-time timeline**, adjusted for your kitchen temperature.
- **Alarms** ring at each step and at every stretch-and-fold. Press **Enter** for Done or **Esc** to snooze.
- Put **several bakes on one timeline**. Hands-on jobs from different bakes that land within 10 minutes of each other are highlighted.
- A **water-temperature calculator** helps the dough hit the temperature the timings assume, with a one-off mixer calibration.

Recipes: classic sourdough, country sourdough (wholemeal + rye), poolish baguette, focaccia, brioche, croissant / pain au chocolat, fresh egg pasta and semola pasta (orecchiette, cavatelli).

## Install (Windows)
1. Download `bakecraft.exe` from the newest **BakeCraft v0.1.x** release on the [Releases page](https://github.com/pink-bourbon-estate/toolkits/releases). A Linux build (`bakecraft-linux-x86_64`) is published too.
2. Run it. If SmartScreen warns about an unknown app, click **More info → Run anyway** (it isn't code-signed).
3. Optional: compare the file with the `.sha256` checksum on the release page.

BakeCraft does not install or update itself. To update, download the newest release.

## The look and feel
- **Simple** layout shows the essentials (recipe, how many, when, the weigh-out sheet). **Advanced** adds hydration, yeast type, bowl loss, manual step lengths and the full baker's-percentage formula.
- **Light and dark** themes (or follow the system) with soft cream-on-charcoal colours and a coral accent. Text and background pairs are chosen to keep WCAG AA contrast.
- **Keyboard first:** `Ctrl+K` (Cmd+K on a Mac) opens a command palette; `Ctrl+1`..`4` switch screen; `Ctrl+Enter` adds the bake; `Tab` moves between controls and shows a clear focus ring.
- **Text size** slider and a **Reduce motion** switch in Settings.
- The weigh-out sheet is grouped in the order you work: levain or poolish the day before, then mix day, then lamination, divide and bake.

## Where the recipes come from
Each recipe is the **average of published formulas from professional schools and bakeries**:

| Recipe | Formulas averaged |
|---|---|
| Sourdough | San Francisco Baking Institute, Jeffrey Hamelman / King Arthur (x2), Tartine, Ken's Artisan Bakery (Forkish), Culinary Institute of America |
| Baguette | CIA, Hamelman, SFBI, Le Cordon Bleu (Paris and Adelaide) |
| Focaccia | CIA, Bread Ahead |
| Brioche | CIA, King Arthur (professional formula), Bouchon Bakery |
| Croissant | CIA, Hamelman / King Arthur, Le Cordon Bleu |
| Fresh egg pasta | Pasta Evangelists, Bologna sfoglia tradition (Bologna Welcome) |
| Semola pasta | Italia Squisita (Antonio Guida's orecchiette), Pasta Evangelists |

A home-baking book is shown on the **Sources** screen **for comparison only**. It is not averaged. For sourdough the schools average about **71.5 % hydration**, against about **65.5 %** in that book. Every number, with its source and a link where one exists, is in [`src/sources.rs`](src/sources.rs). Add a source there and the averages update on the next build.

The formulas are factual ratios with their sources cited. Please buy the books; they're excellent.

> The times are estimates. Fermentation follows temperature, flour and starter strength, not the clock. Every step says what the dough should look like when it's ready. When the dough and the timer disagree, go with the dough.

## How the maths works
| Step | Formula |
|---|---|
| Total flour | total dough ÷ (sum of all baker's % ÷ 100) |
| Each ingredient | total flour × its % ÷ 100 |
| Levain flour | total flour × prefermented-flour % |
| Final-dough water | total water − water already in the levain |
| Water temperature | DDT × N − (flour + room [+ levain]) − friction (N = 3, or 4 with a levain) |
| Fermentation step time | default × 2 ^ ((reference temp − room temp) ÷ 9), limited to 0.4×–2.5× |

The hydration slider is the **true overall hydration**, with the levain's water counted, the way SFBI and Hamelman write their formulas.

## Reading the code (for newcomers)
Every file starts with a plain-English explanation, and functions are commented. A good reading order:

| File | What it does |
|---|---|
| `src/sources.rs` | the research data |
| `src/formulas.rs` | recipes and their steps |
| `src/calculator.rs` | baker's percentages → grams |
| `src/schedule.rs` | clock times, temperature adjustment, clashes |
| `src/water.rs` | dough-temperature water calculator |
| `src/settings.rs` | saved choices and plans |
| `src/app.rs` | the window |

Build and test it yourself with [Rust](https://rustup.rs): `cargo test` then `cargo run --release`.

## Your data
Settings, the open plan (`current_plan.json`) and saved plans are stored in your user config folder (Settings → **Open the data folder**). Exports go to `Documents/BakeCraft`. Nothing is sent anywhere. They are never committed to this repository.

## Versions
| Version | Notes |
|---|---|
| 0.1.N | First Rust version, ported from a Python prototype. N is the GitHub Actions run number, shown in the bottom-right corner of the window. |

## Licence
MIT, see `LICENSE-MIT` in the repository root.
