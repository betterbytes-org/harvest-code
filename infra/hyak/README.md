# HARVEST on Hyak (compute nodes only)

All heavy work runs via **Slurm on compute nodes**. Do not build, download models,
or serve LLMs on login nodes.

## Prerequisites

- Slurm account: `gpu-h200-harvest`
- GPU partition: `gpu-h200` with **2× H200** (`--gres=gpu:h200:2`)
- Scratch: `/gscratch/harvest/rithvik/`

## Quick start

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
chmod +x submit-all.sh scripts/*.sh
./submit-all.sh
```

This submits a dependency chain:

| Job | Partition | GPUs | Purpose |
|-----|-----------|------|---------|
| `01-gpu-smoke-test` | gpu-h200 | 2× H200 | CUDA + matmul sanity check |
| `02-build-harvest` | gpu-h200 | 0 | `cargo build --release` |
| `00-setup-vllm-env` | gpu-h200 | 2× H200 | Install vLLM + DeepGEMM |
| `03-download-dsv4-flash` | gpu-h200 | 2× H200 | Cache model weights |
| `04-vllm-dsv4-flash` | gpu-h200 | 2× H200 | Serve DeepSeek-V4-Flash (long-running) |

After job **04** writes `state/vllm-endpoint.env`:

```bash
sbatch slurm/05-test-structured-output.slurm
sbatch slurm/06-harvest-translate-test.slurm
```

## Monitor

```bash
squeue -u $USER
tail -f /gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-<jobid>.out
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/vllm-endpoint.env
```

## HARVEST config

Once the server is up, copy the endpoint from `state/vllm-endpoint.env` into
`config/harvest-vllm.toml` (replace `REPLACE_WITH_GPU_NODE`) and install:

```bash
mkdir -p ~/.config/harvest
cp config/harvest-vllm.toml ~/.config/harvest/config.toml
# edit address= to match VLLM_HOST from vllm-endpoint.env
```

Or pass `--config` flags (see `06-harvest-translate-test.slurm`).

## 2× H200 constraint

DeepSeek-V4-Flash uses **TP=2 + expert parallelism** on your 2 GPUs (~282 GiB
total). Settings are conservative (`max-model-len=32768`, `max-num-seqs=4`). If
model load OOMs, try lowering `--max-model-len` in
`scripts/start_vllm_background.sh`.

Recommended production sizing is 4× H200; your 2-GPU setup is a tight fit for
the native FP8 checkpoint (~149 GiB weights + KV cache).

## Individual jobs

```bash
export SLURM_ACCOUNT=gpu-h200-harvest
sbatch slurm/01-gpu-smoke-test.slurm
sbatch slurm/04-vllm-dsv4-flash.slurm   # after env + download
```

Interactive **compute-node** shell (still not login):

```bash
srun --account=gpu-h200-harvest --partition=gpu-h200 --gres=gpu:h200:2 \
  --cpus-per-task=8 --mem=64G --time=02:00:00 --pty bash
```
