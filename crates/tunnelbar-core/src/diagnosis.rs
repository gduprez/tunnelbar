//! Finds out why `ssh` exited, from what it printed on stderr.
//!
//! Patterns come from real OpenSSH 9 output; when several match, the most specific wins.

use crate::error::TunnelError;

/// Reason of an `ssh` exit, from its stderr output and exit code.
pub fn diagnose(stderr: &str, exit_code: Option<i32>) -> TunnelError {
    use TunnelError::*;

    let has = |needle: &str| stderr.contains(needle);
    let line_with = |needle: &str| stderr.lines().find(|line| line.contains(needle));

    if has("REMOTE HOST IDENTIFICATION HAS CHANGED")
        || has("has changed and you have requested strict checking")
    {
        HostKeyMismatch
    } else if has("Host key verification failed") {
        HostKeyUnknown
    } else if has("Permission denied") {
        AuthFailed
    } else if let Some(line) = line_with("Address already in use") {
        PortInUse {
            port: port_in_bind_line(line),
        }
    } else if let Some(line) = line_with("cannot listen to port:") {
        PortInUse {
            port: number_after(line, "cannot listen to port:"),
        }
    } else if has("Could not request local forwarding")
        || has("Could not request dynamic forwarding")
    {
        PortInUse { port: None }
    } else if let Some(line) = line_with("remote port forwarding failed") {
        RemoteForwardFailed {
            port: number_after(line, "listen port"),
        }
    } else if has("Could not resolve hostname") {
        HostNotFound
    } else if has("Connection refused") {
        ConnectionRefused
    } else if has("timed out") {
        ConnectionTimedOut
    } else if has("Network is unreachable") || has("No route to host") {
        NetworkUnreachable
    } else if has("not responding")
        || has("Broken pipe")
        || has("closed by")
        || has("Connection reset")
    {
        ConnectionLost
    } else {
        Unknown { exit_code }
    }
}

/// `bind [127.0.0.1]:54321: …` or `bind 127.0.0.1 port 54321: …`
fn port_in_bind_line(line: &str) -> Option<u16> {
    number_after(line, "]:").or_else(|| number_after(line, " port "))
}

fn number_after(line: &str, marker: &str) -> Option<u16> {
    let rest = &line[line.find(marker)? + marker.len()..];
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use TunnelError::*;

    fn check(stderr: &str, expected: TunnelError) {
        assert_eq!(diagnose(stderr, Some(255)), expected, "{stderr}");
    }

    #[test]
    fn authentication_refused() {
        check(
            "user@203.0.113.10: Permission denied (publickey).",
            AuthFailed,
        );
        check(
            "Permission denied (publickey,password,keyboard-interactive).",
            AuthFailed,
        );
    }

    #[test]
    fn unknown_host_key_in_batch_mode() {
        check(
            "No ED25519 host key is known for 203.0.113.10 and you have requested strict checking.\n\
             Host key verification failed.",
            HostKeyUnknown,
        );
    }

    #[test]
    fn changed_host_key_wins_over_the_generic_verification_failure() {
        check(
            "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
             @    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n\
             @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
             IT IS POSSIBLE THAT SOMEONE IS DOING SOMETHING NASTY!\n\
             Host key for 203.0.113.10 has changed and you have requested strict checking.\n\
             Host key verification failed.",
            HostKeyMismatch,
        );
    }

    #[test]
    fn local_port_already_in_use() {
        check(
            "bind [127.0.0.1]:54321: Address already in use\n\
             channel_setup_fwd_listener_tcpip: cannot listen to port: 54321\n\
             Could not request local forwarding.",
            PortInUse { port: Some(54321) },
        );
        check(
            "bind [::1]:1080: Address already in use",
            PortInUse { port: Some(1080) },
        );
        check(
            "bind 127.0.0.1 port 8080: Address already in use",
            PortInUse { port: Some(8080) },
        );
        check(
            "Could not request local forwarding.",
            PortInUse { port: None },
        );
    }

    #[test]
    fn remote_forward_refused_by_the_server() {
        check(
            "Error: remote port forwarding failed for listen port 8080",
            RemoteForwardFailed { port: Some(8080) },
        );
    }

    #[test]
    fn host_name_cannot_be_resolved() {
        check(
            "ssh: Could not resolve hostname db.exmaple.com: nodename nor servname provided, or not known",
            HostNotFound,
        );
    }

    #[test]
    fn connection_failures() {
        check(
            "ssh: connect to host 203.0.113.10 port 22: Connection refused",
            ConnectionRefused,
        );
        check(
            "ssh: connect to host 203.0.113.10 port 22: Operation timed out",
            ConnectionTimedOut,
        );
        check(
            "ssh: connect to host 10.0.0.1 port 22: Connection timed out",
            ConnectionTimedOut,
        );
        check(
            "ssh: connect to host 10.0.0.1 port 22: Network is unreachable",
            NetworkUnreachable,
        );
        check(
            "ssh: connect to host 10.0.0.1 port 22: No route to host",
            NetworkUnreachable,
        );
    }

    #[test]
    fn established_connection_lost() {
        check(
            "Timeout, server 203.0.113.10 not responding.",
            ConnectionLost,
        );
        check("client_loop: send disconnect: Broken pipe", ConnectionLost);
        check(
            "Connection to 203.0.113.10 closed by remote host.",
            ConnectionLost,
        );
        check("Connection closed by 203.0.113.10 port 22", ConnectionLost);
        check(
            "kex_exchange_identification: read: Connection reset by peer",
            ConnectionLost,
        );
    }

    #[test]
    fn harmless_warnings_do_not_hide_the_real_cause() {
        check(
            "Warning: Permanently added '203.0.113.10' (ED25519) to the list of known hosts.\n\
             Timeout, server 203.0.113.10 not responding.",
            ConnectionLost,
        );
    }

    #[test]
    fn anything_else_is_unknown_with_the_exit_code() {
        assert_eq!(
            diagnose("", Some(255)),
            Unknown {
                exit_code: Some(255)
            }
        );
        assert_eq!(diagnose("something new", None), Unknown { exit_code: None });
    }
}
