//! Desktop startup keeps the engine and authored files local; the UI uses the system browser.
use std::{path::PathBuf, process::Command};
pub fn project_root() -> Result<PathBuf, String> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
    } else {
        std::env::var_os("XDG_DATA_HOME").or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".local/share").into_os_string())
        })
    };
    base.map(|path| PathBuf::from(path).join("Schema-s/project"))
        .ok_or_else(|| "Cannot locate app data; set LOCALAPPDATA (Windows) or XDG_DATA_HOME".into())
}
pub fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let status = Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .status();
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(url).status();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let status = Command::new("xdg-open").arg(url).status();
    status.map_err(|e| e.to_string()).and_then(|status| {
        if status.success() {
            Ok(())
        } else {
            Err("Browser launcher failed".into())
        }
    })
}
