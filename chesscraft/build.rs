// Embeds the icon and version info in the Windows .exe (so the Start menu and taskbar show it).
fn main() {
    println!("cargo:rerun-if-changed=assets/chesscraft.ico");
    println!("cargo:rerun-if-env-changed=CHESSCRAFT_BUILD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/chesscraft.ico");
        res.set("ProductName", "ChessCraft");
        res.set("FileDescription", "ChessCraft – system-first chess opening trainer");
        res.set("CompanyName", "pink-bourbon-estate");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
