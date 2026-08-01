#!/bin/bash
#SBATCH --job-name=harvest-vllm-env
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=8
#SBATCH --mem=64G
#SBATCH --time=04:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-vllm-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-vllm-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

echo "=== Setup vLLM env on compute node $(hostname -f) ==="
echo "Date: $(date -u)"
nvidia-smi -L

module load cuda/12.4.1 2>/dev/null || module load cuda/12.6.3 2>/dev/null || true
module load gcc/13.2.0 2>/dev/null || true
module load python3/3.12.3 2>/dev/null || module load python3/3.12.1

PY=$(command -v python3)
echo "Using Python: $PY ($($PY -c 'import sys; print(sys.version)'))"
$PY -c "import ctypes; print('ctypes OK')"

# Home dir is over quota; use scratch for pip builds and caches.
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PIP_CACHE_DIR="${HARVEST_ENVS}/pip-cache"
export HOME="${HARVEST_SCRATCH}/run-home"
mkdir -p "${TMPDIR}" "${PIP_CACHE_DIR}" "${HOME}"

# Recreate venv if it was built with a broken Python (e.g. coenv 3.13 missing _ctypes).
if [[ -d "${VLLM_VENV}" ]]; then
  if ! "${VLLM_VENV}/bin/python3" -c "import ctypes, vllm" 2>/dev/null; then
    echo "Removing broken vLLM venv at ${VLLM_VENV}"
    rm -rf "${VLLM_VENV}"
  fi
fi

if [[ ! -d "${VLLM_VENV}" ]]; then
  "${PY}" -m venv "${VLLM_VENV}"
fi
# shellcheck source=/dev/null
source "${VLLM_VENV}/bin/activate"
pip install --upgrade pip wheel

# vLLM 0.20+ with DeepSeek V4 support
pip install "vllm>=0.20.0" huggingface_hub torch

# DeepGEMM FP8 kernels (required for DeepSeek V4 on Hopper)
if [[ -x "${VLLM_VENV}/bin/python" ]]; then
  bash <(curl -fsSL https://raw.githubusercontent.com/vllm-project/vllm/main/tools/install_deepgemm.sh) || {
    echo "WARN: DeepGEMM install failed; vLLM may still work with fallback kernels"
  }
fi

python -c "import vllm; print('vLLM', vllm.__version__)"
echo "=== vLLM env ready at ${VLLM_VENV} ==="
