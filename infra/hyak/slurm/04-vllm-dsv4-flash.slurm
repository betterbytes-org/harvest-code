#!/bin/bash
#SBATCH --job-name=harvest-vllm-dsv4
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=16
#SBATCH --mem=128G
#SBATCH --time=7-00:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"

echo "=== vLLM DeepSeek-V4-Flash server on $(hostname -f) ==="
echo "Date: $(date -u)"
echo "Job ID: ${SLURM_JOB_ID}"
nvidia-smi -L

module load cuda/12.4.1 2>/dev/null || module load cuda/12.6.3 2>/dev/null || true

bash "${HARVEST_INFRA}/scripts/start_vllm_background.sh"
