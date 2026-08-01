#!/usr/bin/env bash
# Called on SIGUSR1 (5 min before timeout) to chain a new server job.
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
SLURM_DIR="${HARVEST_ROOT}/infra/hyak/slurm"
export SLURM_ACCOUNT="${SLURM_ACCOUNT:-gpu-h200-harvest}"

echo "$(date -u): Requeue/resubmit vLLM server..."
sbatch --dependency=afterany:"${SLURM_JOB_ID}" \
  --account="${SLURM_ACCOUNT}" \
  "${SLURM_DIR}/04-vllm-dsv4-flash.slurm"
