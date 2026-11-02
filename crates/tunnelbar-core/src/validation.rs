//! Semantic checks of a configuration, including the rules preventing `ssh` option injection.
//!
//! Errors are typed so that the UI can translate them and attach them to the faulty field.

use std::collections::HashMap;

use crate::model::{CONFIG_VERSION, Config, Forward};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ValidationError {
    /// Index of the faulty tunnel, if the error is not about the whole file.
    pub tunnel: Option<usize>,
    /// Index of the faulty forward within the tunnel, if any.
    pub forward: Option<usize>,
    #[serde(flatten)]
    pub kind: ValidationErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ValidationErrorKind {
    UnsupportedVersion {
        found: u32,
    },
    /// Ids are 1 to 64 characters among `a-z`, `0-9`, `-` and `_`, starting with a letter or digit.
    InvalidId,
    DuplicateId {
        id: String,
    },
    EmptyName,
    /// Empty, starting with `-`, or containing whitespace or control characters.
    InvalidHost,
    InvalidUser,
    InvalidForwardHost,
    ZeroPort {
        field: PortField,
    },
    NoForward,
    /// The same local port is already listened on by another forward.
    LocalPortConflict {
        port: u16,
        /// Location of the other forward, renamed so as not to clash with the error's own.
        #[serde(rename = "other_tunnel")]
        tunnel: usize,
        #[serde(rename = "other_forward")]
        forward: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortField {
    SshPort,
    LocalPort,
    RemotePort,
}

/// Returns every problem found, so that they can all be shown at once.
pub fn validate(config: &Config) -> Result<(), Vec<ValidationError>> {
    use ValidationErrorKind::*;

    let mut errors = Vec::new();
    if config.version != CONFIG_VERSION {
        errors.push(ValidationError {
            tunnel: None,
            forward: None,
            kind: UnsupportedVersion {
                found: config.version,
            },
        });
    }

    let mut first_index_of_id = HashMap::new();
    // Local port -> (tunnel, forward) of the first forward listening on it.
    let mut listeners: HashMap<u16, (usize, usize)> = HashMap::new();

    for (t, tunnel) in config.tunnels.iter().enumerate() {
        let mut report = |forward, kind| {
            errors.push(ValidationError {
                tunnel: Some(t),
                forward,
                kind,
            })
        };

        if !is_valid_id(&tunnel.id) {
            report(None, InvalidId);
        }
        if first_index_of_id.insert(tunnel.id.as_str(), t).is_some() {
            report(
                None,
                DuplicateId {
                    id: tunnel.id.clone(),
                },
            );
        }
        if tunnel.name.trim().is_empty() {
            report(None, EmptyName);
        }
        if !is_safe_argument(&tunnel.host) {
            report(None, InvalidHost);
        }
        if tunnel
            .user
            .as_deref()
            .is_some_and(|user| !is_safe_argument(user))
        {
            report(None, InvalidUser);
        }
        if tunnel.port == Some(0) {
            report(
                None,
                ZeroPort {
                    field: PortField::SshPort,
                },
            );
        }
        if tunnel.forwards.is_empty() {
            report(None, NoForward);
        }

        for (f, forward) in tunnel.forwards.iter().enumerate() {
            let (ports, host, listen_port) = match forward {
                Forward::Local {
                    local_port,
                    remote_host,
                    remote_port,
                } => (
                    vec![
                        (*local_port, PortField::LocalPort),
                        (*remote_port, PortField::RemotePort),
                    ],
                    Some(remote_host),
                    Some(*local_port),
                ),
                Forward::Remote {
                    remote_port,
                    local_host,
                    local_port,
                } => (
                    vec![
                        (*local_port, PortField::LocalPort),
                        (*remote_port, PortField::RemotePort),
                    ],
                    Some(local_host),
                    None,
                ),
                Forward::Dynamic { local_port } => (
                    vec![(*local_port, PortField::LocalPort)],
                    None,
                    Some(*local_port),
                ),
            };
            for (port, field) in ports {
                if port == 0 {
                    report(Some(f), ZeroPort { field });
                }
            }
            if host.is_some_and(|host| !is_safe_argument(host)) {
                report(Some(f), InvalidForwardHost);
            }
            if let Some(port) = listen_port.filter(|&port| port != 0) {
                if let Some(&(tunnel, forward)) = listeners.get(&port) {
                    report(
                        Some(f),
                        LocalPortConflict {
                            port,
                            tunnel,
                            forward,
                        },
                    );
                } else {
                    listeners.insert(port, (t, f));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn is_valid_id(id: &str) -> bool {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    (1..=64).contains(&id.len())
        && id.chars().all(allowed)
        && id.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// A value passed to `ssh` as (part of) an argument: it must not be read as an option
/// nor be split, whatever the way it is later combined.
fn is_safe_argument(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CONFIG_VERSION, Forward, Tunnel};
    use ValidationErrorKind::*;

    fn tunnel(id: &str, local_port: u16) -> Tunnel {
        Tunnel {
            id: id.into(),
            name: format!("Tunnel {id}"),
            host: "203.0.113.10".into(),
            user: None,
            port: None,
            autostart: false,
            auto_reconnect: true,
            accept_new_host_key: false,
            forwards: vec![Forward::Local {
                local_port,
                remote_host: "127.0.0.1".into(),
                remote_port: 5432,
            }],
        }
    }

    fn config(tunnels: Vec<Tunnel>) -> Config {
        Config {
            version: CONFIG_VERSION,
            tunnels,
        }
    }

    fn errors_of(tunnel: Tunnel) -> Vec<ValidationErrorKind> {
        match validate(&config(vec![tunnel])) {
            Ok(()) => vec![],
            Err(errors) => errors.into_iter().map(|error| error.kind).collect(),
        }
    }

    fn error(
        tunnel: Option<usize>,
        forward: Option<usize>,
        kind: ValidationErrorKind,
    ) -> ValidationError {
        ValidationError {
            tunnel,
            forward,
            kind,
        }
    }

    #[test]
    fn accepts_a_valid_configuration() {
        assert_eq!(
            validate(&config(vec![
                tunnel("prod", 54321),
                tunnel("staging", 54322)
            ])),
            Ok(())
        );
    }

    #[test]
    fn accepts_an_empty_configuration() {
        assert_eq!(validate(&Config::default()), Ok(()));
    }

    #[test]
    fn rejects_an_unsupported_version() {
        let config = Config {
            version: 2,
            tunnels: vec![],
        };
        assert_eq!(
            validate(&config),
            Err(vec![error(None, None, UnsupportedVersion { found: 2 })])
        );
    }

    #[test]
    fn accepts_well_formed_ids() {
        for id in ["a", "prod", "pg-prod_2", "0db", &"x".repeat(64)] {
            assert_eq!(errors_of(tunnel(id, 54321)), vec![], "{id}");
        }
    }

    #[test]
    fn rejects_malformed_ids() {
        for id in [
            "",
            "-prod",
            "_prod",
            "Prod",
            "pg prod",
            "pg/prod",
            "pgé",
            &"x".repeat(65),
        ] {
            assert_eq!(errors_of(tunnel(id, 54321)), vec![InvalidId], "{id:?}");
        }
    }

    #[test]
    fn rejects_duplicate_ids_on_the_second_occurrence() {
        let result = validate(&config(vec![tunnel("prod", 1), tunnel("prod", 2)]));
        assert_eq!(
            result,
            Err(vec![error(
                Some(1),
                None,
                DuplicateId { id: "prod".into() }
            )])
        );
    }

    #[test]
    fn rejects_a_blank_name() {
        let mut t = tunnel("prod", 54321);
        t.name = "  ".into();
        assert_eq!(errors_of(t), vec![EmptyName]);
    }

    #[test]
    fn accepts_hosts_ips_and_aliases() {
        for host in [
            "203.0.113.10",
            "db.example.com",
            "my-alias",
            "::1",
            "fe80::1%en0",
        ] {
            let mut t = tunnel("prod", 54321);
            t.host = host.into();
            assert_eq!(errors_of(t), vec![], "{host}");
        }
    }

    #[test]
    fn rejects_hosts_that_could_inject_ssh_options_or_arguments() {
        for host in [
            "",
            "-oProxyCommand=touch /tmp/pwned",
            "host -p 22",
            "host\tx",
            "host\nx",
            "a\u{0}b",
        ] {
            let mut t = tunnel("prod", 54321);
            t.host = host.into();
            assert_eq!(errors_of(t), vec![InvalidHost], "{host:?}");
        }
    }

    #[test]
    fn rejects_users_that_could_inject_ssh_options_or_arguments() {
        for user in ["", "-oProxyCommand=x", "a b", "a\nb"] {
            let mut t = tunnel("prod", 54321);
            t.user = Some(user.into());
            assert_eq!(errors_of(t), vec![InvalidUser], "{user:?}");
        }
        let mut t = tunnel("prod", 54321);
        t.user = Some("deploy".into());
        assert_eq!(errors_of(t), vec![]);
    }

    #[test]
    fn rejects_port_zero_everywhere() {
        let mut t = tunnel("prod", 0);
        t.port = Some(0);
        t.forwards.push(Forward::Remote {
            remote_port: 0,
            local_host: "localhost".into(),
            local_port: 3000,
        });
        t.forwards.push(Forward::Dynamic { local_port: 0 });
        let result = validate(&config(vec![t]));
        assert_eq!(
            result,
            Err(vec![
                error(
                    Some(0),
                    None,
                    ZeroPort {
                        field: PortField::SshPort
                    }
                ),
                error(
                    Some(0),
                    Some(0),
                    ZeroPort {
                        field: PortField::LocalPort
                    }
                ),
                error(
                    Some(0),
                    Some(1),
                    ZeroPort {
                        field: PortField::RemotePort
                    }
                ),
                error(
                    Some(0),
                    Some(2),
                    ZeroPort {
                        field: PortField::LocalPort
                    }
                ),
            ])
        );
    }

    #[test]
    fn rejects_a_tunnel_without_forward() {
        let mut t = tunnel("prod", 54321);
        t.forwards.clear();
        assert_eq!(errors_of(t), vec![NoForward]);
    }

    #[test]
    fn rejects_invalid_forward_hosts() {
        let mut t = tunnel("prod", 54321);
        t.forwards = vec![
            Forward::Local {
                local_port: 1,
                remote_host: "".into(),
                remote_port: 2,
            },
            Forward::Remote {
                remote_port: 3,
                local_host: "a b".into(),
                local_port: 4,
            },
        ];
        let result = validate(&config(vec![t]));
        assert_eq!(
            result,
            Err(vec![
                error(Some(0), Some(0), InvalidForwardHost),
                error(Some(0), Some(1), InvalidForwardHost),
            ])
        );
    }

    #[test]
    fn rejects_the_same_local_port_across_tunnels() {
        let result = validate(&config(vec![
            tunnel("prod", 54321),
            tunnel("staging", 54321),
        ]));
        assert_eq!(
            result,
            Err(vec![error(
                Some(1),
                Some(0),
                LocalPortConflict {
                    port: 54321,
                    tunnel: 0,
                    forward: 0
                }
            )])
        );
    }

    #[test]
    fn rejects_the_same_local_port_within_a_tunnel_including_socks() {
        let mut t = tunnel("prod", 1080);
        t.forwards.push(Forward::Dynamic { local_port: 1080 });
        let result = validate(&config(vec![t]));
        assert_eq!(
            result,
            Err(vec![error(
                Some(0),
                Some(1),
                LocalPortConflict {
                    port: 1080,
                    tunnel: 0,
                    forward: 0
                }
            )])
        );
    }

    #[test]
    fn remote_forward_ports_do_not_conflict_with_local_ones() {
        let mut t = tunnel("prod", 8080);
        t.forwards.push(Forward::Remote {
            remote_port: 8080,
            local_host: "localhost".into(),
            local_port: 8080,
        });
        assert_eq!(errors_of(t), vec![]);
    }
}
