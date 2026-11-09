//! Linux launch at login: `~/.config/autostart/tunnelbar.desktop` (freedesktop autostart).

use std::path::PathBuf;

use super::LoginItem;
use crate::desktop_entry::{autostart_entry, autostart_executable, autostart_path};

fn entry_path() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let config_home = std::env::var("XDG_CONFIG_HOME").ok();
    Some(autostart_path(&home, config_home.as_deref()))
}

pub fn status() -> LoginItem {
    match entry_path() {
        Some(path) if path.is_file() => LoginItem::Enabled,
        Some(_) => LoginItem::Disabled,
        None => LoginItem::Unavailable,
    }
}

pub fn set(enabled: bool) -> Result<LoginItem, String> {
    let path = entry_path().ok_or("no home directory")?;
    if enabled {
        // An AppImage runs from a temporary mount: start the AppImage file itself.
        let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
        let executable = autostart_executable(appimage.as_deref(), std::env::current_exe())?;
        let entry = autostart_entry(&executable).ok_or("unsupported executable path")?;
        tunnelbar_core::store::write_private(&path, &entry).map_err(|error| error.to_string())?;
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(status())
}
