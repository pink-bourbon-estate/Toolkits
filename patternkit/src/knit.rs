//! Basic knitting pattern: a drop-shoulder crew-neck pullover in stocking stitch with ribbed hem, cuffs and neckband. The same body
//! measurements and fit presets as the sewn tops drive the finished sizes; your gauge turns inches into stitches and rows. Output is
//! a schematic (finished-size shapes with stitch counts) plus written instructions.
use crate::params::*;
use crate::tee::{fmt, Check, Draft, Mark, Piece};

fn r4(v: f64) -> i64 { ((v / 4.0).round() as i64).max(1) * 4 }  // multiple of 4 so k2,p2 rib repeats evenly
fn r2(v: f64) -> i64 { ((v / 2.0).round() as i64).max(1) * 2 }
fn even_rows(v: f64) -> i64 { ((v / 2.0).round() as i64).max(1) * 2 }

pub fn draft(p: &Params) -> Result<Draft, String> {
    let fit = fit_table(&p.fit)?;
    let cm = p.units == "cm";
    for (v, n) in [(p.chest, "chest"), (p.neck, "neck"), (p.shoulder, "shoulder"), (p.wrist, "wrist"), (p.cb_to_waist, "cb_to_waist"), (p.sleeve_to_wrist, "sleeve_to_wrist")] {
        if !(v > 4.0 && v < 80.0) { return Err(format!("{n} = {v} in looks wrong; enter measurements in inches (4-80)")); }
    }
    if !(8.0..=60.0).contains(&p.gauge_sts) || !(8.0..=90.0).contains(&p.gauge_rows) { return Err("gauge looks wrong: enter stitches and rows per 4 in (10 cm), for example 20 and 28".into()); }
    let (spi, rpi) = (p.gauge_sts / 4.0, p.gauge_rows / 4.0);
    let ease = fit.chest_ease + p.chest_ease_adj;
    let w = (p.chest + ease) / 2.0;                    // flat width of front and of back
    let nk = p.neck + p.neck_adj;
    let len = p.cb_to_waist + fit.hem_below_waist + if p.length == "long" { 4.0 } else { 0.0 } + p.length_adj;
    let sigh = p.armhole_depth.unwrap_or(0.24 * p.chest) + fit.armhole_drop + p.armhole_drop_adj;
    let rib = 2.5;                                      // ribbing depth
    let mut warnings = vec![];

    // ---- body ----
    let body_sts = r4(w * spi);
    let w_act = body_sts as f64 / spi;
    let neck_w = nk / 2.3;
    let neck_sts = r2(neck_w * spi);
    let sh_sts = (body_sts - neck_sts) / 2;
    let (fd, bd) = (nk / 6.0 + 0.75, 0.75);             // front and back neck depth
    let fd_rows = even_rows(fd * rpi);
    let bd_rows = even_rows(bd * rpi);
    let c_sts = r2(0.6 * neck_sts as f64);              // centre bound off at the front
    let d_each = (neck_sts - c_sts) / 2;                // then decreased away, 1 st every right-side row
    let dec_rows = 2 * d_each;
    if dec_rows > fd_rows { warnings.push("The neck shaping needs more rows than the front neck depth gives at this gauge; the neck will be a little deeper than planned.".into()); }
    let body_rows = even_rows((len - rib) * rpi);
    let rib_rows = even_rows(rib * rpi);
    let to_neck_rows = body_rows - fd_rows;             // knit this far before the front neck
    let plain_after = (fd_rows - dec_rows).max(0);
    let sh_w = sh_sts as f64 / spi;

    // ---- sleeve ----
    let cuff_sts = r4((p.wrist + 1.5) / 2.0 * spi);
    let top_w = 1.7 * sigh;
    let top_sts = r2(top_w * spi);
    let drop = (w / 2.0 - (p.shoulder / 2.0 + fit.shoulder_extend)).max(0.0);
    let slen = p.sleeve_to_wrist - drop + p.sleeve_length_adj;
    let s_rows = even_rows((slen - rib) * rpi);
    let incs = ((top_sts - cuff_sts) / 2).max(0);       // increases on each edge
    let gap = if incs > 0 { (((s_rows as f64 - 6.0) / incs as f64).floor() as i64 / 2 * 2).max(2) } else { 0 };
    let inc_rows_used = gap * incs;
    let plain_end = (s_rows - inc_rows_used).max(0);
    if incs > 0 && inc_rows_used > s_rows { warnings.push("The sleeve shaping needs more rows than the sleeve length gives; use fewer increases or a looser gauge.".into()); }
    let top_act = (cuff_sts + 2 * incs) as f64 / spi;

    // ---- neckband ----
    let pick_up = r4(neck_sts as f64 + c_sts as f64 + 2.0 * fd_rows as f64 * 0.75);
    let band_rows = even_rows(1.0 * rpi);

    // ---- schematic pieces (finished shapes, inches, y down from the shoulder line) ----
    let (se, spi_, rpi_) = (sh_w, spi, rpi);
    let back_o = vec![(0.0, 0.0), (se, 0.0), (se, bd), (w_act - se, bd), (w_act - se, 0.0), (w_act, 0.0), (w_act, len), (0.0, len)];
    let dx = d_each as f64 / spi_; let dy = dec_rows as f64 / rpi_;
    let front_o = vec![(0.0, 0.0), (se, 0.0), (se, fd - dy), (se + dx, fd), (w_act - se - dx, fd), (w_act - se, fd - dy), (w_act - se, 0.0), (w_act, 0.0), (w_act, len), (0.0, len)];
    let dims_body = |name: &str, neck_note: String| -> Vec<Mark> { vec![
        Mark::Line { a: (-0.2, len - rib), b: (w_act + 0.2, len - rib), label: format!("rib ({} in, {} rows)", rib, rib_rows) },
        Mark::Dim { a: (0.0, len + 0.2), b: (w_act, len + 0.2), off: 0.6, text: format!("{} sts = {} wide (chest {} + ease {}) / 2", body_sts, fmt(w_act, cm), fmt(p.chest, cm), fmt(ease, cm)) },
        Mark::Dim { a: (0.0, 0.0), b: (0.0, len), off: -0.9, text: format!("{} long = {} rows (+ rib {} rows)", fmt(len, cm), body_rows, rib_rows) },
        Mark::Dim { a: (0.0, -0.3), b: (se, -0.3), off: 0.0, text: format!("shoulder {} sts", sh_sts) },
        Mark::Pt { p: (w_act / 2.0, 0.0), id: name.chars().next().unwrap_or('B').to_string() },
        Mark::Line { a: (w_act / 2.0, -0.3), b: (w_act / 2.0, 0.0), label: neck_note }, ] };
    let back = Piece { name: "BACK".into(), cut: "Knit 1".into(), sew: vec![], outline: back_o, fold: None, grain: ((w_act * 0.5, len * 0.3), (w_act * 0.5, len * 0.7)), notches: vec![],
        marks: dims_body("B", format!("back neck: bind off {} sts", neck_sts)) };
    let front = Piece { name: "FRONT".into(), cut: "Knit 1".into(), sew: vec![], outline: front_o, fold: None, grain: ((w_act * 0.5, len * 0.3), (w_act * 0.5, len * 0.7)), notches: vec![],
        marks: dims_body("F", format!("front neck: bind off {} sts, then dec {} each side", c_sts, d_each)) };
    let xm = top_act.max(top_w) / 2.0;
    let wc = cuff_sts as f64 / spi;
    let sl_o = vec![(xm - wc / 2.0, slen), (xm - wc / 2.0, slen - rib), (xm - top_act / 2.0, 0.0), (xm + top_act / 2.0, 0.0), (xm + wc / 2.0, slen - rib), (xm + wc / 2.0, slen)];
    let sleeve = Piece { name: "SLEEVE".into(), cut: "Knit 2".into(), sew: vec![], outline: sl_o, fold: None, grain: ((xm, slen * 0.3), (xm, slen * 0.6)), notches: vec![],
        marks: vec![
            Mark::Dim { a: (xm - top_act / 2.0, -0.2), b: (xm + top_act / 2.0, -0.2), off: -0.3, text: format!("{} sts = {} (1.7 x armhole depth {})", cuff_sts + 2 * incs, fmt(top_act, cm), fmt(sigh, cm)) },
            Mark::Dim { a: (xm - wc / 2.0, slen + 0.2), b: (xm + wc / 2.0, slen + 0.2), off: 0.6, text: format!("cuff {} sts = {} (wrist {} + 1 1/2 in) / 2", cuff_sts, fmt(wc, cm), fmt(p.wrist, cm)) },
            Mark::Dim { a: (xm - top_act / 2.0, 0.0), b: (xm - top_act / 2.0, slen), off: -0.9, text: format!("{} long (shoulder point to wrist {} - drop {})", fmt(slen, cm), fmt(p.sleeve_to_wrist, cm), fmt(drop, cm)) },
            Mark::Line { a: (xm - top_act / 2.0 - 0.2, slen - rib), b: (xm + top_act / 2.0 + 0.2, slen - rib), label: "rib".into() },
        ] };

    // ---- written instructions ----
    let ins = vec![
        format!("Gauge: {} sts and {} rows = 4 in (10 cm) in stocking stitch, after washing and drying a swatch. Everything below is computed from this; if your swatch differs, change the gauge numbers and the counts update.", p.gauge_sts, p.gauge_rows),
        format!("Sizes: finished chest {} (body {} + ease {}), length {} from shoulder to hem, sleeve {} from shoulder point to wrist. Needles: main size for stocking stitch, one or two sizes smaller for ribbing. Abbreviations: k knit, p purl, st(s) stitch(es), RS right side, WS wrong side, dec decrease, inc increase.", fmt(2.0 * w_act, cm), fmt(p.chest, cm), fmt(ease, cm), fmt(len, cm), fmt(p.sleeve_to_wrist, cm)),
        format!("BACK. Cast on {} sts with the smaller needles. Work k2, p2 rib for {} rows ({} in). Change to the main needles and work stocking stitch (knit on RS, purl on WS) for {} rows, until the piece measures {} from the cast-on edge.", body_sts, rib_rows, rib, body_rows - bd_rows, fmt(len - bd, cm)),
        format!("Back neck and shoulders. Next row (RS): knit {} sts, bind off the middle {} sts, knit to end. Work each shoulder separately for {} rows ({}) and bind off its {} sts. (A back neck this shallow needs no further shaping.)", sh_sts, neck_sts, bd_rows, fmt(bd, cm), sh_sts),
        format!("FRONT. Work as the back until the piece measures {} (the neck starts {} rows before the shoulder line: row {} of the stocking stitch). Next row (RS): knit {} sts, bind off the middle {} sts, knit to end.", fmt(len - fd, cm), fd_rows, to_neck_rows, sh_sts + d_each, c_sts),
        format!("Front neck shaping. Working each side separately, dec 1 st at the neck edge on every RS row, {} times ({} rows); you are left with {} sts. Work {} more rows straight, then bind off the {} shoulder sts. Repeat on the other side, reversing the shaping.", d_each, dec_rows, sh_sts, plain_after, sh_sts),
        format!("SLEEVES (make 2). Cast on {} sts with the smaller needles and work k2, p2 rib for {} rows ({} in). Change to the main needles. {} Then work {} rows straight until the sleeve measures {} from the cast-on edge, ending on a WS row. Bind off all {} sts.",
            cuff_sts, rib_rows, rib,
            if incs > 0 { format!("Increase row (RS): k1, make 1, knit to the last st, make 1, k1. Repeat this increase row every {} rows, {} times in all, to reach {} sts.", gap, incs, cuff_sts + 2 * incs) } else { "No shaping is needed at this size.".to_string() },
            plain_end, fmt(slen, cm), cuff_sts + 2 * incs),
        "Finishing. Block each piece flat to the sizes on the schematic. Sew the shoulder seams (bound-off edges together). Fold each sleeve in half and match its centre to the shoulder seam; sew the sleeve top to the body (a dropped shoulder has no armhole curve: the sleeve top is sewn straight onto the side edge). Sew the side seams and the sleeve seams in one line from hem to cuff.".to_string(),
        format!("Neckband. With the smaller needles and RS facing, pick up and knit about {} sts evenly around the neck (about 1 st per bound-off st and 3 sts for every 4 rows along the shaped edges; adjust to a multiple of 4). Work k2, p2 rib for {} rows (1 in), bind off loosely in rib (over a needle two sizes larger if it is tight). Sew the end of the band down.", pick_up, band_rows),
        "Fit: the neck opening should pass over the head (about 21 in or 53 cm around), and the cuffs over the hand. If the neckband is tight, bind off more loosely or pick up one more stitch per inch.".to_string(),
    ];

    let neck_open_in = std::f64::consts::PI * ((neck_w * neck_w + (fd + bd) * (fd + bd)) / 2.0).sqrt();
    let mut checks = vec![];
    let mut chk = |l: &str, v: String, ok: bool| checks.push(Check { label: l.into(), value: v, ok });
    chk("Back stitch count = shoulders + neck", format!("{} + {} + {} = {}", sh_sts, neck_sts, sh_sts, 2 * sh_sts + neck_sts), 2 * sh_sts + neck_sts == body_sts);
    chk("Stitch counts fit the rib (multiple of 4 at the cast-on edge)", format!("body {}, cuff {}", body_sts, cuff_sts), body_sts % 4 == 0 && cuff_sts % 4 == 0);
    chk("Front neck: bound-off + 2 x decreases = neck stitches", format!("{} + 2 x {} = {}", c_sts, d_each, c_sts + 2 * d_each), c_sts + 2 * d_each == neck_sts);
    chk("Neck shaping fits the front neck depth", format!("{} rows of shaping, {} rows available", dec_rows, fd_rows), dec_rows <= fd_rows);
    chk("Sleeve shaping fits the sleeve length", if incs > 0 { format!("{} increases every {} rows = {} of {} rows", incs, gap, inc_rows_used, s_rows) } else { "none needed".into() }, inc_rows_used <= s_rows);
    chk("Neck opening clears the neck (>= neck + 1 in; rib stretches)", format!("about {} (neck {})", fmt(neck_open_in, cm), fmt(nk, cm)), neck_open_in >= nk + 1.0);
    chk("Finished chest", format!("{} (body {} + ease {})", fmt(2.0 * w_act, cm), fmt(p.chest, cm), fmt(2.0 * w_act - p.chest, cm)), true);
    let explain = vec![
        format!("A knitted piece is shaped by counting, not cutting: finished width in inches x stitches per inch = stitches to cast on. Gauge {} sts / 4 in gives {:.2} sts per inch and {} rows / 4 in gives {:.2} rows per inch.", p.gauge_sts, spi, p.gauge_rows, rpi),
        format!("Body width: finished chest = body chest {} + ease {} (the same '{}' fit preset as the sewn tops); each of the front and back is half of it = {}, i.e. {} stitches rounded to a multiple of 4 for the 2x2 rib.", fmt(p.chest, cm), fmt(ease, cm), p.fit, fmt(w, cm), body_sts),
        format!("Drop shoulder: there is no armhole curve. The sleeve top is as wide as 1.7 x the armhole depth ({}), sewn straight onto the body's side edge, which is why the {} fit is roomy under the arm; the armhole depth {} is also what sets the sleeve-top width.", fmt(sigh, cm), p.fit, fmt(sigh, cm)),
        format!("Neck: opening width = neck / 2.3 = {} ({} sts); the back neck is {} deep and the front {} (neck/6 + 3/4 in); the front centre {} sts are bound off and the rest decreased away on RS rows to make a rounded edge.", fmt(neck_w, cm), neck_sts, fmt(bd, cm), fmt(fd, cm), c_sts),
        format!("Sleeve: the cuff is wrist + 1 1/2 in ({} sts). The increases are spaced evenly so the sleeve widens to the top ({} sts) by {} rows from the rib; sleeve length = shoulder-point-to-wrist {} minus the {} the dropped shoulder uses up.", cuff_sts, top_sts, s_rows, fmt(p.sleeve_to_wrist, cm), fmt(drop, cm)),
    ];
    let title = format!("{} knitted pullover (drop shoulder)", p.fit);
    let info = vec![
        format!("chest {}  length {}  sleeve {}  gauge {} sts x {} rows / 4 in", fmt(p.chest, cm), fmt(len, cm), fmt(p.sleeve_to_wrist, cm), p.gauge_sts, p.gauge_rows),
        "Schematic: finished sizes. Print at 100% to lay your knitting on it.".to_string(),
    ];
    Ok(Draft { title, info, pieces: vec![back, front, sleeve], explain, instructions: ins, checks, warnings })
}
