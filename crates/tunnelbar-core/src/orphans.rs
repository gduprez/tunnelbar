//! Cleans up `ssh` processes left behind by a TunnelBar that crashed or was killed.
//!
//! The PIDs of running tunnels are recorded in a file. At the next start, a recorded process is
//! killed only if it was orphaned (its parent is now launchd) and still looks like a TunnelBar
//! `ssh`, so that a recycled PID never hits an unrelated process.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;

/// Start of the command line of every `ssh` run by TunnelBar.
pub const SSH_SIGNATURE: &str = "/usr/bin/ssh -N -o ExitOnForwardFailure=yes";

/// Persistent set of the PIDs of running tunnel processes.
#[derive(Debug)]
pub struct PidRegistry {
    path: PathBuf,
    pids: Mutex<BTreeSet<u32>>,
}

impl PidRegistry {
    /// Starts empty; [`PidRegistry::previous`] reads what an earlier run left.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        PidRegistry {
            path: path.into(),
            pids: Mutex::new(BTreeSet::new()),
        }
    }

    pub fn record(&self, pid: u32) {
        let mut pids = self.lock();
        pids.insert(pid);
        self.write(&pids);
    }

    pub fn forget(&self, pid: u32) {
        let mut pids = self.lock();
        if pids.remove(&pid) {
            self.write(&pids);
        }
    }

    /// PIDs currently in the file.
    pub fn previous(&self) -> Vec<u32> {
        std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect::<BTreeSet<u32>>()
            .into_iter()
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeSet<u32>> {
        self.pids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Best effort: losing the file only means orphans may survive a crash.
    fn write(&self, pids: &BTreeSet<u32>) {
        let text: String = pids.iter().map(|pid| format!("{pid}\n")).collect();
        if let Err(error) = crate::store::write_private(&self.path, &text) {
            log::warn!("cannot write {}: {error}", self.path.display());
        }
    }
}

pub trait ProcessInspector {
    /// Parent PID and command line of a running process, if it exists.
    fn describe(&self, pid: u32) -> Option<(u32, String)>;
    fn terminate(&self, pid: u32);
}

/// Inspects processes with `ps` and terminates them with `kill`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemInspector;

impl ProcessInspector for SystemInspector {
    fn describe(&self, pid: u32) -> Option<(u32, String)> {
        let output = std::process::Command::new("/bin/ps")
            .args(["-o", "ppid=,command=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let (ppid, command) = text.trim().split_once(char::is_whitespace)?;
        Some((ppid.parse().ok()?, command.trim().to_owned()))
    }

    fn terminate(&self, pid: u32) {
        let _ = std::process::Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .status();
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
        .filter(|pid| {
            inspector
                .describe(*pid)
                .is_some_and(|(ppid, command)| ppid == 1 && command.starts_with(signature))
        })
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

    fn registry() -> (tempfile::TempDir, PidRegistry) {
        let dir = tempfile::tempdir().unwrap();
        let registry = PidRegistry::new(dir.path().join("TunnelBar").join("ssh.pids"));
        (dir, registry)
    }

    #[test]
    fn records_and_forgets_pids_in_a_private_file() {
        let (dir, registry) = registry();
        assert_eq!(registry.previous(), Vec::<u32>::new());
        registry.record(42);
        registry.record(7);
        assert_eq!(registry.previous(), vec![7, 42]);
        registry.forget(42);
        assert_eq!(registry.previous(), vec![7]);

        let path = dir.path().join("TunnelBar").join("ssh.pids");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // Another run reads the same file.
        assert_eq!(PidRegistry::new(&path).previous(), vec![7]);
    }

    #[test]
    fn the_file_is_replaced_atomically() {
        let (dir, registry) = registry();
        registry.record(1);
        let path = dir.path().join("TunnelBar").join("ssh.pids");
        let witness = dir.path().join("witness");
        std::fs::hard_link(&path, &witness).unwrap();
        registry.record(2);
        assert_eq!(PidRegistry::new(&witness).previous(), vec![1]);
    }

    #[test]
    fn ignores_garbage_in_the_file() {
        let (dir, registry) = registry();
        let path = dir.path().join("TunnelBar").join("ssh.pids");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "12\nnot a pid\n\n34\n").unwrap();
        assert_eq!(registry.previous(), vec![12, 34]);
    }

    #[derive(Default)]
    struct FakeInspector {
        processes: HashMap<u32, (u32, String)>,
        terminated: RefCell<Vec<u32>>,
    }

    impl ProcessInspector for FakeInspector {
        fn describe(&self, pid: u32) -> Option<(u32, String)> {
            self.processes.get(&pid).cloned()
        }

        fn terminate(&self, pid: u32) {
            self.terminated.borrow_mut().push(pid);
        }
    }

    #[test]
    fn kills_only_orphaned_tunnel_processes() {
        let (dir, registry) = registry();
        for pid in [10, 20, 30, 40] {
            registry.record(pid);
        }
        let ssh = format!("{SSH_SIGNATURE} -o ServerAliveInterval=15 -- host");
        let mut inspector = FakeInspector::default();
        inspector.processes.insert(10, (1, ssh.clone())); // orphan tunnel
        inspector.processes.insert(20, (555, ssh)); // still has a parent
        inspector
            .processes
            .insert(30, (1, "/usr/bin/vim notes".into())); // recycled PID
        // 40 is gone.

        let fresh = PidRegistry::new(dir.path().join("TunnelBar").join("ssh.pids"));
        assert_eq!(clean_orphans(&fresh, &inspector, SSH_SIGNATURE), vec![10]);
        assert_eq!(*inspector.terminated.borrow(), vec![10]);
        assert_eq!(fresh.previous(), Vec::<u32>::new());
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
        let (ppid, command) = SystemInspector.describe(pid).expect("sleep runs");
        assert_eq!((ppid, command.as_str()), (1, "sleep 3600"));

        SystemInspector.terminate(pid);
        for _ in 0..50 {
            if SystemInspector.describe(pid).is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("process {pid} still alive");
    }
}
