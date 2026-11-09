//! Domain logic of TunnelBar, independent from any UI.

pub mod command;
pub mod diagnosis;
pub mod error;
#[cfg(any(test, feature = "testing"))]
pub mod fake;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod manager;
pub mod model;
pub mod orphans;
pub mod paths;
pub mod process;
pub mod state;
pub mod store;
pub mod supervisor;
pub mod system_events;
pub mod validation;

/// Version of the core crate, shown in the app's About information.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semantic() {
        let parts: Vec<_> = version().split('.').collect();
        assert_eq!(parts.len(), 3, "{}", version());
        assert!(
            parts.iter().all(|part| part.parse::<u32>().is_ok()),
            "{}",
            version()
        );
    }
}
