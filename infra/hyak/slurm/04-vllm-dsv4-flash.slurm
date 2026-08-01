#!/bin/bash
#SBATCH --job-name=harvest-vllm-dsv4
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=16
#SBATCH --mem=128G
#SBATCH --time=7-00:00:00
#SBATCH --requeue
#SBATCH --signal=B:USR1@300
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"

trap 'bash "${HARVEST_INFRA}/scripts/resubmit_vllm.sh"' USR1

echo "=== vLLM DeepSeek-V4-Flash-0731 server on $(hostname -f) ==="
echo "Date: $(date -u)"
echo "Job ID: ${SLURM_JOB_ID}"
echo "Model: ${VLLM_MODEL}"
nvidia-smi -L

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"
hyak_harvest_load_modules

bash "${HARVEST_INFRA}/scripts/start_vllm_background.sh"
