#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"

: "${LITELLM_MASTER_KEY:?LITELLM_MASTER_KEY required in secrets.env}"
: "${LITELLM_SALT_KEY:?LITELLM_SALT_KEY required in secrets.env}"
: "${DATABASE_URL:?DATABASE_URL required — run start_postgres.sh first}"

[[ -f "${HARVEST_STATE}/postgres.path" ]] && source "${HARVEST_STATE}/postgres.path"

# Home is over quota; prisma/litellm must not write there.
export HOME="${HARVEST_SCRATCH}/run-home"
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PYTHONUNBUFFERED=1
mkdir -p "${HOME}" "${TMPDIR}"
if [[ -x "${HARVEST_ENVS}/nodejs/bin/node" ]]; then
  export PATH="${HARVEST_ENVS}/nodejs/bin:${PATH}"
fi

export LITELLM_MASTER_KEY
export LITELLM_SALT_KEY
export LITELLM_UI_ACCESS_MODE="${LITELLM_UI_ACCESS_MODE:-admin_only}"

# shellcheck source=/dev/null
source "${LITELLM_VENV}/bin/activate"

_litellm_schema() {
  python -c "import litellm_proxy_extras, os; print(os.path.join(os.path.dirname(litellm_proxy_extras.__file__), 'schema.prisma'))"
}

_reset_litellm_db() {
  echo "WARN: resetting litellm database after stalled prisma migrate"
  dropdb -h 127.0.0.1 -p "${HARVEST_PGPORT}" litellm 2>/dev/null || true
  createdb -h 127.0.0.1 -p "${HARVEST_PGPORT}" -O litellm litellm
}

# Pre-migrate so the proxy does not hang forever on first boot (GPFS/prisma).
_schema="$(_litellm_schema)"
_migrated=0
if timeout 90 prisma migrate deploy --schema "${_schema}"; then
  _migrated=1
else
  _reset_litellm_db
  if timeout 90 prisma migrate deploy --schema "${_schema}"; then
    _migrated=1
  else
    echo "WARN: prisma migrate still incomplete; LiteLLM will retry on startup"
  fi
fi
if [[ "${_migrated}" -eq 1 ]]; then
  export DISABLE_SCHEMA_UPDATE=true
else
  export DISABLE_SCHEMA_UPDATE="${DISABLE_SCHEMA_UPDATE:-false}"
fi

LOG="${HARVEST_LOGS}/litellm-${SLURM_JOB_ID:-local}.log"
PIDFILE="${HARVEST_STATE}/litellm.pid"

if [[ -f "${PIDFILE}" ]]; then
  old_pid="$(cat "${PIDFILE}")"
  kill "${old_pid}" 2>/dev/null || true
  sleep 1
fi

echo "Starting LiteLLM on 0.0.0.0:${LITELLM_PORT} ..."
litellm --config "${LITELLM_CONFIG}" --host 0.0.0.0 --port "${LITELLM_PORT}" >>"${LOG}" 2>&1 &
echo $! > "${PIDFILE}"

ready=0
for _ in $(seq 1 300); do
  if curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" -ne 1 ]]; then
  echo "FAIL: LiteLLM did not become ready on port ${LITELLM_PORT} (see ${LOG})" >&2
  tail -n 50 "${LOG}" >&2 || true
  exit 1
fi
echo "LiteLLM ready on 0.0.0.0:${LITELLM_PORT} (UI: /ui)"
