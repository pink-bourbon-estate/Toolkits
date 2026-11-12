use patternkit::{draft, params::Params, render_json};

fn p(json: &str) -> Params { serde_json::from_str(json).unwrap() }

#[test]
fn trousers_pass_their_checks_in_every_fit_and_rise() {
    for fit in ["fitted", "regular", "baggy"] {
        for rise in ["standard", "high"] {
            let d = draft(&p(&format!(r#"{{"garment":"trousers","fit":"{fit}","rise_style":"{rise}"}}"#))).unwrap();
            assert_eq!(d.pieces.len(), 3);
            for c in &d.checks { assert!(c.ok, "{fit}/{rise}: {} -> {}", c.label, c.value); }
        }
    }
}

/// Numbers from the drafting video: waist 33.5, seat 40.5, rise 10, inseam 33, knee 20, hem 14.5, crotch length 28.
#[test]
fn trousers_reproduce_the_tutorial_example() {
    let d = draft(&p(r#"{"garment":"trousers","fit":"fitted","rise_style":"standard","waist":33.5,"seat":40.5,"rise":10,"inseam":33,"knee_height":20,"hem_width":14.5,"crotch_length":28}"#)).unwrap();
    let ease = d.checks.iter().find(|c| c.label.starts_with("Back easing")).unwrap();
    // video: 28 - (11.75 back curve + 9.75 front curve + 4 waistband) = 2.5; our back curve is a little shorter, so a little more easing
    let e: f64 = ease.value.split(' ').next().unwrap().parse().unwrap();
    assert!(ease.ok && (2.0..4.5).contains(&e), "{}", ease.value);
    let front_fly = d.pieces[0].outline.iter().map(|q| q.0).fold(f64::MAX, f64::min);
    assert!((front_fly + 6.84).abs() < 0.05, "crotch point at {front_fly}");   // seat/8 - 1/2 = 4.5625, plus half again
    let waist_edge = d.pieces[0].outline.iter().filter(|q| q.1.abs() < 1e-9).map(|q| q.0).fold(f64::MIN, f64::max);
    assert!((waist_edge - 5.3125).abs() < 0.01, "front waist outside at {waist_edge}"); // -4.0625 + (33.5/4 + 1)
}

#[test]
fn shirt_and_hoodie_pass_their_checks() {
    for g in ["shirt", "hoodie"] {
        for fit in ["baggy"] {
            for sleeve in ["short", "long"] {
                let d = draft(&p(&format!(r#"{{"garment":"{g}","fit":"{fit}","sleeve":"{sleeve}"}}"#))).unwrap();
                for c in &d.checks { assert!(c.ok, "{g}/{fit}/{sleeve}: {} -> {}", c.label, c.value); }
            }
        }
    }
    let shirt = draft(&p(r#"{"garment":"shirt","sleeve":"long"}"#)).unwrap();
    assert!(shirt.pieces.iter().any(|x| x.name == "CUFF") && shirt.pieces.iter().any(|x| x.name == "COLLAR"));
    let hood = draft(&p(r#"{"garment":"hoodie"}"#)).unwrap();
    assert!(hood.pieces.iter().any(|x| x.name == "HOOD") && hood.pieces.iter().any(|x| x.name == "POCKET"));
}

#[test]
fn knitting_counts_add_up_and_follow_the_gauge() {
    for fit in ["fitted", "regular", "baggy"] {
        let d = draft(&p(&format!(r#"{{"garment":"knit","fit":"{fit}"}}"#))).unwrap();
        for c in &d.checks { assert!(c.ok, "{fit}: {} -> {}", c.label, c.value); }
        assert!(d.instructions.len() >= 8);
    }
    let coarse = draft(&p(r#"{"garment":"knit","gauge_sts":12,"gauge_rows":18}"#)).unwrap();
    let fine = draft(&p(r#"{"garment":"knit","gauge_sts":24,"gauge_rows":32}"#)).unwrap();
    assert!(coarse.instructions[2].contains("Cast on 7") || coarse.instructions[2].contains("Cast on 6"), "{}", coarse.instructions[2]);
    assert!(fine.instructions[2].contains("Cast on 14"), "{}", fine.instructions[2]);
    assert!(draft(&p(r#"{"garment":"knit","gauge_sts":1}"#)).is_err());
}

#[test]
fn json_api_handles_every_garment_and_rejects_nonsense() {
    for g in ["tshirt", "shirt", "hoodie", "trousers", "knit"] {
        let out: serde_json::Value = serde_json::from_str(&render_json(&format!(r#"{{"garment":"{g}"}}"#))).unwrap();
        assert_eq!(out["ok"], true, "{g}");
        assert!(out["pages"].as_array().unwrap().len() > 3);
    }
    assert!(draft(&p(r#"{"garment":"trousers","inseam":10,"knee_height":20}"#)).is_err());
    assert!(draft(&p(r#"{"garment":"cape"}"#)).is_err());
}
