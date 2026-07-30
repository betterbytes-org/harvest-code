#!/usr/bin/env bash
# Start vLLM in the background, wait for readiness, write endpoint file.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
# shellcheck source=/dev/null
source "${VLLM_VENV}/bin/activate"

export CUDA_VISIBLE_DEVICES="${CUDA_VISIBLE_DEVICES:-0,1}"
export VLLM_ENGINE_READY_TIMEOUT_S="${VLLM_ENGINE_READY_TIMEOUT_S:-7200}"

NODE="$(hostname -s)"
ENDPOINT_FILE="${HARVEST_STATE}/vllm-endpoint.env"
LOG_FILE="${HARVEST_LOGS}/vllm-${SLURM_JOB_ID:-local}.log"

vllm serve "${VLLM_MODEL}" \
  --served-model-name "${VLLM_SERVED_NAME}" \
  --host 0.0.0.0 \
  --port "${VLLM_PORT}" \
  --tensor-parallel-size "${VLLM_TP_SIZE}" \
  --enable-expert-parallel \
  --trust-remote-code \
  --tokenizer-mode deepseek_v4 \
  --tool-call-parser deepseek_v4 \
  --enable-auto-tool-choice \
  --reasoning-parser deepseek_v4 \
  --kv-cache-dtype fp8 \
  --block-size 256 \
  --max-model-len 32768 \
  --max-num-seqs 4 \
  --max-num-batched-tokens 8192 \
  --gpu-memory-utilization 0.92 \
  --enable-chunked-prefill \
  >>"${LOG_FILE}" 2>&1 &

VLLM_PID=$!
echo "${VLLM_PID}" >"${HARVEST_STATE}/vllm.pid"

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "127.0.0.1" "${VLLM_PORT}" 7200

cat >"${ENDPOINT_FILE}" <<EOF
# Written by start_vllm_background.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
VLLM_HOST=${NODE}
VLLM_PORT=${VLLM_PORT}
VLLM_ENDPOINT=http://${NODE}:${VLLM_PORT}/v1
VLLM_SERVED_NAME=${VLLM_SERVED_NAME}
SLURM_JOB_ID=${SLURM_JOB_ID:-}
VLLM_PID=${VLLM_PID}
EOF

echo "vLLM ready: ${ENDPOINT_FILE}"
echo "PID ${VLLM_PID}, log ${LOG_FILE}"

# Keep the allocation alive serving requests.
wait "${VLLM_PID}"
