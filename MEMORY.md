# HARVEST agent memory

## Where We Left Off

APC job **38852790** RUNNING on **g3129**. Tunnel:
`https://kathy-buck-when-excluded.trycloudflare.com` (2026-08-25T23:32:55Z).
`enable_prefix_caching=True`, KV 6,233,763 tokens. Old **38809797** cancelled
only. Dreamer jobs left alone. Do not rotate `LITELLM_VIRTUAL_KEY`. Do not
drop LiteLLM Postgres.

## Decisions

- Same LiteLLM, same served name `qwen3.8-27b`.
- 1M concurrency: KV pool ~6.26M tokens; cap **4** (`VLLM_MAX_NUM_SEQS`).
- APC: vLLM 0.26 defaults hybrid APC off; we now pass `--enable-prefix-caching`.
  Same GPU KV pool (no extra HBM allocation).
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

APC job is live. Handoff: `PROGRESS.md`. Optional: named Cloudflare
tunnel (`hyak.harvest.hurrypeng.cc`); writeup.

## Tentative

- Shell on login01 private workers often fails to spawn; login03 exec works.
- APC off on 38809797: `enable_prefix_caching=False`; 0.0% prefix hits.
- Submitted 38852790 at 2026-08-25 ~23:12Z from klone-login03 after adding
  `--enable-prefix-caching`.
- Cache-read tokens missing: vLLM emits `prompt_tokens_details: null` unless
  `--enable-prompt-tokens-details`. Flag added to serve scripts; live job
  not restarted.
