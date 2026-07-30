#!/bin/bash
#SBATCH --job-name=harvest-gpu-smoke
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=4
#SBATCH --mem=32G
#SBATCH --time=00:15:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-smoke-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-smoke-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

echo "=== GPU smoke test on $(hostname -f) ==="
echo "Date: $(date -u)"
nvidia-smi

module load cuda/12.4.1 2>/dev/null || module load cuda/12.6.3 2>/dev/null || true

# Install torch in a lightweight test venv if not present
TEST_VENV="${HARVEST_ENVS}/gpu-smoke"
if [[ ! -d "${TEST_VENV}" ]]; then
  python3 -m venv "${TEST_VENV}"
  # shellcheck source=/dev/null
  source "${TEST_VENV}/bin/activate"
  pip install --upgrade pip
  pip install torch --index-url https://download.pytorch.org/whl/cu124
else
  # shellcheck source=/dev/null
  source "${TEST_VENV}/bin/activate"
fi

python "${HARVEST_INFRA}/scripts/gpu_smoke_test.py"
echo "=== GPU smoke test PASSED ==="
