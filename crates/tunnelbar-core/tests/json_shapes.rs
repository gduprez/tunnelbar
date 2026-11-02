//! JSON shapes consumed by the UI (TypeScript types in src/lib/types.ts mirror them).

use std::time::{Duration, UNIX_EPOCH};

use serde_json::{Value, json, to_value};
use tunnelbar_core::error::TunnelError;
use tunnelbar_core::state::TunnelState;
use tunnelbar_core::supervisor::TunnelStatus;
use tunnelbar_core::validation::{PortField, ValidationError, ValidationErrorKind};

fn json_of(value: impl serde::Serialize) -> Value {
    to_value(value).unwrap()
}

#[test]
fn errors_are_tagged_by_code() {
    assert_eq!(
        json_of(TunnelError::AuthFailed),
        json!({ "code": "auth_failed" })
    );
    assert_eq!(
        json_of(TunnelError::PortInUse { port: Some(54321) }),
        json!({ "code": "port_in_use", "port": 54321 })
    );
    assert_eq!(
        json_of(TunnelError::Unknown { exit_code: None }),
        json!({ "code": "unknown", "exit_code": null })
    );
}

#[test]
fn states_are_tagged_and_delays_are_milliseconds() {
    assert_eq!(json_of(TunnelState::Stopped), json!({ "state": "stopped" }));
    assert_eq!(
        json_of(TunnelState::Connecting { attempt: 2 }),
        json!({ "state": "connecting", "attempt": 2 })
    );
    assert_eq!(
        json_of(TunnelState::Reconnecting {
            attempt: 3,
            delay: Duration::from_secs(4),
            last_error: TunnelError::ConnectionLost
        }),
        json!({
            "state": "reconnecting",
            "attempt": 3,
            "delay_ms": 4000,
            "last_error": { "code": "connection_lost" }
        })
    );
    assert_eq!(
        json_of(TunnelState::Failed {
            error: TunnelError::HostKeyMismatch
        }),
        json!({ "state": "failed", "error": { "code": "host_key_mismatch" } })
    );
}

#[test]
fn status_uses_unix_milliseconds() {
    let status = TunnelStatus {
        id: "prod".into(),
        state: TunnelState::Up,
        up_since: Some(UNIX_EPOCH + Duration::from_millis(1_793_600_000_123)),
        reconnections: 2,
        logs: vec!["hello".into()],
    };
    assert_eq!(
        json_of(status),
        json!({
            "id": "prod",
            "state": { "state": "up" },
            "up_since_ms": 1_793_600_000_123_u64,
            "reconnections": 2,
            "logs": ["hello"]
        })
    );
}

#[test]
fn validation_errors_carry_their_location() {
    let error = ValidationError {
        tunnel: Some(1),
        forward: Some(0),
        kind: ValidationErrorKind::ZeroPort {
            field: PortField::LocalPort,
        },
    };
    assert_eq!(
        json_of(error),
        json!({ "tunnel": 1, "forward": 0, "code": "zero_port", "field": "local_port" })
    );
}

#[test]
fn port_conflicts_name_the_other_forward_without_hiding_their_own_location() {
    let error = ValidationError {
        tunnel: Some(1),
        forward: Some(2),
        kind: ValidationErrorKind::LocalPortConflict {
            port: 54321,
            tunnel: 0,
            forward: 3,
        },
    };
    assert_eq!(
        serde_json::to_string(&error).unwrap(),
        r#"{"tunnel":1,"forward":2,"code":"local_port_conflict","port":54321,"other_tunnel":0,"other_forward":3}"#
    );
}
