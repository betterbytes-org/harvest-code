# DeepSeek-V4-Flash-0731 API Access

Shared inference endpoint on Hyak H200 GPUs. Requests go through a rate-limited
proxy with API key auth before reaching vLLM.

## Prerequisites

- UW Hyak account with access to the harvest GPU allocation
- `VLLM_API_KEY` (ask the server admin)
- SSH access to Hyak

## Find the endpoint

```bash
cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/vllm-endpoint.env
```

Look for `VLLM_HOST` and `VLLM_PROXY_PORT` (default 18080; `VLLM_PORT` in endpoint is an alias).

## Connect (SSH tunnel)

From your laptop:

```bash
ssh -L 18080:<COMPUTE_NODE>:18080 <user>@hyak.uw.edu
```

Replace `<COMPUTE_NODE>` with `VLLM_HOST` from the endpoint file.

## Call the API

```bash
curl http://localhost:18080/v1/chat/completions \
  -H "Authorization: Bearer <VLLM_API_KEY>" \
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

client = OpenAI(
    base_url="http://localhost:18080/v1",  # via SSH tunnel
    api_key="<VLLM_API_KEY>",
)

resp = client.chat.completions.create(
    model="deepseek-v4-flash-0731",
    messages=[{"role": "user", "content": "Hello"}],
)
print(resp.choices[0].message.content)
```

## Rate limits

Default: **60 requests/minute** per API key, burst **10**. Admins can change
limits without restarting vLLM by editing `env/rate-limit.env` and restarting
the server job:

```bash
export RATE_LIMIT_ENABLED=true
export RATE_LIMIT_REQUESTS_PER_MINUTE=60
export RATE_LIMIT_BURST=10
```

When rate limited, the API returns HTTP **429** with `Retry-After: 60`.

To disable rate limiting (auth still required):

```bash
export RATE_LIMIT_ENABLED=false
```

## Security notes

- Hyak compute nodes are not internet-facing; use SSH tunnel or campus VPN.
- The API key is shared-team auth, not per-user identity.
- Do not commit `secrets.env` to git.

## Health check

```bash
bash /gscratch/harvest/rithvik/harvest/infra/hyak/scripts/check_vllm_health.sh
```
