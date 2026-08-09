#!/usr/bin/env bash
# Install LiteLLM proxy + Python deps into HARVEST scratch venv.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

mkdir -p "${LITELLM_VENV}" "${HARVEST_ENVS}"

# Home dir is over quota; use scratch for pip builds and caches.
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export PIP_CACHE_DIR="${HARVEST_ENVS}/pip-cache"
mkdir -p "${TMPDIR}" "${PIP_CACHE_DIR}"

# Hyak coenv python modules lack _ctypes on compute nodes; prefer miniforge.
litellm_find_python() {
  local candidates=(
    "${HARVEST_PYTHON312:-}"
    "/mmfs1/sw/miniforge3/25.9.1-0/bin/python3.12"
    "/mmfs1/sw/ondemand/miniconda3/bin/python3.12"
  )
  local cand py=""
  for cand in "${candidates[@]}"; do
    [[ -z "$cand" ]] && continue
    if [[ -x "$cand" ]] && "$cand" -c "import ctypes" 2>/dev/null; then
      py="$cand"
      break
    fi
  done
  if [[ -z "$py" ]]; then
    module load python/3.12 2>/dev/null || true
    if python3 -c "import ctypes" 2>/dev/null; then
      py="$(command -v python3)"
    fi
  fi
  if [[ -z "$py" ]]; then
    echo "FAIL: no working Python 3.12+ with ctypes found" >&2
    return 1
  fi
  echo "$py"
}

PY="$(litellm_find_python)"

# Recreate venv if it was built with a broken Python.
if [[ -d "${LITELLM_VENV}" ]]; then
  if ! "${LITELLM_VENV}/bin/python" -c "import ctypes" 2>/dev/null; then
    echo "Removing broken LiteLLM venv at ${LITELLM_VENV}"
    rm -rf "${LITELLM_VENV}"
  fi
fi

if [[ ! -x "${LITELLM_VENV}/bin/python" ]]; then
  "${PY}" -m venv "${LITELLM_VENV}"
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
