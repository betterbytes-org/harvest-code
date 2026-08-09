# HARVEST LLM Gateway (LiteLLM)

The LiteLLM gateway sits in front of vLLM on the compute node. It handles API key
auth, per-key rate limits (RPM/TPM), usage tracking, and an admin UI. The gateway is
**not** on the public internet; reach it via SSH tunnel from the UW network.

## Keys: master vs virtual

| Key | Who has it | Use for |
|-----|------------|---------|
| `LITELLM_MASTER_KEY` | Server admins only (`env/secrets.env`) | Admin UI login, create/revoke virtual keys, usage API |
| Virtual key (`sk-...`) | Individual developers / teams | OpenAI-compatible API calls (`/v1/chat/completions`, etc.) |

**Do not** share the master key with developers or use it in client code. Issue each
user or team a virtual key with its own RPM/TPM limits. Default bootstrap limits are
**60 RPM** / **100k TPM** (see `env/gateway.env.example` and
`gateway/bootstrap_admin.sh`). When exceeded, LiteLLM returns HTTP **429**.

## Admin UI

1. Find endpoint:

   ```bash
   cat /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
   ```

   Look for `GATEWAY_HOST` and `LITELLM_PORT` (default **4000**).

2. From the UW network, SSH tunnel (replace `<GATEWAY_HOST>` from the file above):

   ```bash
   ssh -L 4000:<GATEWAY_HOST>:4000 <user>@hyak.uw.edu
   ```

   If `LITELLM_PORT` is not 4000, use that port on both sides of the tunnel.

3. Open http://localhost:4000/ui (or `http://localhost:<LITELLM_PORT>/ui`).

4. Login with `LITELLM_MASTER_KEY` from `env/secrets.env`.

## Issue virtual keys

**UI:** Virtual Keys → Create Key (set RPM/TPM optional).

**CLI (bootstrap team key):**

```bash
export HARVEST_INFRA=/gscratch/harvest/rithvik/harvest/infra/hyak
bash infra/hyak/gateway/bootstrap_admin.sh
cat infra/hyak/state/virtual-keys.json
```

Distribute the `key` field to developers out-of-band. Do not commit `virtual-keys.json`.

## Usage statistics

- **UI:** Usage tab (spend by model, date, key)
- **API:** `GET /user/daily/activity` with master or admin key

Example (via tunnel):

```bash
source /gscratch/harvest/rithvik/harvest/infra/hyak/state/gateway-endpoint.env
curl "http://localhost:${LITELLM_PORT}/user/daily/activity" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}"
```

## UW network access

| Location | How to reach gateway |
|----------|---------------------|
| Hyak login node | SSH to compute node from login, or tunnel |
| UW campus (VPN) | SSH to hyak.uw.edu, then tunnel to `GATEWAY_HOST` |
| Off-campus | UW VPN + SSH tunnel |

Gateway is NOT on public internet by design.

## Client API access

Developers use their **virtual key** (not the master key) with the OpenAI-compatible
endpoint. See `doc/API-ACCESS.md` for curl and Python examples.

## Rollback to stdlib proxy

If LiteLLM or Postgres is misbehaving, disable the gateway and fall back to the
legacy stdlib proxy (`scripts/vllm_api_proxy.py` on port **18080**):

1. Set in `env/gateway.env` (copy from `env/gateway.env.example` if needed):

   ```bash
   export GATEWAY_ENABLED=false
   ```

2. Ensure `VLLM_API_KEY` is set in `env/secrets.env` (required when the gateway is off).

3. Cancel and resubmit job **04** (or restart `scripts/start_vllm_background.sh` on the
   compute node).

4. Clients use the shared `VLLM_API_KEY` and `VLLM_PROXY_PORT` (default 18080) from
   `state/vllm-endpoint.env`. Rate limits use `env/rate-limit.env` instead of
   per-key LiteLLM limits.

Verify proxy unit tests locally:

```bash
python3 infra/hyak/scripts/test_vllm_api_proxy.py
```
