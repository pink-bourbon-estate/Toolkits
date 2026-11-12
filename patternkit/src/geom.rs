//! Small 2-D helpers. All lengths are inches; y points down (paper coordinates).
pub type P = (f64, f64);

pub fn dist(a: P, b: P) -> f64 { ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() }
pub fn lerp(a: P, b: P, t: f64) -> P { (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t) }
pub fn length(pl: &[P]) -> f64 { pl.windows(2).map(|w| dist(w[0], w[1])).sum() }

/// Catmull-Rom spline through `pts`, sampled. Passes through every point; ends keep the chord direction.
pub fn spline(pts: &[P]) -> Vec<P> {
    if pts.len() < 3 { return pts.to_vec(); }
    let mut v = vec![pts[0]];
    v.extend_from_slice(pts);
    v.push(pts[pts.len() - 1]);
    let mut out = vec![];
    for i in 1..v.len() - 2 {
        let (p0, p1, p2, p3) = (v[i - 1], v[i], v[i + 1], v[i + 2]);
        for s in 0..14 {
            let t = s as f64 / 14.0;
            let f = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (-a + 3.0 * b - 3.0 * c + d) * t * t * t)
            };
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out.push(pts[pts.len() - 1]);
    out
}

pub fn bbox(pts: &[P]) -> (P, P) {
    let mut lo = (f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN);
    for p in pts { lo = (lo.0.min(p.0), lo.1.min(p.1)); hi = (hi.0.max(p.0), hi.1.max(p.1)); }
    (lo, hi)
}

fn area2(poly: &[P]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| { let (a, b) = (poly[i], poly[(i + 1) % n]); a.0 * b.1 - b.0 * a.1 }).sum()
}

/// Offset a closed polygon inward. `dist_of(a, b)` gives the offset for each edge (0 for a fold edge).
pub fn offset_inward(poly: &[P], dist_of: &dyn Fn(P, P) -> f64) -> Vec<P> {
    let mut p: Vec<P> = vec![];
    for q in poly { if p.last().map_or(true, |l| dist(*l, *q) > 1e-6) { p.push(*q); } }
    if p.len() > 1 && dist(p[0], p[p.len() - 1]) < 1e-6 { p.pop(); }
    let n = p.len();
    let sign = if area2(&p) > 0.0 { 1.0 } else { -1.0 }; // interior is on the left (-dy, dx) when area2 > 0
    // inward normal of edge a->b
    let nrm = |a: P, b: P| -> P {
        let d = (b.0 - a.0, b.1 - a.1);
        let l = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-12);
        (-sign * d.1 / l, sign * d.0 / l)
    };
    let mut out = vec![];
    for i in 0..n {
        let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
        let (d1, d2) = (dist_of(a, b), dist_of(b, c));
        let (n1, n2) = (nrm(a, b), nrm(b, c));
        // intersect the two offset lines; fall back to a simple shift when nearly parallel
        let (a1, b1) = ((a.0 + n1.0 * d1, a.1 + n1.1 * d1), (b.0 + n1.0 * d1, b.1 + n1.1 * d1));
        let (a2, b2) = ((b.0 + n2.0 * d2, b.1 + n2.1 * d2), (c.0 + n2.0 * d2, c.1 + n2.1 * d2));
        let (r, s) = ((b1.0 - a1.0, b1.1 - a1.1), (b2.0 - a2.0, b2.1 - a2.1));
        let den = r.0 * s.1 - r.1 * s.0;
        let pt = if den.abs() < 1e-6 { (b.0 + n1.0 * d1, b.1 + n1.1 * d1) } else {
            let t = ((a2.0 - a1.0) * s.1 - (a2.1 - a1.1) * s.0) / den;
            let q = (a1.0 + r.0 * t, a1.1 + r.1 * t);
            let lim = d1.max(d2).max(0.1) * 3.0;
            if dist(q, b) > lim { (b.0 + (n1.0 + n2.0) * 0.5 * d1.max(d2), b.1 + (n1.1 + n2.1) * 0.5 * d1.max(d2)) } else { q }
        };
        out.push(pt);
    }
    out
}
