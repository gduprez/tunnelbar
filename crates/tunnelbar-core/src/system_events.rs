//! Detects sleep/wake and network changes by polling, without OS notification APIs.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::time::{Duration, Instant, SystemTime};

/// The Mac slept when the wall clock advanced this much more than the monotonic clock,
/// which stops during sleep on macOS.
pub const WAKE_THRESHOLD: Duration = Duration::from_secs(10);

pub struct WakeDetector {
    wall: SystemTime,
    monotonic: Instant,
}

impl WakeDetector {
    pub fn new(wall: SystemTime, monotonic: Instant) -> Self {
        WakeDetector { wall, monotonic }
    }

    /// Records a new observation; true when the Mac slept since the previous one.
    pub fn woke_up(&mut self, wall: SystemTime, monotonic: Instant) -> bool {
        let wall_elapsed = wall.duration_since(self.wall).unwrap_or_default();
        let monotonic_elapsed = monotonic.saturating_duration_since(self.monotonic);
        self.wall = wall;
        self.monotonic = monotonic;
        wall_elapsed > monotonic_elapsed + WAKE_THRESHOLD
    }
}

/// Remembers the local addresses to tell when the network changed.
#[derive(Debug, Default)]
pub struct NetworkWatcher {
    addresses: Option<BTreeSet<IpAddr>>,
}

impl NetworkWatcher {
    /// True when `addresses` differ from the previous observation (never on the first one).
    pub fn changed(&mut self, addresses: impl IntoIterator<Item = IpAddr>) -> bool {
        let current: BTreeSet<IpAddr> = addresses.into_iter().collect();
        let previous = self.addresses.replace(current.clone());
        previous.is_some_and(|previous| previous != current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_ticks_are_not_a_wake_up() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let mut detector = WakeDetector::new(wall, mono);
        let step = Duration::from_secs(5);
        assert!(!detector.woke_up(wall + step, mono + step));
        // Small drifts or NTP adjustments are tolerated.
        assert!(!detector.woke_up(wall + step * 2 + Duration::from_secs(3), mono + step * 2));
    }

    #[test]
    fn a_wall_clock_jump_is_a_wake_up() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let mut detector = WakeDetector::new(wall, mono);
        let step = Duration::from_secs(5);
        assert!(detector.woke_up(wall + Duration::from_secs(3600), mono + step));
        // The next regular tick is measured from the wake up.
        assert!(!detector.woke_up(wall + Duration::from_secs(3605), mono + step * 2));
    }

    #[test]
    fn a_wall_clock_going_backwards_is_not_a_wake_up() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let mut detector = WakeDetector::new(wall, mono);
        let step = Duration::from_secs(5);
        assert!(!detector.woke_up(wall - Duration::from_secs(60), mono + step));
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn network_changes_are_detected_after_the_first_observation() {
        let mut watcher = NetworkWatcher::default();
        assert!(!watcher.changed([ip("192.168.1.10")]));
        assert!(!watcher.changed([ip("192.168.1.10")]));
        assert!(watcher.changed([ip("10.0.0.4")]));
        assert!(watcher.changed([ip("10.0.0.4"), ip("fe80::1")]));
        assert!(watcher.changed([]));
    }

    #[test]
    fn address_order_does_not_matter() {
        let mut watcher = NetworkWatcher::default();
        watcher.changed([ip("10.0.0.4"), ip("fe80::1")]);
        assert!(!watcher.changed([ip("fe80::1"), ip("10.0.0.4")]));
    }
}
