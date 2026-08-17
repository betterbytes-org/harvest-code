# HARVEST agent memory

## Where We Left Off

Job `38514770` on `g3129` is serving via Cloudflare quick tunnel
`https://isle-repeated-north-surprised.trycloudflare.com`. OpenCode 400 was
`max_tokens=32000` + ~769 prompt on a 32768 context. Fix: LiteLLM
`clamp_max_tokens` hook + `examples/opencode.json` with `limit.output: 8192`.
LiteLLM must be restarted on the job node for the hook to load.

## Decisions

- Client key: `sk-harvest-ext-9K2mQ7nP4wX8vL3bC6tY` (not the master key).
- `*.trycloudflare.com` is a Quick Tunnel (random hostname), not DDNS. Named
  tunnel + cs.washington.edu/personal domain is the stable option.

## Next step

`srun --jobid=38514770 --overlap bash infra/hyak/scripts/restart_litellm.sh`
then retest OpenCode with `examples/opencode.json`.

## Tentative

- Home quota exceeded on login; this worker's Shell often fails to spawn.
