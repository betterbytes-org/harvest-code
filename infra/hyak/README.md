# HARVEST on Hyak (compute nodes only)

All heavy work runs via **Slurm on compute nodes**. Do not build, download models,
or serve LLMs on login nodes.

## Prerequisites

- Slurm account: `gpu-h200-harvest`
- GPU partition: `gpu-h200` with **2× H200** (`--gres=gpu:h200:2`)
- Scratch: `/gscratch/harvest/rithvik/`
- Copy `env/secrets.env.example` → `env/secrets.env` and set `LITELLM_MASTER_KEY`,
  `LITELLM_SALT_KEY` (and optionally `HF_TOKEN`)
- Optional: `cp env/gateway.env.example env/gateway.env` (LiteLLM port, defaults)

## Always-on DeepSeek-V4-Flash-0731 API

Serve the model for team use via the LiteLLM gateway (virtual keys + admin UI):

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
cp env/secrets.env.example env/secrets.env   # edit LITELLM_MASTER_KEY, LITELLM_SALT_KEY
cp env/gateway.env.example env/gateway.env    # optional overrides
chmod +x submit-vllm-always-on.sh scripts/*.sh gateway/*.sh
./submit-vllm-always-on.sh
```

| Job | GPUs | Purpose |
|-----|------|---------|
| `00-setup-vllm-env` | 2× H200 | Install vLLM + DeepGEMM |
| `00b-setup-litellm-env` | 0 | Install LiteLLM proxy venv |
| `03-download-dsv4-flash` | 2× H200 | Cache `DeepSeek-V4-Flash-0731` weights |
| `04-vllm-dsv4-flash` | 2× H200 | Long-running vLLM + LiteLLM gateway |

After job **04** starts, see `state/gateway-endpoint.env` and `state/vllm-endpoint.env`.
User guide: `doc/API-ACCESS.md`. Admin / keys: `doc/GATEWAY-ACCESS.md`.

**HF token:** optional (model is public/MIT). Set `HF_TOKEN` in `secrets.env` if downloads rate-limit.

**Virtual keys:** admins issue per-user keys via UI or `gateway/bootstrap_admin.sh` (default 60 RPM).

## Quick start (full HARVEST pipeline)

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
| `00b-setup-litellm-env` | gpu-h200 | 0 | Install LiteLLM proxy venv |
| `03-download-dsv4-flash` | gpu-h200 | 2× H200 | Cache model weights |
| `04-vllm-dsv4-flash` | gpu-h200 | 2× H200 | Serve DeepSeek-V4-Flash-0731 |

After job **04** writes `state/gateway-endpoint.env`:

```bash
sbatch slurm/05-test-structured-output.slurm
sbatch slurm/06-harvest-translate-test.slurm
```

## Monitor

```bash
squeue -u $USER
tail -f /gscratch/harvest/rithvik/harvest/infra/hyak/logs/vllm-dsv4-<jobid>.out
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
```

## HARVEST config

Once the server is up, copy the endpoint from `state/gateway-endpoint.env` into
`config/harvest-vllm.toml` (replace `REPLACE_WITH_GPU_NODE` and port) and install:

```bash
mkdir -p ~/.config/harvest
cp config/harvest-vllm.toml ~/.config/harvest/config.toml
# edit address= to match GATEWAY_HOST:LITELLM_PORT from gateway-endpoint.env
```

Or pass `--config` flags (see `06-harvest-translate-test.slurm`).

## 2× H200 constraint

DeepSeek-V4-Flash-0731 uses **TP=2 + expert parallelism** on your 2 GPUs (~282 GiB
total). Settings are conservative (`max-model-len=32768`, `max-num-seqs=4`). If
model load OOMs, try lowering `--max-model-len` in
`scripts/start_vllm_background.sh`.

Recommended production sizing is 4× H200; your 2-GPU setup is a tight fit for
the native FP8 checkpoint (~149 GiB weights + KV cache).

**Architecture:** vLLM listens on `127.0.0.1:8001` (internal). **LiteLLM** on
`0.0.0.0:4000` (see `LITELLM_PORT` in `gateway-endpoint.env`) provides the admin UI,
virtual key auth, and rate limits before forwarding to vLLM.

## Individual jobs

```bash
export SLURM_ACCOUNT=gpu-h200-harvest
sbatch slurm/01-gpu-smoke-test.slurm
sbatch slurm/00b-setup-litellm-env.slurm   # LiteLLM venv (once)
sbatch slurm/04-vllm-dsv4-flash.slurm      # after env + download
```

Interactive **compute-node** shell (still not login):

```bash
srun --account=gpu-h200-harvest --partition=gpu-h200 --gres=gpu:h200:2 \
  --cpus-per-task=8 --mem=64G --time=02:00:00 --pty bash
```
