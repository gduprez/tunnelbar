#!/usr/bin/env bash
# Runs locally the checks of the CI (.github/workflows/ci.yml), to know in a minute whether
# a change will pass, without waiting for GitHub.
#
#   scripts/check.sh            all checks except the app bundle build
#   scripts/check.sh --build    also build TunnelBar.app, as the macOS CI job does
set -euo pipefail
cd "$(dirname "$0")/.."

# rustup's toolchain carries llvm-tools, needed by the coverage step.
if [ -x "$HOME/.cargo/bin/rustup" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

wanted_node="$(cat .nvmrc)"
node_major="$(node --version 2>/dev/null | sed -E 's/^v([0-9]+).*/\1/' || true)"
if [ "${node_major:-0}" -lt "$wanted_node" ]; then
  echo "Node.js $wanted_node or later is needed (found: $(node --version 2>/dev/null || echo none))." >&2
  echo "Run: nvm use" >&2
  exit 1
fi

step() { printf '\n\033[1m▶ %s\033[0m\n' "$*"; }

step "Rust: format";        cargo fmt --all --check
step "Rust: clippy";        cargo clippy --workspace --all-targets -- -D warnings
step "Rust: tests";         cargo nextest run --workspace
step "Rust: core coverage"; cargo llvm-cov nextest -p tunnelbar-core --fail-under-lines 85
step "Rust: dependencies";  cargo deny check
step "npm: dependencies";   npm audit --omit=dev --audit-level=high
step "UI: format";          npm run -s format:check
step "UI: lint";            npm run -s lint
step "UI: types";           npm run -s check
step "UI: tests";           npm test
if [ "${1:-}" = "--build" ]; then
  step "App bundle";        npx tauri build --bundles app
fi

printf '\n\033[1;32m✔ All checks passed.\033[0m\n'
