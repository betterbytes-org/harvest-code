#!/bin/bash
#SBATCH --job-name=harvest-vllm-dsv4
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=16
#SBATCH --mem=128G
#SBATCH --time=7-00:00:00
#SBATCH --requeue
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.err

set -euo pipefail
export PYTHONUNBUFFERED=1

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
export VLLM_ALLOW_LONG_MAX_MODEL_LEN=1
export VLLM_MAX_MODEL_LEN="${VLLM_MAX_MODEL_LEN:-1010000}"

# Queue the next 04 now. USR1-at-timeout missed last time (job 38886754
# TIMED OUT with no successor). afterany starts the next job when this one
# ends. scancel the pending 04 too if you intend to stop the API.
bash "${HARVEST_INFRA}/scripts/resubmit_vllm.sh" || echo "WARN: successor sbatch failed"

echo "=== vLLM Qwen3.8-27B 1M server on $(hostname -f) ==="
echo "Date: $(date -u)"
echo "Job ID: ${SLURM_JOB_ID}"
echo "Model: ${VLLM_MODEL}"
nvidia-smi -L

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"
hyak_harvest_load_modules

bash "${HARVEST_INFRA}/scripts/start_vllm_background.sh"
