//! Reading and writing `tunnels.toml`.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::model::{Config, ParseError};
use crate::validation::{ValidationError, validate};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot access {path}: {message}")]
    Io {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    #[error("syntax error: {0}")]
    Parse(#[from] ParseError),
    #[error("invalid configuration ({} problem(s))", .0.len())]
    Invalid(Vec<ValidationError>),
}

/// Where tunnel definitions live; abstracted so that callers can be tested without a disk.
pub trait ConfigStore {
    /// Loads and validates the configuration. A missing file is an empty configuration.
    fn load(&self) -> Result<Config, ConfigError>;
    /// Validates and saves the configuration, refusing to write an invalid one.
    fn save(&self, config: &Config) -> Result<(), ConfigError>;
}

#[derive(Debug, Clone)]
pub struct FileConfigStore {
    path: PathBuf,
}

impl FileConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        FileConfigStore { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl FileConfigStore {
    fn io_error(&self, path: &Path) -> impl FnOnce(io::Error) -> ConfigError {
        let path = path.to_owned();
        move |error| ConfigError::Io {
            path,
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

/// Writes `text` to `path`, readable by the user only (0600, directory 0700 when created).
/// The text goes to a temporary file which then replaces `path`, so that a crash never
/// leaves a truncated file.
pub fn write_private(path: &Path, text: &str) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;

    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = dir.join(format!(".{file_name}.tmp"));
    let result = (|| {
        let _ = fs::remove_file(&temporary);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

impl ConfigStore for FileConfigStore {
    fn load(&self) -> Result<Config, ConfigError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(error) => return Err(self.io_error(&self.path)(error)),
        };
        let config = Config::parse(&text)?;
        validate(&config).map_err(ConfigError::Invalid)?;
        Ok(config)
    }

    fn save(&self, config: &Config) -> Result<(), ConfigError> {
        validate(config).map_err(ConfigError::Invalid)?;
        write_private(&self.path, &config.to_toml()).map_err(self.io_error(&self.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Forward, Tunnel};
    use crate::validation::ValidationErrorKind;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn store() -> (tempfile::TempDir, FileConfigStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileConfigStore::new(dir.path().join("TunnelBar").join("tunnels.toml"));
        (dir, store)
    }

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn sample() -> Config {
        Config {
            version: 1,
            tunnels: vec![Tunnel {
                id: "prod".into(),
                name: "Prod server".into(),
                host: "203.0.113.10".into(),
                user: None,
                port: None,
                autostart: false,
                auto_reconnect: true,
                accept_new_host_key: false,
                forwards: vec![Forward::Local {
                    local_port: 54321,
                    remote_host: "127.0.0.1".into(),
                    remote_port: 5432,
                }],
            }],
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_configuration() {
        let (_dir, store) = store();
        assert_eq!(store.load(), Ok(Config::default()));
    }

    #[test]
    fn saved_configuration_loads_back() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        assert_eq!(store.load(), Ok(sample()));
    }

    #[test]
    fn save_creates_a_private_directory_and_file() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        assert_eq!(mode(store.path()), 0o600);
        assert_eq!(mode(store.path().parent().unwrap()), 0o700);
    }

    #[test]
    fn save_tightens_the_permissions_of_an_existing_file() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o644)).unwrap();
        store.save(&sample()).unwrap();
        assert_eq!(mode(store.path()), 0o600);
    }

    #[test]
    fn save_leaves_no_temporary_file_behind() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        store.save(&Config::default()).unwrap();
        let entries: Vec<_> = fs::read_dir(store.path().parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec!["tunnels.toml"]);
    }

    #[test]
    fn save_refuses_an_invalid_configuration_and_keeps_the_file() {
        let (_dir, store) = store();
        store.save(&sample()).unwrap();
        let mut invalid = sample();
        invalid.tunnels[0].host = "-oProxyCommand=x".into();

        let Err(ConfigError::Invalid(errors)) = store.save(&invalid) else {
            panic!("an invalid configuration must be refused");
        };
        assert_eq!(errors[0].kind, ValidationErrorKind::InvalidHost);
        assert_eq!(store.load(), Ok(sample()));
    }

    #[test]
    fn load_reports_syntax_errors() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "version = 1\n[[tunnel]\n").unwrap();
        assert!(matches!(
            store.load(),
            Err(ConfigError::Parse(ParseError { line: Some(2), .. }))
        ));
    }

    #[test]
    fn load_reports_validation_errors() {
        let (_dir, store) = store();
        let mut invalid = sample();
        invalid.tunnels[0].forwards.clear();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), invalid.to_toml()).unwrap();
        let Err(ConfigError::Invalid(errors)) = store.load() else {
            panic!("an invalid file must be reported");
        };
        assert_eq!(errors[0].kind, ValidationErrorKind::NoForward);
    }

    #[test]
    fn load_reports_io_errors() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path()).unwrap(); // a directory where the file should be
        assert!(matches!(store.load(), Err(ConfigError::Io { .. })));
    }
}
