//! Which state changes deserve a macOS notification, and their text.

use tunnelbar_core::error::TunnelError;
use tunnelbar_core::state::TunnelState;

use crate::i18n::Translator;

/// The error to notify when a tunnel has just failed for good.
pub fn newly_failed<'a>(
    previous: Option<&TunnelState>,
    current: &'a TunnelState,
) -> Option<&'a TunnelError> {
    match (previous, current) {
        (Some(TunnelState::Failed { .. }) | None, _) => None,
        (Some(_), TunnelState::Failed { error }) => Some(error),
        _ => None,
    }
}

/// Same keys and rules as `errorMessage` in `src/lib/format.ts`.
pub fn error_message(t: &Translator, error: &TunnelError) -> String {
    let json = serde_json::to_value(error).unwrap_or_default();
    let code = json["code"].as_str().unwrap_or("unknown");
    let port = json["port"].as_u64();
    let exit_code = json["exit_code"]
        .as_i64()
        .map_or_else(|| "?".to_owned(), |code| code.to_string());
    let key = match port {
        Some(_) => format!("error.{code}.port"),
        None => format!("error.{code}"),
    };
    let port = port.map(|port| port.to_string()).unwrap_or_default();
    t.t(&key, &[("port", &port), ("exit_code", &exit_code)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn failed(error: TunnelError) -> TunnelState {
        TunnelState::Failed { error }
    }

    #[test]
    fn notifies_on_entering_the_failed_state_only() {
        let auth = failed(TunnelError::AuthFailed);
        assert_eq!(
            newly_failed(Some(&TunnelState::Up), &auth),
            Some(&TunnelError::AuthFailed)
        );
        assert_eq!(
            newly_failed(Some(&TunnelState::Connecting { attempt: 0 }), &auth),
            Some(&TunnelError::AuthFailed)
        );
        assert_eq!(newly_failed(Some(&auth), &auth), None);
        assert_eq!(
            newly_failed(None, &auth),
            None,
            "nothing to report at startup"
        );
        let reconnecting = TunnelState::Reconnecting {
            attempt: 1,
            delay: Duration::from_secs(1),
            last_error: TunnelError::ConnectionLost,
        };
        assert_eq!(newly_failed(Some(&TunnelState::Up), &reconnecting), None);
        assert_eq!(
            newly_failed(Some(&TunnelState::Up), &TunnelState::Stopped),
            None
        );
    }

    #[test]
    fn messages_are_translated_with_their_port() {
        let t = Translator::new("en");
        assert_eq!(
            error_message(&t, &TunnelError::PortInUse { port: Some(54321) }),
            "Local port 54321 is already used by another program."
        );
        assert_eq!(
            error_message(&t, &TunnelError::PortInUse { port: None }),
            "A local port is already used by another program."
        );
        assert_eq!(
            error_message(&Translator::new("fr"), &TunnelError::AuthFailed),
            "Authentification refusée : le serveur n'a accepté aucune de vos clés SSH."
        );
        assert_eq!(
            error_message(
                &t,
                &TunnelError::Unknown {
                    exit_code: Some(255)
                }
            ),
            "ssh stopped unexpectedly (code 255). See the logs."
        );
    }
}
