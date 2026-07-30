#!/bin/bash
#SBATCH --job-name=harvest-llm-test
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=4
#SBATCH --mem=8G
#SBATCH --time=02:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/structured-test-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/structured-test-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

ENDPOINT_FILE="${HARVEST_STATE}/vllm-endpoint.env"
if [[ ! -f "${ENDPOINT_FILE}" ]]; then
  echo "FAIL: ${ENDPOINT_FILE} not found. Start job 04-vllm-dsv4-flash.slurm first." >&2
  exit 1
fi

# shellcheck source=/dev/null
source "${ENDPOINT_FILE}"

echo "=== Structured output test on $(hostname -f) ==="
echo "Target: ${VLLM_ENDPOINT}"

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "${VLLM_HOST}" "${VLLM_PORT}" 7200
python3 "${HARVEST_INFRA}/scripts/test_structured_output.py"
echo "=== Structured output test PASSED ==="
