//! Linux only: the `ssh` processes of TunnelBar end with it, even when it crashes.
//!
//! Each child gets a parent-death signal (`PR_SET_PDEATHSIG`). Linux ties that signal to the
//! *thread* that spawned the child, not to the process: children are therefore spawned from a
//! thread that lives as long as TunnelBar ([`SpawnThread`]).

// The only unsafe code of the core: `pre_exec`, which runs between fork and exec.
#![allow(unsafe_code)]

use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::mpsc;

use nix::sys::prctl::set_pdeathsig;
use nix::sys::signal::Signal;
use nix::unistd::{getpid, getppid};

/// Makes the child receive SIGTERM when the thread that spawned it ends.
pub fn end_with_spawning_thread(command: &mut Command) {
    let parent = getpid();
    // SAFETY: the closure runs in the child between fork and exec, where only
    // async-signal-safe calls are allowed. It calls `prctl` and `getppid`, both
    // async-signal-safe, and builds errors from raw codes, which allocates nothing.
    unsafe {
        command.pre_exec(move || {
            set_pdeathsig(Signal::SIGTERM)
                .map_err(|errno| io::Error::from_raw_os_error(errno as i32))?;
            // If TunnelBar died before the signal was armed, nothing would ever send it.
            if getppid() != parent {
                return Err(io::Error::from_raw_os_error(nix::libc::ESRCH));
            }
            Ok(())
        });
    }
}

type Job = Box<dyn FnOnce() + Send>;

/// A thread living as long as its owner, from which tunnel processes are spawned.
#[derive(Debug)]
pub struct SpawnThread {
    jobs: mpsc::Sender<Job>,
}

impl SpawnThread {
    pub fn new() -> Self {
        let (jobs, received) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("tunnelbar-spawn".into())
            .spawn(move || {
                for job in received {
                    job();
                }
            })
            .expect("cannot start the spawn thread");
        SpawnThread { jobs }
    }

    /// Runs `job` on the spawn thread and returns its result.
    pub fn run<T: Send + 'static>(&self, job: impl FnOnce() -> T + Send + 'static) -> T {
        let (result, received) = mpsc::sync_channel(1);
        self.jobs
            .send(Box::new(move || {
                let _ = result.send(job());
            }))
            .expect("the spawn thread lives as long as its owner");
        received
            .recv()
            .expect("the spawn thread lives as long as its owner")
    }
}

impl Default for SpawnThread {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn sleeper() -> Command {
        let mut command = Command::new("sleep");
        command.arg("30");
        end_with_spawning_thread(&mut command);
        command
    }

    /// Whether the process is gone, or a zombie waiting to be reaped.
    fn ended(pid: u32) -> bool {
        let output = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&output.stdout);
        state.trim().is_empty() || state.trim_start().starts_with('Z')
    }

    fn ends_within(pid: u32, delay: Duration) -> bool {
        let deadline = Instant::now() + delay;
        while Instant::now() < deadline {
            if ended(pid) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        ended(pid)
    }

    fn stop(pid: u32) {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }

    #[test]
    fn a_child_ends_with_the_thread_that_spawned_it() {
        // The pitfall: spawned from a short-lived thread, a tunnel would die with it.
        let pid = std::thread::spawn(|| sleeper().spawn().unwrap().id())
            .join()
            .unwrap();
        let ended = ends_within(pid, Duration::from_secs(3));
        stop(pid);
        assert!(ended, "process {pid} should have received SIGTERM");
    }

    #[test]
    fn children_of_the_spawn_thread_outlive_the_calling_thread() {
        let spawner = Arc::new(SpawnThread::new());
        let caller = Arc::clone(&spawner);
        let pid = std::thread::spawn(move || caller.run(|| sleeper().spawn().unwrap().id()))
            .join()
            .unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let alive = !ended(pid);
        stop(pid);
        assert!(alive, "process {pid} must survive the calling thread");
    }

    #[test]
    fn children_end_when_the_spawn_thread_ends() {
        // Dropping the spawn thread stands for TunnelBar exiting or crashing.
        let spawner = SpawnThread::new();
        let pid = spawner.run(|| sleeper().spawn().unwrap().id());
        drop(spawner);
        let ended = ends_within(pid, Duration::from_secs(3));
        stop(pid);
        assert!(ended, "process {pid} should end with the spawn thread");
    }
}
