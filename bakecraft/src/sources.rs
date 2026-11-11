//! sources.rs - the research data behind every recipe.
//!
//! WHY THIS FILE EXISTS
//! --------------------
//! Instead of trusting one recipe, every formula in BakeCraft is an AVERAGE
//! of several published formulas from well-regarded baking schools and
//! bakeries. Each one is written down as BAKER'S PERCENTAGES so recipes of
//! different sizes can be compared fairly:
//!
//! ```text
//! baker's percentage = ingredient weight / TOTAL flour weight x 100
//! e.g. 1000 g flour, 720 g water, 20 g salt  ->  water 72 %, salt 2 %
//! ```
//!
//! "Overall" percentages are used throughout: flour and water that sit
//! inside a starter, levain or poolish ARE counted in the totals. That is
//! the professional convention.
//!
//! HOW TO ADD YOUR OWN SOURCE
//! --------------------------
//! Copy one `src(...)` block below, change the numbers, and the averages in
//! `formulas.rs` update the next time the program is built. Anything the
//! source does not state is simply left out of the `vals!` list (it is then
//! `None`, which is skipped when averaging - it is NOT treated as zero).
//!
//! A NOTE ON "reference_only"
//! --------------------------
//! A home-baking book can be recorded with `reference_only = true`. It is
//! shown next to the averages for comparison but is NOT included in them,
//! so the averages stay school/bakery-only.
//!
//! YEAST: every yeast figure is stored as INSTANT dried yeast
//! (fresh = 3 x instant, active dried = 1.25 x instant).

/// Which number we want from a source (all are % of total flour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Total water (or main liquid), including water inside a preferment.
    Hydration,
    Salt,
    /// Instant-yeast equivalent.
    YeastInstant,
    /// "Prefermented flour": % of ALL flour that sits in the levain/poolish.
    Pff,
    Eggs,
    Milk,
    Sugar,
    Butter,
    Oil,
    RollinButter,
}

/// All the numbers one source can state. `None` means "not stated".
#[derive(Clone, Copy, Debug)]
pub struct Values {
    pub hydration: Option<f64>,
    pub salt: Option<f64>,
    pub yeast_instant: Option<f64>,
    pub pff: Option<f64>,
    pub pref_hydration: Option<f64>,
    pub eggs: Option<f64>,
    pub milk: Option<f64>,
    pub sugar: Option<f64>,
    pub butter: Option<f64>,
    pub oil: Option<f64>,
    pub rollin_butter: Option<f64>,
}

impl Values {
    /// A `Values` with nothing stated. The `vals!` macro starts from this.
    pub const NONE: Values = Values {
        hydration: None,
        salt: None,
        yeast_instant: None,
        pff: None,
        pref_hydration: None,
        eggs: None,
        milk: None,
        sugar: None,
        butter: None,
        oil: None,
        rollin_butter: None,
    };

    /// Look one number up by its `Key`.
    pub fn get(&self, key: Key) -> Option<f64> {
        match key {
            Key::Hydration => self.hydration,
            Key::Salt => self.salt,
            Key::YeastInstant => self.yeast_instant,
            Key::Pff => self.pff,
            Key::Eggs => self.eggs,
            Key::Milk => self.milk,
            Key::Sugar => self.sugar,
            Key::Butter => self.butter,
            Key::Oil => self.oil,
            Key::RollinButter => self.rollin_butter,
        }
    }
}

/// Small macro so a source reads like `vals!{ hydration: 68, salt: 2.1 }`.
/// (A macro is just code that writes code - here it saves typing `Some(..)`.)
macro_rules! vals {
    ($($k:ident : $v:expr),* $(,)?) => {
        Values { $($k: Some($v as f64),)* ..Values::NONE }
    };
}

/// One published formula, expressed as overall baker's percentages.
#[derive(Clone, Debug)]
pub struct Source {
    /// Recipe family it belongs to (matches `Recipe::family`).
    pub recipe: &'static str,
    pub name: &'static str,
    /// School / bakery / author.
    pub origin: &'static str,
    /// Book + page, or web page.
    pub citation: &'static str,
    /// Where it can be checked (None for print-only sources).
    pub url: Option<&'static str>,
    pub values: Values,
    /// Caveats and conversions.
    pub notes: &'static str,
    /// True = show for comparison, leave out of the average.
    pub reference_only: bool,
}

/// Builds a normal (averaged) source.
fn src(
    recipe: &'static str,
    name: &'static str,
    origin: &'static str,
    citation: &'static str,
    url: Option<&'static str>,
    values: Values,
    notes: &'static str,
) -> Source {
    Source { recipe, name, origin, citation, url, values, notes, reference_only: false }
}

/// Builds a reference-only source (shown, never averaged).
fn reference(
    recipe: &'static str,
    name: &'static str,
    origin: &'static str,
    citation: &'static str,
    values: Values,
    notes: &'static str,
) -> Source {
    Source { recipe, name, origin, citation, url: None, values, notes, reference_only: true }
}

/// Every source, in display order.
pub fn all_sources() -> Vec<Source> {
    vec![
        // ======================= SOURDOUGH (white / country) ===================
        src("sourdough", "Sourdough Bread (Artisan II course)",
            "San Francisco Baking Institute (SFBI)",
            "SFBI Artisan II class formula, as published by a course attendee on The Fresh Loaf",
            Some("https://www.thefreshloaf.com/node/27968/sourdough-bread-sfbi-artisan-ii"),
            vals! { hydration: 68, salt: 2.1, pff: 16.6, pref_hydration: 100, yeast_instant: 0 },
            "0.1 % instant yeast is listed as optional - recorded as 0 because it is optional."),
        src("sourdough", "Pain au Levain", "King Arthur Baking School / Jeffrey Hamelman",
            "Hamelman, 'Bread' (2004); KAF workshop version as posted on The Fresh Loaf",
            Some("https://thefreshloaf.com/comment/81507"),
            vals! { hydration: 68, salt: 1.9, pff: 12.8, pref_hydration: 60, yeast_instant: 0 },
            "Stiff (60 %) levain. 15 % whole wheat in the flour."),
        src("sourdough", "Vermont Sourdough", "King Arthur Baking / Jeffrey Hamelman",
            "Hamelman, 'Bread: A Baker's Book of Techniques and Recipes' (2004), p.156",
            Some("https://www.thefreshloaf.com/node/9598"),
            vals! { hydration: 65, salt: 1.9, pff: 15, pref_hydration: 125, yeast_instant: 0 },
            "Figures from the book; the linked thread confirms the 125 % liquid levain only. 10 % rye in the flour."),
        src("sourdough", "Basic Country Bread", "Tartine Bakery, San Francisco (Chad Robertson)",
            "Robertson, 'Tartine Bread' (2010)",
            Some("https://www.thefreshloaf.com/print/42579"),
            vals! { hydration: 77.3, salt: 1.8, pff: 9.1, pref_hydration: 100, yeast_instant: 0 },
            "1000 g flour (90 % white / 10 % wholemeal) + 200 g leaven, 750 g water, 20 g salt (the extra 50 g water added with the salt is excluded)."),
        src("sourdough", "Overnight Country Blonde", "Ken's Artisan Bakery, Portland (Ken Forkish)",
            "Forkish, 'Flour Water Salt Yeast' (2012)",
            Some("https://www.thefreshloaf.com/print/42579"),
            vals! { hydration: 78, salt: 2.2, pff: 10.8, pref_hydration: 100, yeast_instant: 0 },
            "216 g of a 100 % levain in a 1000 g-flour dough; salt from the book."),
        src("sourdough", "White Wheat Sourdough Bread", "Culinary Institute of America (CIA)",
            "CIA, 'Baking and Pastry: Mastering the Art and Craft' 3rd ed. (2015), p.177",
            None,
            vals! { hydration: 73.0, salt: 2.4, pff: 20.7, pref_hydration: 72, yeast_instant: 0 },
            "Recalculated to overall % (the book lists the starter as an ingredient at 41.8 %)."),
        reference("sourdough", "Classic sourdough", "Aran Bakery, Dunkeld (Flora Shedden)",
            "Shedden, 'Aran' (2019) - home-baking reference",
            vals! { hydration: 65.5, salt: 2.8, pff: 10.3, pref_hydration: 100, yeast_instant: 0 },
            "325 g flour + 75 g starter, 200 g water, 10 g salt. Works at home but runs dry - about 6 points below the school average."),

        // ============================ BAGUETTE ===================================
        src("baguette", "Lean Dough with Poolish (baguette)", "Culinary Institute of America (CIA)",
            "CIA, 'Baking and Pastry' 3rd ed. (2015), p.162", None,
            vals! { hydration: 67.0, salt: 2.2, pff: 30, pref_hydration: 100, yeast_instant: 0.65 }, ""),
        src("baguette", "Baguettes with Poolish", "King Arthur Baking / Jeffrey Hamelman",
            "Hamelman, 'Bread' (2004)", Some("https://www.thefreshloaf.com/comment/389771"),
            vals! { hydration: 66, salt: 2.0, pff: 33, pref_hydration: 100 },
            "Hydration and poolish share confirmed in the linked thread; salt from the book."),
        src("baguette", "Poolish baguette", "San Francisco Baking Institute (SFBI)",
            "SFBI formula as posted on The Fresh Loaf", Some("https://thefreshloaf.com/comment/126419"),
            vals! { hydration: 68, salt: 2.0, pff: 33, pref_hydration: 100, yeast_instant: 0.23 },
            "Original formula is 68 % (the poster pushed it to 75 %)."),
        src("baguette", "Baguette (World Bread Day)", "Le Cordon Bleu (Adelaide)",
            "cordonbleu.edu news recipe", Some("https://www.cordonbleu.edu/news/world-bread-day-baguette-recipe/en"),
            vals! { hydration: 70, salt: 1.67, pff: 10, pref_hydration: 100, yeast_instant: 0.22 },
            "300 g flour / 210 g water / 5 g salt / 2 g fresh yeast."),
        src("baguette", "Traditional French baguette", "Le Cordon Bleu (Paris)",
            "cordonbleu.edu news recipe", Some("https://www.cordonbleu.edu/news/traditional-french-baguette/en"),
            vals! { hydration: 67.3, salt: 1.87, pff: 6.5, pref_hydration: 100, yeast_instant: 0.31 },
            "70 g liquid starter assumed to be 100 % hydration."),

        // ============================ FOCACCIA ===================================
        src("focaccia", "Focaccia (biga)", "Culinary Institute of America (CIA)",
            "CIA, 'Baking and Pastry' 3rd ed. (2015), p.172", None,
            vals! { hydration: 70.2, salt: 2.0, oil: 6.1, yeast_instant: 0.48, pff: 23.1, pref_hydration: 56 }, ""),
        src("focaccia", "Lazy focaccia", "Bread Ahead Bakery & Baking School, London",
            "Bread Ahead recipe via delicious. magazine",
            Some("https://www.deliciousmagazine.co.uk/recipes/bread-aheads-lazy-focaccia/"),
            vals! { hydration: 80, salt: 2.0, oil: 16, yeast_instant: 0.4, pff: 0 },
            "Oil figure includes the oil used in the tray and on top."),
        reference("focaccia", "Focaccia", "Aran Bakery (Flora Shedden)",
            "Shedden, 'Aran' (2019) - home-baking reference",
            vals! { hydration: 90, salt: 2.0, oil: 5, yeast_instant: 1.4, pff: 0 }, ""),

        // ============================ BRIOCHE ====================================
        src("brioche", "Brioche", "Culinary Institute of America (CIA)",
            "CIA, 'Baking and Pastry' 3rd ed. (2015), p.144", None,
            vals! { eggs: 40, milk: 20, butter: 60, yeast_instant: 1.7 },
            "The printed table lists 'sugar 2 %' and no salt, although the method adds both - almost certainly a misprint, so sugar and salt are left out of the average."),
        src("brioche", "Brioche (Pro formula)", "King Arthur Baking (professional formulas)",
            "kingarthurbaking.com/pro/formulas/brioche", Some("https://www.kingarthurbaking.com/pro/formulas/brioche"),
            vals! { eggs: 50, milk: 9, sugar: 12, butter: 50, yeast_instant: 2.33, salt: 2.5 },
            "Liquid is water (9 %) - recorded under 'milk' as 'other liquid'. 7 % fresh yeast."),
        src("brioche", "Brioche", "Bouchon Bakery (Thomas Keller)",
            "Keller & Rouxel, 'Bouchon Bakery' (2012), as reproduced online", None,
            vals! { eggs: 50.4, milk: 17.0, sugar: 11.9, butter: 45.2, yeast_instant: 2.2, salt: 2.6 }, ""),
        reference("brioche", "Brioche", "Aran Bakery (Flora Shedden)",
            "Shedden, 'Aran' (2019) - home-baking reference",
            vals! { eggs: 10, milk: 40, sugar: 25, butter: 50, yeast_instant: 2.0 },
            "Much leaner on egg, wetter on milk/water, much sweeter than the schools."),

        // ============================ CROISSANT ==================================
        src("croissant", "Croissant Dough", "Culinary Institute of America (CIA)",
            "CIA, 'Baking and Pastry' 3rd ed. (2015), p.248", None,
            vals! { milk: 66.7, sugar: 9.8, salt: 2.45, yeast_instant: 0.98, butter: 9.8, rollin_butter: 55.4 }, ""),
        src("croissant", "Classic Croissants", "King Arthur Baking / Jeffrey Hamelman",
            "Hamelman via King Arthur, as reposted on The Fresh Loaf",
            Some("https://www.thefreshloaf.com/comment/526078"),
            vals! { milk: 55.6, sugar: 11.1, butter: 8.3, rollin_butter: 55.5 },
            "Liquid is half milk / half water. Salt and yeast are given only in teaspoons, so they are left out rather than guessed."),
        src("croissant", "Croissants", "Le Cordon Bleu",
            "'Our chef leaks the secrets to the perfect croissant', cordonbleu.edu",
            Some("https://cordonbleu.edu/news/croissant-recipe-chef-secrets/en"),
            vals! { milk: 60, sugar: 12, yeast_instant: 0.67, butter: 0, rollin_butter: 40 },
            "Salt given only as 'a pinch'. A lighter, home-scale recipe (40 % roll-in)."),
        reference("croissant", "Croissants", "Aran Bakery (Flora Shedden)",
            "Shedden, 'Aran' (2019) - home-baking reference",
            vals! { milk: 60, sugar: 15, salt: 1.0, yeast_instant: 1.4, butter: 5, rollin_butter: 50 }, ""),

        // ========================== FRESH PASTA (egg) ============================
        // "hydration" here = whole egg (out of shell) as % of flour.
        // A UK large egg is about 58 g out of the shell.
        src("pasta_egg", "Fresh egg pasta dough", "Pasta Evangelists (London)",
            "Pasta Evangelists blog, 'How to make pasta: a guide to making fresh pasta from scratch'",
            Some("https://pastaevangelists.com/blogs/blog/how-to-make-homemade-pasta"),
            vals! { hydration: 58, salt: 0, yeast_instant: 0 },
            "400 g '00' flour + 4 large eggs ('1 egg and 100 g flour per person'). Eggs taken as 58 g each out of the shell."),
        src("pasta_egg", "Sfoglia (Bolognese hand-rolled egg pasta)", "Bologna Welcome - the city's sfoglina tradition",
            "Bologna Welcome, 'Sfoglia' traditional recipe page",
            Some("https://www.bolognawelcome.com/en/home/live/food-drink/gastronomic-tradition/traditional-recipes/sfoglia"),
            vals! { hydration: 58, salt: 0, yeast_instant: 0 },
            "About 1 egg per 100 g flour (10 eggs per kg). 'Never add water' - adjust with egg white or a little flour."),
        reference("pasta_egg", "Tortellini in brodo - sfoglia with extra yolks",
            "Italia Squisita (Massimo Bottura's tortellini)",
            "ItaliaSquisita.net, 'I tortellini in brodo'",
            vals! { salt: 0, yeast_instant: 0 },
            "Whole eggs plus 3 extra yolks. No weights are published, so it is shown for technique only."),

        // ======================== FRESH PASTA (semola) ===========================
        src("pasta_semola", "Orecchiette con le cime di rapa (dough)", "Italia Squisita (chef Antonio Guida)",
            "ItaliaSquisita.net video recipe, 'Orecchiette con le cime di rapa di Antonio Guida'",
            Some("https://italiasquisita.net/it/video-ricette/orecchiette"),
            vals! { hydration: 50, salt: 0, yeast_instant: 0 }, "500 g semola + 250 g lukewarm water."),
        src("pasta_semola", "Egg-free (pasta bianca) dough, high-protein flour version", "Pasta Evangelists (London)",
            "Pasta Evangelists blog, 'How to make protein-packed vegan pasta'",
            Some("https://pastaevangelists.com/blogs/blog/how-to-make-protein-packed-vegan-pasta"),
            vals! { hydration: 50, salt: 0, yeast_instant: 0 },
            "200 g '00' + 200 g barley-based flour + 200 ml warm water. Their main guide recommends semola for egg-free pasta."),
    ]
}

/// Every Source for one recipe family (optionally without the reference-only ones).
pub fn sources_for(recipe_key: &str, include_reference: bool) -> Vec<Source> {
    all_sources()
        .into_iter()
        .filter(|s| s.recipe == recipe_key && (include_reference || !s.reference_only))
        .collect()
}

/// The numbers one value takes across all NON-reference sources (skipping `None`).
fn numbers(recipe_key: &str, key: Key) -> Vec<f64> {
    sources_for(recipe_key, false).iter().filter_map(|s| s.values.get(key)).collect()
}

/// Mean of one value across the non-reference sources.
/// Missing values are skipped, so a gap never drags the average towards zero.
/// Returns `None` when no source states the value.
pub fn average(recipe_key: &str, key: Key) -> Option<f64> {
    let n = numbers(recipe_key, key);
    if n.is_empty() {
        None
    } else {
        Some(n.iter().sum::<f64>() / n.len() as f64)
    }
}

/// (lowest, highest) value across the non-reference sources.
pub fn spread(recipe_key: &str, key: Key) -> Option<(f64, f64)> {
    let n = numbers(recipe_key, key);
    if n.is_empty() {
        return None;
    }
    let lo = n.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = n.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some((lo, hi))
}
