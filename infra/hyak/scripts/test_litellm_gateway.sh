#!/usr/bin/env bash
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/secrets.env"

ENDPOINT="${HARVEST_INFRA}/state/gateway-endpoint.env"
[[ -f "${ENDPOINT}" ]] || { echo "FAIL: ${ENDPOINT} missing"; exit 1; }
# shellcheck source=/dev/null
source "${ENDPOINT}"

BASE="http://${GATEWAY_HOST}:${LITELLM_PORT}"

curl -sf "${BASE}/health/readiness" >/dev/null
echo "PASS: health/readiness"

curl -sf "${BASE}/v1/models" -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" | grep -q deepseek-v4-flash-0731
echo "PASS: /v1/models"

curl -sf "${BASE}/v1/chat/completions" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"model":"deepseek-v4-flash-0731","messages":[{"role":"user","content":"Say hi in one word"}],"max_tokens":8}' \
  | grep -q '"content"'
echo "PASS: chat completion"

curl -sf "${BASE}/user/daily/activity" -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" >/dev/null
echo "PASS: usage stats endpoint"
