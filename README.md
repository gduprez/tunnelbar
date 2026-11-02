# TunnelBar

> 🇫🇷 [Lire en français](README.fr.md)

[![CI](https://github.com/gduprez/tunnelbar/actions/workflows/ci.yml/badge.svg)](https://github.com/gduprez/tunnelbar/actions/workflows/ci.yml)

A macOS menu bar app to start, stop and monitor SSH tunnels.

## Features

- One click in the menu bar to start or stop a tunnel such as
  `ssh -L 54321:127.0.0.1:5432 my-server -N`.
- Live state of each tunnel, automatic reconnection after sleep or network
  loss, notification when a tunnel fails for good.
- Local (`-L`), remote (`-R`) and SOCKS (`-D`) forwards, several per connection.
- Uses your system `ssh`: your `~/.ssh/config`, keys and agent just work.
- Clear, translated error messages and the `ssh` output at hand.
- English and French; adding a language is adding one file.

## Install

```sh
brew install --cask gduprez/tap/tunnelbar
```

Or download the `.dmg` from the [releases](https://github.com/gduprez/tunnelbar/releases).
TunnelBar is not notarized by Apple yet: see the
[user guide](docs/en/user-guide.md#install) for the first launch.

## Requirements

- macOS 13 Ventura or later
- Key-based SSH authentication (no password prompt)

## Building from source

Prerequisites: Rust (stable), Node.js 24 (see `.nvmrc`), Xcode command line tools.

```sh
npm ci
npm run tauri dev      # run in development mode
npm run tauri build    # build TunnelBar.app and a .dmg
```

Checks run by the CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo llvm-cov -p tunnelbar-core --fail-under-lines 85
cargo deny check
npm run format:check && npm run lint && npm run check && npm test
```

## Documentation

- [User guide](docs/en/user-guide.md)
- [Configuration reference](docs/en/configuration.md)
- [Contributing](CONTRIBUTING.md)
- [Design document](docs/en/design.md)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in TunnelBar by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
