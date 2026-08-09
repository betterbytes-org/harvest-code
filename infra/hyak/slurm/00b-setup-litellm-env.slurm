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
hyak_harvest_load_modules
bash "${HARVEST_INFRA}/gateway/install_litellm_env.sh"
