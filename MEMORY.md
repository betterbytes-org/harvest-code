# HARVEST agent memory

## Where We Left Off

Reporting restart is live. Job **38886754** `RUNNING` on **g3129** with
`--enable-prefix-caching` and `--enable-prompt-tokens-details`. Old job
**38852790** cancelled only. Dreamer jobs left alone. Do not rotate
`LITELLM_VIRTUAL_KEY`. Do not drop LiteLLM Postgres.

Public URL (quick tunnel, 2026-08-26T20:43:17Z):
https://western-doll-rick-rrp.trycloudflare.com

## Decisions

- Same LiteLLM, same served name `qwen3.8-27b`.
- 1M concurrency: KV pool ~6.26M tokens; cap **4** (`VLLM_MAX_NUM_SEQS`).
- APC: vLLM 0.26 defaults hybrid APC off; we now pass `--enable-prefix-caching`.
  Same GPU KV pool (no extra HBM allocation).
- Cache-read tokens: vLLM 0.26 fills `usage.prompt_tokens_details.cached_tokens`
  only with `--enable-prompt-tokens-details` (now on 38886754).
- Quick tunnel is **not** a stable hostname.
- Do not drop the LiteLLM Postgres DB (mentor UI admin lives there).
- Do not start a second serve job while another job holds PGDATA.

## Credentials (saved)

- Disk (`infra/hyak/env/secrets.env`, gitignored): master, salt, client
  virtual key `sk-Uzr2bvSJGxEhrWsX2xPzoA`, vLLM internal key.
- Client key also in `state/virtual-keys.json` (alias `harvest-external`).
- Mentor LiteLLM UI admin: **Postgres only** at
  `/gscratch/harvest/rithvik/pgdata/litellm`.
- Named Cloudflare token: **not saved** (never set).

## Next step

Optional: named Cloudflare tunnel (`hyak.harvest.hurrypeng.cc`); writeup.
Confirm a client request shows `prompt_tokens_details.cached_tokens` on a
prefix hit.

## Tentative

- Shell on login01 private workers often fails to spawn; login03 exec works.
- APC off on 38809797: `enable_prefix_caching=False`; 0.0% prefix hits.
- Submitted 38852790 at 2026-08-25 ~23:12Z from klone-login03 after adding
  `--enable-prefix-caching`.
- Cache-read tokens missing on 38852790: vLLM emitted
  `prompt_tokens_details: null` without `--enable-prompt-tokens-details`.
- 38886754 boot: `enable_prompt_tokens_details=True`,
  `enable_prefix_caching=True`, `max_model_len=1010000`, GPU KV 6,233,763.
  `/v1/models` 200; `/health/readiness` 200; `/ui` 307.
