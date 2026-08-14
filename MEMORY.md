# HARVEST agent memory

## Where We Left Off

Outside-user access is wired: job 04 starts LiteLLM, registers `LITELLM_VIRTUAL_KEY`, and publishes a Cloudflare quick tunnel to `state/public-url.env`. This private worker still cannot spawn a shell on `klone-login01`, so job 04 was not submitted from the agent.

## Decisions

- Outside clients use Cloudflare quick tunnel (`trycloudflare.com`) plus virtual key `sk-harvest-ext-9K2mQ7nP4wX8vL3bC6tY`.
- Do not hand out `LITELLM_MASTER_KEY` as a client key.

## Next step

On a working Hyak login shell: `bash /gscratch/harvest/rithvik/harvest/infra/hyak/submit-vllm-server.sh` then `cat state/public-url.env`.

## Tentative

- Home directory is over quota (write to `$HOME` returns error 122); Shell spawn on this worker times out ~600s.

## Decisions

- External users reach the API with a Hyak SSH tunnel + virtual key. Compute nodes are not internet-facing; do not add a public reverse proxy.
- LiteLLM binds `0.0.0.0:4000`; vLLM stays localhost-only.

## Next step

Submit job 04, wait for `state/gateway-endpoint.env` + `state/virtual-keys.json`, then `bash scripts/test_litellm_gateway.sh` from a login node.

## Tentative

- Shell tool on this cloud agent often fails to spawn on `klone-login01` (zsh/gscratch); file reads still work.
- Prisma migrate on GPFS-backed PGDATA can stall for hours; `timeout 90` + `synchronous_commit=off` is the mitigation.
