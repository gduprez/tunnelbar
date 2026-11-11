# Changelog

## [0.3.1](https://github.com/gduprez/tunnelbar/compare/v0.3.0...v0.3.1) (2026-11-11)


### Bug Fixes

* **deps:** override minimatch to 11.0.3 (GHSA-g3hv-75wg-m249) ([#5](https://github.com/gduprez/tunnelbar/issues/5)) ([17b8e04](https://github.com/gduprez/tunnelbar/commit/17b8e0439f9b74f56c677209cd573b0ef1d9dc2c))

## [0.3.0](https://github.com/gduprez/tunnelbar/compare/v0.2.1...v0.3.0) (2026-11-10)


### Features

* **app:** Linux launch at login through an XDG autostart entry; the app builds on Linux (L3) ([8b4e5d0](https://github.com/gduprez/tunnelbar/commit/8b4e5d0de1e61ba2382bd4d5d1d56f60cb602d00))
* **app:** Linux panel as a normal window, tray detection over D-Bus, explanation when no icon can be shown (L4) ([23aad73](https://github.com/gduprez/tunnelbar/commit/23aad731615e936b860c7085168177b60e30e6e0))
* **app:** xdg-open for the configuration file and white tray icons on Linux (L3) ([4288a9c](https://github.com/gduprez/tunnelbar/commit/4288a9cc0ba5c7ee65b0f86b67fbad057b9f5e01))
* **core:** on Linux, ssh processes end with TunnelBar even if it crashes (PR_SET_PDEATHSIG from a dedicated spawn thread) (L2) ([5933e17](https://github.com/gduprez/tunnelbar/commit/5933e1783f31b5ff40a64806fd23f8c9dbf805af))
* **core:** per-OS application paths, XDG on Linux (L1) ([7955e59](https://github.com/gduprez/tunnelbar/commit/7955e5928b269d2cd4c3678e6c7d671c92adbd2d))


### Bug Fixes

* audit hardening of the Linux port — .deb runtime deps, validated autostart executable, shared executable name, docs ([c6eaed4](https://github.com/gduprez/tunnelbar/commit/c6eaed4e8db200be0b312824dc7d2f9040915d3d))
* **core:** an orphan is an ssh whose parent is no longer TunnelBar, which covers systemd --user (L2) ([1893dc5](https://github.com/gduprez/tunnelbar/commit/1893dc506137490d4349f6d8bd1466553465fd45))
* **core:** fail a tunnel whose local port is taken by another program ([f488174](https://github.com/gduprez/tunnelbar/commit/f488174e22ddf93d7a87973870d0dcb808a38502))
* **security:** harden release, CI and bind detection before going public ([1415384](https://github.com/gduprez/tunnelbar/commit/14153840070fab745c46f6b22531bbdb50f3f61b))

## [0.2.1](https://github.com/gduprez/tunnelbar/compare/v0.2.0...v0.2.1) (2026-11-03)


### Bug Fixes

* **deps:** replace yanked async-executor 1.14.0 with 1.14.1 ([e976106](https://github.com/gduprez/tunnelbar/commit/e9761063c2a1a9eac5ae69fe48c6a17a8b1209d1))
* launch at login can be turned on before the app ever registered ([6347269](https://github.com/gduprez/tunnelbar/commit/6347269bc9a6ff7a58873875446f2b6fab4675f4))
* **ui:** key action menu items by position, not by their translated label ([8543d0b](https://github.com/gduprez/tunnelbar/commit/8543d0b67d5cd71c223bbf10bd60711d8d521611))

## [0.2.0](https://github.com/gduprez/tunnelbar/compare/v0.1.1...v0.2.0) (2026-11-02)


### Features

* **ui:** redesigned panel — state-coloured cards, inline errors, actions menu, grouped form, panel fits its content ([950e8e8](https://github.com/gduprez/tunnelbar/commit/950e8e83f08acaffb58bb62fa649d8bd3dd63465))


### Bug Fixes

* report port clashes on the edited tunnel with their own location; bounded panel resize command ([7f7924b](https://github.com/gduprez/tunnelbar/commit/7f7924b1ed11bdb4f4e9f8ce67256b73b79c04e9))

## [0.1.1](https://github.com/gduprez/tunnelbar/compare/v0.1.0...v0.1.1) (2026-11-02)


### Bug Fixes

* **app:** explicit strict CSP and frozen prototypes in the webview ([bee92d2](https://github.com/gduprez/tunnelbar/commit/bee92d2fda1f539ebae8ed530aacfe439cf46e1a))
* **core:** only kill orphans whose start time matches; signal them directly ([5bfd913](https://github.com/gduprez/tunnelbar/commit/5bfd91360da82792210b279b0ee042006db87229))
* quote zsh-expanded words in copied commands; atomic private writes for settings and PIDs ([db3c2f3](https://github.com/gduprez/tunnelbar/commit/db3c2f32c8fabcad49c52db35a1711ff859196fe))

## 0.1.0 (2026-11-02)


### Features

* TunnelBar, a macOS menu bar app to start, stop and monitor SSH tunnels ([be9561f](https://github.com/gduprez/tunnelbar/commit/be9561ff3e09fa5a69935a4fea640594a8364f66))
