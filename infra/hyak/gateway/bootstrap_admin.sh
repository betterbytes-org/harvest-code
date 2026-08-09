#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
source "${HARVEST_INFRA}/env/secrets.env"
source "${HARVEST_INFRA}/state/gateway-endpoint.env"

BASE="http://${GATEWAY_HOST}:${LITELLM_PORT}"
OUT="${HARVEST_STATE}/virtual-keys.json"

# Create team key for harvest-devs (max budget: unlimited for internal use)
RESP=$(curl -sf "${BASE}/key/generate" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "key_alias": "harvest-devs",
    "team_id": null,
    "max_budget": 1000000,
    "rpm_limit": 60,
    "tpm_limit": 100000,
    "models": ["deepseek-v4-flash-0731"]
  }')

echo "${RESP}" | python3 -m json.tool > "${OUT}"
echo "Virtual key written to ${OUT}"
echo "Distribute key to developers; do not commit this file."
