#!/bin/bash
#SBATCH --job-name=harvest-dl-dsv4
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=8
#SBATCH --mem=64G
#SBATCH --time=08:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/download-dsv4-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/download-dsv4-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"

echo "=== Download DeepSeek-V4-Flash-0731 on $(hostname -f) ==="
echo "Date: $(date -u)"
echo "HF_HOME=${HF_HOME}"
echo "Model: ${VLLM_MODEL}"

# shellcheck source=/dev/null
source "${VLLM_VENV}/bin/activate"

MODEL_DIR="${HARVEST_MODELS}/DeepSeek-V4-Flash-0731"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"

huggingface-cli download "${VLLM_MODEL}" \
  --local-dir "${MODEL_DIR}" \
  --local-dir-use-symlinks False

du -sh "${MODEL_DIR}" "${HF_HOME}"
echo "=== Model download complete: ${MODEL_DIR} ==="
