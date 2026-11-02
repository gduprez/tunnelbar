# TunnelBar user guide

> 🇫🇷 [Version française](../fr/user-guide.md)

TunnelBar lives in the menu bar, next to the clock. It starts, stops and watches
your SSH tunnels, and reconnects them on its own.

## Install

### With Homebrew

```sh
brew install --cask gduprez/tap/tunnelbar
```

### From the disk image

1. Download `TunnelBar_<version>_universal.dmg` from the
   [releases](https://github.com/gduprez/tunnelbar/releases).
2. Open it and drag **TunnelBar** to **Applications**.
3. **First launch only:** TunnelBar is not notarized by Apple yet, so macOS
   blocks it ("Apple could not verify…"). Either:
   - open **System Settings › Privacy & Security**, scroll down and click
     **Open Anyway** next to TunnelBar; or
   - run once in a terminal:

     ```sh
     xattr -dr com.apple.quarantine /Applications/TunnelBar.app
     ```

### From source

See [Building from source](../../README.md#building-from-source).

## Requirements

- macOS 13 Ventura or later.
- **Key-based SSH authentication.** TunnelBar runs `ssh` in the background,
  where nobody can type a password or a passphrase. Check that
  `ssh my-server` connects without asking anything. If your key has a
  passphrase, store it in the macOS keychain once:

  ```sh
  ssh-add --apple-use-keychain ~/.ssh/id_ed25519
  ```

  and add to `~/.ssh/config`:

  ```
  Host *
    UseKeychain yes
    AddKeysToAgent yes
  ```

## Everyday use

| Action | How |
|---|---|
| Start or stop a tunnel | Right-click the icon, then click the tunnel. Or use its switch in the panel. |
| Open the panel | Click the icon. |
| See why a tunnel fails | Open the panel and click the tunnel: the error and the last lines printed by `ssh` are shown. |
| Copy the equivalent `ssh` command | In the tunnel detail, **Copy ssh command**. |
| Quit | Right-click the icon › **Quit TunnelBar**. Every tunnel is closed. |

### The icon

| Icon | Meaning |
|---|---|
| Outlined arch | No tunnel running. |
| Filled arch | Tunnels running normally. |
| Arch with `!` | At least one tunnel failed or is reconnecting. |

### Tunnel states

| State | Meaning |
|---|---|
| Stopped | Not running. |
| Connecting… | `ssh` is running; TunnelBar waits for the local ports to accept connections. |
| Active | The tunnel works. |
| Reconnecting in N s | The connection was lost; TunnelBar retries after 1 s, 2 s, 4 s… up to 60 s. Waking the Mac or changing network retries at once. |
| Error | A problem that retrying will not fix (refused key, port already used, unknown or changed server key…). A notification is shown. Fix the cause, then start the tunnel again. |

## Adding a tunnel

Click **+** in the panel. For the command
`ssh -L 54321:127.0.0.1:5432 203.0.113.10 -N`:

| Field | Value |
|---|---|
| Name | PostgreSQL prod |
| Server | `203.0.113.10` |
| Local port | `54321` |
| Remote host | `127.0.0.1` |
| Remote port | `5432` |

One tunnel can carry several forwards over the same connection
(**Add a forward**). Remote (`-R`) and SOCKS (`-D`) forwards are set in the
configuration file: see the [configuration reference](configuration.md).

Options:

- **Reconnect automatically** (on by default).
- **Start when TunnelBar starts.**
- **Accept the key of a server never seen before**: only for servers whose
  fingerprint you cannot check in a terminal first. A *changed* key is always
  refused.

## Settings

- **Open TunnelBar at login.** macOS may ask you to allow it in
  **System Settings › General › Login Items**.
- **Language**: same as macOS, or any available language.
- **Configuration file**: open `tunnels.toml` to edit it by hand; TunnelBar
  reloads it within two seconds.

## Troubleshooting

| Message | What to do |
|---|---|
| Authentication refused | `ssh my-server` must connect without a prompt (see [Requirements](#requirements)). |
| Unknown server | Run `ssh my-server` once in a terminal and check the fingerprint, then start the tunnel again. |
| The server's identity has CHANGED | Do not bypass it. Ask the server's administrator whether its key changed; if so, remove the old one with `ssh-keygen -R my-server`. |
| Local port N is already used | Another program (or another tunnel) uses that port: `lsof -nP -iTCP:N -sTCP:LISTEN` tells which. |
| Unknown host name | Check the spelling, or your network/VPN. |

Application logs: `~/Library/Logs/io.github.gduprez.tunnelbar/tunnelbar.log`.

## Files

| File | Content |
|---|---|
| `~/Library/Application Support/TunnelBar/tunnels.toml` | Your tunnels. |
| `~/Library/Application Support/TunnelBar/settings.toml` | App settings. |
| `~/Library/Application Support/TunnelBar/ssh.pids` | Running `ssh` processes, to clean up after a crash. |
| `~/Library/Logs/io.github.gduprez.tunnelbar/` | Logs. |
