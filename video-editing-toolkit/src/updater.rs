//! Self-update from GitHub Releases (same approach as the other Toolkits programs).
//!
//! Every push to `main` that touches the Video Editing Toolkit is built on GitHub Actions and published as a
//! release tagged `video-editing-toolkit-v0.1.<N>` with the Windows and Linux programs and their SHA-256
//! checksums. Toolkits holds several programs, so we look through the recent releases for the
//! newest one with our tag prefix, download the program for this platform, check its SHA-256,
//! swap it in place of the running program and restart.
//!
//! Nothing about you is sent: the only requests are anonymous downloads from github.com.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const REPO: &str = "pink-bourbon-estate/toolkits";
const TAG_PREFIX: &str = "video-editing-toolkit-v";

/// The CI build number baked in at compile time (0 for local builds).
pub fn current_build() -> u32 {
    option_env!("VIDEO_EDITING_TOOLKIT_BUILD").and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn minor_version() -> String {
    let v = env!("CARGO_PKG_VERSION");
    v.rsplit_once('.').map(|(a, _)| a.to_string()).unwrap_or_else(|| v.to_string())
}

pub fn version_label() -> String {
    match current_build() {
        0 => format!("v{} (local build)", minor_version()),
        n => format!("v{}.{n}", minor_version()),
    }
}

fn asset_name() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("video-editing-toolkit.exe")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("video-editing-toolkit-linux-x86_64")
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    pub build: u32,
    pub title: String,
    pub notes: String,
    pub page: String,
    pub url: String,
    pub sha_url: Option<String>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .proxy(ureq::Proxy::try_from_env())
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .user_agent(concat!("video-editing-toolkit-updater/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn get_bytes(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut resp = agent().get(url).header("Accept", "application/octet-stream").call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.body_mut().as_reader().take(limit).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Build number from a tag like `video-editing-toolkit-v0.1.17`.
fn build_of(tag: &str) -> Option<u32> {
    tag.strip_prefix(TAG_PREFIX)?.rsplit('.').next()?.parse().ok()
}

fn newest(releases: Vec<GhRelease>) -> Option<GhRelease> {
    releases.into_iter().filter(|r| !r.draft && build_of(&r.tag_name).is_some()).max_by_key(|r| build_of(&r.tag_name).unwrap_or(0))
}

/// Ask GitHub for the newest Video Editing Toolkit release. `Ok(None)` = already up to date.
pub fn check() -> Result<Option<Release>, String> {
    let Some(asset) = asset_name() else {
        return Err("Automatic updates are only built for Windows and Linux (x86-64).".into());
    };
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=50");
    let mut resp = agent().get(&url).header("Accept", "application/vnd.github+json").call().map_err(|e| format!("Could not reach GitHub: {e}"))?;
    let text = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let list: Vec<GhRelease> = serde_json::from_str(&text).map_err(|e| format!("Unexpected reply from GitHub: {e}"))?;
    let Some(rel) = newest(list) else {
        return Err("No Video Editing Toolkit releases have been published yet.".into());
    };
    let build = build_of(&rel.tag_name).unwrap_or(0);
    let Some(bin) = rel.assets.iter().find(|a| a.name == asset) else {
        return Err(format!("The latest release has no {asset}."));
    };
    let sha = rel.assets.iter().find(|a| a.name == format!("{asset}.sha256")).map(|a| a.browser_download_url.clone());
    if build <= current_build() {
        return Ok(None);
    }
    Ok(Some(Release {
        build,
        title: rel.name.unwrap_or_else(|| rel.tag_name.clone()),
        notes: rel.body.unwrap_or_default(),
        page: rel.html_url,
        url: bin.browser_download_url.clone(),
        sha_url: sha,
    }))
}

pub fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video-editing-toolkit".into());
    let ext = exe.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    exe.with_file_name(format!("{stem}{suffix}{ext}"))
}

/// Download, verify (SHA-256) and install. Returns the path of the new program.
pub fn install(rel: &Release) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot find the running program: {e}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let data = get_bytes(&rel.url, 300 * 1024 * 1024)?;
    if data.len() < 100_000 {
        return Err("The download looks incomplete; nothing was changed.".into());
    }
    let Some(sha_url) = &rel.sha_url else {
        return Err("The release has no checksum file; nothing was changed.".into());
    };
    let expected = String::from_utf8_lossy(&get_bytes(sha_url, 4096)?).split_whitespace().next().unwrap_or("").to_lowercase();
    let actual: String = Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect();
    if expected.len() != 64 || expected != actual {
        return Err("The download failed its checksum; nothing was changed.".into());
    }
    let new_path = sibling(&exe, ".new");
    std::fs::write(&new_path, &data).map_err(|e| format!("Cannot write next to {} ({e}). Use 'Install' so it lives in a folder it can update.", exe.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(windows)]
    {
        let old = sibling(&exe, ".old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).map_err(|e| {
            let _ = std::fs::remove_file(&new_path);
            format!("Cannot replace the running program: {e}")
        })?;
        if let Err(e) = std::fs::rename(&new_path, &exe) {
            let _ = std::fs::rename(&old, &exe);
            return Err(format!("Cannot install the update: {e}"));
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&new_path, &exe).map_err(|e| {
            let _ = std::fs::remove_file(&new_path);
            format!("Cannot install the update: {e}")
        })?;
    }
    Ok(exe)
}

/// Remove the previous version left behind by an update.
pub fn cleanup() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(sibling(&exe, ".old"));
        let _ = std::fs::remove_file(sibling(&exe, ".new"));
    }
}

pub fn relaunch(exe: &Path) -> Result<(), String> {
    std::process::Command::new(exe).spawn().map(|_| ()).map_err(|e| format!("Updated, but could not restart: {e}. Please start the Video Editing Toolkit again."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_names() {
        assert_eq!(build_of("video-editing-toolkit-v0.1.17"), Some(17));
        assert_eq!(build_of("chesscraft-v1.0.99"), None);
        assert_eq!(sibling(Path::new("C:/x/VideoEditingToolkit.exe"), ".old"), PathBuf::from("C:/x/VideoEditingToolkit.old.exe"));
    }

    #[test]
    fn picks_newest_of_ours() {
        let j = r#"[{"tag_name":"chesscraft-v1.0.99","html_url":"x","assets":[]},
                    {"tag_name":"video-editing-toolkit-v0.1.3","html_url":"x","assets":[]},
                    {"tag_name":"video-editing-toolkit-v0.1.12","html_url":"x","assets":[]},
                    {"tag_name":"video-editing-toolkit-v0.1.20","html_url":"x","draft":true,"assets":[]}]"#;
        let list: Vec<GhRelease> = serde_json::from_str(j).unwrap_or_default();
        assert_eq!(newest(list).map(|r| r.tag_name), Some("video-editing-toolkit-v0.1.12".to_string()));
    }
}
