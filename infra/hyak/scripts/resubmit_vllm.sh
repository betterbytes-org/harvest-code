#!/usr/bin/env bash
# Queue the next 04 at job start (afterany). Hyak compute nodes cannot
# sbatch (Invalid account or account/partition); submit from a login node.
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
SLURM_DIR="${HARVEST_ROOT}/infra/hyak/slurm"
ACCOUNT="gpu-h200-harvest"
PARTITION="gpu-h200"
SCRIPT="${SLURM_DIR}/04-vllm-dsv4-flash.slurm"
LOGIN="${HARVEST_SBATCH_LOGIN:-klone-login03}"

echo "$(date -u): Requeue/resubmit vLLM server..."

submit_cmd() {
  env -u VLLM_PORT -u SBATCH_ACCOUNT -u SBATCH_PARTITION \
    sbatch --dependency=afterany:"${SLURM_JOB_ID}" \
      --account="${ACCOUNT}" \
      --partition="${PARTITION}" \
      "${SCRIPT}"
}

if submit_cmd; then
  exit 0
fi

echo "WARN: local sbatch failed; retrying via ${LOGIN}"
ssh -o BatchMode=yes -o ConnectTimeout=15 "${LOGIN}" \
  "export HOME=/gscratch/harvest/rithvik/run-home
   export HISTFILE=/dev/null
   export TMPDIR=/gscratch/harvest/rithvik/tmp
   cd ${HARVEST_ROOT}/infra/hyak
   env -u VLLM_PORT sbatch --dependency=afterany:${SLURM_JOB_ID} --account=${ACCOUNT} --partition=${PARTITION} ${SCRIPT}"
