//! Turns a tunnel definition into the `ssh` invocation that carries it.
//!
//! The result is an argument vector meant to be executed directly, never through a shell.

use std::path::PathBuf;

use crate::model::{Forward, Tunnel};

/// The system OpenSSH client.
pub const SSH_PROGRAM: &str = "/usr/bin/ssh";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl SshCommand {
    pub fn for_tunnel(tunnel: &Tunnel) -> Self {
        let mut args: Vec<String> = vec!["-N".into()];
        let mut option = |value: &str| args.extend(["-o".into(), value.into()]);
        // Fail fast instead of keeping a process that forwards nothing.
        option("ExitOnForwardFailure=yes");
        // Detect a dead connection in under a minute.
        option("ServerAliveInterval=15");
        option("ServerAliveCountMax=3");
        // Never wait for a prompt nobody can answer.
        option("BatchMode=yes");
        option("ConnectTimeout=15");
        // Own the connection: a multiplexed client would exit right away and hand the
        // forwards over to a master process we do not supervise.
        option("ControlMaster=no");
        option("ControlPath=none");
        if tunnel.accept_new_host_key {
            option("StrictHostKeyChecking=accept-new");
        }

        if let Some(port) = tunnel.port {
            args.extend(["-p".into(), port.to_string()]);
        }
        if let Some(user) = &tunnel.user {
            args.extend(["-l".into(), user.clone()]);
        }
        for forward in &tunnel.forwards {
            let (flag, spec) = match forward {
                Forward::Local {
                    local_port,
                    remote_host,
                    remote_port,
                } => (
                    "-L",
                    format!("{local_port}:{}:{remote_port}", bracketed(remote_host)),
                ),
                Forward::Remote {
                    remote_port,
                    local_host,
                    local_port,
                } => (
                    "-R",
                    format!("{remote_port}:{}:{local_port}", bracketed(local_host)),
                ),
                Forward::Dynamic { local_port } => ("-D", local_port.to_string()),
            };
            args.extend([flag.into(), spec]);
        }
        // Whatever the host looks like, ssh cannot read it as an option after `--`.
        args.extend(["--".into(), tunnel.host.clone()]);

        SshCommand {
            program: PathBuf::from(SSH_PROGRAM),
            args,
        }
    }

    /// The command as a line to paste in a terminal, quoted for POSIX shells.
    pub fn to_shell_line(&self) -> String {
        std::iter::once(self.program.to_string_lossy().into_owned())
            .chain(self.args.iter().cloned())
            .map(|word| shell_quoted(&word))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn shell_quoted(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_-./:=@%+,".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// IPv6 addresses must be bracketed inside `port:host:port` specifications.
fn bracketed(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Forward;

    fn prod() -> Tunnel {
        Tunnel {
            id: "prod".into(),
            name: "Prod".into(),
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

    const ALWAYS: [&str; 15] = [
        "-N",
        "-o",
        "ExitOnForwardFailure=yes",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=15",
        "-o",
        "ControlMaster=no",
        "-o",
        "ControlPath=none",
    ];

    fn expected(extra: &[&str]) -> Vec<String> {
        ALWAYS
            .iter()
            .chain(extra)
            .map(|arg| arg.to_string())
            .collect()
    }

    #[test]
    fn uses_the_system_ssh() {
        assert_eq!(
            SshCommand::for_tunnel(&prod()).program,
            PathBuf::from("/usr/bin/ssh")
        );
    }

    #[test]
    fn builds_the_original_use_case() {
        assert_eq!(
            SshCommand::for_tunnel(&prod()).args,
            expected(&["-L", "54321:127.0.0.1:5432", "--", "203.0.113.10"])
        );
    }

    #[test]
    fn adds_user_port_and_host_key_policy_when_set() {
        let mut tunnel = prod();
        tunnel.user = Some("deploy".into());
        tunnel.port = Some(2222);
        tunnel.accept_new_host_key = true;
        assert_eq!(
            SshCommand::for_tunnel(&tunnel).args,
            expected(&[
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-p",
                "2222",
                "-l",
                "deploy",
                "-L",
                "54321:127.0.0.1:5432",
                "--",
                "203.0.113.10",
            ])
        );
    }

    #[test]
    fn builds_every_forward_type_in_order() {
        let mut tunnel = prod();
        tunnel.forwards.push(Forward::Remote {
            remote_port: 8080,
            local_host: "localhost".into(),
            local_port: 3000,
        });
        tunnel.forwards.push(Forward::Dynamic { local_port: 1080 });
        assert_eq!(
            SshCommand::for_tunnel(&tunnel).args,
            expected(&[
                "-L",
                "54321:127.0.0.1:5432",
                "-R",
                "8080:localhost:3000",
                "-D",
                "1080",
                "--",
                "203.0.113.10",
            ])
        );
    }

    #[test]
    fn brackets_ipv6_addresses_inside_forward_specifications() {
        let mut tunnel = prod();
        tunnel.forwards = vec![
            Forward::Local {
                local_port: 1,
                remote_host: "::1".into(),
                remote_port: 2,
            },
            Forward::Remote {
                remote_port: 3,
                local_host: "fe80::1".into(),
                local_port: 4,
            },
        ];
        let args = SshCommand::for_tunnel(&tunnel).args;
        assert!(args.contains(&"1:[::1]:2".to_string()), "{args:?}");
        assert!(args.contains(&"3:[fe80::1]:4".to_string()), "{args:?}");
    }

    #[test]
    fn shell_line_of_the_original_use_case_needs_no_quoting() {
        let line = SshCommand::for_tunnel(&prod()).to_shell_line();
        assert!(
            line.starts_with("/usr/bin/ssh -N -o ExitOnForwardFailure=yes "),
            "{line}"
        );
        assert!(
            line.ends_with(" -L 54321:127.0.0.1:5432 -- 203.0.113.10"),
            "{line}"
        );
    }

    #[test]
    fn shell_line_quotes_arguments_with_special_characters() {
        let command = SshCommand {
            program: PathBuf::from("/usr/bin/ssh"),
            args: vec!["$(reboot)".into(), "it's".into(), "[::1]".into(), "".into()],
        };
        assert_eq!(
            command.to_shell_line(),
            r#"/usr/bin/ssh '$(reboot)' 'it'\''s' '[::1]' ''"#
        );
    }

    #[test]
    fn the_destination_always_comes_after_the_end_of_options_marker() {
        let args = SshCommand::for_tunnel(&prod()).args;
        assert_eq!(&args[args.len() - 2..], ["--", "203.0.113.10"]);
    }
}
