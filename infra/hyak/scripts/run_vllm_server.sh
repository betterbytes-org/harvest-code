#!/usr/bin/env bash
# Launch official Qwen3.8-27B via vLLM on 2x H200 (TP=2, YaRN 1M).
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

# shellcheck source=/dev/null
source "${VLLM_VENV}/bin/activate"

export CUDA_VISIBLE_DEVICES="${CUDA_VISIBLE_DEVICES:-0,1}"
export VLLM_ENGINE_READY_TIMEOUT_S="${VLLM_ENGINE_READY_TIMEOUT_S:-7200}"
export VLLM_ALLOW_LONG_MAX_MODEL_LEN="${VLLM_ALLOW_LONG_MAX_MODEL_LEN:-1}"
HF_OVERRIDES="$(cat "${VLLM_HF_OVERRIDES_FILE}")"

NODE="$(hostname -s)"
ENDPOINT_FILE="${HARVEST_STATE}/vllm-endpoint.env"
LOG_FILE="${HARVEST_LOGS}/vllm-${SLURM_JOB_ID:-local}.log"

cat >"${ENDPOINT_FILE}" <<EOF
# Written by run_vllm_server.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
VLLM_HOST=${NODE}
VLLM_PORT=${VLLM_PORT}
VLLM_ENDPOINT=http://${NODE}:${VLLM_PORT}/v1
VLLM_SERVED_NAME=${VLLM_SERVED_NAME}
SLURM_JOB_ID=${SLURM_JOB_ID:-}
EOF

echo "=== vLLM server starting on ${NODE}:${VLLM_PORT} ==="
echo "Endpoint file: ${ENDPOINT_FILE}"
nvidia-smi -L

# 2x H200. Official Qwen3.8-27B, YaRN 1M, TP=2.
exec vllm serve "${VLLM_MODEL}" \
  --served-model-name "${VLLM_SERVED_NAME}" \
  --host 0.0.0.0 \
  --port "${VLLM_PORT}" \
  --tensor-parallel-size "${VLLM_TP_SIZE}" \
  --trust-remote-code \
  --enable-auto-tool-choice \
  --tool-call-parser qwen3_coder \
  --reasoning-parser qwen3 \
  --mm-encoder-tp-mode data \
  --kv-cache-dtype fp8 \
  --max-model-len "${VLLM_MAX_MODEL_LEN}" \
  --max-num-seqs "${VLLM_MAX_NUM_SEQS}" \
  --max-num-batched-tokens 8192 \
  --gpu-memory-utilization 0.90 \
  --enable-chunked-prefill \
  --enable-prefix-caching \
  --enable-prompt-tokens-details \
  --hf-overrides "${HF_OVERRIDES}" \
  2>&1 | tee -a "${LOG_FILE}"
