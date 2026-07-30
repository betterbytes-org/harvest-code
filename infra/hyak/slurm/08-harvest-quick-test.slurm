#!/bin/bash
#SBATCH --job-name=harvest-quick
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:1
#SBATCH --cpus-per-task=8
#SBATCH --mem=32G
#SBATCH --time=02:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/quick-run-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/quick-run-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
TESTCASE="/gscratch/harvest/rithvik/testcases/hello"
OUTPUT="/gscratch/harvest/rithvik/output/quick-hello-${SLURM_JOB_ID}"
OLLAMA_HOME="/gscratch/harvest/rithvik/envs/ollama"

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"

export PATH="${CARGO_HOME}/bin:/gscratch/harvest/rithvik/bin:${PATH}"
export OLLAMA_MODELS="${OLLAMA_HOME}/models"
export OLLAMA_HOST="127.0.0.1:11434"
# Home dir is over quota; run Ollama with a scratch HOME.
export OLLAMA_RUN_HOME="${OLLAMA_HOME}/run-home"
mkdir -p "${OLLAMA_RUN_HOME}/.ollama" "${OLLAMA_HOME}/models" "${OUTPUT}"

echo "=== HARVEST quick run on $(hostname -f) ==="
echo "Date: $(date -u)"
nvidia-smi -L

hyak_harvest_load_modules
hyak_harvest_load_llvm

# --- Ollama (default HARVEST LLM backend, user-local install) ---
mkdir -p "${OLLAMA_HOME}/bin" "${OLLAMA_HOME}/lib"
if [[ ! -x "${OLLAMA_HOME}/bin/ollama" ]]; then
  echo "Installing Ollama to ${OLLAMA_HOME}..."
  curl -fsSL https://ollama.com/download/ollama-linux-amd64.tar.zst \
    | tar --zstd -xf - -C "${OLLAMA_HOME}" 2>/dev/null \
    || { curl -fsSL https://ollama.com/download/ollama-linux-amd64.tar.zst | unzstd | tar -xf - -C "${OLLAMA_HOME}"; }
fi
export PATH="${OLLAMA_HOME}/bin:${PATH}"
export LD_LIBRARY_PATH="${OLLAMA_HOME}/lib/ollama:${OLLAMA_HOME}/lib:${LD_LIBRARY_PATH:-}"

ollama_cmd() { HOME="${OLLAMA_RUN_HOME}" OLLAMA_MODELS="${OLLAMA_MODELS}" "$@"; }

ollama_cmd ollama serve >>"${HARVEST_LOGS}/ollama-${SLURM_JOB_ID}.log" 2>&1 &
OLLAMA_PID=$!

echo "Waiting for Ollama server..."
for i in $(seq 1 60); do
  if curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    break
  fi
  sleep 2
done
curl -sf http://127.0.0.1:11434/api/tags >/dev/null || { echo "FAIL: Ollama did not start"; tail -20 "${HARVEST_LOGS}/ollama-${SLURM_JOB_ID}.log"; exit 1; }

if ! ollama_cmd ollama list 2>/dev/null | grep -q codellama; then
  echo "Pulling codellama:7b (default HARVEST model)..."
  ollama_cmd ollama pull codellama:7b
else
  echo "codellama:7b already cached"
fi
echo "Ollama ready (PID ${OLLAMA_PID})"

# --- Build HARVEST ---
if [[ ! -x "${HARVEST_ROOT}/target/release/translate" ]]; then
  if [[ ! -x "${CARGO_HOME}/bin/cargo" ]]; then
    export HARVEST_INFRA="${HARVEST_INFRA}"
    bash "${HARVEST_INFRA}/scripts/setup_rust.sh"
  fi
  cd "${HARVEST_ROOT}"
  cargo build --release --bin translate
fi

# --- Translate ---
cd "${HARVEST_ROOT}"
echo "Translating ${TESTCASE} -> ${OUTPUT}"
cargo run --bin=translate --release -- \
  --config tools.raw_source_to_cargo_llm.address=http://127.0.0.1:11434 \
  --config tools.raw_source_to_cargo_llm.backend=ollama \
  --config tools.raw_source_to_cargo_llm.model=codellama:7b \
  --config tools.modular_translation_llm.address=http://127.0.0.1:11434 \
  --config tools.modular_translation_llm.backend=ollama \
  --config tools.modular_translation_llm.model=codellama:7b \
  --config tools.fix_declarations_llm.address=http://127.0.0.1:11434 \
  --config tools.fix_declarations_llm.backend=ollama \
  --config tools.fix_declarations_llm.model=codellama:7b \
  "${TESTCASE}" -o "${OUTPUT}"

echo ""
echo "=== DONE ==="
echo "Output: ${OUTPUT}"
ls -la "${OUTPUT}" 2>/dev/null || true
find "${OUTPUT}" -name "*.rs" -o -name "Cargo.toml" 2>/dev/null | head -20

kill "${OLLAMA_PID}" 2>/dev/null || true
