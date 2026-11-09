//! Where TunnelBar keeps its files, per operating system.
//!
//! macOS: `~/Library/Application Support/TunnelBar/`. Linux: the XDG base directories,
//! `~/.config/tunnelbar/` for configuration and `~/.local/state/tunnelbar/` for state.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
}

impl Os {
    /// The system this build runs on.
    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// The tunnels, `tunnels.toml`.
    pub tunnels: PathBuf,
    /// App preferences, `settings.toml`.
    pub settings: PathBuf,
    /// PIDs of running `ssh` processes, `ssh.pids`.
    pub pids: PathBuf,
}

impl AppPaths {
    /// Paths for `os`, given the home directory and a reader of environment variables.
    pub fn for_os(os: Os, home: &Path, env: impl Fn(&str) -> Option<String>) -> AppPaths {
        match os {
            Os::MacOs => {
                let dir = home.join("Library/Application Support/TunnelBar");
                AppPaths {
                    tunnels: dir.join("tunnels.toml"),
                    settings: dir.join("settings.toml"),
                    pids: dir.join("ssh.pids"),
                }
            }
            Os::Linux => {
                let config = xdg_dir(&env, "XDG_CONFIG_HOME", home, ".config").join("tunnelbar");
                let state = xdg_dir(&env, "XDG_STATE_HOME", home, ".local/state").join("tunnelbar");
                AppPaths {
                    tunnels: config.join("tunnels.toml"),
                    settings: config.join("settings.toml"),
                    pids: state.join("ssh.pids"),
                }
            }
        }
    }

    /// Paths of this system, from the real environment.
    pub fn current(home: &Path) -> AppPaths {
        AppPaths::for_os(Os::current(), home, |name| std::env::var(name).ok())
    }
}

/// An XDG base directory: the variable when it holds an absolute path, else its default.
fn xdg_dir(
    env: &impl Fn(&str) -> Option<String>,
    variable: &str,
    home: &Path,
    default: &str,
) -> PathBuf {
    env(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(default))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn macos_keeps_everything_in_application_support() {
        let paths = AppPaths::for_os(Os::MacOs, Path::new("/Users/alice"), no_env);
        let dir = Path::new("/Users/alice/Library/Application Support/TunnelBar");
        assert_eq!(
            paths,
            AppPaths {
                tunnels: dir.join("tunnels.toml"),
                settings: dir.join("settings.toml"),
                pids: dir.join("ssh.pids"),
            }
        );
    }

    #[test]
    fn macos_ignores_xdg_variables() {
        let env = |_: &str| Some("/elsewhere".to_owned());
        let paths = AppPaths::for_os(Os::MacOs, Path::new("/Users/alice"), env);
        assert!(paths.tunnels.starts_with("/Users/alice/Library"));
    }

    #[test]
    fn linux_uses_the_default_xdg_directories() {
        let paths = AppPaths::for_os(Os::Linux, Path::new("/home/alice"), no_env);
        assert_eq!(
            paths,
            AppPaths {
                tunnels: PathBuf::from("/home/alice/.config/tunnelbar/tunnels.toml"),
                settings: PathBuf::from("/home/alice/.config/tunnelbar/settings.toml"),
                pids: PathBuf::from("/home/alice/.local/state/tunnelbar/ssh.pids"),
            }
        );
    }

    #[test]
    fn linux_follows_xdg_variables() {
        let env = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some("/data/config".to_owned()),
            "XDG_STATE_HOME" => Some("/data/state".to_owned()),
            _ => None,
        };
        let paths = AppPaths::for_os(Os::Linux, Path::new("/home/alice"), env);
        assert_eq!(
            paths.tunnels,
            PathBuf::from("/data/config/tunnelbar/tunnels.toml")
        );
        assert_eq!(
            paths.settings,
            PathBuf::from("/data/config/tunnelbar/settings.toml")
        );
        assert_eq!(paths.pids, PathBuf::from("/data/state/tunnelbar/ssh.pids"));
    }

    #[test]
    fn linux_ignores_relative_or_empty_xdg_variables() {
        // The XDG specification says relative paths are invalid and must be ignored.
        let env = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some("relative/config".to_owned()),
            "XDG_STATE_HOME" => Some(String::new()),
            _ => None,
        };
        let paths = AppPaths::for_os(Os::Linux, Path::new("/home/alice"), env);
        assert_eq!(
            paths.tunnels,
            PathBuf::from("/home/alice/.config/tunnelbar/tunnels.toml")
        );
        assert_eq!(
            paths.pids,
            PathBuf::from("/home/alice/.local/state/tunnelbar/ssh.pids")
        );
    }
}
