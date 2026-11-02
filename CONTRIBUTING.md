# Contributing to TunnelBar

> 🇫🇷 [Version française](CONTRIBUTING.fr.md)

Thanks for your interest! Bug reports, translations and pull requests are welcome.

## Before you start

- Read the [design document](docs/en/design.md): it records the decisions the
  code follows. A change to one of them updates that document in the same pull
  request.
- For anything bigger than a fix, open an issue first to agree on the approach.

## Setup

Prerequisites: macOS 13+, Rust (stable, 1.93 or later), Node.js 24 (`.nvmrc`),
Xcode command line tools.

```sh
npm ci
npm run tauri dev      # run the app
```

To work on the panel in a browser, with sample tunnels and no Rust side:

```sh
npm run dev            # then open http://localhost:1420/?preview
```

## Test-driven development

TunnelBar is developed test first: **red → green → refactor**.

1. Write a test that fails for the right reason.
2. Write the simplest code that makes it pass.
3. Clean up, tests still green.

Where tests go:

| Layer | Location | Tools |
|---|---|---|
| Domain (model, validation, `ssh` command, state machine, error diagnosis, supervision) | `crates/tunnelbar-core` | `cargo nextest`; fakes in `src/fake.rs`; Tokio paused time |
| Real processes | `crates/tunnelbar-core/tests/real_processes.rs` | scripted fake `ssh` |
| Tauri layer (tray logic, translations, commands) | `src-tauri/src` | pure functions next to their tests |
| Panel | `src/**/*.test.ts` | Vitest, Testing Library, mocked `src/lib/api.ts` |
| End to end (on demand) | `crates/tunnelbar-core/tests/end_to_end.rs` | a real SSH server |

```sh
TUNNELBAR_E2E_HOST=my-server cargo test -p tunnelbar-core --test end_to_end -- --ignored
```

The core keeps at least **85 %** line coverage (CI-enforced).

## Checks

The CI runs all of these; run them before pushing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo llvm-cov -p tunnelbar-core --fail-under-lines 85
cargo deny check
npm run format:check && npm run lint && npm run check && npm test
```

## Commits and pull requests

- [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`,
  `docs:`, `test:`, `refactor:`, `chore:`… They produce the changelog and the
  version number (release-please).
- One topic per pull request, with its tests.
- Any user-visible text goes through translations (see below) — in both
  English and French.

## Adding a language

1. Copy `locales/en.json` to `locales/<code>.json` (`de`, `es`, `pt-BR` → `pt`…).
2. Translate the values; keep the keys and the `{placeholders}`.
3. Run `npm test`: a test fails if a key is missing or extra.

That's all: the panel and the menu pick the file up automatically, and the
language appears in the settings under its own name.

## Security

Please report vulnerabilities privately: see [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under
`MIT OR Apache-2.0`, like the project.
