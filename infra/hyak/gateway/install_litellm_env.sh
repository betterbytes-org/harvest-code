#!/usr/bin/env bash
# Install LiteLLM proxy + Python deps into HARVEST scratch venv.
# Expects hyak_harvest_load_python_vllm (or equivalent) to have set python3 on PATH.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

mkdir -p "${LITELLM_VENV}" "${HARVEST_ENVS}"

# Home dir is over quota; use scratch for pip builds and caches.
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PIP_CACHE_DIR="${HARVEST_ENVS}/pip-cache"
mkdir -p "${TMPDIR}" "${PIP_CACHE_DIR}"

# Recreate venv if it was built with a broken Python.
if [[ -d "${LITELLM_VENV}" ]]; then
  if ! "${LITELLM_VENV}/bin/python" -c "import ctypes" 2>/dev/null; then
    echo "Removing broken LiteLLM venv at ${LITELLM_VENV}"
    rm -rf "${LITELLM_VENV}"
  fi
fi

if [[ ! -x "${LITELLM_VENV}/bin/python" ]]; then
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
python -c "import litellm, psycopg2; print('ok')"
