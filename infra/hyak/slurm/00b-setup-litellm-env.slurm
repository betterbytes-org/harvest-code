#!/bin/bash
#SBATCH --job-name=harvest-litellm-setup
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=4
#SBATCH --mem=8G
#SBATCH --time=01:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-litellm-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-litellm-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"

echo "=== Setup LiteLLM env on compute node $(hostname -f) ==="
echo "Date: $(date -u)"

hyak_harvest_load_modules
hyak_harvest_load_python_vllm

# Home dir is over quota; use scratch for pip builds and caches.
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PIP_CACHE_DIR="${HARVEST_ENVS}/pip-cache"
export HOME="${HARVEST_SCRATCH}/run-home"
mkdir -p "${TMPDIR}" "${PIP_CACHE_DIR}" "${HOME}"

bash "${HARVEST_INFRA}/gateway/install_litellm_env.sh"
