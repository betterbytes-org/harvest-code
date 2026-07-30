#!/bin/bash
#SBATCH --job-name=harvest-translate
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=8
#SBATCH --mem=16G
#SBATCH --time=04:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/translate-test-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/translate-test-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

ENDPOINT_FILE="${HARVEST_STATE}/vllm-endpoint.env"
# shellcheck source=/dev/null
source "${ENDPOINT_FILE}"

C_INPUT="${C_INPUT:-/tmp/harvest_hello_c}"
C_OUTPUT="${C_OUTPUT:-${HARVEST_SCRATCH}/translate-output/hello}"

# Minimal C hello-world if no TRACTOR corpus is available
if [[ ! -d "${C_INPUT}/src" ]]; then
  mkdir -p "${C_INPUT}/src"
  cat >"${C_INPUT}/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.10)
project(hello C)
add_executable(hello src/main.c)
EOF
  cat >"${C_INPUT}/src/main.c" <<'EOF'
#include <stdio.h>
int main(void) {
    printf("hello harvest\n");
    return 0;
}
EOF
fi

module load llvm/21 2>/dev/null || true

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "${VLLM_HOST}" "${VLLM_PORT}" 600

cd "${HARVEST_ROOT}"
cargo run --bin=translate --release -- \
  --config "tools.raw_source_to_cargo_llm.backend=openai" \
  --config "tools.raw_source_to_cargo_llm.address=${VLLM_ENDPOINT}" \
  --config "tools.raw_source_to_cargo_llm.model=${VLLM_SERVED_NAME}" \
  --config "tools.modular_translation_llm.backend=openai" \
  --config "tools.modular_translation_llm.address=${VLLM_ENDPOINT}" \
  --config "tools.modular_translation_llm.model=${VLLM_SERVED_NAME}" \
  --config "tools.fix_declarations_llm.backend=openai" \
  --config "tools.fix_declarations_llm.address=${VLLM_ENDPOINT}" \
  --config "tools.fix_declarations_llm.model=${VLLM_SERVED_NAME}" \
  "${C_INPUT}" -o "${C_OUTPUT}"

echo "=== Translation output in ${C_OUTPUT} ==="
ls -la "${C_OUTPUT}" || true
