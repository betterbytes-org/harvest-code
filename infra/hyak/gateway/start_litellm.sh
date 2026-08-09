#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"

: "${LITELLM_MASTER_KEY:?LITELLM_MASTER_KEY required in secrets.env}"
: "${LITELLM_SALT_KEY:?LITELLM_SALT_KEY required in secrets.env}"
: "${DATABASE_URL:?DATABASE_URL required — run start_postgres.sh first}"

# Preserve postgres client tools on PATH when start_postgres.sh ran in another step
[[ -f "${HARVEST_STATE}/postgres.path" ]] && source "${HARVEST_STATE}/postgres.path"

export LITELLM_MASTER_KEY
export LITELLM_SALT_KEY
export LITELLM_UI_ACCESS_MODE="${LITELLM_UI_ACCESS_MODE:-admin_only}"
export DISABLE_SCHEMA_UPDATE="${DISABLE_SCHEMA_UPDATE:-false}"

# shellcheck source=/dev/null
source "${LITELLM_VENV}/bin/activate"

LOG="${HARVEST_LOGS}/litellm-${SLURM_JOB_ID:-local}.log"
PIDFILE="${HARVEST_STATE}/litellm.pid"

if [[ -f "${PIDFILE}" ]]; then
  old_pid="$(cat "${PIDFILE}")"
  kill "${old_pid}" 2>/dev/null || true
fi

litellm --config "${LITELLM_CONFIG}" --host 0.0.0.0 --port "${LITELLM_PORT}" >>"${LOG}" 2>&1 &
echo $! > "${PIDFILE}"

# LiteLLM migrations can take 60-120s on first boot
for _ in $(seq 1 180); do
  if curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null
echo "LiteLLM ready on 0.0.0.0:${LITELLM_PORT} (UI: /ui)"
