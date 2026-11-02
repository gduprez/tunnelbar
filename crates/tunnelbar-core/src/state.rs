//! Lifecycle of one tunnel, as a pure state machine: no I/O, no clock.
//!
//! The supervisor feeds it events and performs the actions it returns.

use std::time::Duration;

use serde::{Serialize, Serializer};

use crate::error::TunnelError;

/// Longest wait between two reconnection attempts.
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TunnelState {
    Stopped,
    /// `ssh` is running but the forwards are not usable yet. `attempt` is 0 for the first try.
    Connecting {
        attempt: u32,
    },
    Up,
    /// Waiting `delay` before reconnection attempt number `attempt` (1-based).
    Reconnecting {
        attempt: u32,
        #[serde(rename = "delay_ms", serialize_with = "milliseconds")]
        delay: Duration,
        last_error: TunnelError,
    },
    Failed {
        error: TunnelError,
    },
}

fn milliseconds<S: Serializer>(delay: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u128(delay.as_millis())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The user asks to start the tunnel.
    Start,
    /// The user asks to stop the tunnel.
    Stop,
    /// Every forward is usable (local ports accept connections).
    Ready,
    /// The `ssh` process ended, with the reason found in its output.
    Exited { error: TunnelError },
    /// The reconnection delay elapsed.
    RetryDue,
    /// The Mac woke up or the network changed: waiting is pointless.
    NetworkChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Spawn,
    Kill,
    ScheduleRetry(Duration),
    CancelRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub auto_reconnect: bool,
}

/// Delay before reconnection attempt `attempt` (1-based): 1 s, 2 s, 4 s… up to 60 s.
pub fn retry_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(16);
    Duration::from_secs(1 << exponent).min(MAX_RETRY_DELAY)
}

/// Next state and the actions to perform. Events irrelevant in the current state are ignored.
pub fn transition(
    state: &TunnelState,
    event: &Event,
    policy: Policy,
) -> (TunnelState, Vec<Action>) {
    use Action::*;
    use TunnelState::*;

    match (state, event) {
        (Stopped | Failed { .. }, Event::Start) => (Connecting { attempt: 0 }, vec![Spawn]),

        (Connecting { .. } | Up, Event::Stop) => (Stopped, vec![Kill]),
        (Reconnecting { .. }, Event::Stop) => (Stopped, vec![CancelRetry]),
        (Failed { .. }, Event::Stop) => (Stopped, vec![]),

        (Connecting { .. }, Event::Ready) => (Up, vec![]),

        (Connecting { attempt }, Event::Exited { error }) => after_exit(attempt + 1, error, policy),
        (Up, Event::Exited { error }) => after_exit(1, error, policy),

        (Reconnecting { attempt, .. }, Event::RetryDue) => {
            (Connecting { attempt: *attempt }, vec![Spawn])
        }
        (Reconnecting { attempt, .. }, Event::NetworkChanged) => {
            (Connecting { attempt: *attempt }, vec![CancelRetry, Spawn])
        }

        _ => (state.clone(), vec![]),
    }
}

fn after_exit(attempt: u32, error: &TunnelError, policy: Policy) -> (TunnelState, Vec<Action>) {
    if error.is_fatal() || !policy.auto_reconnect {
        return (
            TunnelState::Failed {
                error: error.clone(),
            },
            vec![],
        );
    }
    let delay = retry_delay(attempt);
    (
        TunnelState::Reconnecting {
            attempt,
            delay,
            last_error: error.clone(),
        },
        vec![Action::ScheduleRetry(delay)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use Action::*;
    use TunnelState::*;

    const RECONNECT: Policy = Policy {
        auto_reconnect: true,
    };
    const NO_RECONNECT: Policy = Policy {
        auto_reconnect: false,
    };
    const LOST: TunnelError = TunnelError::ConnectionLost;
    const AUTH: TunnelError = TunnelError::AuthFailed;

    fn step(state: TunnelState, event: Event) -> (TunnelState, Vec<Action>) {
        transition(&state, &event, RECONNECT)
    }

    fn exited(error: TunnelError) -> Event {
        Event::Exited { error }
    }

    fn reconnecting(attempt: u32, error: TunnelError) -> TunnelState {
        Reconnecting {
            attempt,
            delay: retry_delay(attempt),
            last_error: error,
        }
    }

    #[test]
    fn retry_delay_doubles_up_to_one_minute() {
        let seconds: Vec<u64> = (1..=9).map(|n| retry_delay(n).as_secs()).collect();
        assert_eq!(seconds, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(retry_delay(u32::MAX), MAX_RETRY_DELAY);
    }

    #[test]
    fn start_spawns_ssh() {
        assert_eq!(
            step(Stopped, Event::Start),
            (Connecting { attempt: 0 }, vec![Spawn])
        );
    }

    #[test]
    fn start_after_a_failure_tries_again_from_scratch() {
        assert_eq!(
            step(Failed { error: AUTH }, Event::Start),
            (Connecting { attempt: 0 }, vec![Spawn])
        );
    }

    #[test]
    fn start_is_ignored_while_running() {
        for state in [Connecting { attempt: 0 }, Up, reconnecting(2, LOST)] {
            assert_eq!(step(state.clone(), Event::Start), (state, vec![]));
        }
    }

    #[test]
    fn ready_brings_the_tunnel_up() {
        assert_eq!(step(Connecting { attempt: 3 }, Event::Ready), (Up, vec![]));
    }

    #[test]
    fn stop_kills_a_running_process() {
        assert_eq!(
            step(Connecting { attempt: 0 }, Event::Stop),
            (Stopped, vec![Kill])
        );
        assert_eq!(step(Up, Event::Stop), (Stopped, vec![Kill]));
    }

    #[test]
    fn stop_cancels_a_pending_retry() {
        assert_eq!(
            step(reconnecting(1, LOST), Event::Stop),
            (Stopped, vec![CancelRetry])
        );
    }

    #[test]
    fn stop_clears_a_failure() {
        assert_eq!(step(Failed { error: AUTH }, Event::Stop), (Stopped, vec![]));
        assert_eq!(step(Stopped, Event::Stop), (Stopped, vec![]));
    }

    #[test]
    fn losing_an_up_tunnel_schedules_the_first_retry() {
        assert_eq!(
            step(Up, exited(LOST)),
            (
                reconnecting(1, LOST),
                vec![ScheduleRetry(Duration::from_secs(1))]
            )
        );
    }

    #[test]
    fn failing_to_connect_backs_off_further() {
        assert_eq!(
            step(Connecting { attempt: 3 }, exited(LOST)),
            (
                reconnecting(4, LOST),
                vec![ScheduleRetry(Duration::from_secs(8))]
            )
        );
    }

    #[test]
    fn the_first_connection_is_retried_too() {
        let error = TunnelError::HostNotFound;
        assert_eq!(
            step(Connecting { attempt: 0 }, exited(error.clone())),
            (
                reconnecting(1, error),
                vec![ScheduleRetry(Duration::from_secs(1))]
            )
        );
    }

    #[test]
    fn a_due_retry_spawns_again() {
        assert_eq!(
            step(reconnecting(4, LOST), Event::RetryDue),
            (Connecting { attempt: 4 }, vec![Spawn])
        );
    }

    #[test]
    fn a_network_change_retries_immediately() {
        assert_eq!(
            step(reconnecting(5, LOST), Event::NetworkChanged),
            (Connecting { attempt: 5 }, vec![CancelRetry, Spawn])
        );
    }

    #[test]
    fn a_network_change_does_not_disturb_other_states() {
        for state in [
            Stopped,
            Connecting { attempt: 1 },
            Up,
            Failed { error: AUTH },
        ] {
            assert_eq!(step(state.clone(), Event::NetworkChanged), (state, vec![]));
        }
    }

    #[test]
    fn fatal_errors_stop_retrying() {
        assert_eq!(step(Up, exited(AUTH)), (Failed { error: AUTH }, vec![]));
        let busy = TunnelError::PortInUse { port: Some(54321) };
        assert_eq!(
            step(Connecting { attempt: 2 }, exited(busy.clone())),
            (Failed { error: busy }, vec![])
        );
    }

    #[test]
    fn without_auto_reconnect_any_exit_is_a_failure() {
        assert_eq!(
            transition(&Up, &exited(LOST), NO_RECONNECT),
            (Failed { error: LOST }, vec![])
        );
        assert_eq!(
            transition(&Connecting { attempt: 0 }, &exited(LOST), NO_RECONNECT),
            (Failed { error: LOST }, vec![])
        );
    }

    #[test]
    fn stale_events_are_ignored() {
        let stale = [
            (Stopped, exited(LOST)),
            (Stopped, Event::Ready),
            (Stopped, Event::RetryDue),
            (Up, Event::Ready),
            (Up, Event::RetryDue),
            (Connecting { attempt: 1 }, Event::RetryDue),
            (reconnecting(1, LOST), Event::Ready),
            (reconnecting(1, LOST), exited(LOST)),
            (Failed { error: AUTH }, exited(LOST)),
            (Failed { error: AUTH }, Event::RetryDue),
        ];
        for (state, event) in stale {
            assert_eq!(
                step(state.clone(), event.clone()),
                (state, vec![]),
                "{event:?}"
            );
        }
    }
}
