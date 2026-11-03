# Changelog

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
