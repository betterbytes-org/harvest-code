#!/bin/bash
#SBATCH --job-name=harvest-translate
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:1
#SBATCH --cpus-per-task=8
#SBATCH --mem=32G
#SBATCH --time=01:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/translate-only-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/translate-only-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
TESTCASE="/gscratch/harvest/rithvik/testcases/hello"
OUTPUT="/gscratch/harvest/rithvik/output/quick-hello-${SLURM_JOB_ID}"
OLLAMA_HOME="/gscratch/harvest/rithvik/envs/ollama"
OLLAMA_RUN_HOME="${OLLAMA_HOME}/run-home"

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"

export PATH="${CARGO_HOME}/bin:${OLLAMA_HOME}/bin:${PATH}"
export LD_LIBRARY_PATH="${OLLAMA_HOME}/lib/ollama:${OLLAMA_HOME}/lib:${LD_LIBRARY_PATH:-}"
export OLLAMA_MODELS="${OLLAMA_HOME}/models"
export OLLAMA_HOST="127.0.0.1:11434"

hyak_harvest_load_llvm

command -v cargo >/dev/null || { echo "FAIL: cargo not on PATH (${CARGO_HOME}/bin)"; exit 1; }
command -v rustc >/dev/null || { echo "FAIL: rustc not on PATH"; exit 1; }
echo "cargo: $(command -v cargo) ($(cargo --version))"

[[ -x "${HARVEST_ROOT}/target/release/translate" ]] || { echo "FAIL: translate binary missing"; exit 1; }

ollama_cmd() { HOME="${OLLAMA_RUN_HOME}" OLLAMA_MODELS="${OLLAMA_MODELS}" "$@"; }

ollama_cmd ollama serve >>"${HARVEST_LOGS}/ollama-${SLURM_JOB_ID}.log" 2>&1 &
OLLAMA_PID=$!
for _ in $(seq 1 60); do
  curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1 && break
  sleep 2
done
curl -sf http://127.0.0.1:11434/api/tags >/dev/null || { echo "FAIL: Ollama not ready"; exit 1; }

mkdir -p "${OUTPUT}"
echo "Translating ${TESTCASE} -> ${OUTPUT}"
cd "${HARVEST_ROOT}"
"${HARVEST_ROOT}/target/release/translate" \
  --config tools.raw_source_to_cargo_llm.address=http://127.0.0.1:11434 \
  --config tools.raw_source_to_cargo_llm.backend=ollama \
  --config tools.raw_source_to_cargo_llm.model=codellama:7b \
  --config tools.fix_declarations_llm.address=http://127.0.0.1:11434 \
  --config tools.fix_declarations_llm.backend=ollama \
  --config tools.fix_declarations_llm.model=codellama:7b \
  "${TESTCASE}" -o "${OUTPUT}"

echo "=== DONE ==="
find "${OUTPUT}" -name "*.rs" -o -name "Cargo.toml" 2>/dev/null | head -20
kill "${OLLAMA_PID}" 2>/dev/null || true
