//! Launch at login: SMAppService on macOS, an XDG autostart entry on Linux.

use serde::Serialize;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
pub use linux::{set, status};
#[cfg(target_os = "macos")]
pub use macos::{set, status};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginItem {
    Enabled,
    Disabled,
    /// Registered, but the user must allow it in System Settings › General › Login Items (macOS).
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    RequiresApproval,
    /// Not available, e.g. when running outside an application bundle.
    Unavailable,
}
