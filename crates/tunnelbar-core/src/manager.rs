//! Owns one supervisor per configured tunnel and routes user commands to them.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::model::{Config, Tunnel};
use crate::process::{PortProber, ProcessSpawner};
use crate::state::TunnelState;
use crate::supervisor::{Command, SupervisorHandle, TunnelStatus, spawn_supervisor};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManagerError {
    #[error("no tunnel with id {0:?}")]
    UnknownTunnel(String),
}

pub struct TunnelManager<S: ProcessSpawner, P: PortProber> {
    spawner: Arc<S>,
    prober: Arc<P>,
    /// In configuration order.
    supervisors: Vec<(Tunnel, SupervisorHandle)>,
    updates: broadcast::Sender<TunnelStatus>,
}

impl<S: ProcessSpawner, P: PortProber> TunnelManager<S, P> {
    pub fn new(spawner: S, prober: P) -> Self {
        TunnelManager {
            spawner: Arc::new(spawner),
            prober: Arc::new(prober),
            supervisors: Vec::new(),
            updates: broadcast::channel(256).0,
        }
    }

    /// Every status change of every tunnel.
    pub fn subscribe(&self) -> broadcast::Receiver<TunnelStatus> {
        self.updates.subscribe()
    }

    /// Adopts a new configuration: added tunnels start stopped, removed ones are stopped,
    /// modified ones are restarted with their new definition if they were running.
    pub async fn apply_config(&mut self, config: &Config) {
        let mut previous = std::mem::take(&mut self.supervisors);
        for tunnel in &config.tunnels {
            let existing = previous
                .iter()
                .position(|(old, _)| old.id == tunnel.id)
                .map(|index| previous.remove(index));
            let handle = match existing {
                Some((old, handle)) if old == *tunnel => handle,
                Some((_, handle)) => {
                    let was_running = !matches!(
                        handle.status().state,
                        TunnelState::Stopped | TunnelState::Failed { .. }
                    );
                    handle.shutdown().await;
                    let handle = self.supervise(tunnel);
                    if was_running {
                        handle.send(Command::Start);
                    }
                    handle
                }
                None => self.supervise(tunnel),
            };
            self.supervisors.push((tunnel.clone(), handle));
        }
        for (_, removed) in previous {
            removed.shutdown().await;
        }
    }

    fn supervise(&self, tunnel: &Tunnel) -> SupervisorHandle {
        spawn_supervisor(
            tunnel.clone(),
            self.spawner.clone(),
            self.prober.clone(),
            self.updates.clone(),
        )
    }

    pub fn tunnels(&self) -> Vec<Tunnel> {
        self.supervisors
            .iter()
            .map(|(tunnel, _)| tunnel.clone())
            .collect()
    }

    /// Current status of every tunnel, in configuration order.
    pub fn statuses(&self) -> Vec<TunnelStatus> {
        self.supervisors
            .iter()
            .map(|(_, handle)| handle.status())
            .collect()
    }

    pub fn start(&self, id: &str) -> Result<(), ManagerError> {
        self.send(id, Command::Start)
    }

    pub fn stop(&self, id: &str) -> Result<(), ManagerError> {
        self.send(id, Command::Stop)
    }

    fn send(&self, id: &str, command: Command) -> Result<(), ManagerError> {
        let (_, handle) = self
            .supervisors
            .iter()
            .find(|(tunnel, _)| tunnel.id == id)
            .ok_or_else(|| ManagerError::UnknownTunnel(id.to_owned()))?;
        handle.send(command);
        Ok(())
    }

    /// Starts the tunnels marked `autostart`.
    pub fn autostart(&self) {
        for (tunnel, handle) in &self.supervisors {
            if tunnel.autostart {
                handle.send(Command::Start);
            }
        }
    }

    /// The Mac woke up or changed network: pending reconnections are attempted now.
    pub fn network_changed(&self) {
        for (_, handle) in &self.supervisors {
            handle.send(Command::NetworkChanged);
        }
    }

    /// Stops every tunnel and waits for their `ssh` processes to be gone.
    pub async fn shutdown(&mut self) {
        for (_, handle) in std::mem::take(&mut self.supervisors) {
            handle.shutdown().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{Behavior, FakeNetwork, FakeProber, FakeSpawner};
    use crate::model::{CONFIG_VERSION, Forward};
    use std::time::Duration;

    type Manager = TunnelManager<FakeSpawner, FakeProber>;

    fn tunnel(id: &str, local_port: u16) -> Tunnel {
        Tunnel {
            id: id.into(),
            name: id.into(),
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

    async fn manager(network: &FakeNetwork, tunnels: Vec<Tunnel>) -> Manager {
        let mut manager = TunnelManager::new(network.spawner(), network.prober());
        manager.apply_config(&config(tunnels)).await;
        manager
    }

    fn state_of(manager: &Manager, id: &str) -> TunnelState {
        manager
            .statuses()
            .into_iter()
            .find(|status| status.id == id)
            .expect("known tunnel")
            .state
    }

    /// Lets the supervisors run for a (paused) second.
    async fn settle() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn lists_configured_tunnels_stopped_in_order() {
        let manager = manager(&FakeNetwork::new(), vec![tunnel("b", 2), tunnel("a", 1)]).await;
        let statuses = manager.statuses();
        let ids: Vec<_> = statuses.iter().map(|status| status.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"]);
        assert!(
            statuses
                .iter()
                .all(|status| status.state == TunnelState::Stopped)
        );
        assert_eq!(manager.tunnels(), vec![tunnel("b", 2), tunnel("a", 1)]);
    }

    #[tokio::test(start_paused = true)]
    async fn starts_and_stops_a_tunnel_by_id() {
        let network = FakeNetwork::new();
        let manager = manager(&network, vec![tunnel("a", 1), tunnel("b", 2)]).await;

        manager.start("b").unwrap();
        settle().await;
        assert_eq!(state_of(&manager, "b"), TunnelState::Up);
        assert_eq!(state_of(&manager, "a"), TunnelState::Stopped);

        manager.stop("b").unwrap();
        settle().await;
        assert_eq!(state_of(&manager, "b"), TunnelState::Stopped);
        assert_eq!(network.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_unknown_ids() {
        let manager = manager(&FakeNetwork::new(), vec![tunnel("a", 1)]).await;
        assert_eq!(
            manager.start("nope"),
            Err(ManagerError::UnknownTunnel("nope".into()))
        );
        assert_eq!(
            manager.stop("nope"),
            Err(ManagerError::UnknownTunnel("nope".into()))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn autostart_starts_only_flagged_tunnels() {
        let mut flagged = tunnel("auto", 1);
        flagged.autostart = true;
        let manager = manager(&FakeNetwork::new(), vec![flagged, tunnel("manual", 2)]).await;

        manager.autostart();
        settle().await;
        assert_eq!(state_of(&manager, "auto"), TunnelState::Up);
        assert_eq!(state_of(&manager, "manual"), TunnelState::Stopped);
    }

    #[tokio::test(start_paused = true)]
    async fn removing_a_tunnel_stops_it() {
        let network = FakeNetwork::new();
        let mut manager = manager(&network, vec![tunnel("a", 1), tunnel("b", 2)]).await;
        manager.start("a").unwrap();
        settle().await;

        manager.apply_config(&config(vec![tunnel("b", 2)])).await;
        assert_eq!(network.running(), 0);
        let ids: Vec<_> = manager
            .statuses()
            .into_iter()
            .map(|status| status.id)
            .collect();
        assert_eq!(ids, ["b"]);
    }

    #[tokio::test(start_paused = true)]
    async fn an_unchanged_tunnel_keeps_running_across_reloads() {
        let network = FakeNetwork::new();
        let mut manager = manager(&network, vec![tunnel("a", 1)]).await;
        manager.start("a").unwrap();
        settle().await;

        manager
            .apply_config(&config(vec![tunnel("a", 1), tunnel("new", 2)]))
            .await;
        settle().await;
        assert_eq!(state_of(&manager, "a"), TunnelState::Up);
        assert_eq!(network.spawn_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_modified_running_tunnel_restarts_with_its_new_definition() {
        let network = FakeNetwork::new();
        let mut manager = manager(&network, vec![tunnel("a", 1)]).await;
        manager.start("a").unwrap();
        settle().await;

        manager.apply_config(&config(vec![tunnel("a", 9)])).await;
        settle().await;
        assert_eq!(state_of(&manager, "a"), TunnelState::Up);
        let spawned = network.spawned();
        assert_eq!(spawned.len(), 2);
        assert!(spawned[1].args.contains(&"9:127.0.0.1:5432".to_string()));
        assert_eq!(network.running(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_modified_stopped_tunnel_stays_stopped() {
        let network = FakeNetwork::new();
        let mut manager = manager(&network, vec![tunnel("a", 1)]).await;
        manager.apply_config(&config(vec![tunnel("a", 9)])).await;
        settle().await;
        assert_eq!(state_of(&manager, "a"), TunnelState::Stopped);
        assert_eq!(network.spawn_count(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn network_change_is_sent_to_every_tunnel() {
        let network = FakeNetwork::new();
        network.script(Behavior::exit("Connection closed by 1.2.3.4 port 22"));
        network.script(Behavior::exit("Connection closed by 1.2.3.4 port 22"));
        let manager = manager(&network, vec![tunnel("a", 1), tunnel("b", 2)]).await;
        manager.start("a").unwrap();
        manager.start("b").unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(network.spawn_count(), 2);

        manager.network_changed();
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(
            network.spawn_count(),
            4,
            "both retried before their backoff delay"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn subscribers_receive_every_tunnel_update() {
        let network = FakeNetwork::new();
        let manager = manager(&network, vec![tunnel("a", 1)]).await;
        let mut updates = manager.subscribe();
        manager.start("a").unwrap();
        settle().await;

        let mut states = vec![];
        while let Ok(status) = updates.try_recv() {
            states.push((status.id, status.state));
        }
        assert_eq!(
            states,
            vec![
                ("a".to_string(), TunnelState::Connecting { attempt: 0 }),
                ("a".to_string(), TunnelState::Up)
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_stops_every_ssh_process() {
        let network = FakeNetwork::new();
        let mut manager = manager(&network, vec![tunnel("a", 1), tunnel("b", 2)]).await;
        manager.start("a").unwrap();
        manager.start("b").unwrap();
        settle().await;
        assert_eq!(network.running(), 2);

        manager.shutdown().await;
        assert_eq!(network.running(), 0);
        assert!(manager.statuses().is_empty());
    }
}
