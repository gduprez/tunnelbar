//! What the desktop offers, and how the panel behaves on it.

use tunnelbar_core::paths::Os;

/// Whether a status notifier host shows tray icons, from the answer of
/// `gdbus call … NameHasOwner org.kde.StatusNotifierWatcher`: `(true,)` or `(false,)`.
/// None when the answer cannot be read. Used on Linux; tested everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_name_has_owner(answer: &str) -> Option<bool> {
    match answer.trim() {
        "(true,)" => Some(true),
        "(false,)" => Some(false),
        _ => None,
    }
}

/// Whether to show the panel window when TunnelBar starts: on Linux, when no tray icon can
/// be seen (or this is unknown), otherwise the app would be invisible.
pub fn show_window_at_start(os: Os, tray_available: Option<bool>) -> bool {
    os == Os::Linux && tray_available != Some(true)
}

/// Whether closing the panel only hides it (TunnelBar stays in the tray) rather than quitting:
/// without a visible tray icon, a hidden TunnelBar could neither be reopened nor quit.
pub fn close_hides_panel(os: Os, tray_available: Option<bool>) -> bool {
    !show_window_at_start(os, tray_available)
}

/// Asks D-Bus whether a status notifier host runs (Ubuntu: the AppIndicators extension).
#[cfg(target_os = "linux")]
pub fn tray_available() -> Option<bool> {
    let output = std::process::Command::new("/usr/bin/gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            "org.kde.StatusNotifierWatcher",
        ])
        .output()
        .ok()?;
    parse_name_has_owner(&String::from_utf8_lossy(&output.stdout))
}

/// macOS always has a menu bar.
#[cfg(not(target_os = "linux"))]
pub fn tray_available() -> Option<bool> {
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_hides_the_panel_only_when_the_tray_icon_is_visible() {
        assert!(close_hides_panel(Os::MacOs, Some(true)));
        assert!(close_hides_panel(Os::Linux, Some(true)));
        assert!(!close_hides_panel(Os::Linux, Some(false)));
        assert!(!close_hides_panel(Os::Linux, None));
    }

    #[test]
    fn reads_the_dbus_answer() {
        assert_eq!(parse_name_has_owner("(true,)\n"), Some(true));
        assert_eq!(parse_name_has_owner("(false,)"), Some(false));
        assert_eq!(parse_name_has_owner(""), None);
        assert_eq!(parse_name_has_owner("Error: no bus"), None);
    }

    #[test]
    fn macos_always_starts_in_the_menu_bar() {
        for tray in [Some(true), Some(false), None] {
            assert!(!show_window_at_start(Os::MacOs, tray));
        }
    }

    #[test]
    fn linux_opens_the_window_when_no_icon_can_be_seen() {
        assert!(!show_window_at_start(Os::Linux, Some(true)));
        assert!(show_window_at_start(Os::Linux, Some(false)));
        assert!(show_window_at_start(Os::Linux, None));
    }
}
