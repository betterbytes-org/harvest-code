#!/usr/bin/env bash
# Restart LiteLLM only (vLLM stays up). Run on the GPU job node:
#   srun --jobid=<JOB> --overlap bash infra/hyak/scripts/restart_litellm.sh
set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"
[[ -f "${HARVEST_STATE}/database.url" ]] && export DATABASE_URL="$(cat "${HARVEST_STATE}/database.url")"
bash "${HARVEST_INFRA}/gateway/start_litellm.sh"
echo "LiteLLM restarted. Existing Cloudflare tunnel is unchanged."
