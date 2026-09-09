# HARVEST agent memory

## Where We Left Off

Always-on API is **up**. 7-day job **39581518** `RUNNING` on **g3130**
from **2026-09-08T17:23:25** until **2026-09-15T17:23:25**. Successor
**39848491** is pending `afterany:39581518` (auto-chain). Same virtual
key. Do not rotate `LITELLM_VIRTUAL_KEY`. Do not drop LiteLLM Postgres.

Public URL (quick tunnel, 2026-09-09T00:48:37Z):
https://ceiling-manufacture-surge-sponsors.trycloudflare.com

September8_Maintenance is gone (`scontrol` has no Sept reservation;
next MAGNETIC ALL_NODES is October13_Maintenance 2026-10-13T09:00).
Bridge **39794675** TIMED OUT at 2026-09-08T03:29. Old quick tunnels
`louisville-participating-rat-sms` and
`memories-amenities-drill-queries` are dead.

To stop the API, scancel running and pending 04 jobs (39581518 and
39848491).

## Decisions

- Same LiteLLM, same served name `qwen3.8-27b`.
- 1M concurrency: KV pool ~6.26M tokens; cap **4** (`VLLM_MAX_NUM_SEQS`).
- APC + `--enable-prompt-tokens-details` on.
- Quick tunnel is **not** a stable hostname.
- Do not drop the LiteLLM Postgres DB (mentor UI admin lives there).
- Do not start a second serve job while another job holds PGDATA.
- 7-day `#SBATCH --time` cannot start if the wall overlaps a MAGNETIC
  ALL_NODES maintenance reservation. Use a short `--time` to bridge.

## Credentials (saved)

- Disk (`infra/hyak/env/secrets.env`, gitignored): master, salt, client
  virtual key `sk-Uzr2bvSJGxEhrWsX2xPzoA`, vLLM internal key.
- Client key also in `state/virtual-keys.json` (alias `harvest-external`).
- Mentor LiteLLM UI admin: **Postgres only** at
  `/gscratch/harvest/rithvik/pgdata/litellm`.
- Named Cloudflare token: **not saved** (never set).

## Next step

Optional: named Cloudflare tunnel (`hyak.harvest.hurrypeng.cc`).
Before 2026-09-15T17:23, confirm 39848491 starts and write the new URL.

## Tentative

- Shell on login01 private workers often fails to spawn; login03 exec works.
- 39581377 TIMEOUT 2026-09-07T11:33:24 DUE TO TIME LIMIT. Successor
  39581518 EligibleTime 11:34 but StartTime 2026-09-09T04:00
  (Reserved for maintenance / later AssocGrpGRES while bridge holds GPUs).
- Bridge 39794675 submitted `--time=8:00:00` from login03; started on
  g3130 immediately. Extra chain 39794676 scancelled (duplicate 7-day).
- Compute-node sbatch on g3130 succeeded this time (login fallback not
  needed); still keep the fallback.
- 39794675 boot: APC + prompt-tokens-details, KV 6,233,763.
  `/v1/models` 200; `/health/readiness` 200; `/ui` 307. Key suffix
  `2xPzoA` unchanged.
- September8_Maintenance reservation was gone by 2026-09-08T17:23;
  39581518 started then instead of 2026-09-09T04:00.
- 39581518 boot ~25 min: KV 6,233,763; LiteLLM `/health/readiness` 200
  (db connected); `/v1/models` 200 `qwen3.8-27b`; `/ui` 307; public
  tunnel + chat 200. Key suffix `2xPzoA` unchanged. Stale
  `postmaster.pid` (260417) delayed Postgres ~1 min then released.
