#!/bin/bash
#SBATCH --job-name=harvest-litellm-smoke
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=4
#SBATCH --mem=8G
#SBATCH --time=00:15:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/litellm-smoke-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/litellm-smoke-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"

echo "=== LiteLLM smoke test on $(hostname -f) ==="
echo "Date: $(date -u)"

# Job-scoped pgdata to avoid clobbering a running instance
export HARVEST_PGDATA="${HARVEST_SCRATCH}/pgdata/litellm-smoke-${SLURM_JOB_ID}"
export HARVEST_PGPORT="5433"
export LITELLM_PORT="4000"

# Smoke keys (override if secrets.env already defines them)
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
export LITELLM_MASTER_KEY="${LITELLM_MASTER_KEY:-sk-smoke-master-key-litellm-test}"
export LITELLM_SALT_KEY="${LITELLM_SALT_KEY:-sk-smoke-salt-key-litellm-test}"

bash "${HARVEST_INFRA}/gateway/start_postgres.sh"
export DATABASE_URL="postgresql://litellm@127.0.0.1:${HARVEST_PGPORT}/litellm"

bash "${HARVEST_INFRA}/gateway/start_litellm.sh"

curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness"
curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/liveliness" || true

if [[ -f "${HARVEST_STATE}/litellm.pid" ]]; then
  kill "$(cat "${HARVEST_STATE}/litellm.pid")" 2>/dev/null || true
fi
bash "${HARVEST_INFRA}/gateway/stop_postgres.sh"
rm -rf "${HARVEST_PGDATA}"

echo "=== Smoke test PASSED ==="
