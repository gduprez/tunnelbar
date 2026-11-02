//! Cleans up `ssh` processes left behind by a TunnelBar that crashed or was killed.
//!
//! The PIDs of running tunnels are recorded in a file with the start time of each process.
//! At the next start, a recorded process is killed only if it was orphaned (its parent is now
//! launchd), still looks like a TunnelBar `ssh`, and started at the recorded time: a PID that
//! macOS recycled for another process is never hit.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Start of the command line of every `ssh` run by TunnelBar.
pub const SSH_SIGNATURE: &str = "/usr/bin/ssh -N -o ExitOnForwardFailure=yes";

/// What `ps` tells about a running process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub ppid: u32,
    /// Start time as printed by `ps -o lstart=`, e.g. `Mon Nov  2 11:16:03 2026`.
    pub started: String,
    pub command: String,
}

/// Persistent PIDs of running tunnel processes, with their start time when known.
#[derive(Debug)]
pub struct PidRegistry {
    path: PathBuf,
    entries: Mutex<BTreeMap<u32, Option<String>>>,
}

impl PidRegistry {
    /// Starts empty; [`PidRegistry::previous`] reads what an earlier run left.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        PidRegistry {
            path: path.into(),
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn record(&self, pid: u32, started: Option<String>) {
        let mut entries = self.lock();
        entries.insert(pid, started);
        self.write(&entries);
    }

    pub fn forget(&self, pid: u32) {
        let mut entries = self.lock();
        if entries.remove(&pid).is_some() {
            self.write(&entries);
        }
    }

    /// Entries currently in the file, by PID. Lines are `pid` or `pid<TAB>start time`.
    pub fn previous(&self) -> Vec<(u32, Option<String>)> {
        std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (pid, started) = match line.split_once('\t') {
                    Some((pid, started)) => (pid, Some(started.to_owned())),
                    None => (line, None),
                };
                Some((pid.trim().parse().ok()?, started))
            })
            .collect::<BTreeMap<u32, Option<String>>>()
            .into_iter()
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<u32, Option<String>>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Best effort: losing the file only means orphans may survive a crash.
    fn write(&self, entries: &BTreeMap<u32, Option<String>>) {
        let text: String = entries
            .iter()
            .map(|(pid, started)| match started {
                Some(started) => format!("{pid}\t{started}\n"),
                None => format!("{pid}\n"),
            })
            .collect();
        if let Err(error) = crate::store::write_private(&self.path, &text) {
            log::warn!("cannot write {}: {error}", self.path.display());
        }
    }
}

pub trait ProcessInspector {
    /// What is known of a running process, if it exists.
    fn describe(&self, pid: u32) -> Option<ProcessInfo>;
    fn terminate(&self, pid: u32);
}

/// Inspects processes with `ps` and signals them with `kill(2)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemInspector;

impl ProcessInspector for SystemInspector {
    fn describe(&self, pid: u32) -> Option<ProcessInfo> {
        let output = std::process::Command::new("/bin/ps")
            .args(["-o", "ppid=,lstart=,command=", "-p", &pid.to_string()])
            // A fixed, English start time format of 24 characters.
            .env("LC_ALL", "C")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let (ppid, rest) = text.trim_start().split_once(char::is_whitespace)?;
        let rest = rest.trim_start();
        let started = rest.get(..24)?;
        Some(ProcessInfo {
            ppid: ppid.parse().ok()?,
            started: started.to_owned(),
            command: rest.get(24..)?.trim().to_owned(),
        })
    }

    fn terminate(&self, pid: u32) {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        // 0 and 1 would signal a process group or launchd, never a tunnel.
        match i32::try_from(pid) {
            Ok(pid) if pid > 1 => {
                let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
            }
            _ => {}
        }
    }
}

/// Kills the orphans an earlier run left, then empties the registry file. Returns their PIDs.
pub fn clean_orphans(
    registry: &PidRegistry,
    inspector: &impl ProcessInspector,
    signature: &str,
) -> Vec<u32> {
    let killed: Vec<u32> = registry
        .previous()
        .into_iter()
        .filter(|(pid, recorded)| {
            inspector.describe(*pid).is_some_and(|info| {
                info.ppid == 1
                    && info.command.starts_with(signature)
                    && recorded
                        .as_ref()
                        .is_none_or(|started| *started == info.started)
            })
        })
        .map(|(pid, _)| pid)
        .collect();
    for pid in &killed {
        inspector.terminate(*pid);
    }
    registry.write(&registry.lock());
    killed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::os::unix::fs::PermissionsExt;

    const STARTED: &str = "Mon Nov  2 11:16:03 2026";

    fn registry() -> (tempfile::TempDir, PidRegistry) {
        let dir = tempfile::tempdir().unwrap();
        let registry = PidRegistry::new(dir.path().join("TunnelBar").join("ssh.pids"));
        (dir, registry)
    }

    fn started(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    #[test]
    fn records_and_forgets_pids_in_a_private_file() {
        let (dir, registry) = registry();
        assert_eq!(registry.previous(), vec![]);
        registry.record(42, started(STARTED));
        registry.record(7, None);
        assert_eq!(registry.previous(), vec![(7, None), (42, started(STARTED))]);
        registry.forget(42);
        assert_eq!(registry.previous(), vec![(7, None)]);

        let path = dir.path().join("TunnelBar").join("ssh.pids");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // Another run reads the same file.
        assert_eq!(PidRegistry::new(&path).previous(), vec![(7, None)]);
    }

    #[test]
    fn the_file_is_replaced_atomically() {
        let (dir, registry) = registry();
        registry.record(1, None);
        let path = dir.path().join("TunnelBar").join("ssh.pids");
        let witness = dir.path().join("witness");
        std::fs::hard_link(&path, &witness).unwrap();
        registry.record(2, None);
        assert_eq!(PidRegistry::new(&witness).previous(), vec![(1, None)]);
    }

    #[test]
    fn reads_older_files_and_ignores_garbage() {
        let (dir, registry) = registry();
        let path = dir.path().join("TunnelBar").join("ssh.pids");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("12\nnot a pid\n\n34\t{STARTED}\n")).unwrap();
        assert_eq!(
            registry.previous(),
            vec![(12, None), (34, started(STARTED))]
        );
    }

    #[derive(Default)]
    struct FakeInspector {
        processes: HashMap<u32, ProcessInfo>,
        terminated: RefCell<Vec<u32>>,
    }

    impl FakeInspector {
        fn add(&mut self, pid: u32, ppid: u32, started: &str, command: &str) {
            let info = ProcessInfo {
                ppid,
                started: started.into(),
                command: command.into(),
            };
            self.processes.insert(pid, info);
        }
    }

    impl ProcessInspector for FakeInspector {
        fn describe(&self, pid: u32) -> Option<ProcessInfo> {
            self.processes.get(&pid).cloned()
        }

        fn terminate(&self, pid: u32) {
            self.terminated.borrow_mut().push(pid);
        }
    }

    #[test]
    fn kills_only_orphaned_tunnel_processes_started_at_the_recorded_time() {
        let (dir, registry) = registry();
        for pid in [10, 20, 30, 40, 50, 60] {
            registry.record(pid, started(STARTED));
        }
        registry.record(70, None);
        let ssh = format!("{SSH_SIGNATURE} -o ServerAliveInterval=15 -- host");
        let mut inspector = FakeInspector::default();
        inspector.add(10, 1, STARTED, &ssh); // orphan tunnel
        inspector.add(20, 555, STARTED, &ssh); // still has a parent
        inspector.add(30, 1, STARTED, "/usr/bin/vim notes"); // another program
        inspector.add(40, 1, "Tue Nov  3 09:00:00 2026", &ssh); // recycled PID
        // 50 is gone.
        inspector.add(60, 1, STARTED, &ssh); // orphan tunnel
        inspector.add(70, 1, STARTED, &ssh); // recorded by an older version, no start time

        let fresh = PidRegistry::new(dir.path().join("TunnelBar").join("ssh.pids"));
        assert_eq!(
            clean_orphans(&fresh, &inspector, SSH_SIGNATURE),
            vec![10, 60, 70]
        );
        assert_eq!(*inspector.terminated.borrow(), vec![10, 60, 70]);
        assert_eq!(fresh.previous(), vec![]);
    }

    #[test]
    fn real_inspector_describes_and_terminates_processes() {
        // `sh` exits at once, leaving `sleep` orphaned.
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", "sleep 3600 >/dev/null 2>&1 & echo $!"])
            .output()
            .unwrap();
        let pid: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap();
        let info = SystemInspector.describe(pid).expect("sleep runs");
        assert_eq!((info.ppid, info.command.as_str()), (1, "sleep 3600"));
        assert_eq!(info.started.len(), STARTED.len(), "{:?}", info.started);
        assert_eq!(
            SystemInspector.describe(pid),
            Some(info),
            "stable start time"
        );

        SystemInspector.terminate(pid);
        for _ in 0..50 {
            if SystemInspector.describe(pid).is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("process {pid} still alive");
    }

    #[test]
    fn terminating_a_missing_process_is_harmless() {
        SystemInspector.terminate(u32::MAX);
        SystemInspector.terminate(0);
    }
}
