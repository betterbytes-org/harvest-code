#!/bin/bash
#SBATCH --job-name=gpu-perf-bench
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:1
#SBATCH --cpus-per-task=8
#SBATCH --mem=32G
#SBATCH --time=00:45:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-perf-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-perf-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
OLLAMA_HOME="/gscratch/harvest/rithvik/envs/ollama"
MODEL="${MODEL:-codellama:7b}"

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"

export PATH="${OLLAMA_HOME}/bin:${PATH}"
export LD_LIBRARY_PATH="${OLLAMA_HOME}/lib/ollama:${OLLAMA_HOME}/lib:${LD_LIBRARY_PATH:-}"
export OLLAMA_MODELS="${OLLAMA_HOME}/models"
export OLLAMA_HOST="127.0.0.1:11434"
export OLLAMA_RUN_HOME="${OLLAMA_HOME}/run-home"
mkdir -p "${OLLAMA_RUN_HOME}/.ollama" "${OLLAMA_HOME}/models"

echo "=== GPU performance benchmark on $(hostname -f) ==="
echo "Date: $(date -u)"
nvidia-smi -L
nvidia-smi --query-gpu=name,memory.total,driver_version,compute_cap --format=csv

hyak_harvest_load_modules

# PyTorch for raw GPU compute benchmarks
TEST_VENV="${HARVEST_ENVS}/gpu-smoke"
PYVER="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
if [[ ! -d "${TEST_VENV}" ]] || [[ "$("${TEST_VENV}/bin/python3" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")' 2>/dev/null || echo 0.0)" != "${PYVER}" ]]; then
  rm -rf "${TEST_VENV}"
  python3 -m venv "${TEST_VENV}"
  # shellcheck source=/dev/null
  source "${TEST_VENV}/bin/activate"
  pip install --upgrade pip -q
  pip install torch --index-url https://download.pytorch.org/whl/cu124 -q
else
  # shellcheck source=/dev/null
  source "${TEST_VENV}/bin/activate"
fi

ollama_cmd() { HOME="${OLLAMA_RUN_HOME}" OLLAMA_MODELS="${OLLAMA_MODELS}" "$@"; }

ollama_cmd ollama serve >>"${HARVEST_LOGS}/ollama-perf-${SLURM_JOB_ID}.log" 2>&1 &
OLLAMA_PID=$!
for _ in $(seq 1 60); do
  curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1 && break
  sleep 2
done
curl -sf http://127.0.0.1:11434/api/tags >/dev/null || { echo "FAIL: Ollama not ready"; exit 1; }

if ! ollama_cmd ollama list 2>/dev/null | grep -q "${MODEL%%:*}"; then
  echo "Pulling ${MODEL}..."
  ollama_cmd ollama pull "${MODEL}"
fi

python3 "${HARVEST_INFRA}/scripts/gpu_perf_benchmark.py" "127.0.0.1:11434" "${MODEL}"

kill "${OLLAMA_PID}" 2>/dev/null || true
echo "=== Benchmark complete ==="
