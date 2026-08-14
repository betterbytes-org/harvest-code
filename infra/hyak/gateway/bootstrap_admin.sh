#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"
[[ -f "${HARVEST_INFRA}/state/gateway-endpoint.env" ]] && source "${HARVEST_INFRA}/state/gateway-endpoint.env"

# Same-node bring-up must use loopback; GATEWAY_HOST is the compute hostname for clients.
PORT="${LITELLM_PORT:-4000}"
BASE="http://127.0.0.1:${PORT}"
OUT="${HARVEST_STATE}/virtual-keys.json"

ready=0
for _ in $(seq 1 30); do
  if curl -sf "${BASE}/health/readiness" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
if [[ "${ready}" -ne 1 ]]; then
  echo "FAIL: gateway not reachable at ${BASE}" >&2
  exit 1
fi

ALIAS="${LITELLM_KEY_ALIAS:-harvest-external}"
KEY_JSON_FIELD=""
if [[ -n "${LITELLM_VIRTUAL_KEY:-}" ]]; then
  KEY_JSON_FIELD=", \"key\": \"${LITELLM_VIRTUAL_KEY}\""
fi

RESP=$(curl -sS "${BASE}/key/generate" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d "{
    \"key_alias\": \"${ALIAS}\",
    \"team_id\": null,
    \"max_budget\": 1000000,
    \"rpm_limit\": ${LITELLM_DEFAULT_RPM:-60},
    \"tpm_limit\": ${LITELLM_DEFAULT_TPM:-100000},
    \"models\": [\"deepseek-v4-flash-0731\"]
    ${KEY_JSON_FIELD}
  }")

echo "${RESP}" | python3 -m json.tool > "${OUT}" || echo "${RESP}" > "${OUT}"
KEY=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('key','') or '')" "${OUT}" 2>/dev/null || true)
if [[ -z "${KEY}" ]]; then
  KEY="${LITELLM_VIRTUAL_KEY:-}"
fi
if [[ -z "${KEY}" ]]; then
  echo "FAIL: no virtual key in ${OUT}" >&2
  echo "${RESP}" >&2
  exit 1
fi

curl --max-time 30 -sf "${BASE}/v1/models" -H "Authorization: Bearer ${KEY}" | grep -q deepseek-v4-flash-0731
CHAT_JSON=$(curl --max-time 120 -sf "${BASE}/v1/chat/completions" \
  -H "Authorization: Bearer ${KEY}" \
  -H "Content-Type: application/json" \
  -d '{"model":"deepseek-v4-flash-0731","messages":[{"role":"user","content":"Say hi in one word"}],"max_tokens":8}')
echo "${CHAT_JSON}" | grep -q '"choices"'

python3 - <<PY
import json
path = "${OUT}"
try:
    data = json.load(open(path))
except Exception:
    data = {}
data["key"] = """${KEY}"""
data["key_alias"] = "${ALIAS}"
json.dump(data, open(path, "w"), indent=2)
print("wrote", path)
PY

echo "Virtual key written to ${OUT}"
echo "API base: ${BASE}/v1"
echo "Key alias: ${ALIAS}"
echo "Distribute the key field to developers; do not commit this file."
