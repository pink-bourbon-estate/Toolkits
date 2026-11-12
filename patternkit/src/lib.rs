pub mod geom;
pub mod knit;
pub mod params;
pub mod render;
pub mod tee;
pub mod trousers;

use params::Params;
use tee::Draft;
use serde_json::{json, Value};

/// Pick the drafter for the chosen garment.
pub fn draft(p: &Params) -> Result<Draft, String> {
    match p.garment.as_str() {
        "tshirt" | "shirt" | "hoodie" => tee::draft(p),
        "trousers" => trousers::draft(p),
        "knit" => knit::draft(p),
        other => Err(format!("unknown garment '{other}' (use tshirt, shirt, hoodie, trousers or knit)")),
    }
}

/// JSON in (a `Params`, any missing field defaulted) -> JSON out: sheet svg, page tiles, checks, explanation, warnings.
pub fn render_json(input: &str) -> String {
    let out: Result<Value, String> = (|| {
        let p: Params = serde_json::from_str(input).map_err(|e| format!("bad parameters: {e}"))?;
        let d = draft(&p)?;
        let sh = render::sheet(&d, &p);
        let (tiles, paper) = render::tiles(&sh, &d, &p);
        let pages: Vec<Value> = tiles.iter().filter(|t| render::tile_has_ink(&sh.inner, t)).map(|t| json!({
            "x": t.x, "y": t.y, "w": t.w, "h": t.h, "row": t.row, "col": t.col, "rows": t.rows, "cols": t.cols, "marks": render::tile_marks(t)
        })).collect();
        Ok(json!({
            "ok": true,
            "sheet_w": sh.w, "sheet_h": sh.h, "sheet": sh.inner, "paper_w": paper.0, "paper_h": paper.1, "pages": pages,
            "checks": d.checks.iter().map(|c| json!({"label": c.label, "value": c.value, "ok": c.ok})).collect::<Vec<_>>(),
            "explain": d.explain, "instructions": d.instructions, "warnings": d.warnings,
        }))
    })();
    match out { Ok(v) => v.to_string(), Err(e) => json!({"ok": false, "error": e}).to_string() }
}

/// Standalone SVG pages (for the CLI). Returns (file name, svg).
pub fn standalone_pages(p: &Params) -> Result<(Vec<(String, String)>, String, Vec<String>), String> {
    let d = draft(p)?;
    let sh = render::sheet(&d, p);
    let (tiles, paper) = render::tiles(&sh, &d, p);
    let pages = tiles.iter().filter(|t| render::tile_has_ink(&sh.inner, t))
        .map(|t| (format!("page_r{}_c{}.svg", t.row, t.col), render::page_svg(&sh, t, paper))).collect();
    let full = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}in\" height=\"{h}in\" viewBox=\"0 0 {w} {h}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n{}</svg>\n", sh.inner, w = sh.w, h = sh.h);
    let mut report: Vec<String> = d.checks.iter().map(|c| format!("[{}] {}: {}", if c.ok { "ok" } else { "CHECK" }, c.label, c.value)).collect();
    report.extend(d.warnings.iter().map(|w| format!("[warning] {w}")));
    Ok((pages, full, report))
}

// ---- WebAssembly entry point (plain C ABI, no wasm-bindgen): JS writes the JSON into memory from pk_alloc, calls pk_render,
// reads the returned (ptr<<32 | len), then frees both buffers. ----
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::render_json;
    #[no_mangle] pub extern "C" fn pk_alloc(len: usize) -> *mut u8 { let mut b = vec![0u8; len].into_boxed_slice(); let p = b.as_mut_ptr(); std::mem::forget(b); p }
    #[no_mangle] pub unsafe extern "C" fn pk_free(ptr: *mut u8, len: usize) { drop(Vec::from_raw_parts(ptr, 0, len)); }
    #[no_mangle] pub unsafe extern "C" fn pk_render(ptr: *const u8, len: usize) -> u64 {
        let s = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap_or("{}");
        let out = render_json(s).into_bytes();
        let n = out.len(); let mut b = out.into_boxed_slice(); let p = b.as_mut_ptr(); std::mem::forget(b);
        ((p as u64) << 32) | n as u64
    }
}
