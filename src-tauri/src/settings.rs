//! User preferences of the app itself, next to the tunnels file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// `"auto"` follows macOS; otherwise a locale code such as `"fr"`.
pub const AUTO: &str = "auto";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: AUTO.into(),
        }
    }
}

/// `settings.toml`, in the directory of the tunnels file.
pub fn settings_path(config_path: &Path) -> PathBuf {
    config_path.with_file_name("settings.toml")
}

/// Missing or unreadable settings are the defaults: they are never worth blocking the app.
pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    let text = toml::to_string(settings).expect("settings are representable in TOML");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(text.as_bytes())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

/// Locale to display: the chosen one when available, else the system's.
pub fn resolve_locale(settings: &Settings, system: &str, available: &[&str]) -> String {
    if available.contains(&settings.language.as_str()) {
        settings.language.clone()
    } else {
        system.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn lives_next_to_the_tunnels_file() {
        assert_eq!(
            settings_path(Path::new("/a/TunnelBar/tunnels.toml")),
            PathBuf::from("/a/TunnelBar/settings.toml")
        );
    }

    #[test]
    fn missing_or_broken_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        assert_eq!(load(&path), Settings::default());
        std::fs::write(&path, "language = [").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn saved_settings_load_back_privately() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("TunnelBar").join("settings.toml");
        let settings = Settings {
            language: "fr".into(),
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn resolves_the_display_locale() {
        let available = ["en", "fr"];
        let chosen = |language: &str| Settings {
            language: language.into(),
        };
        assert_eq!(resolve_locale(&chosen("auto"), "fr", &available), "fr");
        assert_eq!(resolve_locale(&chosen("en"), "fr", &available), "en");
        assert_eq!(resolve_locale(&chosen("de"), "fr", &available), "fr");
    }
}
