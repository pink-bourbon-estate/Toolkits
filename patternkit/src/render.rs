//! Lay the pieces out on one big sheet (inches), draw them, and cut the sheet into printable pages.
use crate::geom::*;
use crate::params::Params;
use crate::tee::{Draft, Mark, Piece};

fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
fn f(v: f64) -> String { format!("{:.3}", v) }

fn text(x: f64, y: f64, size: f64, fill: &str, s: &str, extra: &str) -> String {
    format!("<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" font-family=\"Helvetica,Arial,sans-serif\" {}>{}</text>\n", f(x), f(y), f(size), fill, extra, esc(s))
}
fn line(a: P, b: P, stroke: &str, w: f64, dash: &str) -> String {
    format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" {}/>\n", f(a.0), f(a.1), f(b.0), f(b.1), stroke, f(w), if dash.is_empty() { String::new() } else { format!("stroke-dasharray=\"{}\"", dash) })
}
fn poly(pts: &[P], stroke: &str, w: f64, dash: &str, fill: &str) -> String {
    let s: Vec<String> = pts.iter().map(|p| format!("{},{}", f(p.0), f(p.1))).collect();
    format!("<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\" {}/>\n", s.join(" "), fill, stroke, f(w), if dash.is_empty() { String::new() } else { format!("stroke-dasharray=\"{}\"", dash) })
}

fn draw_piece(pc: &Piece, p: &Params) -> (String, (P, P)) {
    let cm = p.units == "cm";
    let mut s = String::new();
    // construction layer
    if p.show_construction {
        s += "<g class=\"construction\">\n";
        for m in &pc.marks {
            match m {
                Mark::Line { a, b, label } => {
                    s += &line(*a, *b, "#6aa7d8", 0.012, "0.09 0.06");
                    s += &text(b.0 + 0.05, b.1 - 0.04, 0.15, "#4a86b8", label, if (b.0 - a.0).abs() < 1e-9 { "transform-box=\"fill-box\"" } else { "" });
                }
                Mark::Pt { p: q, id } => {
                    s += &format!("<circle cx=\"{}\" cy=\"{}\" r=\"0.04\" fill=\"#c0392b\"/>\n", f(q.0), f(q.1));
                    s += &text(q.0 + 0.07, q.1 - 0.06, 0.2, "#c0392b", id, "font-weight=\"bold\"");
                }
                Mark::Dim { a, b, off, text: t } => {
                    let vertical = (b.0 - a.0).abs() < (b.1 - a.1).abs();
                    if vertical {
                        let x = a.0 + off;
                        s += &line((a.0, a.1), (x, a.1), "#999", 0.008, "");
                        s += &line((b.0, b.1), (x, b.1), "#999", 0.008, "");
                        s += &line((x, a.1), (x, b.1), "#555", 0.016, "");
                        s += &line((x - 0.06, a.1), (x + 0.06, a.1), "#555", 0.016, "");
                        s += &line((x - 0.06, b.1), (x + 0.06, b.1), "#555", 0.016, "");
                        let (cx, cy) = (x - 0.06, (a.1 + b.1) / 2.0);
                        s += &text(cx, cy, 0.17, "#333", t, &format!("text-anchor=\"middle\" transform=\"rotate(-90 {} {})\"", f(cx), f(cy)));
                    } else {
                        let y = a.1 + off;
                        if off.abs() > 1e-9 { s += &line((a.0, a.1), (a.0, y), "#999", 0.008, ""); s += &line((b.0, b.1), (b.0, y), "#999", 0.008, ""); }
                        s += &line((a.0, y), (b.0, y), "#555", 0.016, "");
                        s += &line((a.0, y - 0.06), (a.0, y + 0.06), "#555", 0.016, "");
                        s += &line((b.0, y - 0.06), (b.0, y + 0.06), "#555", 0.016, "");
                        s += &text((a.0 + b.0) / 2.0, y - 0.07, 0.17, "#333", t, "text-anchor=\"middle\"");
                    }
                }
            }
        }
        s += "</g>\n";
    }
    s += &poly(&pc.outline, "#111", 0.035, "", "#fffef8");
    if !pc.sew.is_empty() { s += &poly(&pc.sew, "#777", 0.018, "0.1 0.07", "none"); }
    if let Some((a, b)) = pc.fold {
        s += &line(a, b, "#1a8a3a", 0.07, "");
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        s += &text(mid.0 + 0.25, mid.1, 0.2, "#1a8a3a", "PLACE ON FOLD", &format!("transform=\"rotate(-90 {} {})\" text-anchor=\"middle\"", f(mid.0 + 0.25), f(mid.1)));
    }
    // grain line with arrowheads
    let (ga, gb) = pc.grain;
    s += &line(ga, gb, "#111", 0.02, "");
    let ar = |tip: P, dir: f64| -> String { format!("<polygon points=\"{},{} {},{} {},{}\" fill=\"#111\"/>\n", f(tip.0), f(tip.1), f(tip.0 - 0.07), f(tip.1 + dir * 0.2), f(tip.0 + 0.07), f(tip.1 + dir * 0.2)) };
    if (gb.0 - ga.0).abs() < 1e-9 { s += &ar(ga, 1.0); s += &ar(gb, -1.0); s += &text(ga.0 + 0.12, (ga.1 + gb.1) / 2.0, 0.15, "#111", "GRAINLINE", &format!("transform=\"rotate(-90 {} {})\" text-anchor=\"middle\"", f(ga.0 + 0.12), f((ga.1 + gb.1) / 2.0))); }
    else { s += &text((ga.0 + gb.0) / 2.0, ga.1 - 0.06, 0.15, "#111", "GRAINLINE (along the stretch)", "text-anchor=\"middle\""); }
    for (a, b) in &pc.notches { s += &line(*a, *b, "#111", 0.03, ""); }
    let (lo, hi) = bbox(&pc.outline);
    // name + cut instruction
    let cx = (lo.0 + hi.0) / 2.0; let cy = lo.1 + (hi.1 - lo.1) * 0.55;
    s += &text(cx, cy, 0.42, "#111", &pc.name, "text-anchor=\"middle\" font-weight=\"bold\"");
    s += &text(cx, cy + 0.32, 0.2, "#111", &pc.cut, "text-anchor=\"middle\"");
    // ruler along the bottom and right edge, zero at the construction origin
    if p.show_ruler {
        s += "<g class=\"ruler\">\n";
        let unit = if cm { 1.0 / 2.54 } else { 1.0 };
        let by = hi.1 + 0.55; let rx = hi.0 + 0.55;
        s += &line((lo.0, by), (hi.0, by), "#555", 0.012, "");
        s += &line((rx, lo.1.min(0.0)), (rx, hi.1), "#555", 0.012, "");
        let mut k = (lo.0 / unit).floor() as i64;
        while (k as f64) * unit <= hi.0 {
            let x = k as f64 * unit; if x >= lo.0 - 1e-9 {
                s += &line((x, by), (x, by + if k % 5 == 0 || !cm { 0.14 } else { 0.09 }), "#555", 0.012, "");
                if !cm || k % 2 == 0 { s += &text(x, by + 0.3, 0.12, "#555", &format!("{}", k), "text-anchor=\"middle\""); }
            }
            k += 1;
        }
        let mut k = 0i64;
        while (k as f64) * unit <= hi.1 {
            let y = k as f64 * unit;
            s += &line((rx, y), (rx + if k % 5 == 0 || !cm { 0.14 } else { 0.09 }, y), "#555", 0.012, "");
            if !cm || k % 2 == 0 { s += &text(rx + 0.18, y + 0.04, 0.12, "#555", &format!("{}", k), ""); }
            k += 1;
        }
        s += &text(lo.0, by + 0.5, 0.13, "#555", if cm { "ruler: cm from the construction origin (the 0 mark)" } else { "ruler: inches from the construction origin (the 0 mark)" }, "");
        s += "</g>\n";
    }
    let mut all = pc.outline.clone();
    all.push((lo.0 - 3.6, lo.1 - 2.2));
    all.push((hi.0 + 1.0, hi.1 + 1.3));
    (s, bbox(&all))
}

pub struct Sheet { pub w: f64, pub h: f64, pub inner: String }

pub fn sheet(d: &Draft, p: &Params) -> Sheet {
    let max_w = 40.0;
    let mut body = String::new();
    let (mut cx, mut cy, mut row_h) = (0.5, 3.0, 0.0f64);
    let mut width: f64 = 0.0;
    for pc in &d.pieces {
        let (g, (lo, hi)) = draw_piece(pc, p);
        let (w, h) = (hi.0 - lo.0, hi.1 - lo.1);
        if cx + w > max_w && cx > 0.5 { cx = 0.5; cy += row_h + 0.4; row_h = 0.0; }
        body += &format!("<g transform=\"translate({},{})\">\n{}</g>\n", f(cx - lo.0), f(cy - lo.1), g);
        cx += w + 0.3; row_h = row_h.max(h); width = width.max(cx);
    }
    let h = cy + row_h + 0.5;
    let mut head = String::new();
    head += &text(0.5, 0.7, 0.45, "#111", &d.title, "font-weight=\"bold\"");
    for (i, l) in d.info.iter().take(3).enumerate() { head += &text(0.5, 1.05 + 0.25 * i as f64, 0.18, "#333", l, ""); }
    head += &poly(&[(0.5, 1.8), (1.5, 1.8), (1.5, 2.8), (0.5, 2.8)], "#111", 0.02, "", "none");
    head += &text(1.65, 2.35, 0.2, "#111", "<- this square must measure 1 in (2.54 cm)", "");
    Sheet { w: width.max(max_w * 0.0 + 8.0) + 0.5, h, inner: format!("{}{}", head, body) }
}

pub struct Tile { pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub row: usize, pub col: usize, pub rows: usize, pub cols: usize, pub marks: String }

pub fn tiles(sh: &Sheet, d: &Draft, p: &Params) -> (Vec<Tile>, (f64, f64)) {
    let (pw, ph) = if p.page == "letter" { (8.5, 11.0) } else { (8.27, 11.69) };
    let (aw, ah) = (pw - 0.5, ph - 0.5);          // printable area (0.25 in margins)
    let ov = 0.5;                                  // neighbouring pages overlap by this much (the unprintable margin)
    let (sx, sy) = (aw - ov, ah - ov);
    let cols = (((sh.w - ov) / sx).ceil() as usize).max(1);
    let rows = (((sh.h - ov) / sy).ceil() as usize).max(1);
    // ink bounding boxes per piece, to skip blank pages
    let mut ink: Vec<(P, P)> = vec![((0.0, 0.0), (8.0, 3.2))];
    let _ = d;
    let mut out = vec![];
    // recompute piece boxes by parsing placement is overkill; use a coarse occupancy grid from the sheet's polygons instead
    ink.clear();
    out.reserve(rows * cols);
    for r in 0..rows { for c in 0..cols {
        let (x, y) = (c as f64 * sx, r as f64 * sy);
        out.push(Tile { x, y, w: aw, h: ah, row: r + 1, col: c + 1, rows, cols, marks: String::new() });
    } }
    (out, (pw, ph))
}

/// Registration crosses (sit inside the overlap, so neighbours' crosses coincide) and the page label.
pub fn tile_marks(t: &Tile) -> String {
    let mut s = String::new();
    let inset = 0.25;
    for (cx, cy) in [(t.x + inset, t.y + inset), (t.x + t.w - inset, t.y + inset), (t.x + inset, t.y + t.h - inset), (t.x + t.w - inset, t.y + t.h - inset)] {
        s += &line((cx - 0.15, cy), (cx + 0.15, cy), "#d00", 0.012, "");
        s += &line((cx, cy - 0.15), (cx, cy + 0.15), "#d00", 0.012, "");
    }
    s += &poly(&[(t.x + 0.25, t.y + 0.25), (t.x + t.w - 0.25, t.y + 0.25), (t.x + t.w - 0.25, t.y + t.h - 0.25), (t.x + 0.25, t.y + t.h - 0.25)], "#d00", 0.006, "0.05 0.1", "none");
    let (lx, ly) = (t.x + t.w - 2.45, t.y + t.h - 0.62);
    s += &format!("<rect x=\"{}\" y=\"{}\" width=\"2.1\" height=\"0.3\" fill=\"#fff\" stroke=\"#d00\" stroke-width=\"0.01\"/>\n", f(lx), f(ly));
    s += &text(lx + 0.08, ly + 0.21, 0.16, "#d00", &format!("row {}/{}  column {}/{}", t.row, t.rows, t.col, t.cols), "font-weight=\"bold\"");
    s
}

/// Does any piece outline reach into this tile? (Cheap test on the drawn sheet via a coarse scan of coordinates.)
pub fn tile_has_ink(inner: &str, t: &Tile) -> bool {
    // scan polygon point lists in the sheet markup, honouring the translate() of the enclosing group
    let mut tx = (0.0, 0.0);
    for chunk in inner.split("<g transform=\"translate(").skip(1) {
        if let Some(end) = chunk.find(")\">") {
            let nums: Vec<f64> = chunk[..end].split(',').filter_map(|v| v.trim().parse().ok()).collect();
            if nums.len() == 2 { tx = (nums[0], nums[1]); }
        }
        for pg in chunk.split("<polygon points=\"").skip(1) {
            if let Some(e) = pg.find('"') {
                for pt in pg[..e].split(' ') {
                    let v: Vec<f64> = pt.split(',').filter_map(|n| n.parse().ok()).collect();
                    if v.len() == 2 {
                        let (x, y) = (v[0] + tx.0, v[1] + tx.1);
                        if x >= t.x && x <= t.x + t.w && y >= t.y && y <= t.y + t.h { return true; }
                    }
                }
            }
        }
    }
    false
}

pub fn page_svg(sh: &Sheet, t: &Tile, paper: (f64, f64)) -> String {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}in\" height=\"{}in\" viewBox=\"{} {} {} {}\">\n<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" overflow=\"hidden\">\n{}</svg>\n{}</svg>\n",
        f(paper.0), f(paper.1), f(t.x - 0.25), f(t.y - 0.25), f(paper.0), f(paper.1),
        f(t.x), f(t.y), f(t.w), f(t.h), f(t.x), f(t.y), f(t.w), f(t.h), sh.inner, tile_marks(t))
}
