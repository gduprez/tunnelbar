//! End-to-end test against a real SSH server, run on demand only:
//!
//! ```sh
//! TUNNELBAR_E2E_HOST=my-server cargo test -p tunnelbar-core --test end_to_end -- --ignored
//! ```
//!
//! It forwards a local port to the server's own SSH daemon (`127.0.0.1:22` seen from the
//! server) and reads the daemon's banner through the tunnel.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tunnelbar_core::manager::TunnelManager;
use tunnelbar_core::model::{Config, Forward, Tunnel};
use tunnelbar_core::process::{SystemSpawner, TcpPortProber};
use tunnelbar_core::state::TunnelState;

#[tokio::test]
#[ignore = "needs TUNNELBAR_E2E_HOST, an SSH server reachable with key authentication"]
async fn forwards_a_local_port_through_a_real_ssh_connection() {
    let host = std::env::var("TUNNELBAR_E2E_HOST").expect("TUNNELBAR_E2E_HOST");
    let local_port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let tunnel = Tunnel {
        id: "e2e".into(),
        name: "End to end".into(),
        host,
        user: std::env::var("TUNNELBAR_E2E_USER").ok(),
        port: None,
        autostart: false,
        auto_reconnect: false,
        accept_new_host_key: false,
        forwards: vec![Forward::Local {
            local_port,
            remote_host: "127.0.0.1".into(),
            remote_port: 22,
        }],
    };
    let mut manager = TunnelManager::new(SystemSpawner::new(), TcpPortProber);
    manager
        .apply_config(&Config {
            version: 1,
            tunnels: vec![tunnel],
        })
        .await;
    manager.start("e2e").unwrap();

    let mut status = manager.statuses().remove(0);
    for _ in 0..300 {
        status = manager.statuses().remove(0);
        if !matches!(
            status.state,
            TunnelState::Stopped | TunnelState::Connecting { .. }
        ) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(status.state, TunnelState::Up, "logs: {:?}", status.logs);

    let mut stream = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();
    let mut banner = [0; 7];
    tokio::time::timeout(Duration::from_secs(10), stream.read_exact(&mut banner))
        .await
        .expect("banner in time")
        .unwrap();
    assert_eq!(&banner, b"SSH-2.0");

    manager.shutdown().await;
    assert!(
        TcpStream::connect(("127.0.0.1", local_port)).await.is_err(),
        "tunnel closed"
    );
}
