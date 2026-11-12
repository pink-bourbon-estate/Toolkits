//! Parametric trousers, following the numbered-points method in Cornelius Quiring's public men's-pant drafting video:
//! four panels (front and back, each cut twice and mirrored), seat line a quarter of the rise above the crotch line, the back made
//! bigger than the front (more rise, more width, a dart), then the back crotch seam "eased" by pivoting the upper back about the
//! outside seat point until the seam is long enough to wrap over the seat. The fit table opens the block up for looser trousers; the
//! alteration knobs are his common fit complaints as numbers.
use crate::geom::*;
use crate::params::*;
use crate::tee::{fmt, Check, Draft, Mark, Piece};

const SA: f64 = 0.5;
const HEM_ALLOW: f64 = 1.5; // trousers are measured to the floor, so the finished hem stops about this far above it

fn rot(q: P, c: P, a: f64) -> P {
    let (dx, dy) = (q.0 - c.0, q.1 - c.1);
    (c.0 + dx * a.cos() - dy * a.sin(), c.1 + dx * a.sin() + dy * a.cos())
}

pub fn draft(p: &Params) -> Result<Draft, String> {
    let fit = trouser_fit(&p.fit)?;
    let cm = p.units == "cm";
    for (v, n) in [(p.waist, "waist"), (p.seat, "seat"), (p.rise, "rise"), (p.inseam, "inseam"), (p.knee_height, "knee_height")] {
        if !(v > 4.0 && v < 80.0) { return Err(format!("{n} = {v} in looks wrong; enter measurements in inches (4-80)")); }
    }
    if p.knee_height >= p.inseam { return Err("knee_height must be less than the inseam (both are measured from the floor)".into()); }
    let mut warnings = vec![];
    let high = p.rise_style == "high";
    let wb_h = if high { 2.5 } else { 2.0 };
    let rise_total = p.rise + if high { 1.5 } else { 0.0 } + p.rise_adj;
    let r = rise_total + SA - wb_h;                    // waist line (y = 0) to crotch line
    if r < 4.0 { return Err("rise is too short for this waistband".into()); }
    let yc = r; let ys = r - r / 4.0;                  // crotch line, seat line (a quarter of the rise above the crotch line)
    let yh = yc + p.inseam + p.length_adj;             // hem edge
    let yk = yh - p.knee_height;
    if yk <= yc + 3.0 { return Err("knee is too close to the crotch; check inseam and knee_height".into()); }
    let hem = p.hem_width.filter(|v| *v > 1.0).unwrap_or(fit.hem);
    let q_e = (fit.seat_extra + p.seat_adj) / 4.0;     // extra ease added to each of the four seat widths
    let xc = p.seat / 8.0 - 0.5;           // centre line to the crotch (fly) line
    let x_in = -xc;
    let cp_x = x_in - xc / 2.0;                        // crotch point
    let thigh = p.thigh_adj / 4.0;                     // each leg half's share of the thigh adjustment

    // ---------------- FRONT ----------------
    let (fw_seat, fw_waist) = (p.seat / 4.0 + 1.0 + q_e, p.waist / 4.0 + 1.0);
    let fly_top = (x_in + 0.5, 0.0);
    let f_wo = (fly_top.0 + fw_waist, 0.0);
    let f_so = (x_in + fw_seat, ys);
    let f_ko = (hem / 4.0 + 0.75, yk);  let f_ho = (hem / 4.0, yh);
    let f_ki = (-(hem / 4.0 + 0.75), yk); let f_hi = (-hem / 4.0, yh);
    let f_cp = (cp_x, yc);
    let fly = spline(&[fly_top, (x_in + 0.1, ys * 0.5), (x_in, ys), (x_in - xc * 0.2, ys + (yc - ys) * 0.6), f_cp]);
    let thigh_y = ys + (yk - ys) * 0.38;
    let lx = |a: P, b: P, y: f64| a.0 + (b.0 - a.0) * (y - a.1) / (b.1 - a.1);
    let up = |s: P, so: P, mid_k: f64| -> Vec<P> { spline(&[s, ((s.0 + (so.0 - s.0) * 0.6) + mid_k, ys * 0.5), so]) };
    let f_up = up(f_wo, f_so, 0.1);
    let f_dn = { let mut v = spline(&[f_so, (lx(f_so, f_ko, thigh_y) + thigh, thigh_y), f_ko]); v.push(f_ho); v };
    let f_ins = { let mut v = spline(&[f_cp, (lx(f_cp, f_ki, thigh_y + 1.0) - thigh, thigh_y + 1.0), f_ki]); v.push(f_hi); v }; // crotch -> hem
    let f_side_len = length(&f_up) + length(&f_dn);
    let f_ins_len = length(&f_ins);
    let f_curve_len = length(&fly);

    // ---------------- BACK ----------------
    let xb = x_in - xc / 4.0;                          // back crotch line is further out than the front's (the squashed-flat bum)
    let (bw_seat, bw_waist) = (p.seat / 4.0 + 1.25 + q_e, p.waist / 4.0 + 1.75);
    let b_top0 = (xb + 1.0, -0.5);
    let b_cp = (xb - xc / 4.0, yc + 0.25);
    let b_so = (xb + bw_seat, ys);
    let b_ko = (hem / 4.0 + 1.5, yk); let b_ho = (hem / 4.0 + 0.75, yh);
    let b_ki = (-(hem / 4.0 + 1.5), yk); let b_hi = (-(hem / 4.0 + 0.75), yh);
    let b_mid0 = (xb, r / 2.0);
    let mut s_y = 0.0;
    let mk_s = |sy: f64| -> P { ((b_top0.0 + (bw_waist * bw_waist - (sy - b_top0.1).powi(2)).max(0.0).sqrt()), sy) };
    let b_dn = { let mut v = spline(&[b_so, (lx(b_so, b_ko, thigh_y) + thigh, thigh_y), b_ko]); v.push(b_ho); v };
    let b_ins = { let mut v = spline(&[b_cp, (lx(b_cp, b_ki, thigh_y + 1.0) - thigh, thigh_y + 1.0), b_ki]); v.push(b_hi); v };
    // the outside seams must be the same length: if the back is longer, taper it down at the waist (as in the video)
    for _ in 0..8 {
        let bu = up(mk_s(s_y), b_so, 0.1);
        let d = length(&bu) + length(&b_dn) - f_side_len;
        if d.abs() < 0.01 { break; }
        s_y += d;
    }
    let b_s0 = mk_s(s_y);
    let b_up0 = up(b_s0, b_so, 0.1);
    // easing: pivot the part above the seat line about the outside seat point until the back crotch seam has grown by `ease`
    let h = xc / 4.0; // the back crotch seam drops almost straight, then hooks out to the crotch point
    let cb = |t: P, m: P| spline(&[t, m, (xb - 0.03, ys), (xb - 0.1 * h, yc - 0.6), (xb - 0.45 * h, yc + 0.08), (xb - 0.8 * h, yc + 0.2), b_cp]);
    let cb0 = cb(b_top0, b_mid0);
    let cb0_len = length(&cb0);
    let wbl = 2.0 * wb_h;                               // both waistband heights count toward the crotch length
    let base_ease = fit.back_ease + p.crotch_ease_adj;
    let ease = match p.crotch_length {
        Some(cl) if cl > 10.0 => {
            let e = cl - (cb0_len + f_curve_len + wbl);
            if e < 0.0 || e > 6.0 { warnings.push(format!("Your crotch-length measurement implies {} of back easing, which is unusual (usual range 0 to 6 in); re-measure, or leave it blank to use the fit's default.", fmt(e, cm))); }
            e.clamp(0.0, 6.0) + p.crotch_ease_adj
        }
        _ => base_ease,
    }.max(0.0);
    let piv = b_so;
    let (mut lo, mut hi) = (0.0_f64, 0.9_f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        let l = length(&cb(rot(b_top0, piv, mid), rot(b_mid0, piv, mid)));
        if l - cb0_len < ease { lo = mid } else { hi = mid }
    }
    let ang = (lo + hi) / 2.0;
    let b_top = rot(b_top0, piv, ang);
    let b_s = rot(b_s0, piv, ang);
    let b_up: Vec<P> = b_up0.iter().map(|q| rot(*q, piv, ang)).collect();
    let b_mid = rot(b_mid0, piv, ang);
    let cbc = cb(b_top, b_mid);
    let b_side_len = length(&b_up) + length(&b_dn);
    let b_ins_len = length(&b_ins);
    let b_curve_len = length(&cbc);
    if ang > 0.45 { warnings.push("The back easing is very large; check the crotch-length measurement or reduce the seat ease adjustment.".into()); }
    // dart
    let wd = (b_s.0 - b_top.0, b_s.1 - b_top.1); let wl = (wd.0 * wd.0 + wd.1 * wd.1).sqrt(); let wu = (wd.0 / wl, wd.1 / wl);
    let mut wn = (-wu.1, wu.0); if wn.1 < 0.0 { wn = (-wn.0, -wn.1); }
    let dm = lerp(b_top, b_s, 0.5);
    let (d1, d2, dapex) = ((dm.0 - wu.0 * 0.5, dm.1 - wu.1 * 0.5), (dm.0 + wu.0 * 0.5, dm.1 + wu.1 * 0.5), (dm.0 + wn.0 * 4.5, dm.1 + wn.1 * 4.5));

    // ---------------- outlines, sew lines, marks ----------------
    let f_outline = { let mut v = vec![fly_top, f_wo]; v.extend(f_up.iter().skip(1).cloned()); v.extend(f_dn.iter().skip(1).cloned());
        v.push(f_hi); v.extend(f_ins.iter().rev().skip(1).cloned()); v.extend(fly.iter().rev().skip(1).take(fly.len() - 2).cloned()); v };
    let b_outline = { let mut v = vec![b_top, b_s]; v.extend(b_up.iter().skip(1).cloned()); v.extend(b_dn.iter().skip(1).cloned());
        v.push(b_hi); v.extend(b_ins.iter().rev().skip(1).cloned()); v.extend(cbc.iter().rev().skip(1).take(cbc.len() - 2).cloned()); v };
    let hem_d = move |a: P, b: P| -> f64 { if (a.1 - yh).abs() < 1e-6 && (b.1 - yh).abs() < 1e-6 { HEM_ALLOW } else { SA } };
    let xl = cp_x - 0.7;
    let common = |front: bool| -> Vec<Mark> {
        let xe = if front { f_wo.0.max(f_so.0) + 0.4 } else { b_so.0.max(b_s.0) + 0.5 };
        let mut m = vec![];
        for (y, lab) in [(0.0, "waist line (waistband is added on top)"), (ys, "seat line: 1/4 of the rise above the crotch line"), (yc, "crotch (body rise) line"), (yk, "knee line"), (yh, "hem edge (measured to the floor)")] {
            m.push(Mark::Line { a: (xl + 0.3, y), b: (xe, y), label: lab.into() });
        }
        m.push(Mark::Line { a: (0.0, -0.3), b: (0.0, yh + 0.3), label: "leg centre line".into() });
        m
    };
    let dx_dim = |front: bool| -> Vec<Mark> {
        let (hi_, ho_) = if front { (f_hi.0, f_ho.0) } else { (b_hi.0, b_ho.0) };
        vec![
            Mark::Dim { a: (xl, 0.0), b: (xl, yc), off: -0.8, text: format!("rise {} (= body rise {} + SA {} - waistband {})", fmt(r, cm), fmt(rise_total, cm), fmt(SA, cm), fmt(wb_h, cm)) },
            Mark::Dim { a: (xl, yc), b: (xl, yh), off: -0.8, text: format!("inseam {}", fmt(yh - yc, cm)) },
            Mark::Dim { a: (xl, yk), b: (xl, yh), off: -2.0, text: format!("knee {} up", fmt(p.knee_height, cm)) },
            Mark::Dim { a: (hi_, yh), b: (ho_, yh), off: 0.9, text: format!("hem {}", fmt(ho_ - hi_, cm)) },
        ]
    };
    let mut fm = common(true);
    fm.push(Mark::Line { a: (x_in, -0.3), b: (x_in, yc + 0.3), label: "".into() });
    for (pt, id) in [(fly_top, "1"), (f_wo, "2"), (f_so, "3"), (f_ko, "4"), (f_ho, "5"), (f_hi, "6"), (f_ki, "7"), (f_cp, "8"), ((x_in, ys), "9"), ((x_in, yc), "10"), ((0.0, 0.0), "0")] {
        fm.push(Mark::Pt { p: pt, id: id.into() });
    }
    fm.extend(dx_dim(true));
    fm.push(Mark::Dim { a: (x_in, ys), b: f_so, off: 0.0, text: format!("1/4 seat + 1 = {}", fmt(fw_seat, cm)) });
    fm.push(Mark::Dim { a: fly_top, b: f_wo, off: -0.6, text: format!("1/4 waist + 1 = {}", fmt(fw_waist, cm)) });
    fm.push(Mark::Dim { a: (x_in, yc + 0.5), b: (0.0, yc + 0.5), off: 0.0, text: format!("seat/8 - 1/2 = {}", fmt(xc, cm)) });
    fm.push(Mark::Dim { a: (x_in, yc + 1.4), b: (cp_x, yc + 1.4), off: 0.0, text: format!("+ half = {}", fmt(xc / 2.0, cm)) });
    let front = Piece { name: "FRONT".into(), cut: "Cut 2 (mirror: one left, one right)".into(), sew: if p.show_sew_line { offset_inward(&f_outline, &hem_d) } else { vec![] },
        outline: f_outline, fold: None, grain: ((0.0, ys + 1.0), (0.0, yk - 1.0)),
        notches: vec![((f_so.0, ys), (f_so.0 - 0.3, ys)), ((f_ki.0, yk), (f_ki.0 + 0.3, yk))], marks: fm };

    let mut bm = common(false);
    bm.push(Mark::Line { a: (xb, -0.3), b: (xb, yc + 0.3), label: "".into() });
    bm.push(Mark::Line { a: d1, b: dapex, label: "".into() });
    bm.push(Mark::Line { a: d2, b: dapex, label: "waist dart: 1 in wide, 4 1/2 in long".into() });
    for (pt, id) in [(b_top, "1"), (b_s, "2"), (b_so, "3"), (b_ko, "4"), (b_ho, "5"), (b_hi, "6"), (b_ki, "7"), (b_cp, "8"), (b_mid, "9"), ((0.0, 0.0), "0")] {
        bm.push(Mark::Pt { p: pt, id: id.into() });
    }
    bm.extend(dx_dim(false));
    bm.push(Mark::Dim { a: (xb, ys), b: b_so, off: 0.0, text: format!("1/4 seat + 1 1/4 = {}", fmt(bw_seat, cm)) });
    bm.push(Mark::Dim { a: (xb, yc + 0.5), b: (x_in, yc + 0.5), off: 0.0, text: format!("front-to-back line shift = 1/4 of {} = {}", fmt(xc, cm), fmt(xc / 4.0, cm)) });
    bm.push(Mark::Dim { a: b_top, b: b_s, off: -0.7, text: format!("1/4 waist + 1 3/4 = {} (1 in is the dart)", fmt(bw_waist, cm)) });
    let back = Piece { name: "BACK".into(), cut: "Cut 2 (mirror: one left, one right)".into(), sew: if p.show_sew_line { offset_inward(&b_outline, &hem_d) } else { vec![] },
        outline: b_outline, fold: None, grain: ((0.0, ys + 1.0), (0.0, yk - 1.0)),
        notches: vec![((b_so.0, ys), (b_so.0 - 0.3, ys)), ((b_ki.0, yk), (b_ki.0 + 0.3, yk))], marks: bm };

    // waistband: finished waist opening = what the panels give once the seam allowances are taken out
    let fin_waist = 2.0 * (fw_waist + bw_waist - 1.0) - 4.0 * SA;
    let band_l = fin_waist + 1.5 + 2.0 * SA;            // 1.5 in underlap for the button
    let band_h = 2.0 * wb_h + 2.0 * SA;
    let wb = Piece { name: "WAISTBAND".into(), cut: "Cut 1 (+ interfacing)".into(), outline: vec![(0.0, 0.0), (band_l, 0.0), (band_l, band_h), (0.0, band_h)],
        sew: if p.show_sew_line { vec![(SA, SA), (band_l - SA, SA), (band_l - SA, band_h - SA), (SA, band_h - SA)] } else { vec![] }, fold: None,
        grain: ((band_l * 0.3, band_h / 2.0), (band_l * 0.7, band_h / 2.0)), notches: vec![((band_l / 2.0, 0.0), (band_l / 2.0, 0.3))],
        marks: vec![Mark::Line { a: (-0.2, band_h / 2.0), b: (band_l + 0.2, band_h / 2.0), label: "fold line".into() },
            Mark::Dim { a: (0.0, band_h + 0.4), b: (band_l, band_h + 0.4), off: 0.0, text: format!("{} = waist opening {} + 1 1/2 in underlap + 2 SA", fmt(band_l, cm), fmt(fin_waist, cm)) },
            Mark::Dim { a: (band_l + 0.4, 0.0), b: (band_l + 0.4, band_h), off: 0.0, text: format!("{} = 2 x {} + 2 SA", fmt(band_h, cm), fmt(wb_h, cm)) }] };

    // ---------------- checks ----------------
    let mut checks = vec![];
    let mut chk = |l: &str, v: String, ok: bool| checks.push(Check { label: l.into(), value: v, ok });
    chk("Outside seams match (front vs back)", format!("{} vs {}", fmt(f_side_len, cm), fmt(b_side_len, cm)), (f_side_len - b_side_len).abs() < 0.1);
    chk("Inside leg seams match within 1/2 in (front vs back)", format!("{} vs {}", fmt(f_ins_len, cm), fmt(b_ins_len, cm)), (f_ins_len - b_ins_len).abs() < 0.6);
    chk("Back easing pivoted in (about 2 to 3 in is normal)", format!("{} (back crotch seam {} -> {}, front {})", fmt(ease, cm), fmt(cb0_len, cm), fmt(b_curve_len, cm), fmt(f_curve_len, cm)), ease <= 4.5 && (b_curve_len - cb0_len - ease).abs() < 0.1);
    let fin_seat = 2.0 * (fw_seat + bw_seat) - 4.0 * SA;
    chk("Finished seat", format!("{} (body {} + ease {})", fmt(fin_seat, cm), fmt(p.seat, cm), fmt(fin_seat - p.seat, cm)), fin_seat >= p.seat + 1.0);
    chk("Finished waist opening (waist to waist + 3 in)", format!("{} (body {})", fmt(fin_waist, cm), fmt(p.waist, cm)), fin_waist >= p.waist && fin_waist <= p.waist + 3.0);
    chk("Leg opening around the hem", format!("{} (front {} + back {}, minus seam allowances)", fmt(f_ho.0 - f_hi.0 + b_ho.0 - b_hi.0 - 2.0 * SA, cm), fmt(f_ho.0 - f_hi.0, cm), fmt(b_ho.0 - b_hi.0, cm)), true);
    chk("Finished inseam (crotch to hem)", fmt(yh - yc - HEM_ALLOW, cm), true);
    chk("Finished rise, crotch to top of waistband", format!("{} (front) / {} (back)", fmt(r + wb_h - SA, cm), fmt(b_cp.1 - b_top.1 + wb_h - SA, cm)), true);

    let explain = vec![
        format!("Four panels: front and back, each cut twice and mirrored. The back is bigger than the front because the muscle and seat at the back need more cloth; seam allowance is {} in the cut line (the hem has {} since it is measured to the floor).", fmt(SA, cm), fmt(HEM_ALLOW, cm)),
        format!("Rise: the seated body rise {} {} {} = {} below the top of the waistband; the waistband height {} is subtracted because it is added back as a separate piece, and {} seam allowance is added for sewing it on. The seat line sits a quarter of this rise ({}) above the crotch line, where the seat is fullest.", fmt(p.rise, cm), if high { "+ 1 1/2 in (high-waisted)" } else { "" }, if p.rise_adj != 0.0 { format!("+ adjustment {}", fmt(p.rise_adj, cm)) } else { String::new() }, fmt(rise_total, cm), fmt(wb_h, cm), fmt(SA, cm), fmt(r / 4.0, cm)),
        format!("Fit '{}': seat ease beyond the tutorial block = {} in total (preset {} + your adjustment {}), spread as {} on each of the four widths; the block itself gives about 2 1/2 in at the seat once the seam allowances are taken out.", p.fit, fmt(fit.seat_extra + p.seat_adj, cm), fmt(fit.seat_extra, cm), fmt(p.seat_adj, cm), fmt(q_e, cm)),
        format!("Front crotch line: seat/8 - 1/2 in = {} from the centre line (the 1/2 in takes up the way the leg swivels inside the hip); the crotch point is half as much again ({}) beyond it. The fly curve runs from 1/2 in inside the line at the waist, down through the seat line, to the crotch point.", fmt(xc, cm), fmt(xc / 2.0, cm)),
        format!("Front widths: waist = 1/4 waist + 1 in = {}; seat = 1/4 seat + 1 in {} = {}, measured from the fly line; the side seam is drawn through those and smoothed over the hip.", fmt(fw_waist, cm), if q_e > 0.0 { format!("+ ease {}", fmt(q_e, cm)) } else { String::new() }, fmt(fw_seat, cm)),
        format!("Leg: hem opening {} (preset {} unless you entered one); the front goes out 1/4 of that each side at the hem and 3/4 in more at the knee. The back goes 3/4 in further at the hem and again at the knee, because the back of the leg is flattened out when the leg is squashed to make the pattern.", fmt(hem, cm), fmt(fit.hem, cm)),
        format!("Back: the crotch line is shifted a quarter of the front offset ({}) further out; the centre-back seam rises 1/2 in above the waist line and starts 1 in in (more rise at the back), then curves to a crotch point {} lower than the front's so the two inside leg seams stay the same length.", fmt(xc / 4.0, cm), fmt(0.25, cm)),
        format!("Back waist = 1/4 waist + 1 3/4 in = {}: the extra 3/4 in over the front plus 1 in that the dart takes out. The dart is 1 in wide and 4 1/2 in long at the middle of the waist edge.", fmt(bw_waist, cm)),
        format!("Outside seam check: the back outside seam was tapered down at the waist by {} so it matches the front ({} each) before easing.", fmt(s_y, cm), fmt(f_side_len, cm)),
        format!("Easing: the back crotch seam is lengthened by {} ({}) by pivoting everything above the seat line about the outside seat point by {:.1} degrees, then redrawing the seam. That is the extra cloth that wraps over the seat. If you see drag lines in a V under the seat, increase the easing; if the seat sags, reduce it.", fmt(ease, cm), match p.crotch_length { Some(cl) if cl > 10.0 => format!("from your crotch length {} minus the pattern's curves and waistband heights", fmt(cl, cm)), _ => format!("preset {} + your adjustment {}", fmt(fit.back_ease, cm), fmt(p.crotch_ease_adj, cm)) }, ang.to_degrees()),
        format!("Waistband: finished waist opening {} (the panels' waist edges minus the darts and seam allowances) + 1 1/2 in underlap, {} wide when finished, folded.", fmt(fin_waist, cm), fmt(wb_h, cm)),
        "Thigh too loose or tight: use the thigh slider; it moves the thigh (just below the crotch) on both the outside and inside seams, as in the fit video. Walk the seams afterwards: the outside seams must still match, and the inside seams too.".to_string(),
    ];
    let title = format!("{} {} trousers", p.fit, if high { "high-waisted" } else { "standard-rise" });
    let info = vec![
        format!("waist {}  seat {}  rise {}  inseam {}", fmt(p.waist, cm), fmt(p.seat, cm), fmt(p.rise, cm), fmt(p.inseam, cm)),
        format!("knee height {}  leg opening {}", fmt(p.knee_height, cm), fmt(hem, cm)),
        "Print at 100%. Seam allowance 1/2 in included (hem 1 1/2 in).".to_string(),
    ];
    Ok(Draft { title, info, pieces: vec![front, back, wb], explain, instructions: vec![], checks, warnings })
}
