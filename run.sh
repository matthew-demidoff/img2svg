#!/usr/bin/env bash
#
# Bootstrap and run img2svg. Installs the Rust toolchain (via rustup), the
# wasm32 target, wasm-pack, and the web dependencies, builds the WebAssembly
# core, then starts the app.
#
#   ./run.sh           install everything and start the dev server (default)
#   ./run.sh --build   build the production bundle and preview it
#   ./run.sh --test    run the Rust test suite and exit
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

log() { printf '\n\033[1;34m==>\033[0m %s\n' "$1"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

MODE="dev"
for arg in "$@"; do
  case "$arg" in
    --dev) MODE="dev" ;;
    --build) MODE="build" ;;
    --test) MODE="test" ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) die "unknown option: $arg (try --help)" ;;
  esac
done

if ! command -v rustup >/dev/null 2>&1; then
  log "Installing rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

# A Homebrew-installed rustc can shadow rustup on PATH and has no wasm32 std,
# which breaks wasm-pack. Resolve rustup's own toolchain bin and put it first.
log "Selecting the rustup toolchain"
rustup show >/dev/null 2>&1 || true
TOOLCHAIN_BIN="$(dirname "$(rustup which rustc)")"
export PATH="$TOOLCHAIN_BIN:$PATH"

log "Adding the wasm32 target"
rustup target add wasm32-unknown-unknown

if ! command -v wasm-pack >/dev/null 2>&1; then
  log "Installing wasm-pack"
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

command -v npm >/dev/null 2>&1 || die "Node.js (npm) not found; install Node 20+ from https://nodejs.org and re-run"

if [ "$MODE" = "test" ]; then
  log "Running the Rust test suite"
  cargo test --all
  exit 0
fi

log "Building the WebAssembly core"
wasm-pack build crates/img2svg-core --target web --out-dir ../../web/src/wasm/pkg -- --features wasm

log "Installing web dependencies"
if [ -f web/package-lock.json ]; then
  ( cd web && npm ci )
else
  ( cd web && npm install )
fi

if [ "$MODE" = "build" ]; then
  log "Building the production bundle"
  ( cd web && npm run build )
  log "Serving the production build (Ctrl+C to stop)"
  ( cd web && npm run preview )
else
  log "Starting the dev server (Ctrl+C to stop)"
  ( cd web && npm run dev )
fi
