#!/usr/bin/env bash
# Install LiteLLM proxy + Python deps into HARVEST scratch venv.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

mkdir -p "${LITELLM_VENV}" "${HARVEST_ENVS}"

if [[ ! -x "${LITELLM_VENV}/bin/python" ]]; then
  module load python/3.12 2>/dev/null || true
  python3 -m venv "${LITELLM_VENV}"
fi

# shellcheck source=/dev/null
source "${LITELLM_VENV}/bin/activate"
pip install --upgrade pip
pip install 'litellm[proxy]' psycopg2-binary pyyaml

# PostgreSQL client tools (for initdb if module available)
if module avail postgresql 2>&1 | grep -q postgresql; then
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
fi

echo "LiteLLM venv ready at ${LITELLM_VENV}"
litellm --version || pip show litellm | grep Version
