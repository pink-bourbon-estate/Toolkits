//! formulas.rs - the recipes, built from the averaged research in sources.rs.
//!
//! Every recipe is described in three parts:
//!
//! 1. THE FORMULA   - baker's percentages (flour = 100 %). The key numbers
//!    (hydration, salt, yeast, butter, eggs ...) are NOT typed by hand: they
//!    are the average of the school/bakery formulas in `sources.rs`, rounded
//!    to sensible baking precision.
//! 2. THE PREFERMENT - levain (sourdough), poolish or none. Described by
//!    `pff` (prefermented flour = % of all the flour that goes into the
//!    preferment) and the preferment's own hydration.
//! 3. THE STEPS      - an ordered timeline. Each step has a default duration
//!    (minutes, at the recipe's reference room temperature), a range a baker
//!    might reasonably use, and flags for temperature sensitivity and alarms.
//!
//! To tweak a recipe permanently, this is the file to edit.

use crate::sources::{self, Key};

/// Average of a value across the school sources, rounded to `step`.
/// `fallback` is only used if NO source states the value, so the program
/// still works if someone deletes sources while experimenting.
fn avg(family: &str, key: Key, fallback: f64, step: f64) -> f64 {
    let value = sources::average(family, key).unwrap_or(fallback);
    // round to the nearest `step`, then tidy floating-point dust to 2 decimals
    let rounded = (value / step).round() * step;
    (rounded * 100.0).round() / 100.0
}

/// One stage of the bake, e.g. "Bulk ferment".
#[derive(Clone, Debug)]
pub struct Step {
    pub name: &'static str,
    /// Default duration at the recipe's reference temperature.
    pub minutes: i64,
    /// Shortest / longest sensible duration (used as slider limits).
    pub min_minutes: i64,
    pub max_minutes: i64,
    /// True = yeast/bacteria work here, so room temperature changes the time.
    pub temp_sensitive: bool,
    /// Ring when this step STARTS.
    pub alarm: bool,
    /// Minutes relative to the END of the previous step.
    ///  0 = starts when the previous step ends (normal)
    /// -45 = starts 45 min BEFORE the previous step ends (overlapping jobs, e.g. oven preheat)
    pub start_offset: i64,
    /// Minutes into the step at which to fold the dough.
    pub folds_at: Vec<i64>,
    /// Short instruction shown in the timeline.
    pub how: &'static str,
    /// The sensory cue - always trust the dough over the clock.
    pub done_when: &'static str,
}

impl Step {
    /// Start a step with its name and three durations; chain the methods below for the rest.
    fn new(name: &'static str, minutes: i64, min_minutes: i64, max_minutes: i64) -> Self {
        Step {
            name,
            minutes,
            min_minutes,
            max_minutes,
            temp_sensitive: false,
            alarm: true,
            start_offset: 0,
            folds_at: Vec::new(),
            how: "",
            done_when: "",
        }
    }
    fn sensitive(mut self) -> Self {
        self.temp_sensitive = true;
        self
    }
    fn silent(mut self) -> Self {
        self.alarm = false;
        self
    }
    fn offset(mut self, minutes: i64) -> Self {
        self.start_offset = minutes;
        self
    }
    fn folds(mut self, at: &[i64]) -> Self {
        self.folds_at = at.to_vec();
        self
    }
    fn how(mut self, text: &'static str) -> Self {
        self.how = text;
        self
    }
    fn done(mut self, text: &'static str) -> Self {
        self.done_when = text;
        self
    }
}

/// A complete recipe: formula + preferment + steps.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub key: &'static str,
    pub name: &'static str,
    /// Which sources family the averages come from.
    pub family: &'static str,
    pub description: &'static str,
    /// "loaf", "baguette", "croissant" ...
    pub piece_name: &'static str,
    pub default_pieces: u32,
    /// Grams of RAW dough per piece.
    pub default_piece_weight: f64,
    /// (flour, % of total flour) - must total 100.
    pub flour_blend: Vec<(&'static str, f64)>,
    /// What the hydration slider is called.
    pub liquid_label: &'static str,
    /// (liquid, share of the liquid %) - must total 100.
    pub liquids: Vec<(&'static str, f64)>,
    /// Default liquid % of flour.
    pub hydration: f64,
    /// Slider limits.
    pub hydration_range: (f64, f64),
    pub salt: f64,
    /// % instant yeast (0 for pure sourdough).
    pub yeast_instant: f64,
    /// Other ingredients, % of flour.
    pub extras: Vec<(&'static str, f64)>,
    /// Lamination butter, % of flour (croissant).
    pub rollin_butter: f64,
    /// "levain", "poolish" or None.
    pub preferment: Option<&'static str>,
    /// % of total flour that is prefermented.
    pub pff: f64,
    /// Preferment hydration %.
    pub pref_hydration: f64,
    /// Levain only: ripe starter as % of the levain's flour (1:5:5 = 20 %).
    pub seed_pct: f64,
    /// Desired dough temperature, deg C.
    pub ddt_c: f64,
    /// Room temperature the step durations assume.
    pub ref_temp_c: f64,
    /// Oven / cooking instructions.
    pub bake: &'static str,
    pub steps: Vec<Step>,
    pub notes: &'static str,
}

impl Recipe {
    /// (min, max) liquid % across the school sources - shown as a guide under the slider.
    pub fn school_range(&self) -> Option<(f64, f64)> {
        let key = if matches!(self.family, "brioche" | "croissant") { Key::Milk } else { Key::Hydration };
        sources::spread(self.family, key)
    }
}

/// Defaults shared by all recipes, so each recipe below only states what is different.
fn base() -> Recipe {
    Recipe {
        key: "",
        name: "",
        family: "",
        description: "",
        piece_name: "loaf",
        default_pieces: 1,
        default_piece_weight: 500.0,
        flour_blend: vec![("Strong white bread flour", 100.0)],
        liquid_label: "Hydration (water % of flour)",
        liquids: vec![("Water", 100.0)],
        hydration: 70.0,
        hydration_range: (50.0, 90.0),
        salt: 2.0,
        yeast_instant: 0.0,
        extras: vec![],
        rollin_butter: 0.0,
        preferment: None,
        pff: 0.0,
        pref_hydration: 100.0,
        seed_pct: 20.0,
        ddt_c: 25.0,
        ref_temp_c: 24.0,
        bake: "",
        steps: vec![],
        notes: "",
    }
}

/// Create all recipes. Called once at start-up.
pub fn build_recipes() -> Vec<Recipe> {
    let mut recipes = Vec::new();
    let sd_hydration = avg("sourdough", Key::Hydration, 72.0, 0.5);
    let sd_salt = avg("sourdough", Key::Salt, 2.0, 0.1);
    let sd_pff = avg("sourdough", Key::Pff, 14.0, 0.5);

    // 1. CLASSIC SOURDOUGH ---------------------------------------------------
    recipes.push(Recipe {
        key: "sourdough_classic",
        name: "Classic sourdough",
        family: "sourdough",
        description: "Open-crumb white sourdough with a little wholemeal - the school-average country loaf. About 6 % wetter than a typical home recipe.",
        default_pieces: 2,
        default_piece_weight: 900.0,
        flour_blend: vec![("Strong white bread flour", 90.0), ("Wholemeal flour", 10.0)],
        hydration: sd_hydration,
        hydration_range: (62.0, 85.0),
        salt: sd_salt,
        preferment: Some("levain"),
        pff: sd_pff,
        bake: "Preheat oven + Dutch oven / stone to 250 C. Bake 20 min lid on (or with steam), then 230 C lid off for 20-25 min until deep brown and 96-98 C inside.",
        steps: vec![
            Step::new("Build levain", 360, 240, 600).sensitive()
                .how("Mix ripe starter + flour + water (amounts in the bakesheet). Cover, keep warm.")
                .done("Doubled, domed, bubbly, smells yoghurty-sweet. A spoonful floats."),
            Step::new("Autolyse (flour + most of the water)", 45, 20, 90).offset(-45)
                .how("Mix the flours with the water EXCEPT ~50 g kept back for the salt. Cover.")
                .done("Shaggy dough has hydrated and feels smoother when pulled."),
            Step::new("Mix: add levain, salt, reserved water", 15, 10, 25)
                .how("Pinch and fold until fully combined, then slap-and-fold 2-3 min.")
                .done("Dough holds together and starts to feel elastic."),
            Step::new("Bulk ferment with folds", 240, 180, 360).sensitive().folds(&[30, 60, 90, 120])
                .how("Keep covered and warm (24-26 C). Coil-fold or stretch-and-fold at each alarm.")
                .done("+50-75 % volume, domed edges, bubbles on the sides and top, jiggles."),
            Step::new("Pre-shape + bench rest", 30, 20, 45)
                .how("Tip out, divide (weights in bakesheet), round gently, rest uncovered.")
                .done("Rounds have relaxed and spread slightly."),
            Step::new("Final shape into bannetons", 10, 5, 20)
                .how("Shape tight boules/batards, seam-up into floured bannetons.")
                .done("Surface is taut and smooth."),
            Step::new("Cold retard (fridge 3-5 C)", 720, 480, 960)
                .how("Cover bannetons and refrigerate overnight - this is where flavour builds.")
                .done("Slightly risen; a floured finger-poke springs back slowly."),
            Step::new("Preheat oven + pot/stone", 45, 30, 60).offset(-45)
                .how("Oven to 250 C with Dutch oven or stone inside."),
            Step::new("Score + bake", 45, 40, 55)
                .how("Straight from the fridge: tip out, score, bake (see bake notes).")
                .done("Deep mahogany crust, hollow knock, 96-98 C in the centre."),
            Step::new("Cool before slicing", 120, 60, 180)
                .how("Cool on a rack. The crumb is still setting - don't cut early."),
        ],
        notes: "Wholemeal ferments faster and drinks more water: if you raise the wholemeal, raise the hydration by ~1 % for every 5 % extra wholemeal.",
        ..base()
    });

    // 2. COUNTRY SOURDOUGH ---------------------------------------------------
    recipes.push(Recipe {
        key: "sourdough_country",
        name: "Country sourdough (wholemeal + rye)",
        family: "sourdough",
        description: "Same averaged base, but 25 % wholegrain (Pain au Levain / Vermont style). Hydration +3 points because bran and rye absorb more water.",
        default_pieces: 2,
        default_piece_weight: 900.0,
        flour_blend: vec![("Strong white bread flour", 75.0), ("Wholemeal flour", 15.0), ("Rye flour", 10.0)],
        hydration: sd_hydration + 3.0,
        hydration_range: (62.0, 88.0),
        salt: sd_salt,
        preferment: Some("levain"),
        pff: sd_pff,
        bake: "As classic sourdough. Rye browns faster - drop to 220 C for the lid-off stage.",
        steps: vec![
            Step::new("Build levain", 360, 240, 600).sensitive()
                .how("Mix ripe starter + flour + water. Cover, keep warm.")
                .done("Doubled, domed, bubbly."),
            Step::new("Autolyse (flour + most of the water)", 45, 20, 90).offset(-45)
                .how("Mix flours and water, keeping ~50 g water back for the salt."),
            Step::new("Mix: add levain, salt, reserved water", 15, 10, 25).how("Pinch and fold until combined."),
            Step::new("Bulk ferment with folds", 210, 150, 330).sensitive().folds(&[30, 60, 90])
                .how("Wholegrain doughs move faster - watch from 3 h.")
                .done("+50 % volume, airy, domed."),
            Step::new("Pre-shape + bench rest", 30, 20, 45).how("Divide and round."),
            Step::new("Final shape into bannetons", 10, 5, 20).how("Shape and basket."),
            Step::new("Cold retard (fridge 3-5 C)", 720, 480, 960).how("Overnight."),
            Step::new("Preheat oven + pot/stone", 45, 30, 60).offset(-45).how("250 C."),
            Step::new("Score + bake", 45, 40, 55).done("96-98 C centre."),
            Step::new("Cool before slicing", 120, 60, 180),
        ],
        notes: "The flour blend is a reasoned average of the wholegrain shares in the source formulas, not a copy of any one of them.",
        ..base()
    });

    // 3. BAGUETTE WITH POOLISH -------------------------------------------------
    recipes.push(Recipe {
        key: "baguette",
        name: "Baguette (poolish)",
        family: "baguette",
        description: "Classic French baguette on an overnight poolish - average of CIA, Hamelman, SFBI and two Le Cordon Bleu formulas.",
        piece_name: "baguette",
        default_pieces: 4,
        default_piece_weight: 350.0,
        flour_blend: vec![("Strong white bread flour (or French T65)", 100.0)],
        hydration: avg("baguette", Key::Hydration, 68.0, 0.5),
        hydration_range: (62.0, 78.0),
        salt: avg("baguette", Key::Salt, 2.0, 0.1),
        yeast_instant: avg("baguette", Key::YeastInstant, 0.35, 0.05),
        preferment: Some("poolish"),
        pff: avg("baguette", Key::Pff, 25.0, 0.5),
        bake: "Preheat to 250 C with a stone/steel and a steam tray. Load, add steam, bake at 240 C for 22-25 min. Vent the steam after 10 min.",
        steps: vec![
            Step::new("Mix poolish", 720, 600, 960).sensitive()
                .how("Flour + water + pinch of yeast (bakesheet). Cover at room temp.")
                .done("Bubbly, domed, just starting to dimple/recede in the middle."),
            Step::new("Autolyse", 30, 20, 45).offset(-30).how("Mix remaining flour and water."),
            Step::new("Mix: add poolish, yeast, salt", 10, 8, 15).how("Mix until smooth; moderate gluten development only."),
            Step::new("Bulk ferment with folds", 90, 60, 150).sensitive().folds(&[30, 60])
                .how("Fold gently at each alarm.")
                .done("Nearly doubled, light and airy."),
            Step::new("Divide + pre-shape + rest", 25, 15, 30).how("Divide, pre-shape into loose cylinders, rest covered."),
            Step::new("Shape baguettes", 15, 10, 20).how("Shape and lay seam-up on a floured couche."),
            Step::new("Final proof", 50, 30, 75).sensitive()
                .how("Covered on the couche.")
                .done("Springs back slowly - bake slightly UNDER-proofed for good ears."),
            Step::new("Preheat oven + steam tray", 60, 45, 60).offset(-60).how("250 C."),
            Step::new("Score + bake with steam", 25, 20, 30)
                .how("5 overlapping, shallow, nearly-parallel cuts.")
                .done("Deep golden, crackling, hollow."),
            Step::new("Cool", 30, 20, 45),
        ],
        ..base()
    });

    // 4. FOCACCIA --------------------------------------------------------------
    recipes.push(Recipe {
        key: "focaccia",
        name: "Focaccia",
        family: "focaccia",
        description: "High-hydration olive-oil tray bread - average of the CIA and Bread Ahead formulas (straight dough, no preferment).",
        piece_name: "tray (20x30 cm)",
        default_pieces: 1,
        default_piece_weight: 1000.0,
        hydration: avg("focaccia", Key::Hydration, 75.0, 0.5),
        hydration_range: (65.0, 95.0),
        salt: avg("focaccia", Key::Salt, 2.0, 0.1),
        yeast_instant: avg("focaccia", Key::YeastInstant, 0.45, 0.05),
        extras: vec![("Extra virgin olive oil (dough + tray + top)", avg("focaccia", Key::Oil, 11.0, 0.5))],
        ddt_c: 26.0,
        bake: "Preheat to 230 C (220 C fan). Bake 18-22 min until golden on top AND underneath. Brush with oil as it comes out.",
        steps: vec![
            Step::new("Mix", 10, 5, 15).how("Mix flour, water, yeast and salt to a loose, glossy dough."),
            Step::new("Bulk ferment with folds", 120, 90, 180).sensitive().folds(&[30, 60, 90])
                .how("Oil the bowl. Coil-fold at each alarm.")
                .done("Doubled, very bubbly and wobbly."),
            Step::new("Tray + dimple", 10, 5, 15).how("Oil the tray generously, ease the dough to the corners."),
            Step::new("Proof in tray", 60, 30, 90).sensitive()
                .how("Cover. Dimple deeply with oiled fingers near the end, add toppings.")
                .done("Puffy, bubbles on top, dimples hold."),
            Step::new("Preheat oven", 45, 30, 60).offset(-45).how("230 C."),
            Step::new("Bake", 20, 18, 25).done("Golden top, crisp golden base."),
            Step::new("Cool in tray, then on rack", 15, 10, 20),
        ],
        notes: "For more flavour, refrigerate the dough 8-24 h after the folds (the CIA formula uses a biga for the same reason).",
        ..base()
    });

    // 5. BRIOCHE ---------------------------------------------------------------
    recipes.push(Recipe {
        key: "brioche",
        name: "Brioche loaf",
        family: "brioche",
        description: "Rich, egg-and-butter brioche - average of CIA, King Arthur (pro) and Bouchon Bakery formulas. Overnight cold rest for easy shaping.",
        piece_name: "loaf tin (900 g)",
        default_pieces: 1,
        default_piece_weight: 650.0,
        liquid_label: "Milk % of flour (eggs fixed)",
        liquids: vec![("Whole milk", 100.0)],
        hydration: avg("brioche", Key::Milk, 15.0, 0.5),
        hydration_range: (0.0, 30.0),
        salt: avg("brioche", Key::Salt, 2.4, 0.1),
        yeast_instant: avg("brioche", Key::YeastInstant, 2.0, 0.05),
        extras: vec![
            ("Whole eggs (out of shell)", avg("brioche", Key::Eggs, 47.0, 0.5)),
            ("Caster sugar", avg("brioche", Key::Sugar, 12.0, 0.5)),
            ("Unsalted butter, soft but cool", avg("brioche", Key::Butter, 50.0, 0.5)),
        ],
        ddt_c: 24.0,
        bake: "Egg wash twice. 175 C (160 C fan) for 30-35 min for a tin loaf (12-15 min for small buns) until deep golden and 88-90 C inside.",
        steps: vec![
            Step::new("Mix (butter added gradually)", 25, 20, 35)
                .how("Mix everything except butter to full gluten, then add butter a piece at a time.")
                .done("Silky, pulls from the bowl, passes the windowpane test."),
            Step::new("First rise", 60, 45, 90).sensitive().how("Covered, room temperature, then knock back."),
            Step::new("Cold rest overnight (fridge)", 720, 480, 1080)
                .how("Flatten on a lined tray, wrap, refrigerate.")
                .done("Firm and cold - easy to shape."),
            Step::new("Divide + shape into tin", 20, 15, 30)
                .how("Divide (weights in bakesheet), roll tight balls, line them up in the tin."),
            Step::new("Final proof", 150, 90, 180).sensitive()
                .how("Covered, warm (26-28 C, not hotter or the butter leaks). Egg wash once.")
                .done("Domed ~1 cm above the tin rim, springs back slowly."),
            Step::new("Preheat oven", 40, 30, 50).offset(-40).how("175 C."),
            Step::new("Second egg wash + bake", 35, 30, 40).done("88-90 C in the centre."),
            Step::new("Cool (10 min in tin)", 60, 30, 90),
        ],
        ..base()
    });

    // 6. CROISSANT -------------------------------------------------------------
    recipes.push(Recipe {
        key: "croissant",
        name: "Croissant / pain au chocolat",
        family: "croissant",
        description: "Laminated yeasted dough - average of CIA, Hamelman and Le Cordon Bleu. Two-day process: dough day, then lamination + bake day.",
        piece_name: "croissant",
        default_pieces: 12,
        default_piece_weight: 85.0,
        liquid_label: "Liquid % of flour (milk + water)",
        liquids: vec![("Whole milk", 80.0), ("Water", 20.0)],
        hydration: avg("croissant", Key::Milk, 60.0, 0.5),
        hydration_range: (50.0, 70.0),
        salt: avg("croissant", Key::Salt, 2.2, 0.1),
        yeast_instant: avg("croissant", Key::YeastInstant, 1.0, 0.05),
        extras: vec![
            ("Caster sugar", avg("croissant", Key::Sugar, 11.0, 0.5)),
            ("Unsalted butter (in the dough), soft", avg("croissant", Key::Butter, 6.0, 0.5)),
        ],
        rollin_butter: avg("croissant", Key::RollinButter, 50.0, 0.5),
        ddt_c: 24.0,
        ref_temp_c: 26.0, // croissants are proofed warm - but below 28 C or the butter melts
        bake: "200 C (180 C fan) for 18-22 min after a second egg wash, until deep golden and the layers at the sides look set.",
        steps: vec![
            Step::new("Mix détrempe (dough)", 15, 10, 20)
                .how("Mix everything except roll-in butter to a smooth, not over-developed dough."),
            Step::new("Short rise", 60, 45, 120).sensitive().how("Covered at room temperature."),
            Step::new("Flatten + retard overnight", 720, 480, 960)
                .how("Press into a rectangle, wrap, fridge. Make the butter block now and chill it too.")
                .done("Dough and butter both cold but butter still bendable."),
            Step::new("Lock in butter + fold 1 (letter)", 20, 15, 30)
                .how("Enclose butter, roll to 3x length, letter fold. Keep everything cold."),
            Step::new("Chill", 30, 30, 60).how("Wrapped, fridge."),
            Step::new("Fold 2 (letter)", 15, 10, 20).how("Turn 90 degrees, roll, letter fold."),
            Step::new("Chill", 30, 30, 60).how("Wrapped, fridge."),
            Step::new("Fold 3 (letter)", 15, 10, 20).how("Turn 90 degrees, roll, letter fold."),
            Step::new("Chill (or 30 min freezer + fridge)", 60, 45, 120),
            Step::new("Roll out, cut + shape", 30, 20, 45)
                .how("Roll to ~4-5 mm, cut triangles (or rectangles for pain au chocolat), shape.")
                .done("Clean cuts - visible layers on the edges."),
            Step::new("Proof (24-27 C, humid)", 120, 90, 180).sensitive()
                .how("Covered, warm but NEVER over 28 C. Egg wash once.")
                .done("Nearly doubled, wobble like jelly, layers visible and separating."),
            Step::new("Preheat oven", 40, 30, 50).offset(-40).how("200 C (180 C fan)."),
            Step::new("Second egg wash + bake", 20, 16, 25).done("Deep golden all over."),
            Step::new("Cool", 20, 15, 30),
        ],
        notes: "Use high-fat (82 %+) butter for the roll-in. Pain au chocolat: 2 chocolate batons per piece, same dough and timings.",
        ..base()
    });

    // 7. FRESH EGG PASTA ---------------------------------------------------------
    recipes.push(Recipe {
        key: "pasta_egg",
        name: "Fresh egg pasta",
        family: "pasta_egg",
        description: "Tagliatelle, pappardelle, lasagne, ravioli, tortellini. The Bologna rule: 100 g flour + 1 egg per person.",
        piece_name: "portion (100 g flour)",
        default_pieces: 4,
        default_piece_weight: 158.0,
        flour_blend: vec![("Tipo 00 flour", 100.0)],
        liquid_label: "Egg % of flour (whole eggs, out of shell)",
        liquids: vec![("Whole eggs (out of shell)", 100.0)],
        hydration: avg("pasta_egg", Key::Hydration, 58.0, 0.5),
        hydration_range: (50.0, 66.0),
        salt: 0.0,
        ddt_c: 22.0,
        ref_temp_c: 22.0,
        bake: "Cook in plenty of boiling water salted at 10 g per litre: tagliatelle 2-3 min, pappardelle 3-4 min, filled pasta 3-5 min (until they float + 1 min). Save a mug of pasta water for the sauce.",
        steps: vec![
            Step::new("Mix (well method) + knead", 15, 10, 20)
                .how("Flour in a mound with a well; eggs in the middle; fork in the flour from the inside, then knead with the heel of your hand ~10 min.")
                .done("Smooth and silky, springs back slowly when pressed. Dry? A few drops of water or egg white. Sticky? A little flour."),
            Step::new("Rest, wrapped", 60, 15, 120)
                .how("Clingfilm, room temperature (or up to a day in the fridge, then 1 h out).")
                .done("Relaxed: it doesn't spring back when you start rolling."),
            Step::new("Roll the sheets", 20, 15, 40)
                .how("Machine: widest setting, fold in thirds and re-roll twice, then thinner one notch at a time. Pin: work from the centre outward, turning the sheet.")
                .done("Tagliatelle: you can see your hand through it. Filled pasta: about 1 mm or less."),
            Step::new("Cut / fill + shape", 25, 15, 60)
                .how("Tagliatelle ~6-7 mm, pappardelle ~2 cm. Dust with semola, form loose nests.")
                .done("Strands separate cleanly, no sticking."),
            Step::new("Dry on the board (optional)", 20, 0, 60).silent()
                .how("A short air-dry helps cut shapes hold; skip for filled pasta."),
            Step::new("Boil the water", 15, 10, 20).offset(-15).how("Big pan; salt when boiling."),
            Step::new("Cook + sauce", 5, 2, 8).how("See the cooking times under 'Bake'.")
                .done("Al dente: tender with a slight bite."),
        ],
        notes: "Richer dough (Emilian style): replace some whole egg with yolks - about 2 yolks for each whole egg removed - and keep the slider about the same. For more bite, swap 20-30 % of the 00 for semola. Freeze nests on a tray, then bag; cook from frozen.",
        ..base()
    });

    // 8. SEMOLA + WATER PASTA ------------------------------------------------------
    recipes.push(Recipe {
        key: "pasta_semola",
        name: "Semola pasta (orecchiette, cavatelli)",
        family: "pasta_semola",
        description: "Egg-free southern Italian dough: re-milled durum semola + lukewarm water, shaped by hand.",
        piece_name: "portion (100 g semola)",
        default_pieces: 4,
        default_piece_weight: 150.0,
        flour_blend: vec![("Semola rimacinata (re-milled durum wheat)", 100.0)],
        liquid_label: "Water % of semola",
        liquids: vec![("Lukewarm water (35-40 C)", 100.0)],
        hydration: avg("pasta_semola", Key::Hydration, 50.0, 0.5),
        hydration_range: (44.0, 56.0),
        salt: 0.0,
        ddt_c: 24.0,
        ref_temp_c: 22.0,
        bake: "Boil in water salted at 10 g per litre: fresh orecchiette or cavatelli 4-6 min, dried ones 8-12 min. Classic: with cime di rapa, garlic, chilli and anchovy.",
        steps: vec![
            Step::new("Mix + knead", 15, 10, 20)
                .how("Add the water gradually, mix, then knead 10 min.")
                .done("Firm, smooth and not sticky - stiffer than egg dough."),
            Step::new("Rest, covered", 30, 15, 60).how("Under a bowl or clingfilm so the surface doesn't dry."),
            Step::new("Roll ropes + shape", 45, 30, 90)
                .how("Ropes about 1 cm thick, cut 1-1.5 cm pieces. Orecchiette: drag each with a table knife, then flip over your thumb. Cavatelli: drag with two fingers.")
                .done("Thin centre, thicker rim, rough outside to hold sauce."),
            Step::new("Dry on a semola-dusted tray", 30, 0, 120).silent()
                .how("Turn once. Keeps 2-3 days fully dried, or freeze on the tray."),
            Step::new("Boil the water", 15, 10, 20).offset(-15).how("Big pan; salt when boiling."),
            Step::new("Cook + sauce", 6, 4, 12).done("Tender, still with chew."),
        ],
        notes: "Making a lot? Shaping is the slow step: allow about 10-12 min per portion by hand.",
        ..base()
    });

    recipes
}

/// Look a recipe up by key (None if the key is unknown). Used by the tests.
#[cfg(test)]
pub fn find<'a>(recipes: &'a [Recipe], key: &str) -> Option<&'a Recipe> {
    recipes.iter().find(|r| r.key == key)
}
