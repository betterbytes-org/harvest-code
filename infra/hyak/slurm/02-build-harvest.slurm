#!/bin/bash
#SBATCH --job-name=harvest-build
#SBATCH --account=u_hyak_harvest
#SBATCH --partition=compute
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

cd "${HARVEST_ROOT}"
cargo build --release --bin translate --bin benchmark
cargo run --bin=translate -- --print-config-path
echo "=== HARVEST build PASSED ==="
