//! water.rs - desired dough temperature (DDT) water calculator.
//!
//! WHY DOUGH TEMPERATURE MATTERS
//! -----------------------------
//! Fermentation times in every recipe assume the dough comes off the mix at
//! a particular temperature (the DDT, usually 24-26 C). A dough that starts
//! 3 C colder can take an hour longer in bulk. Water is the one ingredient
//! whose temperature is easy to control, so we solve for it.
//!
//! THE PROFESSIONAL FORMULA
//! ```text
//! water temp = (DDT x N) - (flour temp + room temp [+ preferment temp]) - friction
//!
//! N = 3 for a straight dough (flour, room, water)
//! N = 4 with a levain / poolish (flour, room, preferment, water)
//! ```
//! "Friction" is the heat the mixing adds. It depends on YOUR mixer, so the
//! values below are starting points - calibrate once with `friction_from_mix`.

/// The three mixing methods and their starting-point friction (deg C).
pub const MIXING: [(&str, f64); 3] = [
    ("by hand", 4.0),
    ("stand mixer", 12.0),
    ("spiral mixer", 14.0),
];

/// Friction for a mixing method name (falls back to "by hand").
pub fn friction_for(method: &str) -> f64 {
    MIXING.iter().find(|(n, _)| *n == method).map(|(_, f)| *f).unwrap_or(4.0)
}

/// Below this you need ice or chilled flour instead.
pub const MIN_SENSIBLE_WATER_C: f64 = 2.0;
/// Above this you risk stressing yeast and starter.
pub const MAX_SENSIBLE_WATER_C: f64 = 40.0;

/// Water temperature (deg C) needed to hit the desired dough temperature.
/// Pass `preferment_c = None` for a straight dough.
pub fn water_temperature(ddt_c: f64, flour_c: f64, room_c: f64, preferment_c: Option<f64>, friction_c: f64) -> f64 {
    match preferment_c {
        None => ddt_c * 3.0 - (flour_c + room_c) - friction_c,
        Some(p) => ddt_c * 4.0 - (flour_c + room_c + p) - friction_c,
    }
}

/// CALIBRATE your mixer: after a normal mix, probe the dough and enter the
/// temperatures you actually used. The result is YOUR friction factor.
/// `friction = (final dough temp x N) - (sum of everything you put in)`
pub fn friction_from_mix(final_dough_c: f64, flour_c: f64, room_c: f64, water_c: f64, preferment_c: Option<f64>) -> f64 {
    match preferment_c {
        None => final_dough_c * 3.0 - (flour_c + room_c + water_c),
        Some(p) => final_dough_c * 4.0 - (flour_c + room_c + water_c + p),
    }
}

/// Plain-English guidance for the answer.
pub fn advice(water_c: f64) -> &'static str {
    if water_c < MIN_SENSIBLE_WATER_C {
        "Below 2 C - use iced water, and chill the flour or mix the levain cooler. Or accept a warmer dough and shorten the bulk."
    } else if water_c > MAX_SENSIBLE_WATER_C {
        "Above 40 C - too hot for the starter/yeast. Use 38-40 C water and give the bulk a warmer spot or a little more time instead."
    } else {
        "OK - weigh the water at this temperature."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_example() {
        // DDT 25 x 4 = 100; minus flour 20, room 21, levain 24, friction 4 -> 31
        assert!((water_temperature(25.0, 20.0, 21.0, Some(24.0), 4.0) - 31.0).abs() < 1e-9);
        // straight dough: 25 x 3 = 75 - 20 - 21 - 4 = 30
        assert!((water_temperature(25.0, 20.0, 21.0, None, 4.0) - 30.0).abs() < 1e-9);
    }

    #[test]
    fn friction_round_trip() {
        let f = friction_from_mix(25.0, 20.0, 21.0, 31.0, Some(24.0));
        assert!((water_temperature(25.0, 20.0, 21.0, Some(24.0), f) - 31.0).abs() < 1e-9);
    }
}
