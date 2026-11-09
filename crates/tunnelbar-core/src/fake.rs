//! Scriptable stand-ins for `ssh` and the local network, for tests.
//!
//! Every fake `ssh` process follows the next scripted [`Behavior`]; when the script is empty
//! it serves its forwards right away. Its local ports become open on the shared [`FakeNetwork`],
//! which the [`FakeProber`] reads.

use std::collections::{HashSet, VecDeque};
use std::future::pending;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{Instant, sleep_until};

use crate::command::SshCommand;
use crate::process::{PortProber, ProcessSpawner, TunnelProcess};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Behavior {
    /// Opens its local ports after `after`, then runs until killed.
    Serve { after: Duration },
    /// Prints `stderr` then exits with `code`, `after` the spawn.
    Exit {
        after: Duration,
        stderr: Vec<String>,
        code: i32,
    },
    /// Serves its forwards, then loses the connection: prints `stderr` and exits with code 255
    /// `up_for` after the spawn.
    ServeThenExit {
        up_for: Duration,
        stderr: Vec<String>,
    },
    /// Prints `stderr` at once, then serves like [`Behavior::serve`]: `ssh` keeps running when
    /// some, but not all, addresses of a forward cannot be bound.
    WarnThenServe { stderr: Vec<String> },
    /// Runs until killed without ever opening its ports.
    Hang,
    /// Cannot be started at all.
    SpawnError,
}

impl Behavior {
    pub fn serve() -> Self {
        Behavior::Serve {
            after: Duration::from_millis(100),
        }
    }

    /// Exits with code 255 after printing `stderr`, `after` the spawn.
    pub fn exit_after(after: Duration, stderr: &str) -> Self {
        Behavior::Exit {
            after,
            stderr: stderr.lines().map(str::to_owned).collect(),
            code: 255,
        }
    }

    /// Serves, then exits with code 255 after printing `stderr`, `up_for` after the spawn.
    pub fn serve_then_exit(up_for: Duration, stderr: &str) -> Self {
        Behavior::ServeThenExit {
            up_for,
            stderr: stderr.lines().map(str::to_owned).collect(),
        }
    }

    pub fn warn_then_serve(stderr: &str) -> Self {
        Behavior::WarnThenServe {
            stderr: stderr.lines().map(str::to_owned).collect(),
        }
    }

    /// Exits at once with code 255 after printing `stderr`.
    pub fn exit(stderr: &str) -> Self {
        Self::exit_after(Duration::from_millis(100), stderr)
    }
}

#[derive(Debug, Default)]
struct Inner {
    script: VecDeque<Behavior>,
    spawned: Vec<SshCommand>,
    open_ports: HashSet<u16>,
    kills: usize,
    running: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FakeNetwork {
    inner: Arc<Mutex<Inner>>,
}

impl FakeNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Queues the behavior of the next spawned process.
    pub fn script(&self, behavior: Behavior) -> &Self {
        self.lock().script.push_back(behavior);
        self
    }

    pub fn spawner(&self) -> FakeSpawner {
        FakeSpawner {
            network: self.clone(),
        }
    }

    pub fn prober(&self) -> FakeProber {
        FakeProber {
            network: self.clone(),
        }
    }

    /// Commands of every process spawned so far.
    pub fn spawned(&self) -> Vec<SshCommand> {
        self.lock().spawned.clone()
    }

    pub fn spawn_count(&self) -> usize {
        self.lock().spawned.len()
    }

    pub fn kills(&self) -> usize {
        self.lock().kills
    }

    /// Processes neither exited nor killed.
    pub fn running(&self) -> usize {
        self.lock().running
    }

    /// Simulates another program listening on a port.
    pub fn occupy(&self, port: u16) {
        self.lock().open_ports.insert(port);
    }

    fn set_open(&self, ports: &[u16], open: bool) {
        let mut inner = self.lock();
        for port in ports {
            if open {
                inner.open_ports.insert(*port);
            } else {
                inner.open_ports.remove(port);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FakeSpawner {
    network: FakeNetwork,
}

impl ProcessSpawner for FakeSpawner {
    type Process = FakeProcess;

    fn spawn(
        &self,
        command: &SshCommand,
        output: UnboundedSender<String>,
    ) -> io::Result<FakeProcess> {
        let behavior = {
            let mut inner = self.network.lock();
            let behavior = inner.script.pop_front().unwrap_or_else(Behavior::serve);
            if behavior == Behavior::SpawnError {
                return Err(io::Error::new(io::ErrorKind::NotFound, "no such program"));
            }
            inner.spawned.push(command.clone());
            inner.running += 1;
            behavior
        };
        let behavior = match behavior {
            Behavior::WarnThenServe { stderr } => {
                for line in stderr {
                    let _ = output.send(line);
                }
                Behavior::serve()
            }
            other => other,
        };
        Ok(FakeProcess {
            network: self.network.clone(),
            ports: listened_ports(command),
            started: Instant::now(),
            behavior,
            output,
            finished: false,
        })
    }
}

/// Local ports `ssh` would listen on: the first field of `-L` and `-D` specifications.
fn listened_ports(command: &SshCommand) -> Vec<u16> {
    command
        .args
        .windows(2)
        .filter(|pair| pair[0] == "-L" || pair[0] == "-D")
        .filter_map(|pair| pair[1].split(':').next()?.parse().ok())
        .collect()
}

pub struct FakeProcess {
    network: FakeNetwork,
    ports: Vec<u16>,
    started: Instant,
    behavior: Behavior,
    output: UnboundedSender<String>,
    finished: bool,
}

impl FakeProcess {
    fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            self.network.set_open(&self.ports, false);
            self.network.lock().running -= 1;
        }
    }
}

impl TunnelProcess for FakeProcess {
    async fn wait(&mut self) -> Option<i32> {
        match self.behavior.clone() {
            Behavior::Serve { after } => {
                sleep_until(self.started + after).await;
                self.network.set_open(&self.ports, true);
                pending().await
            }
            Behavior::Exit {
                after,
                stderr,
                code,
            } => {
                sleep_until(self.started + after).await;
                if !self.finished {
                    for line in stderr {
                        let _ = self.output.send(line);
                    }
                    self.finish();
                }
                Some(code)
            }
            Behavior::ServeThenExit { up_for, stderr } => {
                let ready_at = self.started + Duration::from_millis(100);
                sleep_until(ready_at).await;
                if !self.finished {
                    self.network.set_open(&self.ports, true);
                }
                sleep_until(self.started + up_for.max(Duration::from_millis(100))).await;
                if !self.finished {
                    for line in stderr {
                        let _ = self.output.send(line);
                    }
                    self.finish();
                }
                Some(255)
            }
            Behavior::Hang | Behavior::SpawnError => pending().await,
            Behavior::WarnThenServe { .. } => unreachable!("replaced by Serve at spawn"),
        }
    }

    async fn kill(&mut self) {
        self.network.lock().kills += 1;
        self.finish();
    }
}

#[derive(Debug, Clone)]
pub struct FakeProber {
    network: FakeNetwork,
}

impl PortProber for FakeProber {
    async fn is_open(&self, port: u16) -> bool {
        self.network.lock().open_ports.contains(&port)
    }
}
