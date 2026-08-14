#!/usr/bin/env bash
# Expose LiteLLM on a public HTTPS URL via Cloudflare quick tunnel.
# Must run on a host that can reach the gateway (compute node: 127.0.0.1, or login: GATEWAY_HOST).
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"
[[ -f "${HARVEST_STATE}/gateway-endpoint.env" ]] && source "${HARVEST_STATE}/gateway-endpoint.env"

export HOME="${HARVEST_SCRATCH}/run-home"
export TMPDIR="${HARVEST_SCRATCH}/tmp"
mkdir -p "${HOME}" "${TMPDIR}" "${HARVEST_ENVS}/bin" "${HARVEST_LOGS}" "${HARVEST_STATE}"

TARGET="${PUBLIC_TUNNEL_TARGET:-http://127.0.0.1:${LITELLM_PORT:-4000}}"
BIN="${HARVEST_ENVS}/bin/cloudflared"
LOG="${HARVEST_LOGS}/cloudflared-${SLURM_JOB_ID:-local}.log"
PIDFILE="${HARVEST_STATE}/cloudflared.pid"
OUT="${HARVEST_STATE}/public-url.env"

if [[ ! -x "${BIN}" ]]; then
  echo "Downloading cloudflared ..."
  curl -fsSL -o "${BIN}" \
    "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64"
  chmod +x "${BIN}"
fi

if [[ -f "${PIDFILE}" ]]; then
  kill "$(cat "${PIDFILE}")" 2>/dev/null || true
  rm -f "${PIDFILE}"
fi
: > "${LOG}"

"${BIN}" tunnel --no-autoupdate --url "${TARGET}" >>"${LOG}" 2>&1 &
echo $! > "${PIDFILE}"

URL=""
for _ in $(seq 1 60); do
  URL="$(grep -oE 'https://[a-z0-9-]+\.trycloudflare.com' "${LOG}" 2>/dev/null | head -1 || true)"
  if [[ -n "${URL}" ]]; then
    break
  fi
  sleep 1
done
if [[ -z "${URL}" ]]; then
  echo "FAIL: cloudflared did not print a public URL (see ${LOG})" >&2
  tail -n 40 "${LOG}" >&2 || true
  exit 1
fi

cat >"${OUT}" <<EOF
# Written by start_public_tunnel.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
PUBLIC_URL=${URL}
PUBLIC_API=${URL}/v1
PUBLIC_UI=${URL}/ui
PUBLIC_TUNNEL_TARGET=${TARGET}
EOF

if [[ -f "${HARVEST_STATE}/gateway-endpoint.env" ]]; then
  grep -v '^PUBLIC_' "${HARVEST_STATE}/gateway-endpoint.env" > "${HARVEST_STATE}/gateway-endpoint.env.tmp" || true
  cat "${HARVEST_STATE}/gateway-endpoint.env.tmp" "${OUT}" > "${HARVEST_STATE}/gateway-endpoint.env"
  rm -f "${HARVEST_STATE}/gateway-endpoint.env.tmp"
fi

echo "Public URL: ${URL}"
echo "API:        ${URL}/v1"
echo "Dashboard:  ${URL}/ui"
