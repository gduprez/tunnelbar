//! Why a tunnel stopped working. Typed so that the UI can translate it.

/// Failure of a tunnel's `ssh` process.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum TunnelError {
    /// The server refused every key (`Permission denied`).
    AuthFailed,
    /// Host never seen before; its fingerprint must be checked once in a terminal.
    HostKeyUnknown,
    /// The host key changed: possible man-in-the-middle attack.
    HostKeyMismatch,
    /// A local port of a forward is already used on this Mac.
    PortInUse {
        port: Option<u16>,
    },
    /// The server refused a remote forward (port busy or forbidden there).
    RemoteForwardFailed {
        port: Option<u16>,
    },
    /// The host name cannot be resolved (also happens while offline).
    HostNotFound,
    ConnectionRefused,
    ConnectionTimedOut,
    /// No route, network down…
    NetworkUnreachable,
    /// The established connection was lost (keepalive timeout, reset, server restart).
    ConnectionLost,
    /// `ssh` could not be started at all.
    SpawnFailed,
    /// Anything else, with the process exit code when there is one.
    Unknown {
        exit_code: Option<i32>,
    },
}

impl TunnelError {
    /// A fatal error will not go away by retrying: reconnection stops.
    pub fn is_fatal(&self) -> bool {
        use TunnelError::*;
        // HostNotFound stays transient: name resolution also fails while offline.
        matches!(
            self,
            AuthFailed
                | HostKeyUnknown
                | HostKeyMismatch
                | PortInUse { .. }
                | RemoteForwardFailed { .. }
                | SpawnFailed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::TunnelError::*;

    #[test]
    fn errors_needing_the_user_are_fatal() {
        for error in [
            AuthFailed,
            HostKeyUnknown,
            HostKeyMismatch,
            PortInUse { port: Some(54321) },
            RemoteForwardFailed { port: None },
            SpawnFailed,
        ] {
            assert!(error.is_fatal(), "{error:?}");
        }
    }

    #[test]
    fn network_errors_are_transient() {
        for error in [
            HostNotFound,
            ConnectionRefused,
            ConnectionTimedOut,
            NetworkUnreachable,
            ConnectionLost,
            Unknown {
                exit_code: Some(255),
            },
        ] {
            assert!(!error.is_fatal(), "{error:?}");
        }
    }
}
