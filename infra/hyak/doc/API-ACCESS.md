# DeepSeek-V4-Flash-0731 API Access

Shared inference endpoint on Hyak H200 GPUs. Requests go through the **LiteLLM
gateway** (auth, per-key rate limits, usage tracking) before reaching vLLM.

## Prerequisites

- A **virtual key** (issued when job **04** starts; also in `env/secrets.env` as `LITELLM_VIRTUAL_KEY`)
- For the public URL: nothing else (Cloudflare quick tunnel)
- For the SSH path: UW Hyak SSH login

## Find the endpoint

```bash
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/public-url.env
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
```

`PUBLIC_URL` is the HTTPS address an outside user should call. `GATEWAY_HOST` / `LITELLM_PORT` are the on-cluster bind.

## Outside users (no Hyak SSH)

Job **04** starts a Cloudflare quick tunnel next to LiteLLM. Share `PUBLIC_API` and the virtual key:

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/public-url.env
curl "${PUBLIC_API}/chat/completions" \
  -H "Authorization: Bearer <YOUR_VIRTUAL_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-v4-flash-0731",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 64
  }'
```

Dashboard: `${PUBLIC_UI}` (login with `LITELLM_MASTER_KEY`).

`*.trycloudflare.com` is a **Cloudflare Quick Tunnel**, not DDNS. The GPU node has
no public IP, so `cloudflared` opens an outbound tunnel and Cloudflare assigns a
random hostname. That name changes when the Slurm job or tunnel restarts — not
because a Hyak IP is "dynamic" in the DDNS sense (compute IPs are private and
the node itself also changes across jobs). A `cs.washington.edu` or personal
domain is better: create a **named** Cloudflare tunnel, CNAME the domain at it,
and set `CLOUDFLARE_TUNNEL_TOKEN` + `PUBLIC_NAMED_URL` in `env/secrets.env`.

## OpenCode

OpenCode defaults `max_tokens` to ~32000 if `limit.output` is unset. This model's
context is 32768 (input + output), so that overflows. Copy
`examples/opencode.json` and set `HARVEST_API_KEY`. The gateway also clamps
oversized `max_tokens` so other clients do not hit the same 400.

## Connect (SSH tunnel)

Hyak GPU nodes are not on the public internet. Anyone with a Hyak SSH login can
reach the gateway with a local tunnel, then use only the virtual API key:

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
ssh -L ${LITELLM_PORT}:${GATEWAY_HOST}:${LITELLM_PORT} <user>@hyak.uw.edu
```

Replace values from the endpoint file if you prefer not to source it. Example with
default port:

```bash
ssh -L 4000:<GATEWAY_HOST>:4000 <user>@hyak.uw.edu
```

From a Hyak login node you can skip the tunnel and call `http://${GATEWAY_HOST}:${LITELLM_PORT}` directly.

The virtual key is in `state/virtual-keys.json` (field `key`) after job **04** finishes starting LiteLLM. Do not use the master key in client code.

## Call the API

Use your **virtual key** (`sk-...`), not the admin master key:

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
curl "http://localhost:${LITELLM_PORT}/v1/chat/completions" \
  -H "Authorization: Bearer <YOUR_VIRTUAL_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-v4-flash-0731",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 64
  }'
```

## Python (OpenAI SDK)

```python
from openai import OpenAI

# After: ssh -L 4000:<GATEWAY_HOST>:4000 <user>@hyak.uw.edu
client = OpenAI(
    base_url="http://localhost:4000/v1",  # match LITELLM_PORT from gateway-endpoint.env
    api_key="<YOUR_VIRTUAL_KEY>",
)

resp = client.chat.completions.create(
    model="deepseek-v4-flash-0731",
    messages=[{"role": "user", "content": "Hello"}],
)
print(resp.choices[0].message.content)
```

## Rate limits

LiteLLM enforces **per virtual key** limits (RPM/TPM) set when the key is created.
Default bootstrap key (`harvest-devs`): **60 requests/minute**, **100k tokens/minute**.

Admins can adjust limits in the UI (Virtual Keys → edit) or when generating keys via
`gateway/bootstrap_admin.sh`. When rate limited, the API returns HTTP **429**.

Legacy stdlib proxy mode (`GATEWAY_ENABLED=false`) still uses `env/rate-limit.env`;
the always-on path uses LiteLLM with `GATEWAY_ENABLED=true` (default in
`env/gateway.env.example`).

## Security notes

- Hyak compute nodes are not internet-facing; use SSH tunnel or campus VPN.
- Virtual keys identify usage per key; the master key is admin-only.
- Do not commit `secrets.env` or `state/virtual-keys.json` to git.

## Health check

```bash
bash /gscratch/harvest/rithvik/harvest/infra/hyak/scripts/test_litellm_gateway.sh
```

Or manually:

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
curl "http://localhost:${LITELLM_PORT}/health/readiness"
```

## Admin / gateway docs

See `doc/GATEWAY-ACCESS.md` for admin UI, virtual key issuance, and usage statistics.
