#!/usr/bin/env bash
set -euo pipefail
source "${HARVEST_INFRA:?}/env/common.env"

PGCTL=""
for candidate in pg_ctl "${HARVEST_PGCONDA:-${HARVEST_ENVS}/postgres}/bin/pg_ctl"; do
  [[ -x "${candidate}" ]] && PGCTL="${candidate}" && break
done
if [[ -z "${PGCTL}" ]]; then
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
  PGCTL="$(command -v pg_ctl || true)"
fi

[[ -n "${PGCTL}" && -d "${HARVEST_PGDATA}" ]] && "${PGCTL}" -D "${HARVEST_PGDATA}" stop -m fast || true
rm -f "${HARVEST_STATE}/postgres.pid"
