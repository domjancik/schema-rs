//! Desktop startup keeps the engine and authored files local; the UI uses the system browser.
use std::{path::PathBuf, process::Command};
pub fn project_root_for(
    os: &str,
    home: Option<&std::path::Path>,
    xdg: Option<&std::path::Path>,
    local: Option<&std::path::Path>,
) -> Result<PathBuf, String> {
    let base = match os {
        "windows" => local.map(PathBuf::from),
        "macos" => home.map(|h| h.join("Library/Application Support")),
        _ => xdg
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/share"))),
    };
    base.map(|p| p.join("Schema-s/project")).ok_or_else(|| {
        "Cannot locate app data; set HOME, LOCALAPPDATA (Windows) or XDG_DATA_HOME (Linux)".into()
    })
}
pub fn project_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    project_root_for(
        std::env::consts::OS,
        home.as_deref(),
        xdg.as_deref(),
        local.as_deref(),
    )
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
