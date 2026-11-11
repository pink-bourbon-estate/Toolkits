//! calculator.rs - turns a recipe + "how many, how big, how wet" into grams.
//!
//! THE CORE IDEA (baker's maths in four lines)
//! 1. Add up every overall baker's percentage:  flour 100 + water 72 + salt 2 ... = 174 %
//! 2. Total dough you want:                     2 loaves x 900 g = 1800 g
//! 3. Total flour = dough / (sum of % / 100):   1800 / 1.74 = 1034.5 g
//! 4. Every ingredient = total flour x its % / 100:  water = 1034.5 x 0.72 = 744.8 g
//!
//! THE PREFERMENT TWIST
//! The levain (or poolish) contains part of the flour and water. We work out
//! the OVERALL totals first (so the hydration on the slider is the TRUE
//! hydration of the whole dough), then subtract what is already inside the
//! preferment to get the "final dough" amounts you weigh on mix day.
//!
//! ```text
//! levain flour = total flour x pff / 100
//! levain water = levain flour x levain hydration / 100
//! final water  = total water - levain water
//! final flour  = total flour - levain flour
//! ```
//!
//! The levain is itself built from a spoonful of ripe starter (the "seed"),
//! whose own flour and water (100 % hydration) are part of the levain:
//! `seed = levain flour x seed% / 100`, `build flour = levain flour - seed/2`,
//! `build water = levain water - seed/2`.

use crate::formulas::Recipe;

/// The yeast types we can convert between. Formulas are stored as INSTANT yeast.
pub const YEAST_TYPES: [(&str, f64); 3] = [("instant", 1.0), ("active dried", 1.25), ("fresh", 3.0)];

/// How many grams of this yeast type equal 1 g of instant yeast.
pub fn yeast_factor(kind: &str) -> Option<f64> {
    YEAST_TYPES.iter().find(|(n, _)| *n == kind).map(|(_, f)| *f)
}

/// A poolish only needs a tiny amount of yeast because it ferments 12+ hours.
const POOLISH_YEAST_PCT: f64 = 0.1; // % of the poolish's own flour (instant yeast)

/// A UK large egg, out of the shell, in grams.
const EGG_G: f64 = 58.0;

/// One row on the bakesheet.
#[derive(Clone, Debug)]
pub struct Line {
    pub name: String,
    pub grams: f64,
    /// Baker's % of total flour; None for sub-totals.
    pub pct: Option<f64>,
    pub note: String,
}

impl Line {
    fn new(name: impl Into<String>, grams: f64) -> Self {
        Line { name: name.into(), grams, pct: None, note: String::new() }
    }
    fn pct(mut self, pct: f64) -> Self {
        self.pct = Some(pct);
        self
    }
    fn note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }
    /// True for the "= Levain total" style summary rows.
    pub fn is_total(&self) -> bool {
        self.name.starts_with('=')
    }
}

/// Everything you need to weigh out, grouped the way you actually work.
#[derive(Clone, Debug)]
pub struct Bakesheet {
    pub recipe: Recipe,
    pub pieces: u32,
    pub piece_weight: f64,
    pub hydration: f64,
    pub total_flour: f64,
    pub total_dough: f64,
    /// The full formula (for checking).
    pub overall: Vec<Line>,
    /// What to mix the night before.
    pub preferment: Vec<Line>,
    /// What to weigh on mix day.
    pub final_dough: Vec<Line>,
    /// Butter block (croissant only).
    pub lamination: Vec<Line>,
    /// How to divide.
    pub divide: Vec<Line>,
}

/// Round to `dp` decimal places.
fn round_dp(x: f64, dp: i32) -> f64 {
    let m = 10f64.powi(dp);
    (x * m).round() / m
}

/// Round grams to what a kitchen scale can weigh:
/// 1 decimal place for small amounts (yeast, salt), whole grams otherwise.
pub fn round_g(grams: f64) -> f64 {
    if grams.abs() < 20.0 {
        round_dp(grams, 1)
    } else {
        grams.round()
    }
}

/// "about 4 large eggs" beside a whole-egg line, so you know how many to crack.
fn egg_note(name: &str, grams: f64) -> String {
    if !name.to_lowercase().contains("egg") {
        return String::new();
    }
    let n = grams / EGG_G;
    if (n - n.round()).abs() > 0.2 {
        format!("about {n:.1} large eggs")
    } else {
        format!("about {} large eggs", n.round() as i64)
    }
}

/// The settings the user chooses (everything except the recipe itself).
#[derive(Clone, Debug)]
pub struct Inputs {
    pub pieces: u32,
    pub piece_weight: f64,
    pub hydration: f64,
    pub yeast_type: String,
    /// Extra dough for what sticks to the bowl and bench (1 % is typical).
    pub waste_pct: f64,
}

impl Inputs {
    /// The recipe's own defaults.
    pub fn defaults(recipe: &Recipe) -> Self {
        Inputs {
            pieces: recipe.default_pieces,
            piece_weight: recipe.default_piece_weight,
            hydration: recipe.hydration,
            yeast_type: "instant".to_string(),
            waste_pct: 1.0,
        }
    }
}

/// Build a full bakesheet. Returns a friendly error message for bad input.
pub fn calculate(recipe: &Recipe, input: &Inputs) -> Result<Bakesheet, String> {
    if input.pieces < 1 {
        return Err("You need at least one piece.".into());
    }
    if input.piece_weight <= 0.0 {
        return Err("Piece weight must be more than 0 g.".into());
    }
    let ycf = yeast_factor(&input.yeast_type).ok_or("Unknown yeast type.")?;
    let hydration = input.hydration;

    // ---- 1. Add up every overall percentage (flour is always 100) ----------
    let mut total_pct = 100.0 + hydration + recipe.salt + recipe.yeast_instant + recipe.rollin_butter;
    total_pct += recipe.extras.iter().map(|(_, p)| p).sum::<f64>();

    // ---- 2. Total dough and total flour ----------------------------------
    let total_dough = input.pieces as f64 * input.piece_weight * (1.0 + input.waste_pct / 100.0);
    let total_flour = total_dough / (total_pct / 100.0);
    let grams = |pct: f64| total_flour * pct / 100.0;

    let mut sheet = Bakesheet {
        recipe: recipe.clone(),
        pieces: input.pieces,
        piece_weight: input.piece_weight,
        hydration,
        total_flour,
        total_dough,
        overall: vec![],
        preferment: vec![],
        final_dough: vec![],
        lamination: vec![],
        divide: vec![],
    };

    // ---- 3. OVERALL formula (what the whole dough contains) -----------------
    for (flour, share) in &recipe.flour_blend {
        sheet.overall.push(Line::new(*flour, round_g(grams(*share))).pct(*share));
    }
    for (liquid, share) in &recipe.liquids {
        let pct = hydration * share / 100.0;
        sheet.overall.push(Line::new(*liquid, round_g(grams(pct))).pct(round_dp(pct, 1)).note(egg_note(liquid, grams(pct))));
    }
    if recipe.salt > 0.0 {
        // pasta dough has none - the salt goes in the cooking water
        sheet.overall.push(Line::new("Salt", round_g(grams(recipe.salt))).pct(recipe.salt));
    }
    if recipe.yeast_instant > 0.0 {
        let y_pct = recipe.yeast_instant * ycf;
        sheet.overall.push(Line::new(format!("Yeast ({})", input.yeast_type), round_g(grams(y_pct))).pct(round_dp(y_pct, 2)));
    }
    for (name, pct) in &recipe.extras {
        sheet.overall.push(Line::new(*name, round_g(grams(*pct))).pct(*pct));
    }
    if recipe.rollin_butter > 0.0 {
        sheet.overall.push(Line::new("Roll-in butter (lamination)", round_g(grams(recipe.rollin_butter))).pct(recipe.rollin_butter));
    }

    // ---- 4. Split out the PREFERMENT ------------------------------------------
    let pref_flour = if recipe.preferment.is_some() { grams(recipe.pff) } else { 0.0 };
    let pref_water = pref_flour * recipe.pref_hydration / 100.0;
    let mut pref_yeast = 0.0; // instant-equivalent grams

    // The preferment always uses the first flour and first liquid in the lists.
    let main_flour = recipe.flour_blend[0].0;
    let main_liquid = recipe.liquids[0].0;

    match recipe.preferment {
        Some("levain") => {
            let seed = pref_flour * recipe.seed_pct / 100.0;
            sheet.preferment = vec![
                Line::new("Ripe starter (100 % hydration)", round_g(seed)).note("the 'seed'"),
                Line::new(main_flour, round_g(pref_flour - seed / 2.0)),
                Line::new(format!("{main_liquid} (~26-28 C)"), round_g(pref_water - seed / 2.0)),
                Line::new("= Levain total", round_g(pref_flour + pref_water))
                    .note(format!("{:.0} % hydration; holds {} % of the flour", recipe.pref_hydration, recipe.pff)),
            ];
        }
        Some("poolish") => {
            pref_yeast = pref_flour * POOLISH_YEAST_PCT / 100.0;
            sheet.preferment = vec![
                Line::new(main_flour, round_g(pref_flour)),
                Line::new(format!("{main_liquid} (~20 C)"), round_g(pref_water)),
                Line::new(format!("Yeast ({})", input.yeast_type), round_dp(pref_yeast * ycf, 2))
                    .note("a pinch - use scales that read 0.1 g or a 1/16 tsp"),
                Line::new("= Poolish total", round_g(pref_flour + pref_water + pref_yeast * ycf)),
            ];
        }
        _ => {}
    }

    // ---- 5. FINAL DOUGH = overall minus what is already in the preferment ----
    for (flour, share) in &recipe.flour_blend {
        let mut g = grams(*share);
        if *flour == main_flour {
            g -= pref_flour;
        }
        sheet.final_dough.push(Line::new(*flour, round_g(g)));
    }
    for (liquid, share) in &recipe.liquids {
        let mut g = grams(hydration * share / 100.0);
        if *liquid == main_liquid {
            g -= pref_water;
        }
        let note = if liquid.to_lowercase().starts_with("water") {
            "temperature: see the Water tab".to_string()
        } else {
            egg_note(liquid, g)
        };
        sheet.final_dough.push(Line::new(*liquid, round_g(g)).note(note));
    }
    if let Some(kind) = recipe.preferment {
        let label = if kind == "levain" { "Levain (all of it)" } else { "Poolish (all of it)" };
        sheet.final_dough.push(Line::new(label, round_g(pref_flour + pref_water + pref_yeast * ycf)));
    }
    if recipe.salt > 0.0 {
        sheet.final_dough.push(Line::new("Salt", round_g(grams(recipe.salt))));
    }
    if recipe.yeast_instant > 0.0 {
        let final_yeast = (grams(recipe.yeast_instant) - pref_yeast).max(0.0) * ycf;
        sheet.final_dough.push(Line::new(format!("Yeast ({})", input.yeast_type), round_g(final_yeast)));
    }
    for (name, pct) in &recipe.extras {
        sheet.final_dough.push(Line::new(*name, round_g(grams(*pct))));
    }

    // ---- 6. Lamination (croissant) -------------------------------------------
    if recipe.rollin_butter > 0.0 {
        sheet.lamination = vec![Line::new("Cold unsalted butter (82 %+ fat) for the block", round_g(grams(recipe.rollin_butter)))
            .note("pound into a flat square; same temperature-feel as the dough")];
    }

    // ---- 7. Dividing ------------------------------------------------------------
    sheet.divide = if recipe.rollin_butter > 0.0 {
        // The dough is laminated before dividing, so pieces include the butter.
        vec![Line::new(format!("Cut {} x {}", input.pieces, recipe.piece_name), round_g(input.piece_weight)).note("each piece (dough + butter)")]
    } else {
        vec![Line::new(format!("Divide into {} x {}", input.pieces, recipe.piece_name), round_g(input.piece_weight)).note("raw dough per piece")]
    };

    Ok(sheet)
}

/// A printable plain-text bakesheet (used by Export).
pub fn sheet_as_text(sheet: &Bakesheet) -> String {
    let r = &sheet.recipe;
    let mut out = vec![
        format!("{}  -  {} x {} @ {} g", r.name, sheet.pieces, r.piece_name, sheet.piece_weight),
        format!(
            "{}: {} %   |   total flour {:.0} g   |   total dough {:.0} g (incl. bowl loss)",
            r.liquid_label, sheet.hydration, sheet.total_flour, sheet.total_dough
        ),
        String::new(),
    ];
    let mut block = |title: &str, lines: &[Line], show_pct: bool| {
        if lines.is_empty() {
            return;
        }
        out.push(title.to_string());
        out.push("-".repeat(title.len()));
        for l in lines {
            let pct = match (show_pct, l.pct) {
                (true, Some(p)) => format!("{p:>6.1} %"),
                _ => " ".repeat(8),
            };
            let note = if l.note.is_empty() { String::new() } else { format!("   ({})", l.note) };
            out.push(format!("  {:<46} {:>8} g  {}{}", l.name, l.grams, pct, note));
        }
        out.push(String::new());
    };
    block("OVERALL FORMULA (baker's %)", &sheet.overall, true);
    block("PREFERMENT - mix first", &sheet.preferment, false);
    block("FINAL DOUGH - mix day", &sheet.final_dough, false);
    block("LAMINATION", &sheet.lamination, false);
    block("DIVIDE", &sheet.divide, false);
    out.push(format!("BAKE: {}", r.bake));
    if !r.notes.is_empty() {
        out.push(format!("NOTE: {}", r.notes));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{build_recipes, find};

    /// Sum of the real ingredient rows (skips "= total" rows).
    fn total(lines: &[Line]) -> f64 {
        lines.iter().filter(|l| !l.is_total()).map(|l| l.grams).sum()
    }
    fn with(r: &Recipe, f: impl FnOnce(&mut Inputs)) -> Bakesheet {
        let mut i = Inputs::defaults(r);
        f(&mut i);
        calculate(r, &i).unwrap()
    }

    #[test]
    fn egg_pasta_is_100g_flour_and_one_egg_per_portion() {
        let rs = build_recipes();
        let r = find(&rs, "pasta_egg").unwrap();
        let s = with(r, |i| {
            i.pieces = 4;
            i.waste_pct = 0.0;
        });
        let flour = s.overall.iter().find(|l| l.name.to_lowercase().contains("flour")).unwrap().grams;
        let eggs = s.overall.iter().find(|l| l.name.to_lowercase().contains("egg")).unwrap();
        assert!((flour / 4.0 - 100.0).abs() < 1.5);
        assert!(eggs.note.contains("4 large eggs"), "{}", eggs.note);
    }

    #[test]
    fn pasta_has_no_salt_or_yeast_in_the_dough() {
        let rs = build_recipes();
        for key in ["pasta_egg", "pasta_semola"] {
            let s = calculate(find(&rs, key).unwrap(), &Inputs::defaults(find(&rs, key).unwrap())).unwrap();
            assert!(!s.final_dough.iter().any(|l| l.name == "Salt" || l.name.starts_with("Yeast")), "{key}");
        }
    }

    #[test]
    fn semola_is_half_water() {
        let rs = build_recipes();
        assert!((find(&rs, "pasta_semola").unwrap().hydration - 50.0).abs() < 1e-9);
    }

    #[test]
    fn every_recipe_adds_up() {
        // Final dough + lamination must weigh what was asked for (within rounding).
        for r in build_recipes() {
            let s = calculate(&r, &Inputs::defaults(&r)).unwrap();
            let got = total(&s.final_dough) + total(&s.lamination);
            assert!((got - s.total_dough).abs() < 3.0, "{}: {} vs {}", r.key, got, s.total_dough);
        }
    }

    #[test]
    fn true_hydration_includes_levain() {
        let rs = build_recipes();
        let r = find(&rs, "sourdough_classic").unwrap();
        let s = with(r, |i| i.hydration = 75.0);
        let flour: f64 = s.overall.iter().filter(|l| l.name.to_lowercase().contains("flour")).map(|l| l.grams).sum();
        let water = s.overall.iter().find(|l| l.name == "Water").unwrap().grams;
        assert!((water / flour * 100.0 - 75.0).abs() < 0.3);
        let lev_water = r.pff / 100.0 * s.total_flour * r.pref_hydration / 100.0;
        let final_water = s.final_dough.iter().find(|l| l.name == "Water").unwrap().grams;
        assert!((final_water + lev_water - water).abs() < 1.5);
    }

    #[test]
    fn levain_build_adds_up() {
        let rs = build_recipes();
        let r = find(&rs, "sourdough_classic").unwrap();
        let s = calculate(r, &Inputs::defaults(r)).unwrap();
        let parts = total(&s.preferment);
        let levain_total = s.preferment.iter().find(|l| l.is_total()).unwrap().grams;
        assert!((parts - levain_total).abs() < 1.5);
    }

    #[test]
    fn hand_check_simple_case() {
        let rs = build_recipes();
        let r = find(&rs, "focaccia").unwrap();
        let s = with(r, |i| {
            i.piece_weight = 1000.0;
            i.hydration = 75.0;
            i.waste_pct = 0.0;
        });
        let pct_sum = 100.0 + 75.0 + r.salt + r.yeast_instant + r.extras.iter().map(|(_, p)| p).sum::<f64>();
        assert!((s.total_flour - 1000.0 / (pct_sum / 100.0)).abs() < 1e-6);
    }

    #[test]
    fn fresh_yeast_is_three_times_instant() {
        let rs = build_recipes();
        let r = find(&rs, "brioche").unwrap();
        let y = |kind: &str| {
            let s = with(r, |i| i.yeast_type = kind.to_string());
            s.overall.iter().find(|l| l.name.contains("Yeast")).unwrap().grams
        };
        assert!((y("fresh") / y("instant") - 3.0).abs() < 0.1);
    }

    #[test]
    fn bad_input_is_an_error_not_a_crash() {
        let rs = build_recipes();
        let r = find(&rs, "baguette").unwrap();
        let mut i = Inputs::defaults(r);
        i.pieces = 0;
        assert!(calculate(r, &i).is_err());
    }

    #[test]
    fn reference_sources_are_not_averaged() {
        use crate::sources::{average, sources_for, Key};
        let schools: Vec<f64> = sources_for("sourdough", false).iter().filter_map(|s| s.values.hydration).collect();
        let mean = schools.iter().sum::<f64>() / schools.len() as f64;
        assert!((average("sourdough", Key::Hydration).unwrap() - mean).abs() < 1e-9);
    }

    #[test]
    fn school_sourdough_is_wetter_than_the_home_reference() {
        let rs = build_recipes();
        assert!(find(&rs, "sourdough_classic").unwrap().hydration > 65.5);
    }

    #[test]
    fn blends_total_100() {
        for r in build_recipes() {
            assert!((r.flour_blend.iter().map(|(_, p)| p).sum::<f64>() - 100.0).abs() < 1e-9, "{}", r.key);
            assert!((r.liquids.iter().map(|(_, p)| p).sum::<f64>() - 100.0).abs() < 1e-9, "{}", r.key);
        }
    }
}
