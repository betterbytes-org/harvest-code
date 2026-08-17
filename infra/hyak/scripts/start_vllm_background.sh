#!/usr/bin/env bash
# Start vLLM in the background, wait for readiness, write endpoint file.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
# shellcheck source=/dev/null
source "${VLLM_VENV}/bin/activate"

# Optional secrets and rate-limit overrides
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/rate-limit.env" ]] && source "${HARVEST_INFRA}/env/rate-limit.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"

# Home dir is over quota; keep vLLM/HF caches on scratch.
export HOME="${HARVEST_SCRATCH}/run-home"
export XDG_CACHE_HOME="${HARVEST_SCRATCH}/cache"
export VLLM_CACHE_ROOT="${HARVEST_SCRATCH}/cache/vllm"
export HF_HOME="${HF_HOME:-${HARVEST_MODELS}/huggingface}"
mkdir -p "${HOME}" "${XDG_CACHE_HOME}" "${VLLM_CACHE_ROOT}"

if [[ "${GATEWAY_ENABLED}" == "true" ]]; then
  if [[ -z "${LITELLM_MASTER_KEY:-}" || -z "${LITELLM_SALT_KEY:-}" ]]; then
    echo "FAIL: LITELLM_MASTER_KEY and LITELLM_SALT_KEY required in secrets.env when GATEWAY_ENABLED=true" >&2
    exit 1
  fi
else
  if [[ -z "${VLLM_API_KEY:-}" ]]; then
    echo "FAIL: VLLM_API_KEY not set. Copy env/secrets.env.example to env/secrets.env" >&2
    exit 1
  fi
fi

PROXY_API_KEY="${VLLM_API_KEY:-}"  # kept for bootstrap compat; clients use virtual keys in litellm mode
# Auth + rate limits live on the proxy/gateway; internal vLLM stays localhost-only.
unset VLLM_API_KEY
# vLLM uses VLLM_PORT for NCCL rendezvous (default 8000 when API is on 8001).
unset VLLM_PORT

export CUDA_VISIBLE_DEVICES="${CUDA_VISIBLE_DEVICES:-0,1}"
export VLLM_ENGINE_READY_TIMEOUT_S="${VLLM_ENGINE_READY_TIMEOUT_S:-7200}"

NODE="$(hostname -s)"
ENDPOINT_FILE="${HARVEST_STATE}/vllm-endpoint.env"
GATEWAY_ENDPOINT_FILE="${HARVEST_STATE}/gateway-endpoint.env"
LOG_FILE="${HARVEST_LOGS}/vllm-${SLURM_JOB_ID:-local}.log"
PROXY_LOG="${HARVEST_LOGS}/vllm-proxy-${SLURM_JOB_ID:-local}.log"

vllm serve "${VLLM_MODEL}" \
  --served-model-name "${VLLM_SERVED_NAME}" \
  --host 127.0.0.1 \
  --port "${VLLM_INTERNAL_PORT}" \
  --tensor-parallel-size "${VLLM_TP_SIZE}" \
  --enable-expert-parallel \
  --trust-remote-code \
  --tokenizer-mode deepseek_v4 \
  --tool-call-parser deepseek_v4 \
  --enable-auto-tool-choice \
  --reasoning-parser deepseek_v4 \
  --kv-cache-dtype fp8 \
  --block-size 256 \
  --max-model-len "${VLLM_MAX_MODEL_LEN}" \
  --max-num-seqs 4 \
  --max-num-batched-tokens 8192 \
  --gpu-memory-utilization 0.92 \
  --enable-chunked-prefill \
  >>"${LOG_FILE}" 2>&1 &

VLLM_PID=$!
echo "${VLLM_PID}" >"${HARVEST_STATE}/vllm.pid"

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "127.0.0.1" "${VLLM_INTERNAL_PORT}" 7200

if [[ "${GATEWAY_ENABLED}" == "true" ]]; then
  export LITELLM_MASTER_KEY LITELLM_SALT_KEY
  # Never tear down vLLM if the gateway fails; retry until LiteLLM is healthy.
  if ! bash "${HARVEST_INFRA}/gateway/bring_up.sh"; then
    echo "WARN: LiteLLM gateway failed to start; retrying in background (vLLM stays up)" >&2
    (
      while true; do
        sleep 30
        bash "${HARVEST_INFRA}/gateway/bring_up.sh" && exit 0
      done
    ) >>"${HARVEST_LOGS}/gateway-retry-${SLURM_JOB_ID:-local}.log" 2>&1 &
  fi
  PUBLIC_PORT="${LITELLM_PORT}"
  GATEWAY_MODE="litellm"
  PROXY_PID=""
else
  export VLLM_API_KEY="${PROXY_API_KEY}"
  export VLLM_PROXY_PORT

  if [[ -f "${HARVEST_STATE}/vllm-proxy.pid" ]]; then
    old_proxy_pid="$(cat "${HARVEST_STATE}/vllm-proxy.pid" 2>/dev/null || true)"
    if [[ -n "${old_proxy_pid}" ]]; then
      kill "${old_proxy_pid}" 2>/dev/null || true
    fi
  fi

  python3 "${HARVEST_INFRA}/scripts/vllm_api_proxy.py" >>"${PROXY_LOG}" 2>&1 &
  PROXY_PID=$!
  echo "${PROXY_PID}" >"${HARVEST_STATE}/vllm-proxy.pid"

  # Proxy must be up before writing endpoint
  sleep 1
  curl -sf "http://127.0.0.1:${VLLM_PROXY_PORT}/v1/models" \
    -H "Authorization: Bearer ${VLLM_API_KEY}" >/dev/null
  PUBLIC_PORT="${VLLM_PROXY_PORT}"
  GATEWAY_MODE="stdlib-proxy"
fi

if [[ "${GATEWAY_MODE}" == "litellm" ]]; then
  if [[ -f "${GATEWAY_ENDPOINT_FILE}" ]]; then
    echo "vLLM ready (internal 127.0.0.1:${VLLM_INTERNAL_PORT}, LiteLLM 0.0.0.0:${PUBLIC_PORT})"
    echo "Endpoint: ${ENDPOINT_FILE}"
    echo "Gateway: ${GATEWAY_ENDPOINT_FILE}"
  else
    echo "vLLM ready (internal 127.0.0.1:${VLLM_INTERNAL_PORT}); LiteLLM not yet healthy"
  fi
  echo "vLLM PID ${VLLM_PID}"
else
  cat >"${ENDPOINT_FILE}" <<EOF
# Written by start_vllm_background.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
VLLM_HOST=${NODE}
VLLM_PORT=${PUBLIC_PORT}
VLLM_PROXY_PORT=${PUBLIC_PORT}
VLLM_ENDPOINT=http://${NODE}:${PUBLIC_PORT}/v1
VLLM_SERVED_NAME=${VLLM_SERVED_NAME}
SLURM_JOB_ID=${SLURM_JOB_ID:-}
VLLM_PID=${VLLM_PID}
PROXY_PID=${PROXY_PID}
GATEWAY_MODE=${GATEWAY_MODE}
RATE_LIMIT_ENABLED=${RATE_LIMIT_ENABLED}
RATE_LIMIT_REQUESTS_PER_MINUTE=${RATE_LIMIT_REQUESTS_PER_MINUTE}
RATE_LIMIT_BURST=${RATE_LIMIT_BURST}
# Clients: Authorization: Bearer \$VLLM_API_KEY
# OpenAI SDK: base_url=http://${NODE}:${PUBLIC_PORT}/v1  api_key=<key>  model=${VLLM_SERVED_NAME}
EOF

  echo "vLLM ready (internal 127.0.0.1:${VLLM_INTERNAL_PORT}, proxy 0.0.0.0:${PUBLIC_PORT})"
  echo "Endpoint: ${ENDPOINT_FILE}"
  echo "vLLM PID ${VLLM_PID}, proxy PID ${PROXY_PID}"
fi

# Keep the allocation alive serving requests.
wait "${VLLM_PID}"
