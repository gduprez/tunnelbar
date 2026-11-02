//! Supervision of real processes: a scripted fake `ssh`, real ports, real time.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use std::sync::Arc;
use tunnelbar_core::error::TunnelError;
use tunnelbar_core::manager::TunnelManager;

use tunnelbar_core::model::{Config, Forward, Tunnel};
use tunnelbar_core::orphans::PidRegistry;
use tunnelbar_core::process::{PortProber, SystemSpawner, TcpPortProber};
use tunnelbar_core::state::TunnelState;
use tunnelbar_core::supervisor::TunnelStatus;

/// Writes an executable fake `ssh` running `body` (bash).
fn fake_ssh(dir: &tempfile::TempDir, body: &str) -> PathBuf {
    let path = dir.path().join("ssh");
    fs::write(&path, format!("#!/bin/bash\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Listens on the port of the `-L` forward, like ssh does.
const SERVE: &str = r#"
for arg in "$@"; do
  if [ "$previous" = "-L" ]; then port="${arg%%:*}"; fi
  previous="$arg"
done
echo "fake ssh listening on $port" >&2
exec python3 -c 'import socket, sys, time
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", int(sys.argv[1]))); s.listen(); time.sleep(3600)' "$port"
"#;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn config(local_port: u16) -> Config {
    Config {
        version: 1,
        tunnels: vec![Tunnel {
            id: "prod".into(),
            name: "Prod".into(),
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
        }],
    }
}

async fn until(
    manager: &TunnelManager<SystemSpawner, TcpPortProber>,
    predicate: impl Fn(&TunnelStatus) -> bool,
) -> TunnelStatus {
    for _ in 0..200 {
        let status = manager.statuses().remove(0);
        if predicate(&status) {
            return status;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("condition never met: {:?}", manager.statuses());
}

#[tokio::test]
async fn reports_an_authentication_failure_with_the_ssh_output() {
    let dir = tempfile::tempdir().unwrap();
    let ssh = fake_ssh(
        &dir,
        r#"echo "user@203.0.113.10: Permission denied (publickey)." >&2; exit 255"#,
    );
    let mut manager = TunnelManager::new(SystemSpawner::with_program(ssh), TcpPortProber);
    manager.apply_config(&config(free_port())).await;

    manager.start("prod").unwrap();
    let status = until(&manager, |s| matches!(s.state, TunnelState::Failed { .. })).await;
    assert_eq!(
        status.state,
        TunnelState::Failed {
            error: TunnelError::AuthFailed
        }
    );
    assert_eq!(
        status.logs,
        ["user@203.0.113.10: Permission denied (publickey)."]
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn comes_up_when_the_port_listens_and_frees_it_on_stop() {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port();
    let mut manager = TunnelManager::new(
        SystemSpawner::with_program(fake_ssh(&dir, SERVE)),
        TcpPortProber,
    );
    manager.apply_config(&config(port)).await;

    manager.start("prod").unwrap();
    let status = until(&manager, |s| s.state == TunnelState::Up).await;
    assert_eq!(status.logs, [format!("fake ssh listening on {port}")]);
    assert!(TcpPortProber.is_open(port).await);

    manager.stop("prod").unwrap();
    until(&manager, |s| s.state == TunnelState::Stopped).await;
    assert!(
        !TcpPortProber.is_open(port).await,
        "the process must be gone"
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn shutdown_kills_running_processes() {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port();
    let mut manager = TunnelManager::new(
        SystemSpawner::with_program(fake_ssh(&dir, SERVE)),
        TcpPortProber,
    );
    manager.apply_config(&config(port)).await;
    manager.start("prod").unwrap();
    until(&manager, |s| s.state == TunnelState::Up).await;

    manager.shutdown().await;
    assert!(!TcpPortProber.is_open(port).await);
}

#[tokio::test]
async fn a_missing_program_fails_to_spawn() {
    let mut manager = TunnelManager::new(
        SystemSpawner::with_program("/nonexistent/ssh"),
        TcpPortProber,
    );
    manager.apply_config(&config(free_port())).await;
    manager.start("prod").unwrap();
    let status = until(&manager, |s| matches!(s.state, TunnelState::Failed { .. })).await;
    assert_eq!(
        status.state,
        TunnelState::Failed {
            error: TunnelError::SpawnFailed
        }
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn running_processes_are_recorded_for_orphan_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port();
    let registry = Arc::new(PidRegistry::new(dir.path().join("ssh.pids")));
    let spawner =
        SystemSpawner::with_program(fake_ssh(&dir, SERVE)).with_registry(registry.clone());
    let mut manager = TunnelManager::new(spawner, TcpPortProber);
    manager.apply_config(&config(port)).await;

    manager.start("prod").unwrap();
    until(&manager, |s| s.state == TunnelState::Up).await;
    let previous = registry.previous();
    assert_eq!(previous.len(), 1);
    assert!(previous[0].1.is_some(), "start time recorded: {previous:?}");

    manager.stop("prod").unwrap();
    until(&manager, |s| s.state == TunnelState::Stopped).await;
    assert_eq!(registry.previous(), vec![]);
    manager.shutdown().await;
}

#[tokio::test]
async fn exited_processes_are_forgotten() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(PidRegistry::new(dir.path().join("ssh.pids")));
    let ssh = fake_ssh(
        &dir,
        r#"echo "Permission denied (publickey)." >&2; exit 255"#,
    );
    let spawner = SystemSpawner::with_program(ssh).with_registry(registry.clone());
    let mut manager = TunnelManager::new(spawner, TcpPortProber);
    manager.apply_config(&config(free_port())).await;

    manager.start("prod").unwrap();
    until(&manager, |s| matches!(s.state, TunnelState::Failed { .. })).await;
    assert_eq!(registry.previous(), vec![]);
    manager.shutdown().await;
}
