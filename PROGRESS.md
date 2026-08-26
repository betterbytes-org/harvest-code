# HARVEST progress (2026-08-25)

Handoff for a new Claude / agent session. Canonical memory: `MEMORY.md`.
Hyak playbook leftovers: `infra/hyak/state/CONTINUE-1M.txt` (stale job id — ignore its “do not scancel 38809797”).

## Live serve (do not kill casually)

| | |
|---|---|
| Slurm | **38852790** `RUNNING` on **g3129**, 2× H200, account `gpu-h200-harvest` |
| Model | `Qwen/Qwen3.8-27B` served as **`qwen3.8-27b`** |
| Context | YaRN 1M — `max_model_len=1010000` (`infra/hyak/env/qwen38-yarn-1m.json`) |
| KV | **6,233,763** tokens (enough for ~6 full 1M seqs) |
| Concurrency | `--max-num-seqs=4` |
| APC | **on** — `--enable-prefix-caching` (vLLM 0.26 defaults this **off** for hybrid Qwen3.8) |
| Public URL | https://kathy-buck-when-excluded.trycloudflare.com |

Always re-read `infra/hyak/state/public-url.env` after a 1033. Quick tunnels change on every `cloudflared` / job restart.

**Keys (do not rotate):** `infra/hyak/env/secrets.env` (gitignored).

- UI: username `admin`, password = `LITELLM_MASTER_KEY`
- API: `LITELLM_VIRTUAL_KEY` (also `state/virtual-keys.json`, alias `harvest-external`)
- Mentor LiteLLM UI users live in Postgres at `/gscratch/harvest/rithvik/pgdata/litellm`. **Do not** set `LITELLM_RESET_DB`. Do not drop PGDATA.

## Hyak rules

Login node = `sbatch` / `scancel` / `squeue` only. **Never** `vllm serve` on login. Prefer **klone-login03** (login01 exec often hangs).

```
export HOME=/gscratch/harvest/rithvik/run-home
export HISTFILE=/dev/null
export TMPDIR=/gscratch/harvest/rithvik/tmp
cd /gscratch/harvest/rithvik/harvest/infra/hyak
```

Job 04: `slurm/04-vllm-dsv4-flash.slurm` → `scripts/start_vllm_background.sh`. Do not `scancel` other user jobs (L40 “dreamer” 38747899 / 38747900). Do not start a second serve job while another holds PGDATA.

## What we did

1. Replaced the 256k job **38747864** with 1M YaRN job **38809797** (then **38852790**). Same LiteLLM + served name.
2. Translation run **succeeded** on 38809797. Prefix cache was **dead**: launch had `enable_prefix_caching=False`; vLLM `Prefix cache hit rate: 0.0%` on every sample; LiteLLM `cache_hit=False`; TTFT scaled with prompt (~14s at 162k). Root cause: hybrid default-off, not KV exhaustion.
3. Opted in APC (`cursor/enable-apc-7d79`, commit `c3fe41e`). Resubmitted **38852790**. Confirmed `'enable_prefix_caching': True` in `logs/vllm-38852790.log`.
4. Agent-side “send less context” was **not** done (user said later).

## Not done

- **Named tunnel** for a stable hostname. Domain available: `hyak.harvest.hurrypeng.cc`. Needs CNAME `hyak.harvest` → `<TUNNEL_UUID>.cfargotunnel.com` plus `CLOUDFLARE_TUNNEL_TOKEN` + `PUBLIC_NAMED_URL=https://hyak.harvest.hurrypeng.cc` in `secrets.env`, then resubmit 04. No A record to Hyak (no public IP). Do not CNAME to the current `*.trycloudflare.com`.
- Writeup.
- Home disk quota (~11G) still trips vLLM usage-stats (non-fatal).

## Git

Working branch: `cursor/enable-apc-7d79`. Lots of uncommitted 1M/gateway files remain on disk from earlier work (`qwen38-yarn-1m.json` is untracked but **required** on disk for job 04).
