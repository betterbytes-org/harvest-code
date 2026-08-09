# DeepSeek-V4-Flash-0731 API Access

Shared inference endpoint on Hyak H200 GPUs. Requests go through the **LiteLLM
gateway** (auth, per-key rate limits, usage tracking) before reaching vLLM.

## Prerequisites

- UW Hyak account with access to the harvest GPU allocation
- A **virtual key** from the gateway admin (see `doc/GATEWAY-ACCESS.md`)
- SSH access to Hyak

## Find the endpoint

```bash
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
```

Look for `GATEWAY_HOST` and `LITELLM_PORT` (default **4000**). The legacy
`state/vllm-endpoint.env` is also written and mirrors the same host/port when the
gateway is enabled.

## Connect (SSH tunnel)

From your laptop:

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
ssh -L ${LITELLM_PORT}:${GATEWAY_HOST}:${LITELLM_PORT} <user>@hyak.uw.edu
```

Replace values from the endpoint file if you prefer not to source it. Example with
default port:

```bash
ssh -L 4000:<GATEWAY_HOST>:4000 <user>@hyak.uw.edu
```

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
