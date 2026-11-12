use serde::{Deserialize, Serialize};

fn d_chest() -> f64 { 35.25 }
fn d_neck() -> f64 { 16.25 }
fn d_shoulder() -> f64 { 19.75 }
fn d_bicep() -> f64 { 11.625 }
fn d_wrist() -> f64 { 7.5 }
fn d_cbw() -> f64 { 15.25 }
fn d_sleeve() -> f64 { 24.0 }
fn d_short() -> f64 { 8.5 }
fn d_garment() -> String { "tshirt".into() }
fn d_fit() -> String { "baggy".into() }
fn d_sleeve_kind() -> String { "short".into() }
fn d_len() -> String { "long".into() }
fn d_units() -> String { "in".into() }
fn d_page() -> String { "a4".into() }
fn d_waist() -> f64 { 34.0 }
fn d_seat() -> f64 { 41.0 }
fn d_rise() -> f64 { 10.5 }
fn d_inseam() -> f64 { 32.0 }
fn d_knee() -> f64 { 20.0 }
fn d_rise_style() -> String { "high".into() }
fn d_gs() -> f64 { 20.0 }
fn d_gr() -> f64 { 28.0 }
fn t() -> bool { true }

/// Everything the tool needs. Lengths are inches (the web page converts from cm). Alteration fields are all 0 = "no change".
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Params {
    // --- body measurements ---
    pub chest: f64,          // circumference, arms up, tape snug
    pub neck: f64,           // circumference at the base of the neck
    pub shoulder: f64,       // across the back, bone-to-bone ("dip" just below the shoulder ball)
    pub bicep: f64,
    pub wrist: f64,
    pub cb_to_waist: f64,    // centre-back neck to natural waist
    pub sleeve_to_wrist: f64,
    pub armhole_depth: Option<f64>, // sigh: neck base to underarm; None = estimated from the chest
    pub short_sleeve: f64,   // length of a short sleeve from the shoulder point
    // --- trouser measurements ---
    pub waist: f64,          // where the top of the waistband will sit
    pub seat: f64,           // around the fullest part of the seat
    pub rise: f64,           // body rise: seated, hard seat to the top of the waistband
    pub inseam: f64,         // floor to the crotch
    pub knee_height: f64,    // floor to the middle of the knee
    pub hem_width: Option<f64>,   // leg opening, around; None/0 = the fit's default
    pub crotch_length: Option<f64>, // tape from the back waist, under, to the front waist; None = use the fit's default back ease
    pub rise_style: String,  // standard | high
    // --- knitting ---
    pub gauge_sts: f64,      // stitches per 4 in (10 cm) in stocking stitch
    pub gauge_rows: f64,     // rows per 4 in (10 cm)
    // --- choices ---
    pub garment: String,     // tshirt | shirt | hoodie | trousers | knit
    pub fit: String,         // fitted | regular | baggy
    pub sleeve: String,      // short | long
    pub length: String,      // standard | long
    pub units: String,       // in | cm  (display only)
    pub page: String,        // a4 | letter
    pub show_construction: bool,
    pub show_ruler: bool,
    pub show_sew_line: bool,
    // --- common-complaint alterations (inches) ---
    pub armhole_drop_adj: f64,    // tight/pulling under the arms: lower the underarm
    pub chest_ease_adj: f64,      // tight/loose across the torso: change finished chest
    pub shoulder_slope_adj: f64,  // + = steeper (fixes folds at the neck/high shoulder), - = shallower
    pub shoulder_seam_shift: f64, // + moves the shoulder seam toward the front, - toward the back
    pub sleeve_cap_adj: f64,      // + taller sleeve cap (drag lines), - flatter
    pub sleeve_shift: f64,        // + moves the sleeve centre line forward (drag toward the front), - back
    pub neck_adj: f64,            // neck opening too tight (+) or loose (-), total circumference
    pub balance: f64,             // + shirt rides up at the back / hangs back: lengthens back, shortens front at the centre
    pub length_adj: f64,
    pub sleeve_length_adj: f64,
    pub waist_taper: f64,         // pull the side seam in at the waist, per quarter
    pub hem_flare: f64,           // push the side seam out at the hem, per quarter
    // trousers
    pub rise_adj: f64,            // + longer rise (waistband sits higher), - shorter
    pub seat_adj: f64,            // + more seat ease (total), - less
    pub thigh_adj: f64,           // + looser thighs (total), - tighter
    pub crotch_ease_adj: f64,     // + more seat room at the back (the "easing"), - less; fixes a V of drag lines under the seat
}

impl Default for Params {
    fn default() -> Self {
        Params { chest: d_chest(), neck: d_neck(), shoulder: d_shoulder(), bicep: d_bicep(), wrist: d_wrist(), cb_to_waist: d_cbw(),
            sleeve_to_wrist: d_sleeve(), armhole_depth: None, short_sleeve: d_short(), garment: d_garment(), fit: d_fit(),
            sleeve: d_sleeve_kind(), length: d_len(), units: d_units(), page: d_page(), show_construction: t(), show_ruler: t(), show_sew_line: t(),
            armhole_drop_adj: 0.0, chest_ease_adj: 0.0, shoulder_slope_adj: 0.0, shoulder_seam_shift: 0.0, sleeve_cap_adj: 0.0,
            sleeve_shift: 0.0, neck_adj: 0.0, balance: 0.0, length_adj: 0.0, sleeve_length_adj: 0.0, waist_taper: 0.0, hem_flare: 0.0,
            waist: d_waist(), seat: d_seat(), rise: d_rise(), inseam: d_inseam(), knee_height: d_knee(), hem_width: None, crotch_length: None,
            rise_style: d_rise_style(), gauge_sts: d_gs(), gauge_rows: d_gr(), rise_adj: 0.0, seat_adj: 0.0, thigh_adj: 0.0, crotch_ease_adj: 0.0 }
    }
}

/// How loose each fit is. Numbers are garment-minus-body, in inches. `fitted` reproduces the ease used in the drafting
/// tutorials this tool is based on; `baggy` is the same block opened up (more chest, a lower and wider armhole, dropped shoulder).
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    pub chest_ease: f64,      // finished circumference minus body chest
    pub armhole_drop: f64,    // extra armhole depth
    pub bicep_slack: f64,     // sleeve circumference minus bicep at the bicep line
    pub wrist_slack: f64,
    pub shoulder_extend: f64, // shoulder seam beyond the body's shoulder point (dropped shoulder)
    pub shoulder_drop: f64,   // vertical drop of the shoulder line below the back neck point (slope)
    pub hem_below_waist: f64,
    pub cap_divisor: f64,     // sleeve cap height = armhole depth / divisor (bigger = flatter, roomier sleeve)
}

pub fn fit_table(name: &str) -> Result<Fit, String> {
    Ok(match name {
        "fitted" => Fit { chest_ease: 2.0, armhole_drop: 0.0, bicep_slack: 2.0, wrist_slack: 1.5, shoulder_extend: 0.0, shoulder_drop: 0.75, hem_below_waist: 8.0, cap_divisor: 3.0 },
        "regular" => Fit { chest_ease: 6.0, armhole_drop: 0.6, bicep_slack: 3.0, wrist_slack: 2.0, shoulder_extend: 0.3, shoulder_drop: 0.85, hem_below_waist: 10.0, cap_divisor: 3.5 },
        "baggy" => Fit { chest_ease: 12.0, armhole_drop: 1.75, bicep_slack: 4.5, wrist_slack: 3.0, shoulder_extend: 0.75, shoulder_drop: 1.0, hem_below_waist: 11.0, cap_divisor: 4.5 },
        other => return Err(format!("unknown fit '{other}' (use fitted, regular or baggy)")),
    })
}

/// Trouser fits. `seat_extra` is total seat ease beyond the tutorial block (which already has about 2.5 in at the seat once the 1/2 in
/// seam allowances are taken out); `hem` is the default leg opening (around); `back_ease` is the default amount pivoted into the back
/// crotch seam (the tutorial's "easing", 2.5 in on his example).
#[derive(Clone, Copy, Debug)]
pub struct TrFit { pub seat_extra: f64, pub hem: f64, pub back_ease: f64 }

pub fn trouser_fit(name: &str) -> Result<TrFit, String> {
    Ok(match name {
        "fitted" => TrFit { seat_extra: 0.0, hem: 14.5, back_ease: 2.5 },
        "regular" => TrFit { seat_extra: 3.5, hem: 17.0, back_ease: 2.5 },
        "baggy" => TrFit { seat_extra: 7.5, hem: 20.0, back_ease: 3.0 },
        other => return Err(format!("unknown fit '{other}' (use fitted, regular or baggy)")),
    })
}
