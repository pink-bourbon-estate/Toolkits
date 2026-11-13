// Embeds the icon and version info in the Windows .exe (so the Start menu and taskbar show it).
fn main() {
    println!("cargo:rerun-if-changed=assets/video-editing-toolkit.ico");
    println!("cargo:rerun-if-env-changed=VIDEO_EDITING_TOOLKIT_BUILD");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/video-editing-toolkit.ico");
        res.set("ProductName", "Video Editing Toolkit");
        res.set("FileDescription", "Video Editing Toolkit - compress video, extract audio and frames");
        res.set("CompanyName", "pink-bourbon-estate");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
