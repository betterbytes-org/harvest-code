#!/bin/bash
#SBATCH --job-name=harvest-interactive
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:2
#SBATCH --cpus-per-task=16
#SBATCH --mem=128G
#SBATCH --time=7-00:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/interactive-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/interactive-%j.err

set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
STATE_FILE="${HARVEST_INFRA}/state/interactive.env"
TMUX_SESSION="harvest-interactive"

# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"

NODE="$(hostname -s)"
JOB_ID="${SLURM_JOB_ID}"

mkdir -p "${HARVEST_INFRA}/state"

cat >"${STATE_FILE}" <<EOF
# HARVEST interactive compute session — $(date -u +%Y-%m-%dT%H:%M:%SZ)
SLURM_JOB_ID=${JOB_ID}
SLURM_ACCOUNT=gpu-h200-harvest
NODE=${NODE}
GPUS=2x H200
TMUX_SESSION=${TMUX_SESSION}
WORKDIR=${HARVEST_ROOT}

# From Hyak login node, attach with:
srun --account=gpu-h200-harvest --jobid=${JOB_ID} --overlap --pty tmux attach -t ${TMUX_SESSION}

# Or open a plain shell on the same allocation:
srun --account=gpu-h200-harvest --jobid=${JOB_ID} --overlap --pty bash -l
EOF

export PATH="/gscratch/harvest/rithvik/bin:${PATH}"

# Welcome script runs inside tmux
cat >"${HARVEST_INFRA}/state/interactive-welcome.sh" <<'WELCOME'
#!/usr/bin/env bash
clear
echo "=============================================="
echo " HARVEST interactive compute session"
echo " Node:    $(hostname -f)"
echo " GPUs:    $(nvidia-smi -L 2>/dev/null | wc -l)x H200"
echo " Workdir: /gscratch/harvest/rithvik/harvest"
echo " gh:      $(command -v gh 2>/dev/null || echo 'not in PATH — export PATH=/gscratch/harvest/rithvik/bin:$PATH')"
echo "=============================================="
echo ""
echo "Quick commands:"
echo "  gh auth login              # authenticate GitHub CLI"
echo "  gh auth status"
echo "  nvidia-smi"
echo "  cd /gscratch/harvest/rithvik/harvest"
echo "  cd infra/hyak && ./submit-all.sh"
echo ""
echo "Detach tmux: Ctrl-b then d"
echo "=============================================="
export PATH="/gscratch/harvest/rithvik/bin:$PATH"
cd /gscratch/harvest/rithvik/harvest
exec bash -l
WELCOME
chmod +x "${HARVEST_INFRA}/state/interactive-welcome.sh"

tmux kill-session -t "${TMUX_SESSION}" 2>/dev/null || true
tmux new-session -d -s "${TMUX_SESSION}" "${HARVEST_INFRA}/state/interactive-welcome.sh"

echo "Interactive session ready: job=${JOB_ID} node=${NODE} tmux=${TMUX_SESSION}"
echo "Attach: srun --account=gpu-h200-harvest --jobid=${JOB_ID} --overlap --pty tmux attach -t ${TMUX_SESSION}"
cat "${STATE_FILE}"

# Hold the Slurm allocation open
sleep infinity
