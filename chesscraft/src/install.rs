//! "Install ChessCraft": puts the program in a per-user folder it can update itself in, adds a
//! Start-menu entry (and optional desktop shortcut), and registers it in Settings → Apps so it
//! can be uninstalled normally. No admin rights needed; nothing outside your user profile is touched.
//!
//! Windows: %LOCALAPPDATA%\Programs\ChessCraft\ChessCraft.exe
//!          Start menu: %APPDATA%\Microsoft\Windows\Start Menu\Programs\ChessCraft.lnk
//!          Apps list:  HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ChessCraft
//! Linux:   ~/.local/bin/chesscraft + ~/.local/share/applications/chesscraft.desktop
//! Your progress lives separately (in your app-data folder) and is never removed.

use std::path::{Path, PathBuf};

pub fn install_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        dirs::data_local_dir().map(|d| d.join("Programs").join("ChessCraft"))
    } else {
        dirs::home_dir().map(|h| h.join(".local").join("bin"))
    }
}

pub fn installed_exe() -> Option<PathBuf> {
    install_dir().map(|d| d.join(if cfg!(windows) { "ChessCraft.exe" } else { "chesscraft" }))
}

/// True when the running program is the installed copy.
pub fn is_installed() -> bool {
    let (Ok(me), Some(inst)) = (std::env::current_exe(), installed_exe()) else { return false };
    let me = me.canonicalize().unwrap_or(me);
    let inst = inst.canonicalize().unwrap_or(inst);
    me == inst
}

#[cfg(windows)]
fn start_menu_link() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("ChessCraft.lnk"))
}

#[cfg(windows)]
fn quiet(cmd: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000)
}

/// Create a Windows shortcut (.lnk) with PowerShell's built-in WScript.Shell.
#[cfg(windows)]
fn make_shortcut(link: &Path, target: &Path) -> Result<(), String> {
    let q = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.IconLocation='{},0';$s.Description='ChessCraft';$s.Save()",
        q(link),
        q(target),
        q(target.parent().unwrap_or(Path::new("."))),
        q(target)
    );
    let out = quiet(std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script]))
        .output()
        .map_err(|e| format!("Could not run PowerShell: {e}"))?;
    if out.status.success() && link.exists() {
        Ok(())
    } else {
        Err(format!("Could not create the shortcut: {}", String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("unknown error")))
    }
}

#[cfg(windows)]
fn reg_add(key: &str, name: &str, kind: &str, value: &str) {
    let _ = quiet(std::process::Command::new("reg").args(["add", key, "/v", name, "/t", kind, "/d", value, "/f"])).output();
}

/// Make sure the Start-menu entry exists and points at `exe`.
pub fn ensure_start_menu(exe: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let link = start_menu_link().ok_or("No Start menu folder")?;
        if let Some(p) = link.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        make_shortcut(&link, exe)
    }
    #[cfg(not(windows))]
    {
        let apps = dirs::data_dir().ok_or("No applications folder")?.join("applications");
        std::fs::create_dir_all(&apps).map_err(|e| e.to_string())?;
        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName=ChessCraft\nComment=System-first chess opening trainer\nExec=\"{}\"\nTerminal=false\nCategories=Game;BoardGame;Education;\n",
            exe.display()
        );
        std::fs::write(apps.join("chesscraft.desktop"), desktop).map_err(|e| e.to_string())
    }
}

pub fn has_start_menu_entry() -> bool {
    #[cfg(windows)]
    {
        start_menu_link().is_some_and(|l| l.exists())
    }
    #[cfg(not(windows))]
    {
        dirs::data_dir().is_some_and(|d| d.join("applications").join("chesscraft.desktop").exists())
    }
}

/// Copy the running program into the install folder, add shortcuts and the Apps entry.
/// Returns the installed program's path.
pub fn install(desktop_shortcut: bool) -> Result<PathBuf, String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = install_dir().ok_or("Can't find your local programs folder")?;
    let target = installed_exe().ok_or("Can't find your local programs folder")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Can't create {}: {e}", dir.display()))?;
    if !is_installed() {
        if target.exists() {
            let old = crate::updater::sibling(&target, ".old");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(&target, &old);
        }
        std::fs::copy(&me, &target).map_err(|e| format!("Can't copy ChessCraft to {}: {e}", target.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
        }
    }
    ensure_start_menu(&target)?;
    #[cfg(windows)]
    {
        if desktop_shortcut {
            if let Some(desk) = dirs::desktop_dir() {
                let _ = make_shortcut(&desk.join("ChessCraft.lnk"), &target);
            }
        }
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ChessCraft";
        let t = target.display().to_string();
        reg_add(key, "DisplayName", "REG_SZ", "ChessCraft");
        reg_add(key, "DisplayVersion", "REG_SZ", &crate::updater::version_label());
        reg_add(key, "Publisher", "REG_SZ", "pink-bourbon-estate");
        reg_add(key, "DisplayIcon", "REG_SZ", &t);
        reg_add(key, "InstallLocation", "REG_SZ", &dir.display().to_string());
        reg_add(key, "UninstallString", "REG_SZ", &format!("\"{t}\" --uninstall"));
        reg_add(key, "URLInfoAbout", "REG_SZ", "https://github.com/pink-bourbon-estate/toolkits");
        reg_add(key, "NoModify", "REG_DWORD", "1");
        reg_add(key, "NoRepair", "REG_DWORD", "1");
    }
    #[cfg(not(windows))]
    let _ = desktop_shortcut;
    Ok(target)
}

/// Keep the version shown in Settings → Apps current after an update.
pub fn refresh_registration() {
    #[cfg(windows)]
    if is_installed() {
        reg_add(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ChessCraft", "DisplayVersion", "REG_SZ", &crate::updater::version_label());
        if !has_start_menu_entry() {
            if let Some(exe) = installed_exe() {
                let _ = ensure_start_menu(&exe);
            }
        }
    }
}

/// Remove the program, shortcuts and Apps entry. Your progress is kept.
pub fn uninstall() -> Result<String, String> {
    #[cfg(windows)]
    {
        if let Some(l) = start_menu_link() {
            let _ = std::fs::remove_file(l);
        }
        if let Some(desk) = dirs::desktop_dir() {
            let _ = std::fs::remove_file(desk.join("ChessCraft.lnk"));
        }
        let _ = quiet(std::process::Command::new("reg").args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ChessCraft", "/f"])).output();
        if let Some(dir) = install_dir() {
            let d = dir.display().to_string();
            let _ = quiet(std::process::Command::new("cmd").args(["/C", &format!("ping 127.0.0.1 -n 3 > nul & rmdir /s /q \"{d}\"")])).spawn();
        }
        Ok("ChessCraft was uninstalled. Your progress is still in %APPDATA%\\ChessCraft if you reinstall.".into())
    }
    #[cfg(not(windows))]
    {
        if let Some(d) = dirs::data_dir() {
            let _ = std::fs::remove_file(d.join("applications").join("chesscraft.desktop"));
        }
        if let Some(exe) = installed_exe() {
            let _ = std::fs::remove_file(exe);
        }
        Ok("ChessCraft was uninstalled. Your progress is kept.".into())
    }
}
