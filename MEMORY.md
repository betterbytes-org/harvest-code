# HARVEST agent memory

## Where We Left Off

Service was down because **39581377** hit its 3-day TIME LIMIT at
2026-09-07T11:33. Successor **39581518** is a 7-day job so Slurm holds it
until after September8_Maintenance (StartTime 2026-09-09T04:00). Old
quick tunnel `louisville-participating-rat-sms` is dead.

Bridge job **39794675** `RUNNING` on **g3130** until **2026-09-08T03:29**
(`--time=8:00:00` so it ends before maint 04:00). Same virtual key.
Do not rotate `LITELLM_VIRTUAL_KEY`. Do not drop LiteLLM Postgres.

Public URL (quick tunnel, 2026-09-08T03:06:57Z):
https://memories-amenities-drill-queries.trycloudflare.com

After 03:29 tonight the API goes down again until 39581518 starts
2026-09-09T04:00 (plus ~30 min boot). To stop the API, scancel running
and pending 04 jobs (39794675 and 39581518).

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

After Sept 9 04:00, confirm 39581518 boots and write the new URL.
Optional: named Cloudflare tunnel (`hyak.harvest.hurrypeng.cc`).

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
