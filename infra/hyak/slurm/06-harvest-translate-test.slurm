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
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"

C_INPUT="${C_INPUT:-${HARVEST_SCRATCH}/testcases/hello}"
C_OUTPUT="${C_OUTPUT:-${HARVEST_SCRATCH}/translate-output/hello}"
mkdir -p "$(dirname "${C_OUTPUT}")"

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

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"
hyak_harvest_load_llvm
module load cmake/3.31.2 2>/dev/null || module load cmake 2>/dev/null || true
export PATH="${CARGO_HOME}/bin:${PATH}"

LLM_CFG=(
  "tools.raw_source_to_cargo_llm.backend=openai"
  "tools.raw_source_to_cargo_llm.address=${VLLM_ENDPOINT}"
  "tools.raw_source_to_cargo_llm.model=${VLLM_SERVED_NAME}"
  "tools.raw_source_to_cargo_llm.api_key=${VLLM_API_KEY}"
  "tools.modular_translation_llm.backend=openai"
  "tools.modular_translation_llm.address=${VLLM_ENDPOINT}"
  "tools.modular_translation_llm.model=${VLLM_SERVED_NAME}"
  "tools.modular_translation_llm.api_key=${VLLM_API_KEY}"
  "tools.fix_declarations_llm.backend=openai"
  "tools.fix_declarations_llm.address=${VLLM_ENDPOINT}"
  "tools.fix_declarations_llm.model=${VLLM_SERVED_NAME}"
  "tools.fix_declarations_llm.api_key=${VLLM_API_KEY}"
  "tools.generate_exec_difftests.backend=openai"
  "tools.generate_exec_difftests.address=${VLLM_ENDPOINT}"
  "tools.generate_exec_difftests.model=${VLLM_SERVED_NAME}"
  "tools.generate_exec_difftests.api_key=${VLLM_API_KEY}"
  "tools.generate_difftest_suite.backend=openai"
  "tools.generate_difftest_suite.address=${VLLM_ENDPOINT}"
  "tools.generate_difftest_suite.model=${VLLM_SERVED_NAME}"
  "tools.generate_difftest_suite.api_key=${VLLM_API_KEY}"
  "tools.fix_diff_failures.backend=openai"
  "tools.fix_diff_failures.address=${VLLM_ENDPOINT}"
  "tools.fix_diff_failures.model=${VLLM_SERVED_NAME}"
  "tools.fix_diff_failures.api_key=${VLLM_API_KEY}"
)

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "${VLLM_HOST}" "${VLLM_PROXY_PORT:-${VLLM_PORT}}" 600

cd "${HARVEST_ROOT}"
ARGS=()
for kv in "${LLM_CFG[@]}"; do ARGS+=(--config "$kv"); done
"${HARVEST_ROOT}/target/release/translate" "${ARGS[@]}" \
  "${C_INPUT}" -o "${C_OUTPUT}" --force

echo "=== Translation output in ${C_OUTPUT} ==="
ls -la "${C_OUTPUT}" || true
