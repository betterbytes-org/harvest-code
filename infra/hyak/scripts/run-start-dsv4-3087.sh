#!/usr/bin/env bash
# One-shot runner for subagent bc-82506ad5 — capture all outputs
set -uo pipefail
OUT="/tmp/hyak-dsv4-start-3087.out"
exec > >(tee "$OUT") 2>&1

echo "=== STEP 1 ==="
hostname
whoami
date
which squeue sbatch
squeue -u "$USER" -o "%.18i %.9P %.30j %.8u %.2t %.10M %.6D %R"

echo "=== STEP 2 ==="
squeue -A gpu-h200-harvest -o "%.18i %.9P %.30j %.8u %.2t %.10M %.6D %R" | head -40

echo "=== STEP 3 ==="
test -d /gscratch/harvest/rithvik/envs/vllm-dsv4 && echo VENV_OK
test -d /gscratch/harvest/rithvik/envs/litellm && echo LITELLM_OK
du -sh /gscratch/harvest/rithvik/models/DeepSeek-V4-Flash-0731 2>/dev/null | head -1
ls -la /gscratch/harvest/rithvik/harvest/infra/hyak/state/ 2>/dev/null
echo '--- vllm-endpoint.env ---'
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/vllm-endpoint.env 2>/dev/null
echo '--- gateway-endpoint.env ---'
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env 2>/dev/null

echo "=== STEP 4 CHECK ==="
if squeue -u "$USER" -o "%.30j %.2t" | grep -E 'harvest-vllm-dsv4.* (R|PD)'; then
  echo "SKIP: harvest-vllm-dsv4 already R or PD"
else
  echo "SUBMITTING job 04..."
  cd /gscratch/harvest/rithvik/harvest/infra/hyak
  env -u VLLM_PORT sbatch --account=gpu-h200-harvest slurm/04-vllm-dsv4-flash.slurm
  echo "sbatch exit: $?"
  sleep 2
  squeue -u "$USER" -o "%.18i %.9P %.30j %.8u %.2t %.10M %.6D %R"
fi

echo "=== DONE ==="
