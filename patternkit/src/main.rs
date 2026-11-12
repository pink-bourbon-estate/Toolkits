use patternkit::{params::Params, render_json, standalone_pages};
use std::{fs, io::{Read, Write}, net::TcpListener};

fn apply_args(args: &[String]) -> Result<Params, String> {
    let mut v = serde_json::json!({});
    for a in args {
        if let Some(f) = a.strip_prefix("--params=") { v = serde_json::from_str(&fs::read_to_string(f).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?; }
    }
    for a in args {
        if let Some(kv) = a.strip_prefix("--") {
            if let Some((k, val)) = kv.split_once('=') {
                if k == "params" || k == "out" || k == "port" { continue; }
                v[k] = val.parse::<f64>().map(|n| serde_json::json!(n)).or_else(|_| val.parse::<bool>().map(|b| serde_json::json!(b))).unwrap_or(serde_json::json!(val));
            }
        }
    }
    serde_json::from_value(v).map_err(|e| e.to_string())
}

fn serve(port: u16, web_dir: &str) {
    let l = TcpListener::bind(("127.0.0.1", port)).expect("bind");
    eprintln!("open http://127.0.0.1:{port}/");
    for s in l.incoming().flatten() {
        let mut s = s;
        let mut buf = vec![0u8; 1 << 16];
        let mut n = s.read(&mut buf).unwrap_or(0);
        let mut req = String::from_utf8_lossy(&buf[..n]).to_string();
        // read the rest of a POST body
        if let Some(i) = req.find("\r\n\r\n") {
            let cl = req.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap_or(0);
            while req.len() < i + 4 + cl { let m = s.read(&mut buf).unwrap_or(0); if m == 0 { break; } n += m; req.push_str(&String::from_utf8_lossy(&buf[..m])); }
        }
        let first = req.lines().next().unwrap_or("").to_string();
        let (ctype, body): (&str, Vec<u8>) = if first.starts_with("POST /api/render") {
            let b = req.split("\r\n\r\n").nth(1).unwrap_or("{}");
            ("application/json", render_json(b).into_bytes())
        } else {
            let path = first.split(' ').nth(1).unwrap_or("/");
            let name = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
            if name.contains("..") { ("text/plain", b"no".to_vec()) } else {
                let ct = if name.ends_with(".js") { "text/javascript" } else if name.ends_with(".wasm") { "application/wasm" } else if name.ends_with(".css") { "text/css" } else { "text/html" };
                match fs::read(format!("{web_dir}/{name}")) { Ok(b) => (ct, b), Err(_) => ("text/plain", b"not found".to_vec()) }
            }
        };
        let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
        let _ = s.write_all(&body);
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("tee") => {
            let r = (|| -> Result<(), String> {
                let p = apply_args(&a[2..])?;
                let out = a.iter().find_map(|x| x.strip_prefix("--out=")).unwrap_or("out").to_string();
                let (pages, full, report) = standalone_pages(&p)?;
                fs::create_dir_all(format!("{out}/pages")).map_err(|e| e.to_string())?;
                fs::write(format!("{out}/sheet.svg"), full).map_err(|e| e.to_string())?;
                for (n, s) in &pages { fs::write(format!("{out}/pages/{n}"), s).map_err(|e| e.to_string())?; }
                fs::write(format!("{out}/report.txt"), report.join("\n")).map_err(|e| e.to_string())?;
                for l in &report { println!("{l}"); }
                println!("{} printable pages -> {out}/pages", pages.len());
                Ok(())
            })();
            if let Err(e) = r { eprintln!("error: {e}"); std::process::exit(1); }
        }
        Some("serve") => {
            let port = a.iter().find_map(|x| x.strip_prefix("--port=")).and_then(|v| v.parse().ok()).unwrap_or(8080);
            serve(port, a.iter().find_map(|x| x.strip_prefix("--web=")).unwrap_or("web"));
        }
        _ => eprintln!("usage:\n  patternkit tee [--fit=baggy|regular|fitted] [--chest=35.25 ...] [--params=file.json] --out=dir\n  patternkit serve [--port=8080] [--web=web]"),
    }
}
