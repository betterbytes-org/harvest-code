#!/usr/bin/env bash
# Submit always-on DeepSeek-V4-Flash-0731 API pipeline.
set -euo pipefail

cd "$(dirname "$0")"
export SLURM_ACCOUNT="${SLURM_ACCOUNT:-gpu-h200-harvest}"
export HYAK_ACCOUNT="${SLURM_ACCOUNT}"

echo "=== Always-on DeepSeek-V4-Flash-0731 API ==="
echo "Prerequisites:"
echo "  cp env/secrets.env.example env/secrets.env   # set VLLM_API_KEY"
echo "  optional: cp env/rate-limit.env.example env/rate-limit.env"
echo ""

J00=$(sbatch --account="${SLURM_ACCOUNT}" --parsable slurm/00-setup-vllm-env.slurm)
J03=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J00}" \
  slurm/03-download-dsv4-flash.slurm)
J04=$(sbatch --account="${SLURM_ACCOUNT}" --parsable --dependency=afterok:"${J03}" \
  slurm/04-vllm-dsv4-flash.slurm)

echo "Pipeline:"
echo "  00 setup-vllm-env:  ${J00}"
echo "  03 download-model:  ${J03}  (after ${J00})"
echo "  04 vllm-server:     ${J04}  (after ${J03})"
echo ""
echo "Endpoint: state/vllm-endpoint.env (after ${J04} starts)"
echo "Monitor:  tail -f logs/vllm-dsv4-${J04}.out"
