#!/usr/bin/env bash
# Wait for vLLM to become ready on a compute node.
set -euo pipefail

HOST="${1:-127.0.0.1}"
PORT="${2:-8000}"
TIMEOUT="${3:-3600}"

deadline=$((SECONDS + TIMEOUT))
url="http://${HOST}:${PORT}/v1/models"
auth=()
if [[ -n "${VLLM_API_KEY:-}" ]]; then
  auth=(-H "Authorization: Bearer ${VLLM_API_KEY}")
fi

echo "Waiting for vLLM at ${url} (timeout ${TIMEOUT}s)..."
while (( SECONDS < deadline )); do
  if curl -sf "${url}" "${auth[@]}" >/dev/null 2>&1; then
    echo "vLLM ready at ${url}"
    exit 0
  fi
  sleep 15
done

echo "TIMEOUT: vLLM did not become ready within ${TIMEOUT}s" >&2
exit 1
