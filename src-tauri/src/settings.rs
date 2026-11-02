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
    let text = toml::to_string(settings).expect("settings are representable in TOML");
    tunnelbar_core::store::write_private(path, &text)
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
    fn saving_replaces_the_file_instead_of_rewriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        save(&path, &Settings::default()).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        // A hard link keeps the old file: an in-place rewrite would change it too.
        let witness = dir.path().join("witness");
        std::fs::hard_link(&path, &witness).unwrap();

        save(
            &path,
            &Settings {
                language: "fr".into(),
            },
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&witness).unwrap(), before);
        assert_eq!(load(&path).language, "fr");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names.len(), 2, "no temporary file left: {names:?}");
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
