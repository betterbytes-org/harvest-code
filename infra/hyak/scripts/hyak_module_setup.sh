#!/usr/bin/env bash
# Hyak module setup for harvest Slurm jobs (compute nodes).
set -euo pipefail

hyak_harvest_load_modules() {
  if ! command -v module >/dev/null 2>&1; then
    echo "ERROR: environment modules unavailable" >&2
    return 1
  fi
  export LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}"
  export LD_PRELOAD="${LD_PRELOAD:-}"
  local restore_nounset=0
  if [[ $- == *u* ]]; then
    restore_nounset=1
    set +u
  fi
  module load gcc/13.2.0 cuda/12.4.1 2>/dev/null || module load gcc cuda
  for mod in coenv/python/3.13.11 python3/3.12.3 python3/3.12.1 coenv/python/3.11.9; do
    if module load "$mod" 2>/dev/null; then
      echo "Loaded ${mod}"
      break
    fi
  done
  if [[ "$restore_nounset" -eq 1 ]]; then
    set -u
  fi
  if command -v nvcc >/dev/null 2>&1; then
    export CUDA_HOME="$(dirname "$(dirname "$(command -v nvcc)")")"
    export PATH="${CUDA_HOME}/bin:${PATH}"
    export LD_LIBRARY_PATH="${CUDA_HOME}/lib64:${LD_LIBRARY_PATH:-}"
  fi
  echo "python3: $(command -v python3) ($(python3 --version))"
}

hyak_harvest_load_llvm() {
  if ! command -v module >/dev/null 2>&1; then
    return 1
  fi
  local restore_nounset=0
  if [[ $- == *u* ]]; then
    restore_nounset=1
    set +u
  fi
  module load gcc/13.2.0 2>/dev/null || true
  module load niac/llvm/14.0.3 2>/dev/null || module load mamslab/llvm/14.0.6 2>/dev/null || true
  if [[ "$restore_nounset" -eq 1 ]]; then
    set -u
  fi
  export LIBCLANG_PATH="${LIBCLANG_PATH:-/sw/contrib/niac-src/llvm-project/14.0.3/lib}"
  export LLVM_CONFIG_PATH="${LLVM_CONFIG_PATH:-/sw/contrib/niac-src/llvm-project/14.0.3/bin/llvm-config}"
  export LD_LIBRARY_PATH="${LIBCLANG_PATH}:${LD_LIBRARY_PATH:-}"
  if [[ ! -f "${LIBCLANG_PATH}/libclang.so" && ! -f "${LIBCLANG_PATH}/libclang.so.13" ]]; then
    echo "FAIL: libclang not found under ${LIBCLANG_PATH}" >&2
    return 1
  fi
  echo "LIBCLANG_PATH=${LIBCLANG_PATH}"
}

hyak_harvest_require_python() {
  local ver min_major=3 min_minor=10
  ver="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
  echo "Python ${ver}"
  if [[ "${ver%%.*}" -lt "${min_major}" ]] || [[ "${ver#*.}" -lt "${min_minor}" ]]; then
    echo "FAIL: need Python >= ${min_major}.${min_minor}, got ${ver}" >&2
    return 1
  fi
}
