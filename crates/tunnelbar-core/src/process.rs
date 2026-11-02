//! Side effects of supervision, behind traits so that the supervisor can be tested with fakes.

use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::command::SshCommand;
use crate::orphans::{PidRegistry, ProcessInspector, SystemInspector};

pub trait ProcessSpawner: Send + Sync + 'static {
    type Process: TunnelProcess;

    /// Starts the command; every line it prints on stderr is sent to `output`.
    fn spawn(
        &self,
        command: &SshCommand,
        output: UnboundedSender<String>,
    ) -> io::Result<Self::Process>;
}

pub trait TunnelProcess: Send + 'static {
    /// Waits for the process to exit and for all its output to be sent; returns the exit code.
    /// Must be cancel-safe: it is raced against other events and called again later.
    fn wait(&mut self) -> impl Future<Output = Option<i32>> + Send;

    /// Kills the process and reaps it.
    fn kill(&mut self) -> impl Future<Output = ()> + Send;
}

pub trait PortProber: Send + Sync + 'static {
    /// Whether a local port accepts TCP connections.
    fn is_open(&self, port: u16) -> impl Future<Output = bool> + Send;
}

/// Runs real processes.
#[derive(Debug, Clone, Default)]
pub struct SystemSpawner {
    program: Option<PathBuf>,
    registry: Option<Arc<PidRegistry>>,
}

impl SystemSpawner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs `program` instead of the command's own, e.g. a fake `ssh` in tests.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        SystemSpawner {
            program: Some(program.into()),
            registry: None,
        }
    }

    /// Records the PIDs of running processes, so that a later run can clean up orphans.
    pub fn with_registry(mut self, registry: Arc<PidRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }
}

impl ProcessSpawner for SystemSpawner {
    type Process = SystemProcess;

    fn spawn(
        &self,
        command: &SshCommand,
        output: UnboundedSender<String>,
    ) -> io::Result<SystemProcess> {
        let program = self.program.as_ref().unwrap_or(&command.program);
        let mut child = Command::new(program)
            .args(&command.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stderr = child.stderr.take().expect("stderr is piped");
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = output.send(line);
            }
        });
        let pid = child.id();
        if let (Some(registry), Some(pid)) = (&self.registry, pid) {
            // The start time lets a later run tell this process from a recycled PID.
            let started = SystemInspector.describe(pid).map(|info| info.started);
            registry.record(pid, started);
        }
        Ok(SystemProcess {
            child,
            reader: Some(reader),
            registration: pid.zip(self.registry.clone()),
        })
    }
}

pub struct SystemProcess {
    child: Child,
    reader: Option<JoinHandle<()>>,
    registration: Option<(u32, Arc<PidRegistry>)>,
}

impl SystemProcess {
    /// Operating system identifier of the process, while it runs.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    fn unregister(&mut self) {
        if let Some((pid, registry)) = self.registration.take() {
            registry.forget(pid);
        }
    }
}

impl TunnelProcess for SystemProcess {
    async fn wait(&mut self) -> Option<i32> {
        // Both awaits are cancel-safe: the child caches its status, the reader keeps running.
        let status = self.child.wait().await.ok();
        self.unregister();
        if let Some(reader) = self.reader.as_mut() {
            let _ = reader.await;
            self.reader = None;
        }
        status.and_then(|status| status.code())
    }

    async fn kill(&mut self) {
        let _ = self.child.kill().await;
        self.unregister();
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
    }
}

/// Probes `127.0.0.1`, where `ssh` listens for local and dynamic forwards.
#[derive(Debug, Clone, Copy, Default)]
pub struct TcpPortProber;

impl PortProber for TcpPortProber {
    async fn is_open(&self, port: u16) -> bool {
        let connect = TcpStream::connect(("127.0.0.1", port));
        matches!(
            tokio::time::timeout(Duration::from_secs(1), connect).await,
            Ok(Ok(_))
        )
    }
}
