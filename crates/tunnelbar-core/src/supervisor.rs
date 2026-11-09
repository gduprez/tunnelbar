//! One Tokio task per tunnel: runs `ssh`, watches it and drives the state machine.

use std::collections::VecDeque;
use std::future::pending;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

use crate::command::SshCommand;
use crate::diagnosis::{diagnose, local_bind_failure};
use crate::error::TunnelError;
use crate::model::{Forward, Tunnel};
use crate::process::{PortProber, ProcessSpawner, TunnelProcess};
use crate::state::{Action, Event, Policy, TunnelState, transition};

/// Upper bound of the stderr text kept for diagnosis.
const MAX_STDERR_BYTES: usize = 64 * 1024;

/// How often local ports are probed while connecting.
pub const PROBE_INTERVAL: Duration = Duration::from_millis(500);
/// A connection whose forwards are not ready by then is restarted.
pub const CONNECT_DEADLINE: Duration = Duration::from_secs(30);
/// Without local port to probe (remote forwards only), a process alive this long is up.
pub const READY_GRACE: Duration = Duration::from_secs(5);
/// Number of `ssh` output lines kept for display.
pub const LOG_LINES: usize = 50;

/// What the UI shows about a tunnel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TunnelStatus {
    pub id: String,
    pub state: TunnelState,
    /// When the tunnel last came up, while it is up.
    #[serde(rename = "up_since_ms", serialize_with = "unix_milliseconds")]
    pub up_since: Option<SystemTime>,
    /// Times the tunnel came back up after a loss, since the user started it.
    pub reconnections: u32,
    /// Last lines printed by `ssh`, oldest first.
    pub logs: Vec<String>,
}

fn unix_milliseconds<S: serde::Serializer>(
    time: &Option<SystemTime>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let millis = time
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as u64);
    serde::Serialize::serialize(&millis, serializer)
}

impl TunnelStatus {
    fn stopped(id: &str) -> Self {
        TunnelStatus {
            id: id.to_owned(),
            state: TunnelState::Stopped,
            up_since: None,
            reconnections: 0,
            logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    NetworkChanged,
}

pub struct SupervisorHandle {
    commands: mpsc::UnboundedSender<Command>,
    status: watch::Receiver<TunnelStatus>,
    task: JoinHandle<()>,
}

impl SupervisorHandle {
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn status(&self) -> TunnelStatus {
        self.status.borrow().clone()
    }

    pub fn watch(&self) -> watch::Receiver<TunnelStatus> {
        self.status.clone()
    }

    /// Stops the tunnel if needed and waits for the task to end.
    pub async fn shutdown(self) {
        drop(self.commands);
        let _ = self.task.await;
    }
}

/// Starts the supervisor of `tunnel`, initially stopped. Every status change is also sent to `updates`.
pub fn spawn_supervisor<S: ProcessSpawner, P: PortProber>(
    tunnel: Tunnel,
    spawner: Arc<S>,
    prober: Arc<P>,
    updates: broadcast::Sender<TunnelStatus>,
) -> SupervisorHandle {
    let (commands_tx, commands) = mpsc::unbounded_channel();
    let (status_tx, status) = watch::channel(TunnelStatus::stopped(&tunnel.id));
    let supervisor = Supervisor {
        command: SshCommand::for_tunnel(&tunnel),
        local_ports: local_ports(&tunnel),
        policy: Policy {
            auto_reconnect: tunnel.auto_reconnect,
        },
        id: tunnel.id,
        spawner,
        prober,
        commands,
        status: status_tx,
        updates,
        state: TunnelState::Stopped,
        process: None,
        output: None,
        stderr: String::new(),
        logs: VecDeque::with_capacity(LOG_LINES),
        retry_at: None,
        next_probe: None,
        connecting_since: Instant::now(),
        up_since: None,
        reconnections: 0,
        has_been_up: false,
    };
    SupervisorHandle {
        commands: commands_tx,
        status,
        task: tokio::spawn(supervisor.run()),
    }
}

/// Ports `ssh` listens on locally, which tell when the tunnel is usable.
fn local_ports(tunnel: &Tunnel) -> Vec<u16> {
    tunnel
        .forwards
        .iter()
        .filter_map(|forward| match forward {
            Forward::Local { local_port, .. } | Forward::Dynamic { local_port } => {
                Some(*local_port)
            }
            Forward::Remote { .. } => None,
        })
        .collect()
}

struct Supervisor<S: ProcessSpawner, P: PortProber> {
    id: String,
    command: SshCommand,
    local_ports: Vec<u16>,
    policy: Policy,
    spawner: Arc<S>,
    prober: Arc<P>,
    commands: mpsc::UnboundedReceiver<Command>,
    status: watch::Sender<TunnelStatus>,
    updates: broadcast::Sender<TunnelStatus>,
    state: TunnelState,
    process: Option<S::Process>,
    /// Output of the current process.
    output: Option<mpsc::UnboundedReceiver<String>>,
    /// Output of the current process, for diagnosis when it exits.
    stderr: String,
    logs: VecDeque<String>,
    retry_at: Option<Instant>,
    next_probe: Option<Instant>,
    connecting_since: Instant,
    up_since: Option<SystemTime>,
    reconnections: u32,
    has_been_up: bool,
}

enum Wake {
    Command(Command),
    Shutdown,
    Exited(Option<i32>),
    Line(String),
    RetryDue,
    Probe,
}

impl<S: ProcessSpawner, P: PortProber> Supervisor<S, P> {
    async fn run(mut self) {
        loop {
            let wake = tokio::select! {
                command = self.commands.recv() => command.map_or(Wake::Shutdown, Wake::Command),
                code = wait(&mut self.process) => Wake::Exited(code),
                Some(line) = next_line(&mut self.output) => Wake::Line(line),
                () = sleep_until_some(self.retry_at) => Wake::RetryDue,
                () = sleep_until_some(self.next_probe) => Wake::Probe,
            };
            match wake {
                Wake::Command(Command::Start) => self.apply(Event::Start).await,
                Wake::Command(Command::Stop) => self.apply(Event::Stop).await,
                Wake::Command(Command::NetworkChanged) => self.apply(Event::NetworkChanged).await,
                Wake::Shutdown => {
                    self.kill().await;
                    return;
                }
                Wake::Exited(code) => {
                    self.process = None;
                    self.drain_output();
                    let error = diagnose(&self.stderr, code);
                    self.apply(Event::Exited { error }).await;
                }
                Wake::Line(line) => {
                    let failure = local_bind_failure(&line);
                    self.record(line);
                    self.publish();
                    if let Some(error) = failure {
                        self.kill().await;
                        self.apply(Event::Exited { error }).await;
                    }
                }
                Wake::RetryDue => {
                    self.retry_at = None;
                    self.apply(Event::RetryDue).await;
                }
                Wake::Probe => self.probe().await,
            }
        }
    }

    async fn apply(&mut self, event: Event) {
        let mut next_event = Some(event);
        while let Some(event) = next_event.take() {
            let (state, actions) = transition(&self.state, &event, self.policy);
            for action in actions {
                match action {
                    Action::Spawn => {
                        let taken = first_open(self.prober.clone(), self.local_ports.clone());
                        if let Some(port) = taken.await {
                            self.record(format!(
                                "local port {port} is already used by another program"
                            ));
                            next_event = Some(Event::Exited {
                                error: TunnelError::PortInUse { port: Some(port) },
                            });
                        } else if !self.spawn() {
                            next_event = Some(Event::Exited {
                                error: TunnelError::SpawnFailed,
                            });
                        }
                    }
                    Action::Kill => self.kill().await,
                    Action::ScheduleRetry(delay) => self.retry_at = Some(Instant::now() + delay),
                    Action::CancelRetry => self.retry_at = None,
                }
            }
            self.enter(state, &event);
        }
    }

    /// Bookkeeping when moving to `state`.
    fn enter(&mut self, state: TunnelState, event: &Event) {
        if *event == Event::Start && state != self.state {
            self.reconnections = 0;
            self.has_been_up = false;
        }
        if state == TunnelState::Up && self.state != TunnelState::Up {
            self.up_since = Some(SystemTime::now());
            if self.has_been_up {
                self.reconnections += 1;
            }
            self.has_been_up = true;
        } else if state != TunnelState::Up {
            self.up_since = None;
        }
        self.next_probe = matches!(state, TunnelState::Connecting { .. })
            .then(|| Instant::now() + PROBE_INTERVAL);
        if state != self.state {
            log::info!("tunnel {}: {:?}", self.id, state);
            self.state = state;
            self.publish();
        }
    }

    /// Starts `ssh`; returns false, after logging why, when it cannot be started.
    fn spawn(&mut self) -> bool {
        let (output, lines) = mpsc::unbounded_channel();
        match self.spawner.spawn(&self.command, output) {
            Ok(process) => {
                self.process = Some(process);
                self.output = Some(lines);
                self.stderr.clear();
                self.connecting_since = Instant::now();
                true
            }
            Err(error) => {
                self.record(format!(
                    "cannot start {}: {error}",
                    self.command.program.display()
                ));
                false
            }
        }
    }

    async fn kill(&mut self) {
        if let Some(mut process) = self.process.take() {
            process.kill().await;
        }
        self.output = None;
    }

    async fn probe(&mut self) {
        self.next_probe = Some(Instant::now() + PROBE_INTERVAL);
        if !matches!(self.state, TunnelState::Connecting { .. }) || self.process.is_none() {
            return;
        }
        let elapsed = self.connecting_since.elapsed();
        let ready = if self.local_ports.is_empty() {
            elapsed >= READY_GRACE
        } else {
            all_open(self.prober.clone(), self.local_ports.clone()).await
        };
        if ready {
            self.apply(Event::Ready).await;
        } else if elapsed >= CONNECT_DEADLINE {
            self.kill().await;
            self.record(format!(
                "no forward ready after {} s, restarting",
                CONNECT_DEADLINE.as_secs()
            ));
            self.apply(Event::Exited {
                error: TunnelError::ConnectionTimedOut,
            })
            .await;
        }
    }

    fn drain_output(&mut self) {
        let mut lines = Vec::new();
        if let Some(output) = self.output.as_mut() {
            while let Ok(line) = output.try_recv() {
                lines.push(line);
            }
        }
        self.output = None;
        for line in lines {
            self.record(line);
        }
    }

    fn record(&mut self, line: String) {
        if self.stderr.len() + line.len() < MAX_STDERR_BYTES {
            self.stderr.push_str(&line);
            self.stderr.push('\n');
        }
        if self.logs.len() == LOG_LINES {
            self.logs.pop_front();
        }
        self.logs.push_back(line);
    }

    fn publish(&self) {
        let status = TunnelStatus {
            id: self.id.clone(),
            state: self.state.clone(),
            up_since: self.up_since,
            reconnections: self.reconnections,
            logs: self.logs.iter().cloned().collect(),
        };
        self.status.send_replace(status.clone());
        let _ = self.updates.send(status);
    }
}

/// Owns its inputs so that the supervisor is not borrowed across awaits.
async fn all_open<P: PortProber>(prober: Arc<P>, ports: Vec<u16>) -> bool {
    for port in ports {
        if !prober.is_open(port).await {
            return false;
        }
    }
    true
}

/// The first port that already accepts connections. Checked before `ssh` runs: `ssh` might
/// still start, on the other loopback address, and the tunnel would look up while its port
/// leads to another program.
async fn first_open<P: PortProber>(prober: Arc<P>, ports: Vec<u16>) -> Option<u16> {
    for port in ports {
        if prober.is_open(port).await {
            return Some(port);
        }
    }
    None
}

async fn wait<T: TunnelProcess>(process: &mut Option<T>) -> Option<i32> {
    match process {
        Some(process) => process.wait().await,
        None => pending().await,
    }
}

async fn next_line(output: &mut Option<mpsc::UnboundedReceiver<String>>) -> Option<String> {
    match output {
        Some(output) => output.recv().await,
        None => pending().await,
    }
}

async fn sleep_until_some(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TunnelError;
    use crate::fake::{Behavior, FakeNetwork};
    use crate::model::Forward;
    use crate::state::retry_delay;
    use tokio::time::{advance, sleep};

    fn tunnel() -> Tunnel {
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

    fn supervise(network: &FakeNetwork, tunnel: Tunnel) -> SupervisorHandle {
        let (updates, _) = broadcast::channel(64);
        spawn_supervisor(
            tunnel,
            Arc::new(network.spawner()),
            Arc::new(network.prober()),
            updates,
        )
    }

    /// Waits (in paused time) until the status matches, or fails after an hour.
    async fn until(
        handle: &SupervisorHandle,
        predicate: impl Fn(&TunnelStatus) -> bool,
    ) -> TunnelStatus {
        let mut watch = handle.watch();
        let result = tokio::time::timeout(Duration::from_secs(3600), async {
            loop {
                let status = watch.borrow_and_update().clone();
                if predicate(&status) {
                    return status;
                }
                watch.changed().await.expect("supervisor alive");
            }
        })
        .await;
        result.unwrap_or_else(|_| panic!("condition never met, last status: {:?}", handle.status()))
    }

    fn is(state: TunnelState) -> impl Fn(&TunnelStatus) -> bool {
        move |status| status.state == state
    }

    fn reconnecting(status: &TunnelStatus) -> bool {
        matches!(status.state, TunnelState::Reconnecting { .. })
    }

    const LOST: &str = "Timeout, server 203.0.113.10 not responding.";

    #[tokio::test(start_paused = true)]
    async fn starts_stopped() {
        let handle = supervise(&FakeNetwork::new(), tunnel());
        assert_eq!(handle.status(), TunnelStatus::stopped("prod"));
    }

    #[tokio::test(start_paused = true)]
    async fn start_runs_ssh_and_comes_up_once_the_port_is_open() {
        let network = FakeNetwork::new();
        network.script(Behavior::Serve {
            after: Duration::from_secs(2),
        });
        let handle = supervise(&network, tunnel());

        handle.send(Command::Start);
        until(&handle, is(TunnelState::Connecting { attempt: 0 })).await;
        assert_eq!(network.spawned(), vec![SshCommand::for_tunnel(&tunnel())]);

        let status = until(&handle, is(TunnelState::Up)).await;
        assert!(status.up_since.is_some());
        assert_eq!(status.reconnections, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn stop_kills_ssh() {
        let network = FakeNetwork::new();
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, is(TunnelState::Up)).await;

        handle.send(Command::Stop);
        let status = until(&handle, is(TunnelState::Stopped)).await;
        assert_eq!(status.up_since, None);
        assert_eq!((network.kills(), network.running()), (1, 0));
    }

    #[tokio::test(start_paused = true)]
    async fn reconnects_after_a_loss_and_counts_it() {
        let network = FakeNetwork::new();
        network.script(Behavior::serve_then_exit(Duration::from_secs(10), LOST));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, is(TunnelState::Up)).await;

        let status = until(&handle, reconnecting).await;
        assert_eq!(
            status.state,
            TunnelState::Reconnecting {
                attempt: 1,
                delay: retry_delay(1),
                last_error: TunnelError::ConnectionLost
            }
        );
        assert_eq!(status.logs, vec![LOST.to_owned()]);

        let status = until(&handle, is(TunnelState::Up)).await;
        assert_eq!(status.reconnections, 1);
        assert_eq!(network.spawn_count(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn waits_the_backoff_delay_before_retrying() {
        let network = FakeNetwork::new();
        for _ in 0..3 {
            network.script(Behavior::exit(
                "ssh: connect to host x port 22: Connection refused",
            ));
        }
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        until(&handle, |s| {
            matches!(s.state, TunnelState::Reconnecting { attempt: 3, .. })
        })
        .await;
        assert_eq!(network.spawn_count(), 3);
        advance(retry_delay(3) - Duration::from_millis(10)).await;
        assert_eq!(network.spawn_count(), 3, "retried too early");
        until(&handle, is(TunnelState::Up)).await;
        assert_eq!(network.spawn_count(), 4);
    }

    #[tokio::test(start_paused = true)]
    async fn fatal_errors_are_not_retried() {
        let network = FakeNetwork::new();
        network.script(Behavior::exit("user@host: Permission denied (publickey)."));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        until(
            &handle,
            is(TunnelState::Failed {
                error: TunnelError::AuthFailed,
            }),
        )
        .await;
        sleep(Duration::from_secs(600)).await;
        assert_eq!(network.spawn_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_local_port_taken_by_another_program_fails_without_running_ssh() {
        let network = FakeNetwork::new();
        network.occupy(54321);
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        let status = until(
            &handle,
            is(TunnelState::Failed {
                error: TunnelError::PortInUse { port: Some(54321) },
            }),
        )
        .await;
        assert_eq!(network.spawn_count(), 0);
        assert!(status.logs.iter().any(|line| line.contains("54321")));
    }

    /// `ssh` keeps running when it binds `[::1]` but not `127.0.0.1` (or the reverse): the
    /// forward would then reach another program, so the tunnel must fail, not come up.
    #[tokio::test(start_paused = true)]
    async fn a_forward_bound_on_some_addresses_only_fails() {
        let network = FakeNetwork::new();
        network.script(Behavior::warn_then_serve(
            "bind [127.0.0.1]:54321: Address already in use",
        ));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        until(
            &handle,
            is(TunnelState::Failed {
                error: TunnelError::PortInUse { port: Some(54321) },
            }),
        )
        .await;
        assert_eq!((network.spawn_count(), network.running()), (1, 0));
        sleep(Duration::from_secs(600)).await;
        assert_eq!(network.spawn_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_harmless_warning_does_not_fail_the_tunnel() {
        let network = FakeNetwork::new();
        network.script(Behavior::warn_then_serve(
            "Warning: Permanently added 'localhost' (ED25519) to the list of known hosts.",
        ));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        let status = until(&handle, is(TunnelState::Up)).await;
        assert_eq!(status.logs.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn restarts_a_connection_whose_ports_never_open() {
        let network = FakeNetwork::new();
        network.script(Behavior::Hang);
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        let status = until(&handle, reconnecting).await;
        assert!(matches!(
            status.state,
            TunnelState::Reconnecting {
                last_error: TunnelError::ConnectionTimedOut,
                ..
            }
        ));
        assert_eq!(network.kills(), 1);
        until(&handle, is(TunnelState::Up)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_tunnel_without_local_port_is_up_after_a_grace_period() {
        let network = FakeNetwork::new();
        let mut remote_only = tunnel();
        remote_only.forwards = vec![Forward::Remote {
            remote_port: 8080,
            local_host: "127.0.0.1".into(),
            local_port: 3000,
        }];
        let handle = supervise(&network, remote_only);
        handle.send(Command::Start);

        until(&handle, is(TunnelState::Connecting { attempt: 0 })).await;
        advance(READY_GRACE - Duration::from_secs(1)).await;
        assert_eq!(
            handle.status().state,
            TunnelState::Connecting { attempt: 0 }
        );
        until(&handle, is(TunnelState::Up)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_network_change_cuts_the_backoff_short() {
        let network = FakeNetwork::new();
        for _ in 0..6 {
            network.script(Behavior::exit(LOST));
        }
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, |s| {
            matches!(s.state, TunnelState::Reconnecting { attempt: 6, .. })
        })
        .await;

        handle.send(Command::NetworkChanged);
        until(&handle, is(TunnelState::Up)).await;
        assert_eq!(network.spawn_count(), 7);
    }

    #[tokio::test(start_paused = true)]
    async fn stop_during_backoff_cancels_the_retry() {
        let network = FakeNetwork::new();
        network.script(Behavior::exit(LOST));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, reconnecting).await;

        handle.send(Command::Stop);
        until(&handle, is(TunnelState::Stopped)).await;
        sleep(Duration::from_secs(600)).await;
        assert_eq!(network.spawn_count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_program_that_cannot_start_fails() {
        let network = FakeNetwork::new();
        network.script(Behavior::SpawnError);
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);

        let status = until(
            &handle,
            is(TunnelState::Failed {
                error: TunnelError::SpawnFailed,
            }),
        )
        .await;
        assert_eq!(
            status.logs.len(),
            1,
            "the reason is logged: {:?}",
            status.logs
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_only_the_last_log_lines() {
        let network = FakeNetwork::new();
        let noise: Vec<String> = (0..LOG_LINES + 10).map(|n| format!("line {n}")).collect();
        network.script(Behavior::exit(&noise.join("\n")));
        let mut no_reconnect = tunnel();
        no_reconnect.auto_reconnect = false;
        let handle = supervise(&network, no_reconnect);
        handle.send(Command::Start);

        let status = until(&handle, |s| matches!(s.state, TunnelState::Failed { .. })).await;
        assert_eq!(status.logs, noise[10..]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_start_resets_the_reconnection_count() {
        let network = FakeNetwork::new();
        network.script(Behavior::serve_then_exit(Duration::from_secs(5), LOST));
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, |s| {
            s.reconnections == 1 && s.state == TunnelState::Up
        })
        .await;

        handle.send(Command::Stop);
        until(&handle, is(TunnelState::Stopped)).await;
        handle.send(Command::Start);
        let status = until(&handle, is(TunnelState::Up)).await;
        assert_eq!(status.reconnections, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_kills_ssh() {
        let network = FakeNetwork::new();
        let handle = supervise(&network, tunnel());
        handle.send(Command::Start);
        until(&handle, is(TunnelState::Up)).await;

        handle.shutdown().await;
        assert_eq!(network.running(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn publishes_every_change_to_the_shared_channel() {
        let network = FakeNetwork::new();
        let (updates, mut received) = broadcast::channel(64);
        let handle = spawn_supervisor(
            tunnel(),
            Arc::new(network.spawner()),
            Arc::new(network.prober()),
            updates,
        );
        handle.send(Command::Start);
        until(&handle, is(TunnelState::Up)).await;

        let mut states = vec![];
        while let Ok(status) = received.try_recv() {
            states.push(status.state);
        }
        assert_eq!(
            states,
            vec![TunnelState::Connecting { attempt: 0 }, TunnelState::Up]
        );
    }
}
