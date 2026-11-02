# TunnelBar — Design document

> 🇫🇷 [Version française](../fr/design.md)

TunnelBar is a macOS menu bar app to start, stop and monitor SSH tunnels.
This document records the design decisions taken before development started.
It is the reference for contributors: any change to these decisions should
update this file in the same pull request.

## 1. Goals

- Start / stop a configured SSH tunnel in one click from the menu bar.
- Show at a glance whether each tunnel is up, connecting, reconnecting or failed.
- Reconnect automatically after network loss or sleep.
- Stay small, safe and predictable: quitting the app closes every tunnel.

Typical tunnel (the original use case):

```sh
ssh -L 54321:127.0.0.1:5432 203.0.113.10 -N
```

### Non-goals (v1)

- Traffic graphs or byte counters.
- Importing hosts from `~/.ssh/config`.
- Password / passphrase prompts (`SSH_ASKPASS`).
- Free-form extra `ssh` options.

## 2. Identity

| Item | Value |
|---|---|
| Name | TunnelBar |
| Bundle ID | `io.github.gduprez.tunnelbar` |
| Repository | `github.com/gduprez/tunnelbar` (private until v0.1.0, then public) |
| Crates | `tunnelbar-core` (domain logic), `tunnelbar` (Tauri app) |
| License | `MIT OR Apache-2.0` |
| Minimum macOS | 13 Ventura, universal binary (Apple Silicon + Intel) |

## 3. Technology

- **Tauri v2** for the menu bar icon (tray) and the popover window.
- **Rust** for all domain logic, in the UI-independent `tunnelbar-core` crate.
- **Svelte 5 + TypeScript + Vite** for the UI, which stays "dumb": it renders
  state pushed by the core and sends user intents back.
- **Tokio** for concurrency (already used by Tauri).

## 4. SSH engine

TunnelBar runs the system `/usr/bin/ssh` as a child process. It therefore
honours the user's `~/.ssh/config`, keys, agent, macOS keychain and
`known_hosts`, and behaves exactly like the terminal.

`ssh` is always spawned **with an argument vector, never through a shell**.
Options always added:

| Option | Why |
|---|---|
| `-N` | No remote command. |
| `-o ExitOnForwardFailure=yes` | Exit immediately if a forward cannot be set up (e.g. local port busy). |
| `-o ServerAliveInterval=15` `-o ServerAliveCountMax=3` | Detect dead connections in under a minute. |
| `-o BatchMode=yes` | Never block on an interactive prompt. |
| `-o ConnectTimeout=15` | Fail fast when the server cannot be reached. |
| `-o ControlMaster=no` `-o ControlPath=none` | Own the connection: a multiplexed client would exit at once and hand the forwards to an unsupervised master. |
| `--` before the host | Whatever the host looks like, it cannot be read as an option. |

The panel can copy the equivalent command, quoted for a POSIX shell.

### Authentication

Key-based only (key without passphrase, or passphrase stored in the
ssh-agent / macOS keychain via `UseKeychain yes` and `AddKeysToAgent yes`).
An authentication failure puts the tunnel in an `AuthFailed` error state.

### Host keys

- Default: the user's own `StrictHostKeyChecking` policy is respected. An
  unknown host fails with `HostKeyUnknown`; the UI explains how to verify the
  fingerprint once in a terminal and offers to copy the command.
- Per tunnel, opt-in: `accept_new_host_key = true` adds
  `-o StrictHostKeyChecking=accept-new`.
- A **changed** host key (`HostKeyMismatch`) is always refused and shown as a
  prominent alert.

## 5. Configuration

Stored as TOML in `~/Library/Application Support/TunnelBar/tunnels.toml`,
created with `0600` permissions, editable by hand and reloaded on change.

```toml
version = 1

[[tunnel]]
id = "prod"                 # stable identifier
name = "Prod server"        # label shown in the UI
host = "203.0.113.10"      # or an alias from ~/.ssh/config
# user = "deploy"           # optional
# port = 22                 # optional
autostart = false           # start when TunnelBar starts
auto_reconnect = true
accept_new_host_key = false

[[tunnel.forward]]
type = "local"              # -L
local_port = 54321
remote_host = "127.0.0.1"
remote_port = 5432

# [[tunnel.forward]]
# type = "remote"           # -R
# remote_port = 8080
# local_host = "127.0.0.1"
# local_port = 3000

# [[tunnel.forward]]
# type = "dynamic"          # -D (SOCKS)
# local_port = 1080
```

- One tunnel = one SSH connection carrying one or more forwards; it starts
  and stops as a whole.
- The model supports `local`, `remote` and `dynamic` forwards. The v1 form
  only edits `local` forwards; the others are edited in the file and displayed
  correctly by the UI.

### Validation rules

- Ports are in `1..=65535`; two tunnels may not bind the same local port.
- `id` values are unique.
- `host` and `user` must not start with `-`, nor contain whitespace or
  control characters (prevents option injection such as `-oProxyCommand=…`).
- No free-form `ssh` options; advanced settings go through a
  `~/.ssh/config` alias.

## 6. Tunnel lifecycle

```
Stopped ──start──► Connecting ──port open──► Up
   ▲                  │                        │ connection lost
   │ stop             ▼                        ▼
   └────────────── Failed ◄── fatal ── Reconnecting (attempt n)
```

- **Up** = the `ssh` process is alive **and** each local port (`local` and
  `dynamic` forwards) accepts TCP connections. Ports are probed every 0.5 s
  **while connecting only**: probing an up tunnel would open a connection to
  the remote service every time (noise in its logs); once up, `ssh` keepalives
  detect a dead connection. A tunnel with remote forwards only is up after its
  process has lived 5 s.
- A connection whose ports are still closed after 30 s is killed and retried
  (`ConnectionTimedOut`).
- **Reconnection** (`auto_reconnect = true` by default): exponential backoff
  1 s, 2 s, 4 s… capped at 60 s. Fatal errors (`AuthFailed`, `PortInUse`,
  `HostKeyMismatch`, `HostKeyUnknown`) stop retrying.
- After wake from sleep or a network change, reconnecting tunnels retry
  immediately. Both are detected by polling every 5 s, without OS
  notification APIs: a wake-up shows as the wall clock jumping ahead of the
  monotonic clock (which stops during sleep), a network change as a different
  set of local IP addresses.
- A macOS notification is shown when a tunnel fails for good.

### Application lifecycle

- Menu bar only: no Dock icon, not in Cmd+Tab.
- "Launch at login" setting (off by default), via `SMAppService`.
- Per-tunnel `autostart` (off by default).
- Quitting stops every `ssh` process. The PIDs of running `ssh` processes
  are recorded in `ssh.pids` next to the configuration; at startup, a recorded
  process is killed only if it was orphaned (parent is launchd) and its command
  line starts with TunnelBar's exact `ssh` options, so a recycled PID never
  hits an unrelated process.
- Launch at login uses `SMAppService` (`src-tauri/src/login_item.rs`, the
  only module allowed to use `unsafe`, for the Objective-C calls).

## 7. User interface

**Tray menu** (right click) for quick access:

```
● Prod server          ✓   ← click toggles start/stop
○ Redis staging
⚠ Client X bastion
─────────────
Open panel…
Quit
```

The tray icon is a template image reflecting the global state: no tunnel up,
all up, at least one failed.

**Panel** (left click), a popover anchored to the icon:

- Tunnel list: status dot, name, summary
  (`localhost:54321 → 127.0.0.1:5432 via 203.0.113.10`), toggle, and a menu
  (edit, duplicate, delete, copy equivalent `ssh` command).
- Tunnel detail: uptime, reconnection count, last error (translated), last
  50 lines of `ssh` output with a Copy button.
- Add / edit form with live validation from the core.
- Settings: launch at login, language (auto / FR / EN), open config file.

Logs are written to `~/Library/Logs/io.github.gduprez.tunnelbar/` with rotation.

## 8. Internationalisation

- The UI follows the macOS language; English is the fallback.
- Translations live in `locales/<lang>.json`; adding a language means adding
  one file.
- The core never returns display text: it returns typed errors
  (`PortInUse { port }`, `AuthFailed`, …) that the UI translates.
- A CI check fails if a locale has missing or extra keys compared to `en.json`.
- All documentation exists in English and French (`docs/en`, `docs/fr`,
  `README.md` / `README.fr.md`, `CONTRIBUTING`).

## 9. Architecture

```
          commands (start/stop/reload)
UI ──► Tauri command ──► TunnelManager ──► Supervisor(tunnel A) ──► ProcessSpawner ──► ssh
                             │             Supervisor(tunnel B)        PortProber
                             │                     │                   Clock (tokio time)
                             ◄──── events (broadcast) ◄─┘
UI ◄── emit("tunnel-state") ◄┘
```

- **`TunnelManager`**: owns the loaded configuration, starts and stops
  supervisors, rejects port conflicts.
- **`Supervisor`** (one Tokio task per running tunnel): spawns `ssh`, probes
  ports, reads stderr, applies backoff, publishes state changes.
- **State machine**: a pure function `(state, event) -> (state, actions)`,
  without I/O.
- **`ssh` error parser**: a pure function mapping raw stderr to typed errors
  (`AuthFailed`, `HostUnreachable`, `PortInUse`, `HostKeyUnknown`,
  `HostKeyMismatch`, …).
- **Injected ports** (traits) for every side effect: `ProcessSpawner`,
  `PortProber`, `ConfigStore`. Time is controlled with Tokio's paused clock in
  tests.

## 10. Development process

### TDD

Red → green → refactor, in small steps, each ending with a commit
([Conventional Commits](https://www.conventionalcommits.org/): `test:`,
`feat:`, `fix:`, `refactor:`…).

Test pyramid:

1. `tunnelbar-core` unit tests (most tests): model, validation, command
   building, state machine, error parser.
2. Core integration tests with fake processes and a scripted fake `ssh`.
3. Thin tests of the Tauri command layer.
4. UI component tests (Vitest + Testing Library, mocked Tauri API).
5. Optional end-to-end test against a local `sshd`, run on demand.

### CI (GitHub Actions, macOS runner)

On every pull request and push to `main` (all blocking):

- `cargo fmt --check`, `prettier --check`
- `cargo clippy -- -D warnings`, `svelte-check`, ESLint
- `cargo nextest run`
- `vitest run`
- locale key check
- `cargo llvm-cov` on `tunnelbar-core`, minimum 85 % line coverage
- `cargo deny` (licenses, advisories)
- `cargo tauri build`

On a `v*` tag: universal `.dmg`, GitHub Release with generated notes
(`release-please`, SemVer), update of the `gduprez/homebrew-tap` cask.

`main` is protected: merge only with green CI. Issue and PR templates exist
in English and French.

### Distribution

- No Apple Developer ID for now: the `.dmg` is ad-hoc signed. The
  documentation explains how to open the app the first time
  (right click → Open, or `xattr -dr com.apple.quarantine`).
- Install via the personal tap `gduprez/homebrew-tap`, or build from source
  (`cargo tauri build`).
- The pipeline is designed so that Developer ID signing and notarisation can
  be enabled later by adding secrets only.

## 11. Milestones

| # | Milestone | Outcome |
|---|---|---|
| 0 | Skeleton: workspace, Tauri + Svelte, full CI, licenses, minimal README | Empty icon in the menu bar |
| 1 | Model, validation (incl. security rules), TOML load/save, `0600` | — |
| 2 | `Tunnel` → `ssh` argument vector | — |
| 3 | State machine and backoff (pure) | — |
| 4 | `ssh` stderr parser | — |
| 5 | Supervisor and manager (Tokio actors, fakes, scripted fake `ssh`) | — |
| 6 | Tauri commands, tray menu with start/stop, icon state | **First real use** |
| 7 | Panel: list, detail, logs, form, FR/EN | Full UI |
| 8 | Launch at login, autostart, sleep/network handling, notifications, orphan cleanup | — |
| 9 | Release: `.dmg`, Homebrew tap, full FR/EN docs, CONTRIBUTING | **v0.1.0, public** |
