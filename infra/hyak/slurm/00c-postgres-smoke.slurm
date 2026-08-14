#!/bin/bash
#SBATCH --job-name=harvest-pg-smoke
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=2
#SBATCH --mem=4G
#SBATCH --time=00:10:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/pg-smoke-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/pg-smoke-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"

echo "=== PostgreSQL sidecar smoke test on $(hostname -f) ==="
echo "Date: $(date -u)"

# Use job-scoped pgdata to avoid clobbering a running instance
export HARVEST_PGDATA="${HARVEST_SCRATCH}/pgdata/litellm-smoke-${SLURM_JOB_ID}"
export HARVEST_PGPORT="5433"

bash "${HARVEST_INFRA}/gateway/start_postgres.sh"
# shellcheck source=/dev/null
source "${HARVEST_STATE}/postgres.path"
pg_isready -h 127.0.0.1 -p "${HARVEST_PGPORT}"

# Verify litellm role can connect (matches common.env DATABASE_URL)
psql "${DATABASE_URL}" -c "SELECT current_user, current_database();"

bash "${HARVEST_INFRA}/gateway/stop_postgres.sh"
rm -rf "${HARVEST_PGDATA}"

echo "=== Smoke test PASSED ==="
