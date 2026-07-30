#!/usr/bin/env bash
# Submit HARVEST Hyak jobs in order. All work runs on compute nodes only.
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
SLURM_DIR="${HARVEST_ROOT}/infra/hyak/slurm"
export SLURM_ACCOUNT="${SLURM_ACCOUNT:-u_hyak_harvest}"

submit() {
  local script="$1"
  echo "Submitting ${script}..."
  sbatch --account="${SLURM_ACCOUNT}" --parsable "${SLURM_DIR}/${script}"
}

echo "=== HARVEST Hyak job chain (compute nodes only) ==="
echo "Account: ${SLURM_ACCOUNT}"
echo ""

J01=$(submit "01-gpu-smoke-test.slurm")
J02=$(submit "02-build-harvest.slurm")
J00=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J01}" \
  "${SLURM_DIR}/00-setup-vllm-env.slurm")
J03=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J00}" \
  "${SLURM_DIR}/03-download-dsv4-flash.slurm")
J04=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J03}" \
  "${SLURM_DIR}/04-vllm-dsv4-flash.slurm")

echo ""
echo "Job IDs:"
echo "  01 gpu-smoke-test:  ${J01}"
echo "  02 build-harvest:   ${J02}  (parallel)"
echo "  00 setup-vllm-env:  ${J00}  (after ${J01})"
echo "  03 download-model:  ${J03}  (after ${J00})"
echo "  04 vllm-server:     ${J04}  (after ${J03}, long-running)"
echo ""
echo "After job 04 writes infra/hyak/state/vllm-endpoint.env, submit:"
echo "  sbatch --dependency=after:${J04} ${SLURM_DIR}/05-test-structured-output.slurm"
echo "  sbatch --dependency=after:${J04} ${SLURM_DIR}/06-harvest-translate-test.slurm"
echo ""
echo "Monitor: squeue -u \$USER"
echo "Logs:    ${HARVEST_ROOT}/infra/hyak/logs/"
