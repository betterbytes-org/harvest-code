#!/bin/bash
#SBATCH --job-name=harvest-build
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=8
#SBATCH --mem=16G
#SBATCH --time=02:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/build-harvest-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/build-harvest-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

echo "=== Build HARVEST on compute node $(hostname -f) ==="
echo "Date: $(date -u)"

module load llvm/21 2>/dev/null || {
  echo "WARN: llvm/21 module not found; ensure libclang-dev is available"
}

# Rust toolchain in scratch (see scripts/setup_rust.sh)
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
if [[ ! -x "${CARGO_HOME}/bin/cargo" ]]; then
  echo "Installing Rust toolchain to ${CARGO_HOME}..."
  bash "${HARVEST_INFRA}/scripts/setup_rust.sh"
fi

# Rust toolchain (rustup in scratch; not on compute-node PATH by default)
export PATH="${CARGO_HOME}/bin:${PATH}"

if ! command -v cargo >/dev/null; then
  echo "FAIL: cargo not found after setup" >&2
  exit 1
fi

cd "${HARVEST_ROOT}"
cargo build --release --bin translate --bin benchmark
cargo run --bin=translate -- --print-config-path
echo "=== HARVEST build PASSED ==="
