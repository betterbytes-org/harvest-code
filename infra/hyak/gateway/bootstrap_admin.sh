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
    \"models\": [\"${VLLM_SERVED_NAME}\"]
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

# `/key/generate` with a pinned `key` value (LITELLM_VIRTUAL_KEY) is a no-op
# on the model scope if that key already exists in the DB -- Postgres data
# persists across job restarts, so a served-model switch (e.g.
# deepseek-v4-flash-0731 -> qwen3.8-27b) silently left this key scoped to the
# old model, and every check below failed with no useful error until this was
# root-caused by hand. Force the scope to match the current VLLM_SERVED_NAME
# on every bring-up, whether the key is new or pre-existing.
curl -sS "${BASE}/key/update" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d "{\"key\": \"${KEY}\", \"models\": [\"${VLLM_SERVED_NAME}\"]}" >/dev/null

if ! curl --max-time 30 -sf "${BASE}/v1/models" -H "Authorization: Bearer ${KEY}" | grep -q "${VLLM_SERVED_NAME}"; then
  echo "FAIL: virtual key ${KEY:0:12}... not scoped to ${VLLM_SERVED_NAME} after /key/update -- check ${BASE}/key/info?key=${KEY}" >&2
  exit 1
fi
CHAT_JSON=$(curl --max-time 120 -sf "${BASE}/v1/chat/completions" \
  -H "Authorization: Bearer ${KEY}" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"${VLLM_SERVED_NAME}\",\"messages\":[{\"role\":\"user\",\"content\":\"Say hi in one word\"}],\"max_tokens\":8}")
if ! echo "${CHAT_JSON}" | grep -q '"choices"'; then
  echo "FAIL: chat completion smoke test did not return choices: ${CHAT_JSON}" >&2
  exit 1
fi

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
