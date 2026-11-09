# TunnelBar on Ubuntu

> 🇫🇷 [Version française](../fr/linux.md)

TunnelBar runs on **Ubuntu 24.04 LTS and 26.04 LTS** (GNOME desktop, Wayland or
X11 session, amd64). Other Ubuntu flavours with a tray area may work, untested.

## Install

### `.deb` package (recommended)

```sh
sudo apt install ./TunnelBar_<version>_amd64.deb
```

The package also installs `openssh-client` if missing. TunnelBar shows up in
the applications list.

### AppImage (no installation)

```sh
chmod +x TunnelBar_<version>_amd64.AppImage
./TunnelBar_<version>_amd64.AppImage
```

## The icon in the top bar

Ubuntu shows application icons through the **Ubuntu AppIndicators** extension,
on by default. If it was turned off, TunnelBar opens as a window at start and
explains what to do:

- turn on "Ubuntu AppIndicators" in the **Extensions** app;
- or install it: `sudo apt install gnome-shell-extension-appindicator`, then
  log out and back in.

To check that it is on:

```sh
gnome-extensions list --enabled | grep -i appindicator
```

## Differences with macOS

| | macOS | Ubuntu |
|---|---|---|
| Clicking the icon | opens the panel | opens the menu (tunnels, "Open panel…") |
| Panel | popover under the icon, closes when clicking elsewhere | small normal window, placed by the desktop |
| Closing the panel | — | hides it; TunnelBar stays in the top bar. **Without a visible icon, closing quits TunnelBar** (it would be unreachable otherwise) |
| Launch at login | System Settings › Login Items | `~/.config/autostart/tunnelbar.desktop`, written by TunnelBar's setting |
| Key passphrases | macOS keychain | GNOME SSH agent (gnome-keyring) |

Everything else is the same: tunnels, automatic reconnection, notifications,
translations, configuration file format.

## SSH authentication

As on macOS, TunnelBar runs `ssh` in the background, where nobody can type a
password. Check that `ssh my-server` connects without asking anything. If your
key has a passphrase, the GNOME SSH agent keeps it after the first prompt of
your session.

## Files

| File | Content |
|---|---|
| `~/.config/tunnelbar/tunnels.toml` | Your tunnels ([reference](configuration.md)) |
| `~/.config/tunnelbar/settings.toml` | App settings |
| `~/.local/state/tunnelbar/ssh.pids` | Running `ssh` processes |
| `~/.local/share/io.github.gduprez.tunnelbar/logs/` | Logs |
| `~/.config/autostart/tunnelbar.desktop` | Launch at login, when turned on |

`XDG_CONFIG_HOME` and `XDG_STATE_HOME` are honoured.

## If TunnelBar crashes

On Linux, every `ssh` started by TunnelBar stops automatically when TunnelBar
stops, even abruptly: no tunnel stays open behind your back.

## Uninstall

```sh
sudo apt remove tunnelbar
rm -f ~/.config/autostart/tunnelbar.desktop
```

Your tunnels stay in `~/.config/tunnelbar/`; delete that folder to erase
everything.
