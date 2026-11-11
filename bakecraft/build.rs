// build.rs runs before the program is compiled.
// Its only job: put the icon and version details inside the Windows .exe,
// so the taskbar and Start menu show them.
fn main() {
    println!("cargo:rerun-if-changed=assets/bakecraft.ico");
    println!("cargo:rerun-if-env-changed=BAKECRAFT_BUILD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/bakecraft.ico");
        res.set("ProductName", "BakeCraft");
        res.set("FileDescription", "BakeCraft - bake planner");
        res.set("CompanyName", "pink-bourbon-estate");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
