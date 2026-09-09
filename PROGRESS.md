# HARVEST progress (2026-09-08)

Handoff for a new Claude / agent session. Canonical memory: `MEMORY.md`.
Hyak playbook leftovers: `infra/hyak/state/CONTINUE-1M.txt` (stale job id — ignore).

## Live serve (do not kill casually)

| | |
|---|---|
| Slurm | **39581518** `RUNNING` on **g3130**, 2× H200, wall **7 days** (ends 2026-09-15T17:23) |
| Successor | **39848491** PD `afterany:39581518` |
| Model | `Qwen/Qwen3.8-27B` served as **`qwen3.8-27b`** |
| Context | YaRN 1M — `max_model_len=1010000` (`infra/hyak/env/qwen38-yarn-1m.json`) |
| KV | **6,233,763** tokens (enough for ~6 full 1M seqs) |
| Concurrency | `--max-num-seqs=4` |
| APC | **on** — `--enable-prefix-caching` (vLLM 0.26 defaults this **off** for hybrid Qwen3.8) |
| Cache reporting | **on** — `--enable-prompt-tokens-details` |
| Public URL | https://ceiling-manufacture-surge-sponsors.trycloudflare.com |

Always re-read `infra/hyak/state/public-url.env` after a restart. Quick tunnels change on every `cloudflared` / job restart. Trust the file only if its timestamp is after the new job’s tunnel write.

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

Job 04: `slurm/04-vllm-dsv4-flash.slurm` → `scripts/start_vllm_background.sh`. Do not `scancel` other user jobs. Do not start a second serve job while another holds PGDATA. To stop the API, scancel **running and pending** 04.

Hyak GPU nodes cannot `sbatch` (`Invalid account or account/partition`). `scripts/resubmit_vllm.sh` falls back to `ssh klone-login03`.

## What we did

1. Replaced the 256k job **38747864** with 1M YaRN job **38809797** (then **38852790**). Same LiteLLM + served name.
2. Translation run **succeeded** on 38809797. Prefix cache was **dead**: launch had `enable_prefix_caching=False`; vLLM `Prefix cache hit rate: 0.0%` on every sample; LiteLLM `cache_hit=False`; TTFT scaled with prompt (~14s at 162k). Root cause: hybrid default-off, not KV exhaustion.
3. Opted in APC (`cursor/enable-apc-7d79`, commit `c3fe41e`). Resubmitted **38852790**. Confirmed `'enable_prefix_caching': True` in `logs/vllm-38852790.log`.
4. Agent-side “send less context” was **not** done (user said later).
5. Resubmitted **38886754** with `--enable-prompt-tokens-details`. Confirmed both flags. Public URL was https://western-doll-rick-rrp.trycloudflare.com. Timed out 2026-09-02T13:08:54; USR1 never queued a successor.
6. 2026-09-04: 7-day resubmit **39581357** sat PD `Reserved for maintenance`; scancelled. Submitted **39581377** with `--time=3-00:00:00`. Booted on g3127. Successor **39581518** queued from login. URL was https://louisville-participating-rat-sms.trycloudflare.com.
7. 2026-09-07: **39581377** TIMED OUT at 11:33. Submitted 8h bridge **39794675** (TIMEOUT 03:29 Sept 8).
8. 2026-09-08: Maint reservation gone. **39581518** started 17:23 on g3130. URL: https://ceiling-manufacture-surge-sponsors.trycloudflare.com. `/v1/models` 200; `/health/readiness` 200; `/ui` 307. Virtual key unchanged. Successor **39848491** queued.

## Not done

- **Named tunnel** for a stable hostname. Domain available: `hyak.harvest.hurrypeng.cc`. Needs CNAME `hyak.harvest` → `<TUNNEL_UUID>.cfargotunnel.com` plus `CLOUDFLARE_TUNNEL_TOKEN` + `PUBLIC_NAMED_URL=https://hyak.harvest.hurrypeng.cc` in `secrets.env`, then resubmit 04. No A record to Hyak (no public IP). Do not CNAME to the current `*.trycloudflare.com`.
- Writeup.
- Home disk quota (~11G) still trips vLLM usage-stats (non-fatal).
- Confirm a prefix-hit request reports `usage.prompt_tokens_details.cached_tokens`
  (flag is live; not yet client-verified). Not LiteLLM response-cache (`cache_hit`).
- After 39581518 ends 2026-09-15T17:23, confirm **39848491** starts and publish the new tunnel URL.

## Git

Working branch for this restore: `cursor/restore-api-7d79` (resubmit login fallback). Prior APC work: `cursor/enable-apc-7d79`. Lots of uncommitted 1M/gateway files remain on disk (`qwen38-yarn-1m.json` is untracked but **required** on disk for job 04).
