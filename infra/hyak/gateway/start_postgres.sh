#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

_harvest_pg_bin_dir() {
  if command -v pg_ctl >/dev/null 2>&1; then
    dirname "$(command -v pg_ctl)"
    return 0
  fi
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
  if command -v pg_ctl >/dev/null 2>&1; then
    dirname "$(command -v pg_ctl)"
    return 0
  fi

  local pgconda="${HARVEST_PGCONDA:-${HARVEST_ENVS}/postgres}"
  if [[ -x "${pgconda}/bin/pg_ctl" ]]; then
    echo "${pgconda}/bin"
    return 0
  fi

  local conda="/mmfs1/sw/miniforge3/25.9.1-0/bin/conda"
  [[ -x "${conda}" ]] || conda="$(command -v conda || true)"
  if [[ -n "${conda}" && -x "${conda}" ]]; then
    export HOME="${HARVEST_SCRATCH}/run-home"
    export TMPDIR="${HARVEST_SCRATCH}/tmp"
    export CONDA_PKGS_DIRS="${HARVEST_SCRATCH}/conda-pkgs"
    mkdir -p "${HOME}" "${TMPDIR}" "${CONDA_PKGS_DIRS}" "${pgconda}"
    echo "Installing PostgreSQL via conda at ${pgconda} ..."
    "${conda}" create -y -p "${pgconda}" -c conda-forge postgresql=15
    echo "${pgconda}/bin"
    return 0
  fi

  echo "FAIL: pg_ctl not found. module load postgresql or install via conda." >&2
  return 1
}

export PATH="$(_harvest_pg_bin_dir):${PATH}"
PGCTL="$(command -v pg_ctl)"
cat > "${HARVEST_STATE}/postgres.path" <<EOF
export PATH="${PATH}"
EOF

mkdir -p "${HARVEST_PGDATA}"
PIDFILE="${HARVEST_STATE}/postgres.pid"
LOG="${HARVEST_LOGS}/postgres-${SLURM_JOB_ID:-local}.log"

if [[ ! -f "${HARVEST_PGDATA}/PG_VERSION" ]]; then
  initdb -D "${HARVEST_PGDATA}" --auth=trust --encoding=UTF8
  cat >> "${HARVEST_PGDATA}/postgresql.conf" <<EOF
listen_addresses = '127.0.0.1'
port = ${HARVEST_PGPORT}
EOF
fi

if [[ -f "${HARVEST_PGDATA}/postmaster.pid" ]]; then
  PG_PID="$(head -1 "${HARVEST_PGDATA}/postmaster.pid")"
  if kill -0 "${PG_PID}" 2>/dev/null; then
    echo "PostgreSQL already running (PID ${PG_PID})"
  else
    rm -f "${HARVEST_PGDATA}/postmaster.pid"
    "${PGCTL}" -D "${HARVEST_PGDATA}" -l "${LOG}" -o "-p ${HARVEST_PGPORT}" start
    [[ -f "${HARVEST_PGDATA}/postmaster.pid" ]] && head -1 "${HARVEST_PGDATA}/postmaster.pid" > "${PIDFILE}"
  fi
else
  "${PGCTL}" -D "${HARVEST_PGDATA}" -l "${LOG}" -o "-p ${HARVEST_PGPORT}" start
  [[ -f "${HARVEST_PGDATA}/postmaster.pid" ]] && head -1 "${HARVEST_PGDATA}/postmaster.pid" > "${PIDFILE}"
fi

# Wait for readiness
for _ in $(seq 1 30); do
  if pg_isready -h 127.0.0.1 -p "${HARVEST_PGPORT}" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
pg_isready -h 127.0.0.1 -p "${HARVEST_PGPORT}"

# Role/database match DATABASE_URL in common.env.
# Trust auth (--auth=trust) is safe only on node-local 127.0.0.1 within a Slurm allocation
# (no cross-node exposure; each job gets an isolated compute node).
createuser -h 127.0.0.1 -p "${HARVEST_PGPORT}" litellm 2>/dev/null || true
createdb -h 127.0.0.1 -p "${HARVEST_PGPORT}" -O litellm litellm 2>/dev/null || true
psql -h 127.0.0.1 -p "${HARVEST_PGPORT}" -d postgres -c "ALTER DATABASE litellm OWNER TO litellm" 2>/dev/null || true

export DATABASE_URL="postgresql://litellm@127.0.0.1:${HARVEST_PGPORT}/litellm"
echo "DATABASE_URL=${DATABASE_URL}"
