#!/usr/bin/env bash
# Print attach command for the HARVEST interactive compute session.
set -euo pipefail

STATE="/gscratch/harvest/rithvik/harvest/infra/hyak/state/interactive.env"

if [[ ! -f "${STATE}" ]]; then
  echo "No interactive session found." >&2
  echo "Start one with:" >&2
  echo "  sbatch /gscratch/harvest/rithvik/harvest/infra/hyak/slurm/07-interactive-compute.slurm" >&2
  exit 1
fi

# shellcheck source=/dev/null
source "${STATE}"

JOB="${SLURM_JOB_ID:-}"
if [[ -n "${JOB}" ]] && squeue -j "${JOB}" -h 2>/dev/null | grep -q .; then
  echo "Session is RUNNING (job ${JOB} on ${NODE})"
  echo ""
  echo "Attach:"
  echo "  srun --account=gpu-h200-harvest --jobid=${JOB} --overlap --pty tmux attach -t ${TMUX_SESSION}"
  echo ""
  echo "Plain shell:"
  echo "  srun --account=gpu-h200-harvest --jobid=${JOB} --overlap --pty bash -l"
else
  echo "Session file exists but job ${JOB} is not running." >&2
  echo "Re-submit:" >&2
  echo "  sbatch /gscratch/harvest/rithvik/harvest/infra/hyak/slurm/07-interactive-compute.slurm" >&2
  exit 1
fi
