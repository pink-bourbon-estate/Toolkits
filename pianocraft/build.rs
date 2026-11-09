// Embeds the icon and version info in the Windows .exe (so the Start menu and taskbar show it).
fn main() {
    println!("cargo:rerun-if-changed=assets/pianocraft.ico");
    println!("cargo:rerun-if-env-changed=PIANOCRAFT_BUILD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/pianocraft.ico");
        res.set("ProductName", "PianoCraft");
        res.set("FileDescription", "PianoCraft – piano practice companion");
        res.set("CompanyName", "pink-bourbon-estate");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
