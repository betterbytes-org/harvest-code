#!/usr/bin/env bash
# Submit DeepSeek V4 Flash pipeline only (no build, no interactive shell).
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
SLURM_DIR="${HARVEST_ROOT}/infra/hyak/slurm"
unset SLURM_ACCOUNT HYAK_ACCOUNT SALLOC_ACCOUNT
export SLURM_ACCOUNT="gpu-h200-harvest"

submit() {
  sbatch --account="${SLURM_ACCOUNT}" --parsable "${SLURM_DIR}/$1"
}

echo "=== DeepSeek V4 Flash pipeline (gpu-h200-harvest, 2x H200) ==="

J00=$(submit "00-setup-vllm-env.slurm")
J03=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J00}" \
  "${SLURM_DIR}/03-download-dsv4-flash.slurm")
J04=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J03}" \
  "${SLURM_DIR}/04-vllm-dsv4-flash.slurm")
J05=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=after:"${J04}" \
  "${SLURM_DIR}/05-test-structured-output.slurm")

echo ""
echo "Jobs:"
echo "  00 setup-vllm-env:  ${J00}"
echo "  03 download-model:  ${J03}  (after ${J00})"
echo "  04 vllm-server:     ${J04}  (after ${J03})"
echo "  05 llm-test:         ${J05}  (after ${J04} starts)"
echo ""
echo "Monitor:  squeue -u \$USER"
echo "Logs:     ${HARVEST_ROOT}/infra/hyak/logs/"
echo "Endpoint: ${HARVEST_ROOT}/infra/hyak/state/vllm-endpoint.env  (after job 04)"
