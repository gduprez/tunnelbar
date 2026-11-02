# Configuration reference

> 🇫🇷 [Version française](../fr/configuration.md)

TunnelBar stores your tunnels in a TOML file:

```
~/Library/Application Support/TunnelBar/tunnels.toml
```

You can edit it from the app or by hand. A complete, tested example is in
[`docs/examples/tunnels.toml`](../examples/tunnels.toml).

## Minimal example

The command `ssh -N -L 54321:127.0.0.1:5432 203.0.113.10` becomes:

```toml
version = 1

[[tunnel]]
id = "prod-db"
name = "Prod PostgreSQL"
host = "203.0.113.10"

[[tunnel.forward]]
type = "local"
local_port = 54321
remote_host = "127.0.0.1"
remote_port = 5432
```

## File

| Key | Required | Description |
|---|---|---|
| `version` | yes | Format version. Must be `1`. |
| `[[tunnel]]` | no | Zero or more tunnels. |

## Tunnel (`[[tunnel]]`)

A tunnel is **one SSH connection** carrying one or more forwards. It starts and
stops as a whole.

| Key | Required | Default | Description |
|---|---|---|---|
| `id` | yes | — | Stable identifier: 1 to 64 characters among `a-z`, `0-9`, `-`, `_`, starting with a letter or digit. Unique. |
| `name` | yes | — | Label shown in the menu and the panel. |
| `host` | yes | — | Host name, IP address, or alias from `~/.ssh/config`. |
| `user` | no | from `~/.ssh/config` or your login | SSH user. |
| `port` | no | from `~/.ssh/config` or `22` | SSH port. |
| `autostart` | no | `false` | Start the tunnel when TunnelBar starts. |
| `auto_reconnect` | no | `true` | Reconnect automatically after a connection loss. |
| `accept_new_host_key` | no | `false` | Accept the host key of a server never seen before (`StrictHostKeyChecking=accept-new`). A **changed** key is always refused. |
| `[[tunnel.forward]]` | yes (at least one) | — | Forwards carried by the connection. |

> **Tip:** advanced SSH settings (`ProxyJump`, `IdentityFile`, …) go in
> `~/.ssh/config`. Create a `Host` alias there and use it as `host`.

## Forwards (`[[tunnel.forward]]`)

The `type` key selects the kind of forward.

### `local`: `-L` (access a remote service from your Mac)

| Key | Description |
|---|---|
| `local_port` | Port opened on your Mac (`127.0.0.1`). |
| `remote_host` | Host to reach, **as seen from the SSH server** (often `127.0.0.1`). |
| `remote_port` | Port of that service. |

### `remote`: `-R` (expose a service of your Mac on the server)

| Key | Description |
|---|---|
| `remote_port` | Port opened on the SSH server. |
| `local_host` | Host to reach, as seen from your Mac (often `127.0.0.1`). |
| `local_port` | Port of that service. |

### `dynamic`: `-D` (SOCKS proxy)

| Key | Description |
|---|---|
| `local_port` | Port of the SOCKS proxy opened on your Mac. |

## Validation rules

TunnelBar refuses to load or save a configuration that breaks one of these
rules, and shows which tunnel and field is at fault:

| Error | Rule |
|---|---|
| `UnsupportedVersion` | `version` must be `1`. |
| `InvalidId` | See the `id` format above. |
| `DuplicateId` | Two tunnels have the same `id`. |
| `EmptyName` | `name` is empty or blank. |
| `InvalidHost`, `InvalidUser`, `InvalidForwardHost` | Empty, starting with `-`, or containing spaces or control characters. This prevents a value from being read by `ssh` as an option (e.g. `-oProxyCommand=…`). |
| `ZeroPort` | Ports go from `1` to `65535` (values above `65535` are rejected when reading the file). |
| `NoForward` | A tunnel needs at least one forward. |
| `LocalPortConflict` | Two forwards (`local` or `dynamic`) listen on the same port of your Mac. |

Unknown keys are refused too, so that a typo (`hots = …`) does not go
unnoticed.

## Security

- The file is written with `0600` permissions (readable by you only), in a
  directory created with `0700`.
- Writes are atomic: the file is never left half-written.
- TunnelBar never runs `ssh` through a shell, and has no "free options" field.
