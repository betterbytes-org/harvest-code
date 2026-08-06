#!/bin/bash
#SBATCH --job-name=gpu-matmul
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --gres=gpu:h200:1
#SBATCH --cpus-per-task=4
#SBATCH --mem=16G
#SBATCH --time=00:25:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-matmul-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/gpu-matmul-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
# shellcheck source=/dev/null
source "${HARVEST_ROOT}/infra/hyak/env/common.env"

module load gcc/13.2.0 cuda/12.4.1 2>/dev/null || module load gcc cuda
module load python3/3.12.3 2>/dev/null || module load python3/3.12.1

PY=$(command -v python3)
echo "Using: $PY ($($PY --version 2>&1))"
$PY -c "import ctypes; print('ctypes OK')"
nvidia-smi -L

TEST_VENV="${HARVEST_ENVS}/gpu-bench-py312"
rm -rf "${TEST_VENV}"
$PY -m venv "${TEST_VENV}"
# shellcheck source=/dev/null
source "${TEST_VENV}/bin/activate"
pip install --upgrade pip
pip install torch --index-url https://download.pytorch.org/whl/cu124

python3 - <<'PY'
import time, torch
print("=== GPU Compute Benchmark ===")
print(f"Device: {torch.cuda.get_device_name(0)}")
props = torch.cuda.get_device_properties(0)
print(f"Memory: {props.total_memory/1024**3:.1f} GiB, SMs={props.multi_processor_count}, CC={props.major}.{props.minor}")
for n in [2048, 4096, 8192, 12288]:
    a = torch.randn(n, n, device="cuda", dtype=torch.float16)
    b = torch.randn(n, n, device="cuda", dtype=torch.float16)
    for _ in range(3):
        _ = a @ b
    torch.cuda.synchronize()
    reps = 10
    t0 = time.perf_counter()
    for _ in range(reps):
        c = a @ b
    torch.cuda.synchronize()
    elapsed = time.perf_counter() - t0
    tflops = 2 * n**3 * reps / elapsed / 1e12
    print(f"matmul {n}x{n} fp16: {tflops:.2f} TFLOPS, {elapsed/reps*1000:.2f} ms/iter")

size = 256 * 1024 * 1024 // 4
x = torch.randn(size, device="cuda")
y = torch.empty_like(x)
for _ in range(5):
    y.copy_(x)
torch.cuda.synchronize()
reps = 20
t0 = time.perf_counter()
for _ in range(reps):
    y.copy_(x)
torch.cuda.synchronize()
bw = 2 * size * 4 * reps / (time.perf_counter() - t0) / 1e9
print(f"memory copy bandwidth: {bw:.1f} GB/s")
PY
