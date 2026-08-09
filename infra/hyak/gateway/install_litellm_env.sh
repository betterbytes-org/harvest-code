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

# fastapi 0.141+ removes get_flat_dependant; breaks LiteLLM 1.96.0 proxy imports.
pip install 'fastapi==0.140.6'
pip install 'litellm[proxy]' psycopg2-binary pyyaml

_litellm_ensure_nodejs() {
  local nodeconda="${HARVEST_NODECONDA:-${HARVEST_ENVS}/nodejs}"
  if command -v node >/dev/null 2>&1; then
    return 0
  fi
  if [[ -x "${nodeconda}/bin/node" ]]; then
    export PATH="${nodeconda}/bin:${PATH}"
    return 0
  fi

  local conda="/mmfs1/sw/miniforge3/25.9.1-0/bin/conda"
  [[ -x "${conda}" ]] || conda="$(command -v conda || true)"
  if [[ -n "${conda}" && -x "${conda}" ]]; then
    export HOME="${HARVEST_SCRATCH}/run-home"
    export TMPDIR="${HARVEST_SCRATCH}/tmp"
    export CONDA_PKGS_DIRS="${HARVEST_SCRATCH}/conda-pkgs"
    mkdir -p "${HOME}" "${TMPDIR}" "${CONDA_PKGS_DIRS}" "${nodeconda}"
    echo "Installing nodejs via conda at ${nodeconda} ..."
    "${conda}" create -y -p "${nodeconda}" -c conda-forge nodejs=20
    export PATH="${nodeconda}/bin:${PATH}"
    return 0
  fi

  echo "FAIL: node not found for prisma generate" >&2
  return 1
}

_litellm_prisma_generate() {
  export HOME="${HARVEST_SCRATCH}/run-home"
  mkdir -p "${HOME}"
  _litellm_ensure_nodejs

  local proxy_dir
  proxy_dir="$(python -c "import litellm.proxy, os; print(os.path.dirname(litellm.proxy.__file__))")"
  echo "Running prisma generate in ${proxy_dir} ..."
  (cd "${proxy_dir}" && prisma generate)
  python -c "import prisma; print('prisma ok')"
}

_litellm_prisma_generate

# PostgreSQL client tools (for initdb if module available)
if module avail postgresql 2>&1 | grep -q postgresql; then
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
fi

echo "LiteLLM venv ready at ${LITELLM_VENV}"
python -c "import litellm, psycopg2, prisma; print('ok')"
python -c "import fastapi; print(f'fastapi {fastapi.__version__}')"
