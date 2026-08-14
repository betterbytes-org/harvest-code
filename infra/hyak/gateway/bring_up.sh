#!/usr/bin/env bash
# Start Postgres + LiteLLM on the current node, write endpoint files, issue a virtual key.
# Safe to re-run. Does not start vLLM.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"

export HOME="${HARVEST_SCRATCH}/run-home"
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PYTHONUNBUFFERED=1
mkdir -p "${HOME}" "${TMPDIR}" "${HARVEST_STATE}" "${HARVEST_LOGS}"

bash "${HARVEST_INFRA}/gateway/start_postgres.sh"
DATABASE_URL="$(cat "${HARVEST_STATE}/database.url")"
export DATABASE_URL

bash "${HARVEST_INFRA}/gateway/start_litellm.sh"

NODE="$(hostname -s)"
PUBLIC_PORT="${LITELLM_PORT}"
VLLM_PID="$(cat "${HARVEST_STATE}/vllm.pid" 2>/dev/null || true)"
PROXY_PID="$(cat "${HARVEST_STATE}/litellm.pid" 2>/dev/null || true)"

cat >"${HARVEST_STATE}/vllm-endpoint.env" <<EOF
# Written by gateway/bring_up.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
VLLM_HOST=${NODE}
VLLM_PORT=${PUBLIC_PORT}
VLLM_PROXY_PORT=${PUBLIC_PORT}
VLLM_ENDPOINT=http://${NODE}:${PUBLIC_PORT}/v1
VLLM_SERVED_NAME=${VLLM_SERVED_NAME}
SLURM_JOB_ID=${SLURM_JOB_ID:-}
VLLM_PID=${VLLM_PID}
PROXY_PID=${PROXY_PID}
GATEWAY_MODE=litellm
RATE_LIMIT_ENABLED=${RATE_LIMIT_ENABLED}
RATE_LIMIT_REQUESTS_PER_MINUTE=${RATE_LIMIT_REQUESTS_PER_MINUTE}
RATE_LIMIT_BURST=${RATE_LIMIT_BURST}
# Clients: Authorization: Bearer <virtual-key>
# OpenAI SDK: base_url=http://${NODE}:${PUBLIC_PORT}/v1  api_key=<virtual-key>  model=${VLLM_SERVED_NAME}
EOF

cat >"${HARVEST_STATE}/gateway-endpoint.env" <<EOF
# Written by gateway/bring_up.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
GATEWAY_HOST=${NODE}
LITELLM_PORT=${PUBLIC_PORT}
LITELLM_UI_URL=http://${NODE}:${PUBLIC_PORT}/ui
GATEWAY_MODE=litellm
SLURM_JOB_ID=${SLURM_JOB_ID:-}
# From a laptop: ssh -L ${PUBLIC_PORT}:${NODE}:${PUBLIC_PORT} USER@hyak.uw.edu
# Then API at http://localhost:${PUBLIC_PORT}/v1  UI at http://localhost:${PUBLIC_PORT}/ui
# From a Hyak login node: curl http://${NODE}:${PUBLIC_PORT}/health/readiness
EOF

bash "${HARVEST_INFRA}/gateway/bootstrap_admin.sh"
bash "${HARVEST_INFRA}/gateway/start_public_tunnel.sh"

echo "Gateway ready: http://${NODE}:${PUBLIC_PORT}/v1"
if [[ -f "${HARVEST_STATE}/public-url.env" ]]; then
  # shellcheck source=/dev/null
  source "${HARVEST_STATE}/public-url.env"
  echo "Public API:    ${PUBLIC_API}"
  echo "Public UI:     ${PUBLIC_UI}"
fi
echo "Virtual key:   ${HARVEST_STATE}/virtual-keys.json"
