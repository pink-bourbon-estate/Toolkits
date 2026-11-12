//! Parametric tee. The construction follows the step-by-step in Cornelius Quiring's public "How to Draft a Shirt Pattern"
//! video (numbered points 0-18, half pieces on the fold, 1/2" seam allowance in the drawn edge). The fit table opens the
//! block up for looser fits; the alteration knobs are his "common complaints" fixes expressed as numbers.
use crate::geom::*;
use crate::params::*;

pub struct Check { pub label: String, pub value: String, pub ok: bool }

pub enum Mark {
    Line { a: P, b: P, label: String },
    Pt { p: P, id: String },
    Dim { a: P, b: P, off: f64, text: String },
}

pub struct Piece {
    pub name: String,
    pub cut: String,           // "Cut 1 on fold"
    pub outline: Vec<P>,       // cut line (seam allowance included), closed
    pub sew: Vec<P>,           // sew line, closed (hem edge = hem allowance, fold edge = 0)
    pub fold: Option<(P, P)>,
    pub grain: (P, P),
    pub notches: Vec<(P, P)>,  // short ticks on the outline
    pub marks: Vec<Mark>,      // construction layer
}

pub struct Draft {
    pub title: String,            // sheet heading
    pub info: Vec<String>,        // up to three small lines under the heading
    pub pieces: Vec<Piece>,
    pub explain: Vec<String>,
    pub instructions: Vec<String>, // written steps (knitting)
    pub checks: Vec<Check>,
    pub warnings: Vec<String>,
}

pub struct Geo {
    pub sa: f64, pub hem: f64,
    pub y1: f64, pub y2: f64, pub y3: f64, pub y4: f64, pub y11: f64, pub y15: f64,
    pub x5: f64, pub x12: f64, pub x17: f64, pub wn: f64, pub fd: f64,
    pub sigh: f64, pub ease: f64, pub len: f64,
}

pub fn fmt(v: f64, cm: bool) -> String {
    if cm { format!("{:.1} cm", v * 2.54) } else { let s = format!("{:.2}", v); format!("{} in", s.trim_end_matches('0').trim_end_matches('.')) }
}

struct Body { piece: Piece, armhole: Vec<P>, side_len: f64, shoulder_len: f64, neck_curve: Vec<P>, p17: P, p5: P }

fn body(g: &Geo, p: &Params, _fit: &Fit, front: bool, cm: bool) -> Body {
    let sh_off = if front { p.shoulder_seam_shift } else { -p.shoulder_seam_shift };
    let depth = if front { g.fd } else { g.y1 };
    let n10 = (g.wn, sh_off);
    let p17 = (g.x17, g.y15 + sh_off);
    let p12 = (g.x12 + if front { -0.25 } else { 0.25 }, g.y11 + p.shoulder_slope_adj * 0.5);
    let p5 = (g.x5, g.y2);
    let tear = (p12.0 + (g.x5 - p12.0) * 0.35, p12.1 + (g.y2 - p12.1) * 0.80);
    let armhole = spline(&[p17, p12, tear, p5]);
    let ye = sh_off;
    let neck_curve = spline(&[(0.0, depth), (g.wn * 0.30, ye + (depth - ye) * 0.93), (g.wn * 0.62, ye + (depth - ye) * 0.55), (g.wn, ye)]);

    let mut side = vec![p5];
    if p.waist_taper.abs() > 1e-9 { side.push((g.x5 - p.waist_taper, g.y3)); }
    let hx = g.x5 + p.hem_flare;
    side.push((hx, g.y4));
    let hem_cl = g.y4 + if front { -p.balance } else { p.balance };

    let mut outline: Vec<P> = vec![];
    outline.extend(neck_curve.iter().cloned());   // centre line -> neck/shoulder point
    outline.push(p17);                             // shoulder line (straight)
    outline.extend(armhole.iter().cloned().skip(1)); // armhole down to the underarm
    outline.extend(side.iter().cloned().skip(1));   // side seam to the hem corner
    outline.push((0.0, hem_cl));                  // hem back to the fold
    let shirt_front = front && p.garment == "shirt";
    let band_w = 2.5;
    if shirt_front { outline.pop(); outline.push((-band_w, hem_cl)); outline.insert(0, (-band_w, depth)); }
    let fold = ((0.0, hem_cl), (0.0, depth));
    let (sa, hem) = (g.sa, g.hem);
    let bal = p.balance.abs() + 1e-6;
    let y4 = g.y4;
    let dist_of = move |a: P, b: P| -> f64 {
        if a.0.abs() < 1e-6 && b.0.abs() < 1e-6 { 0.0 }                                   // fold edge: no allowance
        else if (a.1 - y4).abs() <= bal && (b.1 - y4).abs() <= bal && (a.0 - b.0).abs() > 1.0 { hem } // hem edge
        else { sa }
    };
    let sew = if p.show_sew_line { offset_inward(&outline, &dist_of) } else { vec![] };

    // construction layer: Cornelius's numbered points
    let mut marks = vec![];
    let xe = g.x5 + 0.3;
    for (y, lab) in [(0.0, "0  shoulder line"), (g.y1, "1  back-neck depth"), (g.y2, "2  underarm (armhole depth)"), (g.y3, "3  natural waist"), (g.y4, "4  hem + allowance")] {
        marks.push(Mark::Line { a: (-0.3, y), b: (xe, y), label: lab.into() });
    }
    marks.push(Mark::Line { a: (g.x5, -0.3), b: (g.x5, g.y4 + 0.3), label: "side seam".into() });
    marks.push(Mark::Line { a: (g.x12, -0.3), b: (g.x12, g.y2 + 0.3), label: "half-back line".into() });
    marks.push(Mark::Line { a: (g.wn, -0.3), b: (g.wn, g.y1 + 0.3), label: "neck width".into() });
    for (pt, id) in [((0.0, 0.0), "0"), ((0.0, g.y1), "1"), ((0.0, g.y2), "2"), ((0.0, g.y3), "3"), ((0.0, g.y4), "4"), (p5, "5"), ((g.x5, 0.0), "6"),
        ((g.x5, g.y3), "7"), ((g.x5, g.y4), "8"), ((g.wn, g.y1), "9"), (n10, "10"), ((0.0, g.y11), "11"), (p12, "12"), ((g.x12, g.y2), "13"),
        ((g.x12, 0.0), "14"), ((0.0, g.y15), "15"), ((g.x12, g.y15), "16"), (p17, "17")] {
        marks.push(Mark::Pt { p: pt, id: id.into() });
    }
    if front { marks.push(Mark::Pt { p: (0.0, g.fd), id: "18".into() }); }
    let ox = -1.2;
    marks.push(Mark::Dim { a: (0.0, 0.0), b: (0.0, g.y1), off: ox, text: format!("back neck {}", fmt(g.y1, cm)) });
    marks.push(Mark::Dim { a: (0.0, g.y1), b: (0.0, g.y2), off: ox, text: format!("armhole {}", fmt(g.sigh, cm)) });
    marks.push(Mark::Dim { a: (0.0, g.y2), b: (0.0, g.y4), off: ox, text: format!("below arm {}", fmt(g.y4 - g.y2, cm)) });
    marks.push(Mark::Dim { a: (0.0, g.y1), b: (0.0, g.y4), off: ox - 1.1, text: format!("CB neck to hem {}", fmt(g.y4 - g.y1, cm)) });
    marks.push(Mark::Dim { a: (0.0, 0.0), b: (g.wn, 0.0), off: -0.9, text: format!("neck {}", fmt(g.wn, cm)) });
    marks.push(Mark::Dim { a: (0.0, 0.0), b: (g.x5, 0.0), off: -1.7, text: format!("1/4 chest + ease + SA {}", fmt(g.x5, cm)) });
    if front { marks.push(Mark::Dim { a: (0.0, 0.0), b: (0.0, g.fd), off: ox + 0.6, text: format!("front neck {}", fmt(g.fd, cm)) }); }

    if shirt_front {
        marks.push(Mark::Line { a: (-band_w / 2.0, depth - 0.3), b: (-band_w / 2.0, g.y4 + 0.2), label: "fold of button band".into() });
        marks.push(Mark::Dim { a: (-band_w, g.y4 + 0.3), b: (0.0, g.y4 + 0.3), off: 0.5, text: format!("band {} = 2 x 1 1/4 in", fmt(band_w, cm)) });
    }
    let side_len = length(&side);
    let shoulder_len = dist(n10, p17);
    let name = if front { "FRONT" } else { "BACK" };
    let piece = Piece { name: name.into(), cut: if shirt_front { "Cut 2 (mirror: one left, one right)".into() } else { "Cut 1 on the fold".into() }, outline, sew, fold: if shirt_front { None } else { Some(fold) },
        grain: ((g.x5 * 0.5, g.y2 + 1.0), (g.x5 * 0.5, g.y3 + 5.0)), notches: vec![((g.x5, g.y3), (g.x5 - 0.3, g.y3))], marks };
    Body { piece, armhole, side_len, shoulder_len, neck_curve, p17, p5 }
}

/// One half of the sleeve cap, apex (0,0) to the underarm point; bumps scaled until the curve is `target` long.
fn cap_half(apex_to: P, target: f64, warn: &mut Vec<String>, name: &str) -> Vec<P> {
    let p0 = (0.0, 0.0);
    let p3 = apex_to;
    let p4 = lerp(p3, p0, 1.0 / 3.0);
    let d = (p3.0 - p0.0, p3.1 - p0.1);
    let l = (d.0 * d.0 + d.1 * d.1).sqrt();
    let nrm = (d.1 / l, -d.0 / l);
    let nrm = if nrm.1 > 0.0 { (-nrm.0, -nrm.1) } else { nrm }; // bump up (toward smaller y)
    let build = |k: f64| -> Vec<P> {
        let m1 = lerp(p0, p4, 0.5);
        let m2 = lerp(p4, p3, 0.5);
        spline(&[p0, (m1.0 + nrm.0 * 0.75 * k, m1.1 + nrm.1 * 0.75 * k), p4, (m2.0 - nrm.0 * 0.25 * k, m2.1 - nrm.1 * 0.25 * k), p3])
    };
    let (mut lo, mut hi) = (0.0_f64, 5.0_f64);
    if length(&build(0.0)) >= target { warn.push(format!("{name} sleeve cap is already longer than the armhole before any curve is added; lower the cap or widen the armhole")); return build(0.0); }
    for _ in 0..50 { let mid = (lo + hi) / 2.0; if length(&build(mid)) < target { lo = mid } else { hi = mid } }
    let k = (lo + hi) / 2.0;
    if k > 3.5 { warn.push(format!("{name} sleeve cap needs an extreme curve ({k:.1}x); raise the cap height")); }
    build(k)
}

pub fn draft(p: &Params) -> Result<Draft, String> {
    if !["tshirt", "shirt", "hoodie"].contains(&p.garment.as_str()) { return Err(format!("garment '{}' is not a top (use tshirt, shirt or hoodie)", p.garment)); }
    let fit = fit_table(&p.fit)?;
    let cm = p.units == "cm";
    for (v, n) in [(p.chest, "chest"), (p.neck, "neck"), (p.shoulder, "shoulder"), (p.bicep, "bicep"), (p.wrist, "wrist"), (p.cb_to_waist, "cb_to_waist")] {
        if !(v > 4.0 && v < 80.0) { return Err(format!("{n} = {v} in looks wrong; enter measurements in inches (4-80)")); }
    }
    let sa = 0.5; let hem = 1.0;
    let mut warnings = vec![];
    let sigh = p.armhole_depth.unwrap_or(0.24 * p.chest) + fit.armhole_drop + p.armhole_drop_adj;
    let ease = fit.chest_ease + p.chest_ease_adj;
    let y1 = 1.25;
    let len = p.cb_to_waist + fit.hem_below_waist + if p.length == "long" { 4.0 } else { 0.0 } + p.length_adj;
    // a tee has no collar stand, so its neck hole is opened up from the shirt block's neck/5 and neck/4 (+1/2 in wide, +3/4 in deep)
    let kind = p.garment.as_str();
    let nk = p.neck + p.neck_adj;
    // a shirt neck is sized for its collar stand; a hoodie neck is higher than a tee's because the hood sits in it
    let (wn, fd) = match kind { "shirt" => (nk / 5.0, nk / 5.0 + 0.5), "hoodie" => (nk / 5.0 + 0.25, nk / 5.0 + 1.25), _ => (nk / 5.0 + 0.5, nk / 4.0 + 0.75) };
    let x17 = p.shoulder / 2.0 + fit.shoulder_extend + sa;
    let x12 = x17 - 0.5;
    let mut x5 = p.chest / 4.0 + ease / 4.0 + sa;
    if x5 < x12 + 1.25 {
        warnings.push(format!("Your shoulders are wide for this chest/fit, so the side seam was pushed out to keep the armhole shaped (finished chest {} instead of {}).",
            fmt(4.0 * (x12 + 1.25 - sa), cm), fmt(p.chest + ease, cm)));
        x5 = x12 + 1.25;
    }
    let g = Geo { sa, hem, y1, y2: y1 + sigh, y3: y1 + p.cb_to_waist, y4: y1 + len + hem, y11: y1 + sigh / 2.0, y15: y1 + fit.shoulder_drop + p.shoulder_slope_adj,
        x5, x12, x17, wn, fd, sigh, ease, len };
    let f = body(&g, p, &fit, true, cm);
    let b = body(&g, p, &fit, false, cm);

    // ---- sleeve (front half to the right of the centre line, back half to the left) ----
    let mut cap_h = sigh / (fit.cap_divisor - p.sleeve_cap_adj).max(1.5);
    let w3f = dist(f.p5, f.p17); let w3b = dist(b.p5, b.p17);
    let tf = length(&f.armhole) + p.sleeve_shift;
    let tb = length(&b.armhole) - p.sleeve_shift;
    // the cap's straight chord (point 0 to point 3) cannot be longer than the armhole it sews into. A knit sleeve may be eased in a
    // little (up to 1/2 in per half); beyond that the cap is lowered. The back armhole is a touch shorter than the front, so it limits.
    let allowed_ease = 0.5;
    let room = (tf.min(tb) + allowed_ease - 0.15).max(0.0);
    let h_max = (room * room - w3f.max(w3b).powi(2)).max(0.0).sqrt();
    if cap_h > h_max {
        warnings.push(format!("Sleeve cap height lowered from {} to {} so the cap fits the armhole (a lower cap is a roomier, flatter sleeve).", fmt(cap_h, cm), fmt(h_max.max(0.5), cm)));
        cap_h = h_max.max(0.5);
    }
    let chord = |w3: f64| (w3 * w3 + cap_h * cap_h).sqrt();
    let (tf2, tb2) = (tf.max(chord(w3f) + 0.15), tb.max(chord(w3b) + 0.15));
    let capf = cap_half((w3f, cap_h), tf2, &mut warnings, "Front");
    let capb = cap_half((w3b, cap_h), tb2, &mut warnings, "Back");
    let slack = fit.bicep_slack; let wslack = fit.wrist_slack;
    let xb = (p.bicep + slack) / 2.0 + sa;
    let long = p.sleeve == "long" || kind == "hoodie";
    let cuff_h = match kind { "shirt" => 2.5, "hoodie" => 3.0, _ => 0.0 }; // finished cuff height, taken off a long sleeve
    let slen = (if long { p.sleeve_to_wrist - cuff_h } else { p.short_sleeve }) - fit.shoulder_extend * 0.5 + p.sleeve_length_adj;
    let xw = (p.wrist + wslack) / 2.0 + sa;
    let half = |cap: &Vec<P>, s: f64, w3: f64| -> Vec<P> {
        let mut v: Vec<P> = cap.iter().map(|q| (s * q.0, q.1)).collect();
        if long {
            let ye = slen / 2.0 + sa;
            v.push((s * xb, ye));
            v.push((s * xw, slen + sa));
            let sl = (xw - xb) / (slen + sa - ye);
            v.push((s * (xw + sl * hem), slen + sa + hem));
        } else {
            let ys = slen + sa;
            let sl = (xb - w3) / (ys - cap_h);
            v.push((s * xb, ys));
            v.push((s * (xb + sl * hem), ys + hem));
        }
        v
    };
    let hf = half(&capf, 1.0, w3f);
    let hb = half(&capb, -1.0, w3b);
    let mut sleeve_outline = hf.clone();
    let mut back_rev = hb.clone(); back_rev.reverse();
    sleeve_outline.extend(back_rev.into_iter());
    let ymax = sleeve_outline.iter().map(|q| q.1).fold(f64::MIN, f64::max);
    let sleeve_hem_y = ymax;
    let sdist = move |a: P, b: P| -> f64 { if (a.1 - sleeve_hem_y).abs() < 1e-6 && (b.1 - sleeve_hem_y).abs() < 1e-6 { hem } else { sa } };
    let sleeve_sew = if p.show_sew_line { offset_inward(&sleeve_outline, &sdist) } else { vec![] };
    let mut sm = vec![];
    sm.push(Mark::Line { a: (-w3b - 0.5, cap_h), b: (w3f + 0.5, cap_h), label: "cap height line".into() });
    sm.push(Mark::Line { a: (0.0, -0.4), b: (0.0, slen + sa + hem + 0.3), label: "sleeve centre".into() });
    sm.push(Mark::Line { a: (-xb - 0.5, slen + sa), b: (xb + 0.5, slen + sa), label: if long { "wrist line".into() } else { "sleeve end (hem folds up from here)".into() } });
    sm.push(Mark::Pt { p: (0.0, 0.0), id: "0".into() });
    sm.push(Mark::Pt { p: (w3f, cap_h), id: "3F".into() });
    sm.push(Mark::Pt { p: (-w3b, cap_h), id: "3B".into() });
    sm.push(Mark::Dim { a: (0.0, 0.0), b: (0.0, cap_h), off: -w3b - 1.6, text: format!("cap height {} (= armhole {} / {:.1})", fmt(cap_h, cm), fmt(sigh, cm), fit.cap_divisor - p.sleeve_cap_adj) });
    sm.push(Mark::Dim { a: (0.0, 0.0), b: (0.0, slen + sa), off: -w3b - 2.6, text: format!("sleeve length {}", fmt(slen, cm)) });
    sm.push(Mark::Dim { a: (-xb, slen + sa + hem + 0.8), b: (xb, slen + sa + hem + 0.8), off: 0.0, text: format!("sleeve width at bicep line: {} = (bicep {} + slack {})/2 + SA, both sides", fmt(2.0 * xb, cm), fmt(p.bicep, cm), fmt(slack, cm)) });
    let sleeve = Piece { name: "SLEEVE".into(), cut: "Cut 2 (one left, one right; flip the paper)".into(), outline: sleeve_outline, sew: sleeve_sew, fold: None,
        grain: ((0.0, cap_h + 1.0), (0.0, cap_h + 7.0)), notches: vec![((0.0, 0.0), (0.0, 0.35)), ((w3f, cap_h), (w3f - 0.3, cap_h + 0.0))], marks: sm };

    // ---- neck finish and the extra pieces for this garment ----
    let neck_open = 2.0 * (length(&f.neck_curve) + length(&b.neck_curve));
    let rect = |name: &str, cut: &str, w: f64, h: f64, note: String, grain_along: bool| -> Piece {
        let (cw, ch) = (w + 2.0 * sa, h + 2.0 * sa);
        Piece { name: name.into(), cut: cut.into(), outline: vec![(0.0, 0.0), (cw, 0.0), (cw, ch), (0.0, ch)],
            sew: if p.show_sew_line { vec![(sa, sa), (cw - sa, sa), (cw - sa, ch - sa), (sa, ch - sa)] } else { vec![] }, fold: None,
            grain: if grain_along { ((cw * 0.3, ch * 0.5), (cw * 0.7, ch * 0.5)) } else { ((cw * 0.5, ch * 0.25), (cw * 0.5, ch * 0.75)) },
            notches: vec![((cw * 0.5, 0.0), (cw * 0.5, 0.3))],
            marks: vec![Mark::Line { a: (-0.2, ch / 2.0), b: (cw + 0.2, ch / 2.0), label: "fold line".into() },
                Mark::Dim { a: (0.0, ch + 0.4), b: (cw, ch + 0.4), off: 0.0, text: note }] }
    };
    let mut extra: Vec<Piece> = vec![];
    let mut extra_checks: Vec<(String, String, bool)> = vec![];
    let mut extra_explain: Vec<String> = vec![];
    match kind {
        "shirt" => {
            let ls = neck_open + 0.75;
            extra.push(rect("COLLAR STAND", "Cut 2 (outer + inner), interface one", ls, 2.5, format!("{} = neck opening {} + 3/4 in button overlap + 2 SA; {} = 2 x 1 1/4 in + 2 SA", fmt(ls + 2.0 * sa, cm), fmt(neck_open, cm), fmt(2.5 + 2.0 * sa, cm)), true));
            let (hc, lc) = (2.25 + sa, ls + 2.0 * sa);
            let outline = vec![(0.0, 0.0), (lc, 0.0), (lc + 0.6, hc + sa), (-0.6, hc + sa)];
            extra.push(Piece { name: "COLLAR".into(), cut: "Cut 2 (outer + under), interface one".into(), sew: if p.show_sew_line { offset_inward(&outline, &|_, _| sa) } else { vec![] }, outline, fold: None,
                grain: ((lc * 0.3, hc * 0.5), (lc * 0.7, hc * 0.5)), notches: vec![((lc * 0.5, 0.0), (lc * 0.5, 0.3))],
                marks: vec![Mark::Dim { a: (0.0, hc + sa + 0.4), b: (lc, hc + sa + 0.4), off: 0.0, text: format!("lower edge {} = stand length; the top edge is 1 1/4 in longer so the collar rolls over the stand", fmt(lc, cm)) }] });
            if long {
                let cl = p.wrist + 2.5 + 1.0;
                extra.push(rect("CUFF", "Cut 4 (2 pairs), interface 2", cl, 2.0 * cuff_h, format!("{} = wrist {} + 2 1/2 in room + 1 in button overlap + 2 SA", fmt(cl + 2.0 * sa, cm), fmt(p.wrist, cm)), true));
                extra_checks.push(("Cuff clears the hand (wrist + 2 in or more)".into(), format!("{} around (wrist {})", fmt(p.wrist + 2.5, cm), fmt(p.wrist, cm)), true));
            }
            extra_checks.push(("Collar stand matches the neck (neck - 1/2 in to neck + 2 1/2 in)".into(), format!("opening {} (neck {})", fmt(neck_open, cm), fmt(nk, cm)), neck_open >= nk - 0.5 && neck_open <= nk + 2.5));
            extra_explain.push(format!("Shirt neck: width neck/5 = {} and front depth neck/5 + 1/2 = {} (a shirt neck is cut small because the collar stand sits on it). The front has a cut-on button band, {} wide, folded in half twice; button holes go on the right front for men, left for women.", fmt(wn, cm), fmt(fd, cm), fmt(2.5, cm)));
            extra_explain.push(format!("Collar stand = neck opening {} + 3/4 in overlap, 1 1/4 in tall when finished; the collar sews to it and its top edge is 1 1/4 in longer so it rolls. The long sleeve is shortened by the {} cuff and gathered into it.", fmt(neck_open, cm), fmt(cuff_h, cm)));
        }
        "hoodie" => {
            let l = length(&f.neck_curve) + length(&b.neck_curve);
            let hh = 0.9 * nk + 1.0; let hw = 0.75 * nk;
            let mut o = vec![(0.0, hh), (l, hh)];
            o.extend(spline(&[(l, hh), (hw * 0.97, hh * 0.55), (hw, hh * 0.22)]).into_iter().skip(1));
            o.extend(spline(&[(hw, hh * 0.22), (hw * 0.9, hh * 0.04), (hw * 0.55, 0.0), (hw * 0.2, hh * 0.06), (0.0, hh * 0.3)]).into_iter().skip(1));
            extra.push(Piece { name: "HOOD".into(), cut: "Cut 2 (mirror), + 2 lining if double".into(), sew: if p.show_sew_line { offset_inward(&o, &|_, _| sa) } else { vec![] }, outline: o, fold: None,
                grain: ((hw * 0.35, hh * 0.45), (hw * 0.35, hh * 0.85)), notches: vec![((l * 0.5, hh), (l * 0.5, hh - 0.3))],
                marks: vec![Mark::Dim { a: (0.0, hh), b: (l, hh), off: 0.8, text: format!("neck edge {} = front + back neck curve (sews to one half of the neck)", fmt(l, cm)) },
                    Mark::Dim { a: (0.0, 0.0), b: (0.0, hh), off: -0.8, text: format!("height {} = 0.9 x neck + 1 in", fmt(hh, cm)) },
                    Mark::Dim { a: (0.0, hh * 0.1), b: (hw, hh * 0.1), off: 0.0, text: format!("width {} = 0.75 x neck", fmt(hw, cm)) }] });
            let open = 2.0 * (xw - sa);
            extra.push(rect("RIB CUFF", "Cut 2 in rib, stretch along the length", 0.75 * open, 2.0 * cuff_h, format!("{} = 0.75 x sleeve opening {} + 2 SA (rib stretches to fit)", fmt(0.75 * open + 2.0 * sa, cm), fmt(open, cm)), true));
            let hemc = 4.0 * (x5 - sa);
            extra.push(rect("RIB WAISTBAND", "Cut 1 in rib, goes all the way round", 0.75 * hemc, 2.0 * 3.0, format!("{} = 0.75 x finished hem {} + 2 SA", fmt(0.75 * hemc + 2.0 * sa, cm), fmt(hemc, cm)), true));
            let (pb, pt, ph) = (0.36 * (p.chest + ease), 0.26 * (p.chest + ease), 8.5);
            let po = vec![((pb - pt) / 2.0, 0.0), ((pb + pt) / 2.0, 0.0), (pb, ph), (0.0, ph)];
            extra.push(Piece { name: "POCKET".into(), cut: "Cut 1 (kangaroo pocket; hem the angled openings 1 in)".into(), sew: if p.show_sew_line { offset_inward(&po, &|_, _| sa) } else { vec![] }, outline: po, fold: None,
                grain: ((pb * 0.5, ph * 0.2), (pb * 0.5, ph * 0.8)), notches: vec![],
                marks: vec![Mark::Dim { a: (0.0, ph), b: (pb, ph), off: 0.6, text: format!("bottom {} = 0.36 x finished chest; top {} = 0.26 x", fmt(pb, cm), fmt(pt, cm)) }] });
            extra_checks.push(("Hood edge sews to the neck".into(), format!("{} per half; opening {} (neck {})", fmt(l, cm), fmt(neck_open, cm), fmt(nk, cm)), neck_open >= nk + 1.5));
            extra_checks.push(("Hood tall enough for a head (height >= 0.75 x neck + 2 in)".into(), fmt(hh, cm), hh >= 0.75 * nk + 2.0 - 1e-9));
            extra_explain.push(format!("Hoodie neck: width neck/5 + 1/4 = {}, front depth neck/5 + 1 1/4 = {} (a hoodie neck is shallower than a tee's because the hood sits in it). The hood is two mirrored halves {} tall and {} wide, sewn together along the curved top and back edge; its straight lower edge ({}) sews to half of the neck opening, and the face edge is hemmed.", fmt(wn, cm), fmt(fd, cm), fmt(hh, cm), fmt(hw, cm), fmt(l, cm)));
            extra_explain.push(format!("Rib cuffs and waistband are cut at 0.75 x the opening they sew to, so the rib pulls the body in at the hem and wrist (as the loose bodies of a baggy hoodie do). The sleeve is shortened by the {} cuff.", fmt(cuff_h, cm)));
        }
        _ => {
            let band_len = 0.85 * neck_open;
            extra.push(rect("NECK BAND", "Cut 1 in rib / stretch knit, long way along the stretch", band_len, 2.0, format!("{} = 85% of neck opening {} + 2 SA", fmt(band_len + 2.0 * sa, cm), fmt(neck_open, cm)), true));
            extra_checks.push(("Neck opening clears the neck (>= neck + 4 in)".into(), format!("{} (neck {})", fmt(neck_open, cm), fmt(p.neck, cm)), neck_open >= p.neck + p.neck_adj + 4.0));
        }
    }

    // ---- checks ----
    let mut checks = vec![];
    let mut chk = |label: &str, value: String, ok: bool| checks.push(Check { label: label.into(), value, ok });
    chk("Side seams match (front vs back)", format!("{} vs {}", fmt(f.side_len, cm), fmt(b.side_len, cm)), (f.side_len - b.side_len).abs() < 0.02);
    chk("Shoulder seams match", format!("{} vs {}", fmt(f.shoulder_len, cm), fmt(b.shoulder_len, cm)), (f.shoulder_len - b.shoulder_len).abs() < 0.02);
    let (alf, alb) = (length(&f.armhole), length(&b.armhole));
    let (cf, cb) = (length(&capf), length(&capb));
    chk("Sleeve cap vs front armhole (half)", format!("cap {} vs armhole {} ({} to ease in)", fmt(cf, cm), fmt(alf, cm), fmt((cf - alf).max(0.0), cm)), (cf - tf).abs() < 0.3 + allowed_ease && cf - alf > -0.1);
    chk("Sleeve cap vs back armhole (half)", format!("cap {} vs armhole {} ({} to ease in)", fmt(cb, cm), fmt(alb, cm), fmt((cb - alb).max(0.0), cm)), cb - alb <= allowed_ease + 0.3 && cb - alb > -0.1);
    for (l, v, ok) in &extra_checks { chk(l, v.clone(), *ok); }
    chk("Finished chest", format!("{} (body {} + ease {})", fmt(4.0 * (x5 - sa), cm), fmt(p.chest, cm), fmt(4.0 * (x5 - sa) - p.chest, cm)), true);
    chk("Finished sleeve opening at bicep line", fmt(2.0 * (xb - sa) * 1.0, cm), true);
    chk("Garment length, shoulder (neck point) to hem", fmt(g.y4 - hem, cm), true);

    // ---- plain-language explanation ----
    let q = p.chest / 4.0;
    let mut explain = vec![
        format!("Block: the numbered-points method. Everything is drawn as half a piece against the fold (the centre line) with {} seam allowance in the cut line; the hem allowance is {}.", fmt(sa, cm), fmt(hem, cm)),
        format!("Fit '{}': finished chest = body chest {} + ease {} (preset {} + your adjustment {}) = {}.", p.fit, fmt(p.chest, cm), fmt(ease, cm), fmt(fit.chest_ease, cm), fmt(p.chest_ease_adj, cm), fmt(p.chest + ease, cm)),
        format!("Point 5 (side seam) = 1/4 chest {} + 1/4 ease {} + SA {} = {}.", fmt(q, cm), fmt(ease / 4.0, cm), fmt(sa, cm), fmt(x5, cm)),
        format!("Point 2 (underarm) = armhole depth {} below point 1 [{} = 0.24 x chest {} + fit drop {} + adjustment {}].", fmt(sigh, cm), fmt(sigh, cm), fmt(p.armhole_depth.unwrap_or(0.24 * p.chest), cm), fmt(fit.armhole_drop, cm), fmt(p.armhole_drop_adj, cm)),
        format!("Point 3 (waist) = centre-back-to-waist {} below point 1; point 4 (hem) = waist + {} below the waist{} + hem {}.", fmt(p.cb_to_waist, cm), fmt(fit.hem_below_waist, cm), if p.length == "long" { " + 4 in long-line" } else { "" }, fmt(hem, cm)),
        format!("Point 10 (neck at the shoulder) = neck/5 + 1/2 in = {} from the centre; point 18 (front neck depth) = neck/4 + 3/4 in = {} down (a tee neck is wider than a shirt neck because there is no collar stand). Back neck depth (point 1) is a fixed {}.", fmt(wn, cm), fmt(fd, cm), fmt(y1, cm)),
        format!("Points 14/16/17: the shoulder seam ends {} from the centre = half your shoulder width {} + dropped-shoulder allowance {} + SA; it drops {} below point 1 (slope; preset {} + adjustment {}).", fmt(x17, cm), fmt(p.shoulder / 2.0, cm), fmt(fit.shoulder_extend, cm), fmt(g.y15 - y1, cm), fmt(fit.shoulder_drop, cm), fmt(p.shoulder_slope_adj, cm)),
        "Point 12 sits halfway down the armhole: the front is pulled in 1/4 in and the back pushed out 1/4 in, because shoulders roll forward and the back needs more room.".to_string(),
        format!("Sleeve: cap height = armhole depth / {:.1} = {}; the cap curve starts as the straight line 0-3 and is bent (up above point 4, slightly down below it) until its length equals the armhole it sews into. Bigger divisor = flatter cap = roomier sleeve.", fit.cap_divisor - p.sleeve_cap_adj, fmt(cap_h, cm)),
        format!("Sleeve width at the bicep line = (bicep {} + slack {})/2 + SA on each side; the sleeve tapers from the cap corners down to it.", fmt(p.bicep, cm), fmt(slack, cm)),
        format!("Neck band = 85% of the neck opening ({}), folded in half.", fmt(neck_open, cm)),
    ];
    if kind != "tshirt" { explain.truncate(9); explain[5] = format!("Points 10-18 as in the tee; the neck is sized for the {} (see below).", kind); }
    explain.extend(extra_explain);
    let kname = match kind { "shirt" => "shirt", "hoodie" => "hoodie", _ => "t-shirt" };
    let title = if kind == "hoodie" { format!("{} hoodie", p.fit) } else { format!("{} {}, {} sleeve", p.fit, kname, if long { "long" } else { "short" }) };
    let info = vec![
        format!("chest {}  neck {}  shoulder {}  bicep {}", fmt(p.chest, cm), fmt(p.neck, cm), fmt(p.shoulder, cm), fmt(p.bicep, cm)),
        format!("wrist {}  CB neck to waist {}", fmt(p.wrist, cm), fmt(p.cb_to_waist, cm)),
        "Print at 100%. Seam allowance 1/2 in included.".to_string(),
    ];
    let mut pieces = vec![f.piece, b.piece, sleeve];
    pieces.extend(extra);
    Ok(Draft { title, info, pieces, explain, instructions: vec![], checks, warnings })
}
