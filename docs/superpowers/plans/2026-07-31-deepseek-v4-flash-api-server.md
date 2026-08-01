# DeepSeek-V4-Flash-0731 Always-On API Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve `deepseek-ai/DeepSeek-V4-Flash-0731` on Hyak H200 GPUs via an OpenAI-compatible HTTP API that stays up continuously for team use.

**Architecture:** Use **vLLM** (already partially wired in `infra/hyak/`) on a long-running Slurm job with **2× H200** (required for this 304B MoE model). vLLM exposes `/v1/chat/completions` on the compute node; users reach it via SSH port-forward from the Hyak login node. API access is gated with `VLLM_API_KEY`. A resubmit wrapper keeps the service alive across Slurm time limits.

**Tech Stack:** vLLM ≥0.20, DeepGEMM, Hugging Face Hub, Slurm, curl/OpenAI SDK clients

## Global Constraints

- All heavy work on **compute nodes only** (never login node for serving/download)
- Slurm account: `gpu-h200-harvest`, partition: `gpu-h200`
- Model requires **2× H200** (~282 GiB total VRAM); **1 GPU will not fit** this checkpoint
- Scratch paths under `/gscratch/harvest/rithvik/`
- Model ID: `deepseek-ai/DeepSeek-V4-Flash-0731`
- Served name: `deepseek-v4-flash-0731` (OpenAI `model` field clients pass)
- MIT license, public weights — HF token optional but recommended for download reliability

---

## Recommendation Summary (for mentor conversation)

| Tool | Verdict | Why |
|------|---------|-----|
| **vLLM** | ✅ **Use this** | Official DeepSeek recipe, OpenAI-compatible API, production batching, FP8/EP support, already in repo |
| **SGLang** | ⚠️ Alternative | Also officially supported; would require rewriting existing scripts |
| **Unsloth** | ❌ Wrong tool | Fine-tuning only — does not serve models at scale |
| **Ollama** | ❌ Wrong tool | Good for 7B models; cannot serve 304B MoE |
| **TGI / llama.cpp** | ❌ Wrong tool | No official DeepSeek-V4-Flash-0731 support |

---

### Task 1: Update model ID and environment variables

**Files:**
- Modify: `infra/hyak/env/common.env`
- Create: `infra/hyak/env/secrets.env.example`
- Modify: `infra/hyak/README.md`

**Interfaces:**
- Produces: env vars `VLLM_MODEL`, `VLLM_SERVED_NAME`, optional `HF_TOKEN`, `VLLM_API_KEY`

- [ ] **Step 1: Update model references in common.env**

```bash
# infra/hyak/env/common.env — change these lines:
export VLLM_MODEL="${VLLM_MODEL:-deepseek-ai/DeepSeek-V4-Flash-0731}"
export VLLM_SERVED_NAME="${VLLM_SERVED_NAME:-deepseek-v4-flash-0731}"
export VLLM_PORT="${VLLM_PORT:-8000}"
export VLLM_TP_SIZE="${VLLM_TP_SIZE:-2}"
```

- [ ] **Step 2: Create secrets.env.example**

```bash
# infra/hyak/env/secrets.env.example
# Copy to secrets.env (gitignored) and fill in values.
# Source from Slurm scripts AFTER common.env.

# Optional: faster/more reliable HF downloads (model is public, no gating)
export HF_TOKEN="hf_xxxxxxxxxxxxxxxx"

# Required for external API access: shared team key
export VLLM_API_KEY="generate-a-long-random-string-here"
```

- [ ] **Step 3: Add secrets.env to .gitignore**

```gitignore
# infra/hyak/env/secrets.env
infra/hyak/env/secrets.env
```

- [ ] **Step 4: Document HF auth note in README**

Add section: "HF token is optional (model is public/MIT). Set `HF_TOKEN` in `secrets.env` if downloads fail or rate-limit."

- [ ] **Step 5: Commit**

```bash
git add infra/hyak/env/common.env infra/hyak/env/secrets.env.example infra/hyak/README.md .gitignore
git commit -m "feat(infra): point vLLM config at DeepSeek-V4-Flash-0731"
```

---

### Task 2: Update model download script

**Files:**
- Modify: `infra/hyak/slurm/03-download-dsv4-flash.slurm`

**Interfaces:**
- Consumes: `VLLM_MODEL`, `HF_TOKEN`, `HF_HOME` from env
- Produces: weights at `${HARVEST_MODELS}/DeepSeek-V4-Flash-0731/`

- [ ] **Step 1: Update download target path and add HF auth**

Replace the download block with:

```bash
MODEL_DIR="${HARVEST_MODELS}/DeepSeek-V4-Flash-0731"

# Source secrets if present (HF_TOKEN)
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"

huggingface-cli download "${VLLM_MODEL}" \
  --local-dir "${MODEL_DIR}" \
  --local-dir-use-symlinks False

du -sh "${MODEL_DIR}"
echo "=== Model download complete: ${MODEL_DIR} ==="
```

- [ ] **Step 2: Verify download works (manual, on compute node)**

```bash
sbatch --account=gpu-h200-harvest infra/hyak/slurm/03-download-dsv4-flash.slurm
# Watch: tail -f infra/hyak/logs/download-dsv4-<jobid>.out
# Expected: ~300GB+ directory, exit 0
```

- [ ] **Step 3: Commit**

```bash
git add infra/hyak/slurm/03-download-dsv4-flash.slurm
git commit -m "feat(infra): download DeepSeek-V4-Flash-0731 with optional HF token"
```

---

### Task 3: Update vLLM launch script for -0731 recipe

**Files:**
- Modify: `infra/hyak/scripts/start_vllm_background.sh`

**Interfaces:**
- Consumes: `VLLM_MODEL`, `VLLM_SERVED_NAME`, `VLLM_API_KEY`, `VLLM_TP_SIZE`, `CUDA_VISIBLE_DEVICES`
- Produces: running vLLM on `0.0.0.0:${VLLM_PORT}`, writes `state/vllm-endpoint.env`

- [ ] **Step 1: Replace vLLM serve command with -0731 official flags**

```bash
# Load API key from env (never pass on CLI — visible in ps)
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
export VLLM_API_KEY  # vLLM reads this automatically

vllm serve "${VLLM_MODEL}" \
  --served-model-name "${VLLM_SERVED_NAME}" \
  --host 0.0.0.0 \
  --port "${VLLM_PORT}" \
  --tensor-parallel-size "${VLLM_TP_SIZE}" \
  --enable-expert-parallel \
  --trust-remote-code \
  --tokenizer-mode deepseek_v4 \
  --tool-call-parser deepseek_v4 \
  --enable-auto-tool-choice \
  --reasoning-parser deepseek_v4 \
  --kv-cache-dtype fp8 \
  --block-size 256 \
  --moe-backend deep_gemm_mega_moe \
  --attention-config '{"use_fp4_indexer_cache": true}' \
  --speculative-config '{"method":"dspark","num_speculative_tokens":7,"draft_sample_method":"greedy"}' \
  --max-model-len 32768 \
  --max-num-seqs 4 \
  --max-num-batched-tokens 8192 \
  --gpu-memory-utilization 0.92 \
  --enable-chunked-prefill \
  >>"${LOG_FILE}" 2>&1 &
```

- [ ] **Step 2: Update endpoint env file to include auth instructions**

Add to the heredoc in `start_vllm_background.sh`:

```bash
cat >>"${ENDPOINT_FILE}" <<EOF
# Clients: Authorization: Bearer \$VLLM_API_KEY
# OpenAI SDK: base_url=http://${NODE}:${VLLM_PORT}/v1  api_key=<key>  model=${VLLM_SERVED_NAME}
EOF
```

- [ ] **Step 3: Smoke-test on compute node**

```bash
# After model download completes:
sbatch --account=gpu-h200-harvest infra/hyak/slurm/04-vllm-dsv4-flash.slurm
# Wait for: cat infra/hyak/state/vllm-endpoint.env
# Test:
source infra/hyak/state/vllm-endpoint.env
source infra/hyak/env/secrets.env
curl -s http://${VLLM_HOST}:${VLLM_PORT}/v1/models \
  -H "Authorization: Bearer ${VLLM_API_KEY}" | python3 -m json.tool
```

Expected: JSON listing `deepseek-v4-flash-0731`.

- [ ] **Step 4: Commit**

```bash
git add infra/hyak/scripts/start_vllm_background.sh
git commit -m "feat(infra): vLLM launch for DeepSeek-V4-Flash-0731 with DSpark"
```

---

### Task 4: Always-on Slurm job with auto-resubmit

**Files:**
- Modify: `infra/hyak/slurm/04-vllm-dsv4-flash.slurm`
- Create: `infra/hyak/scripts/resubmit_vllm.sh`
- Create: `infra/hyak/submit-vllm-always-on.sh`

**Interfaces:**
- Produces: self-resubmitting long-running vLLM service

- [ ] **Step 1: Add requeue flag to Slurm job**

Add to `04-vllm-dsv4-flash.slurm` header:

```bash
#SBATCH --requeue
#SBATCH --time=7-00:00:00
#SBATCH --signal=B:USR1@300
```

- [ ] **Step 2: Create resubmit wrapper**

```bash
#!/usr/bin/env bash
# infra/hyak/scripts/resubmit_vllm.sh
# Called on SIGUSR1 (5 min before timeout) to chain a new server job.
set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
SLURM_DIR="${HARVEST_ROOT}/infra/hyak/slurm"
export SLURM_ACCOUNT="${SLURM_ACCOUNT:-gpu-h200-harvest}"

echo "$(date -u): Requeue/resubmit vLLM server..."
sbatch --dependency=afterany:"${SLURM_JOB_ID}" \
  --account="${SLURM_ACCOUNT}" \
  "${SLURM_DIR}/04-vllm-dsv4-flash.slurm"
```

- [ ] **Step 3: Trap SIGUSR1 in server job to resubmit before exit**

Add to top of `04-vllm-dsv4-flash.slurm` after `set -euo pipefail`:

```bash
trap 'bash "${HARVEST_INFRA}/scripts/resubmit_vllm.sh"' USR1
```

- [ ] **Step 4: Create one-command always-on launcher**

```bash
#!/usr/bin/env bash
# infra/hyak/submit-vllm-always-on.sh
set -euo pipefail
cd "$(dirname "$0")"
echo "=== Always-on DeepSeek-V4-Flash-0731 API ==="
echo "Prerequisites: secrets.env with VLLM_API_KEY"
echo ""
J00=$(sbatch --parsable slurm/00-setup-vllm-env.slurm)
J03=$(sbatch --parsable --dependency=afterok:${J00} slurm/03-download-dsv4-flash.slurm)
J04=$(sbatch --parsable --dependency=afterok:${J03} slurm/04-vllm-dsv4-flash.slurm)
echo "Pipeline: setup=${J00} download=${J03} serve=${J04}"
echo "Endpoint file: state/vllm-endpoint.env (after ${J04} starts)"
```

- [ ] **Step 5: Commit**

```bash
chmod +x infra/hyak/scripts/resubmit_vllm.sh infra/hyak/submit-vllm-always-on.sh
git add infra/hyak/slurm/04-vllm-dsv4-flash.slurm infra/hyak/scripts/resubmit_vllm.sh infra/hyak/submit-vllm-always-on.sh
git commit -m "feat(infra): always-on vLLM with Slurm auto-resubmit"
```

---

### Task 5: External access guide for team API users

**Files:**
- Create: `infra/hyak/doc/API-ACCESS.md`
- Modify: `infra/hyak/config/harvest-vllm.toml` (update model name)

**Interfaces:**
- Produces: documented access pattern for remote users

- [ ] **Step 1: Write API access doc**

```markdown
# DeepSeek-V4-Flash-0731 API Access

## Endpoint (from Hyak login node)

1. Get compute node hostname:
   cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/vllm-endpoint.env

2. SSH tunnel (run on your laptop):
   ssh -L 8000:<COMPUTE_NODE>:8000 <user>@hyak.uw.edu

3. Call API locally:
   curl http://localhost:8000/v1/chat/completions \
     -H "Authorization: Bearer <VLLM_API_KEY>" \
     -H "Content-Type: application/json" \
     -d '{"model":"deepseek-v4-flash-0731","messages":[{"role":"user","content":"Hello"}]}'

## Python (OpenAI SDK)

from openai import OpenAI
client = OpenAI(
    base_url="http://localhost:8000/v1",  # via SSH tunnel
    api_key="<VLLM_API_KEY>",
)
resp = client.chat.completions.create(
    model="deepseek-v4-flash-0731",
    messages=[{"role": "user", "content": "Hello"}],
)
print(resp.choices[0].message.content)
```

Note: Hyak compute nodes are not internet-facing. Users must be on UW network + use SSH tunnel, OR set up an approved reverse proxy.

- [ ] **Step 2: Update harvest-vllm.toml model name**

Change all `model = "deepseek-v4-flash"` to `model = "deepseek-v4-flash-0731"`.

- [ ] **Step 3: Commit**

```bash
git add infra/hyak/doc/API-ACCESS.md infra/hyak/config/harvest-vllm.toml
git commit -m "docs(infra): API access guide for vLLM endpoint"
```

---

### Task 6: Health check and monitoring script

**Files:**
- Create: `infra/hyak/scripts/check_vllm_health.sh`

**Interfaces:**
- Consumes: `state/vllm-endpoint.env`, `secrets.env`
- Produces: exit 0 if healthy, exit 1 if down

- [ ] **Step 1: Write health check**

```bash
#!/usr/bin/env bash
set -euo pipefail
HARVEST_INFRA="/gscratch/harvest/rithvik/harvest/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_STATE}/vllm-endpoint.env" ]] && source "${HARVEST_STATE}/vllm-endpoint.env"

URL="http://${VLLM_HOST}:${VLLM_PORT}/v1/models"
if curl -sf "${URL}" -H "Authorization: Bearer ${VLLM_API_KEY}" >/dev/null; then
  echo "OK: vLLM healthy at ${URL}"
  exit 0
else
  echo "FAIL: vLLM unreachable at ${URL}"
  exit 1
fi
```

- [ ] **Step 2: Test**

```bash
bash infra/hyak/scripts/check_vllm_health.sh
# Expected (when server running): OK: vLLM healthy at ...
```

- [ ] **Step 3: Commit**

```bash
chmod +x infra/hyak/scripts/check_vllm_health.sh
git add infra/hyak/scripts/check_vllm_health.sh
git commit -m "feat(infra): vLLM health check script"
```

---

### Task 7: End-to-end verification

**Files:**
- Test: `infra/hyak/scripts/test_structured_output.py` (existing)

- [ ] **Step 1: Run full pipeline**

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
cp env/secrets.env.example env/secrets.env   # fill in VLLM_API_KEY (+ optional HF_TOKEN)
./submit-vllm-always-on.sh
```

- [ ] **Step 2: Wait for server, verify chat completion**

```bash
source state/vllm-endpoint.env
source env/secrets.env
curl -s "http://${VLLM_HOST}:${VLLM_PORT}/v1/chat/completions" \
  -H "Authorization: Bearer ${VLLM_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-v4-flash-0731",
    "messages": [{"role": "user", "content": "Say hello in one sentence."}],
    "max_tokens": 64
  }' | python3 -m json.tool
```

Expected: valid JSON with `choices[0].message.content` containing a greeting.

- [ ] **Step 3: Verify resubmit chain (optional)**

After 7-day limit or manual `scancel`, confirm a new job appears via `squeue -u $USER`.

- [ ] **Step 4: Share credentials**

Send teammates: `VLLM_API_KEY`, link to `infra/hyak/doc/API-ACCESS.md`, and current `VLLM_HOST` from endpoint file.

---

## HF Auth: Do You Need It?

| Question | Answer |
|----------|--------|
| Is the model gated? | **No** — MIT license, public weights |
| Will download work without token? | **Yes**, but may hit rate limits |
| Should you set `HF_TOKEN` anyway? | **Yes** — faster, more reliable downloads |
| "Added as a plugin"? | Store token in `secrets.env`; if you have a Cursor HF MCP plugin, use it to generate the token, then paste into `secrets.env` |

---

## Important: 1 GPU vs 2 GPU

**You cannot serve this model on 1× H200.** The checkpoint is ~304B params (MoE) and needs ~149 GiB weights + KV cache. Your existing README already documents **2× H200 (~282 GiB)** as the minimum.

"Dedicate GPUs to serving" = use your **full 2× H200 allocation exclusively for vLLM**, not split one GPU for serving and one for training.

If you truly only have 1 GPU available, you would need a much smaller model (e.g. codellama:7b via Ollama, which you already benchmarked at ~320 tok/s).

---

## Self-Review Checklist

- [x] Model updated to DeepSeek-V4-Flash-0731
- [x] vLLM chosen with official recipe flags
- [x] Always-on via 7-day job + auto-resubmit
- [x] API key auth for team access
- [x] HF token optional but documented
- [x] External access via SSH tunnel documented
- [x] 2-GPU requirement stated clearly
- [x] Unsloth/Ollama ruled out with reasons
