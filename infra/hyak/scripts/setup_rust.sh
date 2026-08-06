#!/usr/bin/env bash
# Install rustup toolchain on a compute node into scratch (run once).
set -euo pipefail

HARVEST_ROOT="${HARVEST_ROOT:-/gscratch/harvest/rithvik/harvest}"
HARVEST_INFRA="${HARVEST_INFRA:-${HARVEST_ROOT}/infra/hyak}"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"

export RUSTUP_HOME="${HARVEST_ENVS}/rustup"
export CARGO_HOME="${HARVEST_ENVS}/cargo"
export PATH="${CARGO_HOME}/bin:${PATH}"

if command -v cargo >/dev/null; then
  echo "Rust already installed: $(cargo --version)"
  exit 0
fi

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
rustup default stable
rustup toolchain install 1.93.0
rustup default 1.93.0
cargo --version
rustc --version
echo "Rust installed to ${CARGO_HOME}"
