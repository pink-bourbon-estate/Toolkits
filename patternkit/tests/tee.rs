use patternkit::{params::Params, tee};

fn p(json: &str) -> Params { serde_json::from_str(json).unwrap() }

#[test]
fn every_fit_and_sleeve_passes_its_own_checks() {
    for fit in ["fitted", "regular", "baggy"] {
        for sleeve in ["short", "long"] {
            let d = tee::draft(&p(&format!(r#"{{"fit":"{fit}","sleeve":"{sleeve}"}}"#))).unwrap();
            for c in &d.checks { assert!(c.ok, "{fit}/{sleeve}: {} -> {}", c.label, c.value); }
        }
    }
}

/// The numbers quoted in the drafting video for a 37.5 in chest: quarter chest + 1 in = 10 3/8, front armhole ~9 1/8, cap chord ~9.
#[test]
fn reproduces_the_tutorial_example() {
    let d = tee::draft(&p(r#"{"chest":37.5,"neck":15,"shoulder":17,"bicep":10.5,"wrist":6.5,"cb_to_waist":19,"armhole_depth":9,"fit":"fitted"}"#)).unwrap();
    let f = &d.pieces[0];
    let xmax = f.outline.iter().map(|q| q.0).fold(0.0, f64::max);
    assert!((xmax - 10.375).abs() < 0.01, "side seam at {xmax}");
    let cap = d.checks.iter().find(|c| c.label.starts_with("Sleeve cap vs front")).unwrap();
    assert!(cap.value.contains("9.2"), "{}", cap.value); // video: 9 1/8; our spline walks 9.24
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    assert!(tee::draft(&p(r#"{"chest":0}"#)).is_err());
    assert!(tee::draft(&p(r#"{"fit":"tight-ish"}"#)).is_err());
    assert!(tee::draft(&p(r#"{"garment":"hat"}"#)).is_err());
}

#[test]
fn alterations_move_the_pattern_the_right_way() {
    let base = tee::draft(&p("{}")).unwrap();
    let deeper = tee::draft(&p(r#"{"armhole_drop_adj":1.0}"#)).unwrap();
    let y2 = |d: &tee::Draft| d.pieces[0].outline.iter().map(|q| q.1).fold(f64::MIN, f64::max);
    assert!(y2(&deeper) >= y2(&base) - 1e-9); // hem unchanged or lower
    let wider = tee::draft(&p(r#"{"chest_ease_adj":4.0}"#)).unwrap();
    let w = |d: &tee::Draft| d.pieces[0].outline.iter().map(|q| q.0).fold(0.0, f64::max);
    assert!((w(&wider) - w(&base) - 1.0).abs() < 1e-6, "4 in more chest = 1 in more per quarter");
}
