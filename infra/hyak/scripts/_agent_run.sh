#!/bin/bash
set -eu
OUT=/gscratch/harvest/rithvik/harvest/infra/hyak/state/agent-run.out
exec >"$OUT" 2>&1
export HOME=/gscratch/harvest/rithvik/run-home
export HISTFILE=/dev/null
mkdir -p "$HOME"
echo START "$(date -u)"
hostname
whoami
command -v sbatch
command -v squeue
squeue -u "$(whoami)" -o '%.18i %.9P %.30j %.8u %.2t %.10M %.6D %R' || true
cd /gscratch/harvest/rithvik/harvest/infra/hyak
JOB=$(env -u VLLM_PORT sbatch --account=gpu-h200-harvest --parsable slurm/04-vllm-dsv4-flash.slurm)
echo "SUBMITTED $JOB"
echo DONE "$(date -u)"
