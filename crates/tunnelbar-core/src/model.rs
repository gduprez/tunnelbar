//! Tunnel definitions as stored in `tunnels.toml`.

use serde::{Deserialize, Serialize};

/// Current version of the configuration file format.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(rename = "tunnel", default, skip_serializing_if = "Vec::is_empty")]
    pub tunnels: Vec<Tunnel>,
}

/// One SSH connection carrying one or more port forwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    /// Host name, IP address or alias from `~/.ssh/config`.
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "enabled")]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub accept_new_host_key: bool,
    #[serde(rename = "forward", default)]
    pub forwards: Vec<Forward>,
}

fn enabled() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Forward {
    /// `-L local_port:remote_host:remote_port`
    Local {
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    },
    /// `-R remote_port:local_host:local_port`
    Remote {
        remote_port: u16,
        local_host: String,
        local_port: u16,
    },
    /// `-D local_port` (SOCKS proxy)
    Dynamic { local_port: u16 },
}

/// A syntax or type error in the configuration file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    pub message: String,
    /// 1-based line of the error, when known.
    pub line: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: CONFIG_VERSION,
            tunnels: Vec::new(),
        }
    }
}

impl Config {
    /// Parses the file content. Semantic checks are done separately by validation.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        toml::from_str(text).map_err(|error| ParseError {
            message: error.message().to_owned(),
            line: error.span().map(|span| line_of(text, span.start)),
        })
    }

    /// Replaces the tunnel whose id was `previous_id` (keeping its position), or appends it.
    pub fn with_tunnel(&self, tunnel: Tunnel, previous_id: Option<&str>) -> Config {
        let mut config = self.clone();
        let existing =
            previous_id.and_then(|previous| config.tunnels.iter().position(|t| t.id == previous));
        match existing {
            Some(index) => config.tunnels[index] = tunnel,
            None => config.tunnels.push(tunnel),
        }
        config
    }

    pub fn without_tunnel(&self, id: &str) -> Config {
        let mut config = self.clone();
        config.tunnels.retain(|tunnel| tunnel.id != id);
        config
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("a configuration is always representable in TOML")
    }
}

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL_USE_CASE: &str = r#"
version = 1

[[tunnel]]
id = "prod"
name = "Prod server"
host = "203.0.113.10"

[[tunnel.forward]]
type = "local"
local_port = 54321
remote_host = "127.0.0.1"
remote_port = 5432
"#;

    fn prod() -> Tunnel {
        Tunnel {
            id: "prod".into(),
            name: "Prod server".into(),
            host: "203.0.113.10".into(),
            user: None,
            port: None,
            autostart: false,
            auto_reconnect: true,
            accept_new_host_key: false,
            forwards: vec![Forward::Local {
                local_port: 54321,
                remote_host: "127.0.0.1".into(),
                remote_port: 5432,
            }],
        }
    }

    #[test]
    fn parses_the_original_use_case_with_defaults() {
        let config = Config::parse(ORIGINAL_USE_CASE).unwrap();
        assert_eq!(
            config,
            Config {
                version: 1,
                tunnels: vec![prod()]
            }
        );
    }

    #[test]
    fn parses_every_field_and_forward_type() {
        let text = r#"
version = 1

[[tunnel]]
id = "lab"
name = "Lab"
host = "lab-alias"
user = "deploy"
port = 2222
autostart = true
auto_reconnect = false
accept_new_host_key = true

[[tunnel.forward]]
type = "remote"
remote_port = 8080
local_host = "127.0.0.1"
local_port = 3000

[[tunnel.forward]]
type = "dynamic"
local_port = 1080
"#;
        let tunnel = &Config::parse(text).unwrap().tunnels[0];
        assert_eq!(tunnel.user.as_deref(), Some("deploy"));
        assert_eq!(tunnel.port, Some(2222));
        assert!(tunnel.autostart && !tunnel.auto_reconnect && tunnel.accept_new_host_key);
        assert_eq!(
            tunnel.forwards,
            vec![
                Forward::Remote {
                    remote_port: 8080,
                    local_host: "127.0.0.1".into(),
                    local_port: 3000
                },
                Forward::Dynamic { local_port: 1080 },
            ]
        );
    }

    #[test]
    fn a_file_without_tunnels_is_valid() {
        assert_eq!(Config::parse("version = 1").unwrap(), Config::default());
    }

    #[test]
    fn default_config_has_current_version_and_no_tunnel() {
        assert_eq!(
            Config::default(),
            Config {
                version: CONFIG_VERSION,
                tunnels: vec![]
            }
        );
    }

    #[test]
    fn rejects_unknown_fields_to_catch_typos() {
        let text = ORIGINAL_USE_CASE.replace("host =", "hots =");
        assert!(Config::parse(&text).is_err());
        let text = ORIGINAL_USE_CASE.replace("remote_port =", "remote_prot =");
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn rejects_an_unknown_forward_type() {
        let text = ORIGINAL_USE_CASE.replace(r#"type = "local""#, r#"type = "tcp""#);
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn rejects_a_port_out_of_range() {
        let text = ORIGINAL_USE_CASE.replace("54321", "70000");
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn requires_the_version() {
        let text = ORIGINAL_USE_CASE.replace("version = 1", "");
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn parse_error_reports_the_line() {
        let error = Config::parse("version = 1\n\n[[tunnel]]\nid = 42\n").unwrap_err();
        assert_eq!(error.line, Some(4));
    }

    fn named(id: &str) -> Tunnel {
        Tunnel {
            id: id.into(),
            ..prod()
        }
    }

    fn ids(config: &Config) -> Vec<&str> {
        config
            .tunnels
            .iter()
            .map(|tunnel| tunnel.id.as_str())
            .collect()
    }

    #[test]
    fn with_tunnel_appends_a_new_tunnel() {
        let config = Config::default().with_tunnel(named("a"), None);
        let config = config.with_tunnel(named("b"), None);
        assert_eq!(ids(&config), ["a", "b"]);
    }

    #[test]
    fn with_tunnel_replaces_in_place_even_when_renamed() {
        let config = Config::default()
            .with_tunnel(named("a"), None)
            .with_tunnel(named("b"), None)
            .with_tunnel(named("c"), None);
        let config = config.with_tunnel(named("b2"), Some("b"));
        assert_eq!(ids(&config), ["a", "b2", "c"]);
    }

    #[test]
    fn with_tunnel_appends_when_the_previous_id_is_unknown() {
        let config = Config::default().with_tunnel(named("a"), Some("gone"));
        assert_eq!(ids(&config), ["a"]);
    }

    #[test]
    fn without_tunnel_removes_it() {
        let config = Config::default()
            .with_tunnel(named("a"), None)
            .with_tunnel(named("b"), None);
        assert_eq!(ids(&config.without_tunnel("a")), ["b"]);
        assert_eq!(ids(&config.without_tunnel("zzz")), ["a", "b"]);
    }

    #[test]
    fn serialization_round_trips() {
        let mut lab = prod();
        lab.id = "lab".into();
        lab.user = Some("deploy".into());
        lab.port = Some(2222);
        lab.forwards.push(Forward::Dynamic { local_port: 1080 });
        let config = Config {
            version: 1,
            tunnels: vec![prod(), lab],
        };
        assert_eq!(Config::parse(&config.to_toml()).unwrap(), config);
    }

    #[test]
    fn serialization_uses_the_documented_layout() {
        let text = Config {
            version: 1,
            tunnels: vec![prod()],
        }
        .to_toml();
        assert!(text.contains("[[tunnel]]"), "{text}");
        assert!(text.contains("[[tunnel.forward]]"), "{text}");
        assert!(text.contains(r#"type = "local""#), "{text}");
        assert!(!text.contains("user"), "absent options are omitted: {text}");
    }
}
