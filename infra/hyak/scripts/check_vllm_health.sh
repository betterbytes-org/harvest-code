#!/usr/bin/env bash
set -euo pipefail

HARVEST_INFRA="/gscratch/harvest/rithvik/harvest/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_STATE}/vllm-endpoint.env" ]] && source "${HARVEST_STATE}/vllm-endpoint.env"

if [[ -z "${VLLM_API_KEY:-}" ]]; then
  echo "FAIL: VLLM_API_KEY not set" >&2
  exit 1
fi

URL="http://${VLLM_HOST:-127.0.0.1}:${VLLM_PORT:-8000}/v1/models"
if curl -sf "${URL}" -H "Authorization: Bearer ${VLLM_API_KEY}" >/dev/null; then
  echo "OK: vLLM proxy healthy at ${URL}"
  exit 0
fi

echo "FAIL: vLLM proxy unreachable at ${URL}" >&2
exit 1
