#!/usr/bin/env bash
# Submit only the always-on vLLM + LiteLLM server (env + weights already on scratch).
set -euo pipefail
cd "$(dirname "$0")"
export SLURM_ACCOUNT="${SLURM_ACCOUNT:-gpu-h200-harvest}"
JOB=$(env -u VLLM_PORT sbatch --account="${SLURM_ACCOUNT}" --parsable slurm/04-vllm-dsv4-flash.slurm)
echo "Submitted vLLM+gateway job ${JOB}"
echo "Logs: logs/vllm-dsv4-${JOB}.out"
echo "When ready: cat state/gateway-endpoint.env && cat state/virtual-keys.json"
