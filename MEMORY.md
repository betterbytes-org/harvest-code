# HARVEST agent memory

## Where We Left Off

API restored after 38886754 TIMED OUT 2026-09-02 (USR1 never chained).
Job **39581377** `RUNNING` on **g3127** (3-day wall so it starts before
Sept 8 maintenance). APC + prompt-tokens-details still on. Same virtual
key. Postgres reused (stale postmaster.pid cleared after 60s wait).
Dreamer jobs left alone. Do not rotate `LITELLM_VIRTUAL_KEY`. Do not drop
LiteLLM Postgres.

Public URL (quick tunnel, 2026-09-04T18:46:12Z):
https://louisville-participating-rat-sms.trycloudflare.com

Successor **39581518** PD `afterany:39581377` (queued from login). Current
job ends 2026-09-07T11:33. The 7-day successor may sit PD through
September8_Maintenance (2026-09-08T04:00–2026-09-09T04:00).

## Decisions

- Same LiteLLM, same served name `qwen3.8-27b`.
- 1M concurrency: KV pool ~6.26M tokens; cap **4** (`VLLM_MAX_NUM_SEQS`).
- APC: vLLM 0.26 defaults hybrid APC off; we now pass `--enable-prefix-caching`.
  Same GPU KV pool (no extra HBM allocation).
- Cache-read tokens: vLLM 0.26 fills `usage.prompt_tokens_details.cached_tokens`
  only with `--enable-prompt-tokens-details`.
- Quick tunnel is **not** a stable hostname.
- Do not drop the LiteLLM Postgres DB (mentor UI admin lives there).
- Do not start a second serve job while another job holds PGDATA.
- Chain next 04 at job start (`afterany`). Hyak compute nodes cannot
  `sbatch` (account/partition error); `resubmit_vllm.sh` falls back to
  `klone-login03`.

## Credentials (saved)

- Disk (`infra/hyak/env/secrets.env`, gitignored): master, salt, client
  virtual key `sk-Uzr2bvSJGxEhrWsX2xPzoA`, vLLM internal key.
- Client key also in `state/virtual-keys.json` (alias `harvest-external`).
- Mentor LiteLLM UI admin: **Postgres only** at
  `/gscratch/harvest/rithvik/pgdata/litellm`.
- Named Cloudflare token: **not saved** (never set).

## Next step

After Sept 9 maintenance, confirm 39581518 starts (or resubmit 04 with
default 7-day time). Optional: named Cloudflare tunnel
(`hyak.harvest.hurrypeng.cc`); writeup. Confirm a client request shows
`prompt_tokens_details.cached_tokens` on a prefix hit.

## Tentative

- Shell on login01 private workers often fails to spawn; login03 exec works.
- APC off on 38809797: `enable_prefix_caching=False`; 0.0% prefix hits.
- Submitted 38852790 at 2026-08-25 ~23:12Z from klone-login03 after adding
  `--enable-prefix-caching`.
- Cache-read tokens missing on 38852790: vLLM emitted
  `prompt_tokens_details: null` without `--enable-prompt-tokens-details`.
- 38886754 boot: `enable_prompt_tokens_details=True`,
  `enable_prefix_caching=True`, `max_model_len=1010000`, GPU KV 6,233,763.
  `/v1/models` 200; `/health/readiness` 200; `/ui` 307. TIMEOUT
  2026-09-02T13:08:54 after 7 days; USR1 resubmit never ran.
- 39581357 PD `Reserved for maintenance` (7-day wall overlapped Sept 8–9);
  scancelled. 39581377 submitted with `--time=3-00:00:00`.
- Compute-node `sbatch` on g3127: `Invalid account or account/partition
  combination specified`. Successor 39581518 submitted from login03.
- 39581377 boot: APC + prompt-tokens-details on, `max_model_len=1010000`,
  GPU KV 6,233,763. `/v1/models` 200 (`qwen3.8-27b`); `/health/readiness`
  200; `/ui` 307. Key suffix `2xPzoA` unchanged.
