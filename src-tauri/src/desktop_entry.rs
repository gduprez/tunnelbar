//! Linux launch at login: a freedesktop autostart entry, `~/.config/autostart/tunnelbar.desktop`.
//!
//! Pure functions, compiled and tested on every system.

use std::path::{Path, PathBuf};

/// File name of the autostart entry.
pub const FILE_NAME: &str = "tunnelbar.desktop";

/// `$XDG_CONFIG_HOME/autostart/tunnelbar.desktop`, `~/.config/…` when the variable is unset
/// or not absolute.
pub fn autostart_path(home: &Path, config_home: Option<&str>) -> PathBuf {
    config_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
        .join("autostart")
        .join(FILE_NAME)
}

/// Content of the autostart entry launching `executable`, or None if its path cannot be
/// written safely (control characters).
pub fn autostart_entry(executable: &Path) -> Option<String> {
    let exec = exec_argument(executable.to_str()?)?;
    Some(format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=TunnelBar\n\
         Comment=Start, stop and monitor SSH tunnels\n\
         Exec={exec}\n\
         Icon=tunnelbar\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    ))
}

/// `path` as one argument of an `Exec` key (Desktop Entry Specification, "The Exec key").
fn exec_argument(path: &str) -> Option<String> {
    if path.chars().any(char::is_control) {
        return None;
    }
    const RESERVED: &str = " \t\n\"'\\><~|&;$*?#()`";
    let argument = if path.chars().any(|c| RESERVED.contains(c)) {
        let mut quoted = String::from('"');
        for c in path.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted
    } else {
        path.to_owned()
    };
    // The Exec value is a string: its backslashes are escaped once more; `%` starts a field code.
    Some(argument.replace('\\', "\\\\").replace('%', "%%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_lives_in_the_xdg_autostart_directory() {
        let home = Path::new("/home/alice");
        assert_eq!(
            autostart_path(home, None),
            PathBuf::from("/home/alice/.config/autostart/tunnelbar.desktop")
        );
        assert_eq!(
            autostart_path(home, Some("/data/config")),
            PathBuf::from("/data/config/autostart/tunnelbar.desktop")
        );
        assert_eq!(
            autostart_path(home, Some("relative")),
            PathBuf::from("/home/alice/.config/autostart/tunnelbar.desktop")
        );
    }

    #[test]
    fn launches_the_executable_at_login() {
        let entry = autostart_entry(Path::new("/usr/bin/tunnelbar")).unwrap();
        assert_eq!(
            entry,
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=TunnelBar\n\
             Comment=Start, stop and monitor SSH tunnels\n\
             Exec=/usr/bin/tunnelbar\n\
             Icon=tunnelbar\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n"
        );
    }

    #[test]
    fn quotes_paths_with_reserved_characters() {
        assert_eq!(
            exec_argument("/home/alice/My Apps/TunnelBar.AppImage").as_deref(),
            Some(r#""/home/alice/My Apps/TunnelBar.AppImage""#)
        );
        // Inside quotes, " ` $ and \ are escaped with a backslash; then, as the value is a
        // string, every backslash is doubled.
        assert_eq!(
            exec_argument(r#"/opt/a"b`c$d\e"#).as_deref(),
            Some(r#""/opt/a\\"b\\`c\\$d\\\\e""#)
        );
    }

    #[test]
    fn doubles_percent_signs_which_introduce_field_codes() {
        assert_eq!(
            exec_argument("/opt/100%/tunnelbar").as_deref(),
            Some("/opt/100%%/tunnelbar")
        );
    }

    #[test]
    fn refuses_paths_with_control_characters() {
        // A newline would end the Exec line and could inject other keys.
        assert_eq!(autostart_entry(Path::new("/tmp/x\nExec=/bin/evil")), None);
        assert_eq!(exec_argument("/tmp/a\tb"), None);
    }
}
