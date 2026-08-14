#!/bin/bash
#SBATCH --job-name=harvest-pg-conda
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=2
#SBATCH --mem=4G
#SBATCH --time=00:20:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/pg-conda-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/pg-conda-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"

PGCONDA="${HARVEST_ENVS}/postgres"
CONDA="/mmfs1/sw/miniforge3/25.9.1-0/bin/conda"
export HOME="${HARVEST_SCRATCH}/run-home"
export TMPDIR="${HARVEST_SCRATCH}/tmp"
export CONDA_PKGS_DIRS="${HARVEST_SCRATCH}/conda-pkgs"
mkdir -p "${HOME}" "${TMPDIR}" "${CONDA_PKGS_DIRS}"

echo "=== Conda PostgreSQL install test on $(hostname -f) ==="
if [[ ! -x "${PGCONDA}/bin/pg_ctl" ]]; then
  "${CONDA}" create -y -p "${PGCONDA}" -c conda-forge postgresql=15
fi
export PATH="${PGCONDA}/bin:${PATH}"
pg_ctl --version
initdb --version
