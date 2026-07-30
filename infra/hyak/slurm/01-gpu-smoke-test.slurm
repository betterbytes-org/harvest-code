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
module load coenv/python/3.13.11 2>/dev/null || module load coenv/python/3.11.9 2>/dev/null || true

PYVER="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
echo "Python: $(python3 --version) (${PYVER})"
if [[ "${PYVER%%.*}" -lt 3 ]] || [[ "${PYVER#*.}" -lt 10 ]]; then
  echo "FAIL: need Python >= 3.10 (got ${PYVER}). Check module load coenv/python." >&2
  exit 1
fi

TEST_VENV="${HARVEST_ENVS}/gpu-smoke"
# Recreate venv if it was built with an older Python
if [[ -d "${TEST_VENV}" ]]; then
  VENV_PY="$("${TEST_VENV}/bin/python3" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")' 2>/dev/null || echo "0.0")"
  if [[ "${VENV_PY}" != "${PYVER}" ]]; then
    echo "Removing stale venv (had Python ${VENV_PY}, want ${PYVER})"
    rm -rf "${TEST_VENV}"
  fi
fi
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
