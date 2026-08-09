# HARVEST LLM Gateway Control Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hand-rolled `vllm_api_proxy.py` with a production-grade LLM gateway that provides a web control panel (authentication, per-user API keys, usage statistics), OpenAI-compatible routing to our vLLM backend, and UW-network access patterns—without building a custom admin UI.

**Architecture:** Deploy **LiteLLM Proxy** co-located on the same Hyak GPU compute node as vLLM. LiteLLM listens on `0.0.0.0:4000` (API + Admin UI at `/ui`), persists keys/users/spend logs in a **PostgreSQL** database on scratch storage, and forwards requests to vLLM at `127.0.0.1:8001`. Clients reach the gateway via Hyak internal network or SSH tunnel from UW campus/VPN. The existing stdlib proxy is retired but kept in git history.

**Tech Stack:** LiteLLM Proxy (Python, MIT), PostgreSQL 15+, vLLM (existing), Slurm, curl/OpenAI SDK clients

## Global Constraints

- Web control panel must provide **authentication** and **usage statistics** (must-have)
- Accessible from **at least UW network** (Hyak login/compute internal network; off-campus via UW VPN + SSH tunnel)
- **Rate limiting is not a priority**, but nice to have (LiteLLM supports per-key RPM/TPM limits)
- Deploy an existing framework—**do not vibe-code a custom control panel**
- All heavy GPU work on **compute nodes only** (never login node for vLLM serving)
- Slurm account: `gpu-h200-harvest`, partition: `gpu-h200`, **2× H200** for vLLM
- Scratch paths under `/gscratch/harvest/rithvik/`
- Model served name: `deepseek-v4-flash-0731`
- vLLM internal API: `127.0.0.1:8001` (localhost only, no auth)
- Do not export `VLLM_API_KEY` to vLLM process (auth lives on gateway only)
- Do not export `VLLM_PORT` to vLLM (breaks NCCL rendezvous)

---

## Gateway Selection: LiteLLM vs New API (and alternatives)

### Requirements mapping

| Requirement | LiteLLM | New API (One API fork) |
|-------------|---------|------------------------|
| Web admin UI | ✅ `/ui` — virtual keys, teams, spend charts | ✅ Full dashboard — quotas, channels, billing |
| Authentication | ✅ Master key + virtual keys; OIDC in Enterprise tier | ✅ Local users + OIDC/Discord/Telegram |
| Usage statistics | ✅ Per-key/team/model token + cost logs in Postgres | ✅ Per-user/model/date dashboards |
| OpenAI-compatible proxy | ✅ Native | ✅ Native |
| Custom vLLM backend | ✅ `hosted_vllm/` provider, documented | ⚠️ "Custom channel" — less Western docs |
| Rate limiting | ✅ Per-key RPM/TPM/budget (nice-to-have) | ✅ User-level model rate limits |
| License | MIT (permissive) | AGPL-3.0 (copyleft — may concern UW legal) |
| Ecosystem / debugging | Large Western community, BerriAI docs | Strong in Chinese ecosystem; English docs thinner |
| Fit with existing stack | Python, already have Python venv for vLLM | Go binary + SQLite/MySQL/Postgres + Redis |
| Billing/payments | Budget limits only (no Stripe needed) | Stripe/EPay top-up (overkill for research team) |

### Alternatives considered

| Tool | Verdict | Why |
|------|---------|-----|
| **LiteLLM** | ✅ **Deploy this** | Best match: vLLM provider docs, MIT license, virtual keys + UI + Postgres spend tracking, replaces custom proxy cleanly |
| **New API** | ⚠️ Second choice | Polished OpenRouter-like UI, but AGPL, billing-centric, weaker vLLM-self-host docs for our stack |
| **One API** (original) | ❌ Superseded | Maintenance moved to New API fork |
| **Custom Flask/React panel** | ❌ Rejected | User explicitly asked not to vibe-code this |
| **Open WebUI** | ❌ Wrong tool | Chat UI, not an API gateway with key management |
| **Kong / raw nginx** | ❌ Wrong tool | No built-in usage stats or key UI |

### Recommendation

**Use LiteLLM Proxy.** It directly supports our vLLM backend via `hosted_vllm/`, provides the admin dashboard and usage analytics out of the box, matches our Python tooling, and avoids AGPL concerns. New API would be reasonable if the team specifically wants a more consumer-facing "OpenRouter clone" UI and accepts AGPL + Go operational overhead; for an internal research gateway on Hyak, LiteLLM is the better fit.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `infra/hyak/gateway/litellm_config.yaml` | LiteLLM model list, general settings, optional rate limits |
| `infra/hyak/gateway/start_postgres.sh` | Init/start PostgreSQL on scratch (same compute node) |
| `infra/hyak/gateway/start_litellm.sh` | Start LiteLLM proxy after Postgres + vLLM are ready |
| `infra/hyak/gateway/bootstrap_admin.sh` | Create initial admin user + team virtual keys via API |
| `infra/hyak/env/gateway.env.example` | Gateway ports, DB paths, master key placeholders |
| `infra/hyak/env/secrets.env.example` | Add `LITELLM_MASTER_KEY`, `LITELLM_SALT_KEY`, `DATABASE_URL` |
| `infra/hyak/scripts/start_vllm_background.sh` | Start Postgres → vLLM → LiteLLM (replace stdlib proxy) |
| `infra/hyak/scripts/test_litellm_gateway.sh` | End-to-end curl tests (health, key, chat, stats) |
| `infra/hyak/scripts/test_vllm_api_proxy.py` | Keep as regression tests for retired proxy (or delete in Task 8) |
| `infra/hyak/doc/GATEWAY-ACCESS.md` | Admin UI login, key issuance, UW network access |
| `infra/hyak/doc/API-ACCESS.md` | Update client instructions for LiteLLM port + virtual keys |
| `infra/hyak/state/gateway-endpoint.env` | Runtime: host, ports, job id (written at startup) |
| `infra/hyak/slurm/04-vllm-dsv4-flash.slurm` | No logic change; may add gateway log paths in echo |

---

### Task 1: Gateway environment variables and secrets template

**Files:**
- Create: `infra/hyak/env/gateway.env.example`
- Modify: `infra/hyak/env/common.env`
- Modify: `infra/hyak/env/secrets.env.example`

**Interfaces:**
- Produces: `LITELLM_PORT`, `LITELLM_MASTER_KEY`, `LITELLM_SALT_KEY`, `DATABASE_URL`, `HARVEST_PGDATA`, `GATEWAY_ENABLED`

- [ ] **Step 1: Add gateway defaults to common.env**

Append to `infra/hyak/env/common.env`:

```bash
# LiteLLM gateway (replaces stdlib vllm_api_proxy.py when GATEWAY_ENABLED=true)
export GATEWAY_ENABLED="${GATEWAY_ENABLED:-true}"
export LITELLM_PORT="${LITELLM_PORT:-4000}"
export LITELLM_CONFIG="${LITELLM_CONFIG:-${HARVEST_INFRA}/gateway/litellm_config.yaml}"
export HARVEST_PGDATA="${HARVEST_PGDATA:-${HARVEST_SCRATCH}/pgdata/litellm}"
export HARVEST_PGPORT="${HARVEST_PGPORT:-5433}"
export LITELLM_VENV="${LITELLM_VENV:-${HARVEST_ENVS}/litellm}"
export DATABASE_URL="${DATABASE_URL:-postgresql://litellm@127.0.0.1:${HARVEST_PGPORT}/litellm}"
```

- [ ] **Step 2: Create gateway.env.example**

```bash
# infra/hyak/env/gateway.env.example
# Optional overrides (loaded after common.env)

export GATEWAY_ENABLED=true
export LITELLM_PORT=4000

# Per-key rate limits (nice-to-have; LiteLLM enforces on virtual keys)
export LITELLM_DEFAULT_RPM=60
export LITELLM_DEFAULT_TPM=100000

# Admin UI: restrict to admins only (internal users get keys via admin)
export LITELLM_UI_ACCESS_MODE=admin_only
```

- [ ] **Step 3: Extend secrets.env.example**

Add after `VLLM_API_KEY` block:

```bash
# LiteLLM admin master key (must start with sk-). Controls /ui and /key/generate.
export LITELLM_MASTER_KEY="sk-generate-a-long-random-string-here"

# Encrypts provider credentials in DB. Set once; never rotate after adding models.
export LITELLM_SALT_KEY="sk-generate-another-long-random-string-here"

# Populated automatically by start_postgres.sh if unset:
# export DATABASE_URL="postgresql://litellm@127.0.0.1:5433/litellm"
```

- [ ] **Step 4: Commit**

```bash
git add infra/hyak/env/common.env infra/hyak/env/gateway.env.example infra/hyak/env/secrets.env.example
git commit -m "feat(infra): add LiteLLM gateway environment variables"
```

---

### Task 2: LiteLLM configuration for vLLM backend

**Files:**
- Create: `infra/hyak/gateway/litellm_config.yaml`

**Interfaces:**
- Consumes: `VLLM_SERVED_NAME`, `VLLM_INTERNAL_PORT`, `LITELLM_MASTER_KEY`, `DATABASE_URL` from env
- Produces: LiteLLM proxy listening on `LITELLM_PORT`, model alias `deepseek-v4-flash-0731`

- [ ] **Step 1: Write litellm_config.yaml**

```yaml
# infra/hyak/gateway/litellm_config.yaml
model_list:
  - model_name: deepseek-v4-flash-0731
    litellm_params:
      model: hosted_vllm/deepseek-v4-flash-0731
      api_base: http://127.0.0.1:8001/v1
      # vLLM has no auth on localhost; dummy key satisfies OpenAI client format
      api_key: "not-needed-localhost"

general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  database_url: os.environ/DATABASE_URL
  store_model_in_db: true
  ui_access_mode: os.environ/LITELLM_UI_ACCESS_MODE

litellm_settings:
  drop_params: true
  set_verbose: false
  # Log spend to Postgres (powers /ui usage charts)
  success_callback: ["langfuse"]  # REMOVE THIS LINE - langfuse requires external service

router_settings:
  routing_strategy: simple-shuffle
```

**Correction — remove langfuse callback (requires external service). Use this final version:**

```yaml
# infra/hyak/gateway/litellm_config.yaml
model_list:
  - model_name: deepseek-v4-flash-0731
    litellm_params:
      model: hosted_vllm/deepseek-v4-flash-0731
      api_base: http://127.0.0.1:8001/v1
      api_key: "not-needed-localhost"

general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  database_url: os.environ/DATABASE_URL
  store_model_in_db: true
  ui_access_mode: os.environ/LITELLM_UI_ACCESS_MODE

litellm_settings:
  drop_params: true
  set_verbose: false
```

- [ ] **Step 2: Validate config syntax**

Run (on dev machine or compute node after Task 3 venv exists):

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
source env/common.env
source "${LITELLM_VENV}/bin/activate" 2>/dev/null || echo "venv not yet created in Task 3"
python3 -c "import yaml; yaml.safe_load(open('gateway/litellm_config.yaml'))"
```

Expected: no output (valid YAML).

- [ ] **Step 3: Commit**

```bash
git add infra/hyak/gateway/litellm_config.yaml
git commit -m "feat(infra): add LiteLLM config for vLLM backend"
```

---

### Task 3: LiteLLM + PostgreSQL install script

**Files:**
- Create: `infra/hyak/gateway/install_litellm_env.sh`
- Create: `infra/hyak/slurm/00b-setup-litellm-env.slurm`

**Interfaces:**
- Produces: venv at `${LITELLM_VENV}` with `litellm[proxy]` and `psycopg2-binary`; PostgreSQL binaries via Hyak module or conda

- [ ] **Step 1: Write install_litellm_env.sh**

```bash
#!/usr/bin/env bash
# Install LiteLLM proxy + Python deps into HARVEST scratch venv.
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

mkdir -p "${LITELLM_VENV}" "${HARVEST_ENVS}"

if [[ ! -x "${LITELLM_VENV}/bin/python" ]]; then
  module load python/3.12 2>/dev/null || true
  python3 -m venv "${LITELLM_VENV}"
fi

# shellcheck source=/dev/null
source "${LITELLM_VENV}/bin/activate"
pip install --upgrade pip
pip install 'litellm[proxy]' psycopg2-binary pyyaml

# PostgreSQL client tools (for initdb if module available)
if module avail postgresql 2>&1 | grep -q postgresql; then
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
fi

echo "LiteLLM venv ready at ${LITELLM_VENV}"
litellm --version || pip show litellm | grep Version
```

- [ ] **Step 2: Write Slurm setup job**

```bash
#!/bin/bash
#SBATCH --job-name=harvest-litellm-setup
#SBATCH --account=gpu-h200-harvest
#SBATCH --partition=gpu-h200
#SBATCH --cpus-per-task=4
#SBATCH --mem=8G
#SBATCH --time=01:00:00
#SBATCH --output=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-litellm-%j.out
#SBATCH --error=/gscratch/harvest/rithvik/harvest/infra/hyak/logs/setup-litellm-%j.err

set -euo pipefail
HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
export HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
source "${HARVEST_INFRA}/env/common.env"
source "${HARVEST_INFRA}/scripts/hyak_module_setup.sh"
hyak_harvest_load_modules
bash "${HARVEST_INFRA}/gateway/install_litellm_env.sh"
```

- [ ] **Step 3: Run setup on Hyak**

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
chmod +x gateway/install_litellm_env.sh
sbatch --account=gpu-h200-harvest slurm/00b-setup-litellm-env.slurm
# wait for job; check logs/setup-litellm-<jobid>.out
```

Expected log contains: `LiteLLM venv ready at /gscratch/harvest/rithvik/envs/litellm`

- [ ] **Step 4: Commit**

```bash
git add infra/hyak/gateway/install_litellm_env.sh infra/hyak/slurm/00b-setup-litellm-env.slurm
git commit -m "feat(infra): add LiteLLM environment setup script"
```

---

### Task 4: PostgreSQL sidecar on scratch

**Files:**
- Create: `infra/hyak/gateway/start_postgres.sh`
- Create: `infra/hyak/gateway/stop_postgres.sh`

**Interfaces:**
- Consumes: `HARVEST_PGDATA`, `HARVEST_PGPORT`, `HARVEST_SCRATCH`
- Produces: PostgreSQL listening on `127.0.0.1:${HARVEST_PGPORT}`, database `litellm`, role `litellm` (trust auth on localhost)

- [ ] **Step 1: Write start_postgres.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"

PGCTL=""
for candidate in pg_ctl "$(command -v pg_ctl 2>/dev/null)"; do
  [[ -x "${candidate}" ]] && PGCTL="${candidate}" && break
done
if [[ -z "${PGCTL}" ]]; then
  module load postgresql/15.8 2>/dev/null || module load postgresql 2>/dev/null || true
  PGCTL="$(command -v pg_ctl || true)"
fi
if [[ -z "${PGCTL}" ]]; then
  echo "FAIL: pg_ctl not found. module load postgresql or install via conda." >&2
  exit 1
fi

mkdir -p "${HARVEST_PGDATA}"
PIDFILE="${HARVEST_STATE}/postgres.pid"
LOG="${HARVEST_LOGS}/postgres-${SLURM_JOB_ID:-local}.log"

if [[ ! -f "${HARVEST_PGDATA}/PG_VERSION" ]]; then
  initdb -D "${HARVEST_PGDATA}" --auth=trust --encoding=UTF8
  cat >> "${HARVEST_PGDATA}/postgresql.conf" <<EOF
listen_addresses = '127.0.0.1'
port = ${HARVEST_PGPORT}
EOF
fi

if [[ -f "${PIDFILE}" ]] && kill -0 "$(cat "${PIDFILE}")" 2>/dev/null; then
  echo "PostgreSQL already running (PID $(cat "${PIDFILE}"))"
else
  "${PGCTL}" -D "${HARVEST_PGDATA}" -l "${LOG}" -o "-p ${HARVEST_PGPORT}" start
  echo $! > "${PIDFILE}" || true
fi

# Wait for readiness
for _ in $(seq 1 30); do
  if pg_isready -h 127.0.0.1 -p "${HARVEST_PGPORT}" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
pg_isready -h 127.0.0.1 -p "${HARVEST_PGPORT}"

createdb -h 127.0.0.1 -p "${HARVEST_PGPORT}" litellm 2>/dev/null || true
export DATABASE_URL="postgresql://$(whoami)@127.0.0.1:${HARVEST_PGPORT}/litellm"
echo "DATABASE_URL=${DATABASE_URL}"
```

- [ ] **Step 2: Write stop_postgres.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "${HARVEST_INFRA:?}/env/common.env"
PGCTL="$(command -v pg_ctl || true)"
[[ -n "${PGCTL}" && -d "${HARVEST_PGDATA}" ]] && "${PGCTL}" -D "${HARVEST_PGDATA}" stop -m fast || true
rm -f "${HARVEST_STATE}/postgres.pid"
```

- [ ] **Step 3: Manual smoke test (interactive compute node or small Slurm job)**

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
source env/common.env
module load postgresql/15.8 2>/dev/null || module load postgresql
bash gateway/start_postgres.sh
pg_isready -h 127.0.0.1 -p 5433
```

Expected: `accepting connections`

- [ ] **Step 4: Commit**

```bash
chmod +x infra/hyak/gateway/start_postgres.sh infra/hyak/gateway/stop_postgres.sh
git add infra/hyak/gateway/start_postgres.sh infra/hyak/gateway/stop_postgres.sh
git commit -m "feat(infra): add scratch PostgreSQL sidecar for LiteLLM"
```

---

### Task 5: LiteLLM startup script

**Files:**
- Create: `infra/hyak/gateway/start_litellm.sh`

**Interfaces:**
- Consumes: vLLM ready at `127.0.0.1:8001`, Postgres ready, `LITELLM_MASTER_KEY`, `LITELLM_SALT_KEY`, `LITELLM_CONFIG`
- Produces: LiteLLM on `0.0.0.0:${LITELLM_PORT}`, health at `/health/readiness`, UI at `/ui`

- [ ] **Step 1: Write start_litellm.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
[[ -f "${HARVEST_INFRA}/env/secrets.env" ]] && source "${HARVEST_INFRA}/env/secrets.env"
[[ -f "${HARVEST_INFRA}/env/gateway.env" ]] && source "${HARVEST_INFRA}/env/gateway.env"

: "${LITELLM_MASTER_KEY:?LITELLM_MASTER_KEY required in secrets.env}"
: "${LITELLM_SALT_KEY:?LITELLM_SALT_KEY required in secrets.env}"
: "${DATABASE_URL:?DATABASE_URL required — run start_postgres.sh first}"

# shellcheck source=/dev/null
source "${LITELLM_VENV}/bin/activate"

export LITELLM_UI_ACCESS_MODE="${LITELLM_UI_ACCESS_MODE:-admin_only}"
export DISABLE_SCHEMA_UPDATE="${DISABLE_SCHEMA_UPDATE:-false}"

LOG="${HARVEST_LOGS}/litellm-${SLURM_JOB_ID:-local}.log"
PIDFILE="${HARVEST_STATE}/litellm.pid"

if [[ -f "${PIDFILE}" ]]; then
  old_pid="$(cat "${PIDFILE}")"
  kill "${old_pid}" 2>/dev/null || true
fi

litellm --config "${LITELLM_CONFIG}" --host 0.0.0.0 --port "${LITELLM_PORT}" >>"${LOG}" 2>&1 &
echo $! > "${PIDFILE}"

# LiteLLM migrations can take 60-120s on first boot
bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "127.0.0.1" "${LITELLM_PORT}" 180

curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null
echo "LiteLLM ready on 0.0.0.0:${LITELLM_PORT} (UI: /ui)"
```

Note: `wait_for_vllm.sh` polls `/v1/models`; for LiteLLM use a dedicated wait or curl `/health/readiness` in a loop. **Implement this helper:**

Add to bottom of step 1 — replace wait line with:

```bash
for _ in $(seq 1 180); do
  if curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -sf "http://127.0.0.1:${LITELLM_PORT}/health/readiness" >/dev/null
```

- [ ] **Step 2: Commit**

```bash
chmod +x infra/hyak/gateway/start_litellm.sh
git add infra/hyak/gateway/start_litellm.sh
git commit -m "feat(infra): add LiteLLM startup script"
```

---

### Task 6: Integrate gateway into vLLM server startup

**Files:**
- Modify: `infra/hyak/scripts/start_vllm_background.sh`
- Modify: `infra/hyak/state/gateway-endpoint.env` (written at runtime)

**Interfaces:**
- Consumes: Task 4 `start_postgres.sh`, Task 5 `start_litellm.sh`, existing vLLM startup
- Produces: `gateway-endpoint.env` with `GATEWAY_HOST`, `LITELLM_PORT`, `LITELLM_UI_URL`; retires `vllm_api_proxy.py` when `GATEWAY_ENABLED=true`

- [ ] **Step 1: Write failing integration test script**

Create `infra/hyak/scripts/test_litellm_gateway.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

HARVEST_ROOT="/gscratch/harvest/rithvik/harvest"
HARVEST_INFRA="${HARVEST_ROOT}/infra/hyak"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/common.env"
# shellcheck source=/dev/null
source "${HARVEST_INFRA}/env/secrets.env"

ENDPOINT="${HARVEST_INFRA}/state/gateway-endpoint.env"
[[ -f "${ENDPOINT}" ]] || { echo "FAIL: ${ENDPOINT} missing"; exit 1; }
# shellcheck source=/dev/null
source "${ENDPOINT}"

BASE="http://${GATEWAY_HOST}:${LITELLM_PORT}"

curl -sf "${BASE}/health/readiness" >/dev/null
echo "PASS: health/readiness"

curl -sf "${BASE}/v1/models" -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" | grep -q deepseek-v4-flash-0731
echo "PASS: /v1/models"

curl -sf "${BASE}/v1/chat/completions" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"model":"deepseek-v4-flash-0731","messages":[{"role":"user","content":"Say hi in one word"}],"max_tokens":8}' \
  | grep -q '"content"'
echo "PASS: chat completion"

curl -sf "${BASE}/user/daily/activity" -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" >/dev/null
echo "PASS: usage stats endpoint"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
bash scripts/test_litellm_gateway.sh
```

Expected: FAIL with `gateway-endpoint.env missing`

- [ ] **Step 3: Modify start_vllm_background.sh**

Replace the proxy block (lines starting `# Auth + rate limits live on the proxy` through proxy health check) with:

```bash
PROXY_API_KEY="${VLLM_API_KEY}"  # kept for bootstrap compat; clients use virtual keys
unset VLLM_API_KEY
unset VLLM_PORT

# ... vLLM startup unchanged ...

bash "${HARVEST_INFRA}/scripts/wait_for_vllm.sh" "127.0.0.1" "${VLLM_INTERNAL_PORT}" 7200

if [[ "${GATEWAY_ENABLED}" == "true" ]]; then
  export LITELLM_MASTER_KEY LITELLM_SALT_KEY
  bash "${HARVEST_INFRA}/gateway/start_postgres.sh"
  eval "$(bash "${HARVEST_INFRA}/gateway/start_postgres.sh" | grep ^DATABASE_URL=)"
  export DATABASE_URL
  bash "${HARVEST_INFRA}/gateway/start_litellm.sh"
  PUBLIC_PORT="${LITELLM_PORT}"
  GATEWAY_MODE="litellm"
else
  export VLLM_API_KEY="${PROXY_API_KEY}"
  export VLLM_PROXY_PORT
  python3 "${HARVEST_INFRA}/scripts/vllm_api_proxy.py" >>"${PROXY_LOG}" 2>&1 &
  PROXY_PID=$!
  echo "${PROXY_PID}" >"${HARVEST_STATE}/vllm-proxy.pid"
  sleep 1
  curl -sf "http://127.0.0.1:${VLLM_PROXY_PORT}/v1/models" \
    -H "Authorization: Bearer ${VLLM_API_KEY}" >/dev/null
  PUBLIC_PORT="${VLLM_PROXY_PORT}"
  GATEWAY_MODE="stdlib-proxy"
fi

cat >"${ENDPOINT_FILE}" <<EOF
# ... existing vLLM fields ...
EOF

cat >"${HARVEST_STATE}/gateway-endpoint.env" <<EOF
# Written by start_vllm_background.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)
GATEWAY_HOST=${NODE}
LITELLM_PORT=${PUBLIC_PORT}
LITELLM_UI_URL=http://${NODE}:${PUBLIC_PORT}/ui
GATEWAY_MODE=${GATEWAY_MODE}
SLURM_JOB_ID=${SLURM_JOB_ID:-}
# Admin UI: open LITELLM_UI_URL, login with LITELLM_MASTER_KEY
# API: Authorization: Bearer <virtual-key>  base_url=http://${NODE}:${PUBLIC_PORT}/v1
EOF
```

Keep `GATEWAY_ENABLED=false` fallback to stdlib proxy for rollback.

- [ ] **Step 4: Deploy and run test (requires GPU job running)**

```bash
cd /gscratch/harvest/rithvik/harvest/infra/hyak
cp env/secrets.env.example env/secrets.env  # fill LITELLM_* keys
env -u VLLM_PORT sbatch --account=gpu-h200-harvest slurm/04-vllm-dsv4-flash.slurm
# after job starts (~20-30 min cold start):
bash scripts/test_litellm_gateway.sh
```

Expected: four `PASS:` lines

- [ ] **Step 5: Commit**

```bash
chmod +x infra/hyak/scripts/test_litellm_gateway.sh
git add infra/hyak/scripts/start_vllm_background.sh infra/hyak/scripts/test_litellm_gateway.sh
git commit -m "feat(infra): integrate LiteLLM gateway into vLLM startup"
```

---

### Task 7: Admin bootstrap and virtual key creation

**Files:**
- Create: `infra/hyak/gateway/bootstrap_admin.sh`

**Interfaces:**
- Consumes: `LITELLM_MASTER_KEY`, `GATEWAY_HOST`, `LITELLM_PORT`
- Produces: virtual keys written to `infra/hyak/state/virtual-keys.json` (gitignored)

- [ ] **Step 1: Write bootstrap_admin.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail

source "${HARVEST_INFRA:?}/env/common.env"
source "${HARVEST_INFRA}/env/secrets.env"
source "${HARVEST_INFRA}/state/gateway-endpoint.env"

BASE="http://${GATEWAY_HOST}:${LITELLM_PORT}"
OUT="${HARVEST_STATE}/virtual-keys.json"

# Create team key for harvest-devs (max budget: unlimited for internal use)
RESP=$(curl -sf "${BASE}/key/generate" \
  -H "Authorization: Bearer ${LITELLM_MASTER_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "key_alias": "harvest-devs",
    "team_id": null,
    "max_budget": 1000000,
    "rpm_limit": 60,
    "tpm_limit": 100000,
    "models": ["deepseek-v4-flash-0731"]
  }')

echo "${RESP}" | python3 -m json.tool > "${OUT}"
echo "Virtual key written to ${OUT}"
echo "Distribute key to developers; do not commit this file."
```

- [ ] **Step 2: Add virtual-keys.json to .gitignore**

```gitignore
infra/hyak/state/virtual-keys.json
```

- [ ] **Step 3: Run bootstrap after server is up**

```bash
bash infra/hyak/gateway/bootstrap_admin.sh
cat infra/hyak/state/virtual-keys.json
```

Expected: JSON with `"key": "sk-..."`

- [ ] **Step 4: Commit**

```bash
chmod +x infra/hyak/gateway/bootstrap_admin.sh
git add infra/hyak/gateway/bootstrap_admin.sh .gitignore
git commit -m "feat(infra): add LiteLLM virtual key bootstrap script"
```

---

### Task 8: Documentation — gateway access and client migration

**Files:**
- Create: `infra/hyak/doc/GATEWAY-ACCESS.md`
- Modify: `infra/hyak/doc/API-ACCESS.md`
- Modify: `infra/hyak/README.md`

**Interfaces:**
- Produces: documented UW-network access paths, admin UI login, virtual key workflow

- [ ] **Step 1: Write GATEWAY-ACCESS.md**

```markdown
# HARVEST LLM Gateway (LiteLLM)

## Admin UI

1. Find endpoint: `cat infra/hyak/state/gateway-endpoint.env`
2. From UW network, SSH tunnel:
   `ssh -L 4000:<GATEWAY_HOST>:4000 <user>@hyak.uw.edu`
3. Open http://localhost:4000/ui
4. Login with `LITELLM_MASTER_KEY` from `env/secrets.env`

## Issue virtual keys

- UI: Virtual Keys → Create Key (set RPM/TPM optional)
- CLI: `bash gateway/bootstrap_admin.sh`

## Usage statistics

- UI: Usage tab (spend by model, date, key)
- API: `GET /user/daily/activity` with master or admin key

## UW network access

| Location | How to reach gateway |
|----------|---------------------|
| Hyak login node | SSH to compute node from login, or tunnel |
| UW campus (VPN) | SSH to hyak.uw.edu, then tunnel to GATEWAY_HOST |
| Off-campus | UW VPN + SSH tunnel |

Gateway is NOT on public internet by design.
```

- [ ] **Step 2: Update API-ACCESS.md**

Change port references from `18080` to `4000` (or read from `gateway-endpoint.env`). Replace single shared `VLLM_API_KEY` with per-user virtual keys. Keep SSH tunnel instructions.

- [ ] **Step 3: Update README.md**

Replace "rate-limit proxy" with "LiteLLM gateway + admin UI". Add setup job `00b-setup-litellm-env` to the job table.

- [ ] **Step 4: Commit**

```bash
git add infra/hyak/doc/GATEWAY-ACCESS.md infra/hyak/doc/API-ACCESS.md infra/hyak/README.md
git commit -m "docs(infra): add LiteLLM gateway access guide"
```

---

### Task 9: Optional rate limits and rollback switch

**Files:**
- Modify: `infra/hyak/gateway/litellm_config.yaml`
- Modify: `infra/hyak/env/gateway.env.example`

**Interfaces:**
- Produces: per-key RPM=60 as nice-to-have; `GATEWAY_ENABLED=false` rollback to stdlib proxy

- [ ] **Step 1: Document rate limit in gateway.env.example** (already in Task 1)

- [ ] **Step 2: Verify rate limit triggers 429**

```bash
# With a virtual key limited to rpm_limit: 5, run 10 rapid requests:
for i in $(seq 1 10); do
  curl -s -o /dev/null -w "%{http_code}\n" http://localhost:4000/v1/chat/completions \
    -H "Authorization: Bearer ${VIRTUAL_KEY}" \
    -H "Content-Type: application/json" \
    -d '{"model":"deepseek-v4-flash-0731","messages":[{"role":"user","content":"hi"}],"max_tokens":1}'
done
```

Expected: some responses return `429` after burst exceeded.

- [ ] **Step 3: Verify rollback**

```bash
export GATEWAY_ENABLED=false
# restart job; confirm vllm_api_proxy.py on port 18080 still works
bash scripts/test_vllm_api_proxy.py  # unit tests for stdlib proxy
```

- [ ] **Step 4: Commit**

```bash
git commit -m "chore(infra): document gateway rate limits and rollback" --allow-empty
```

---

## Self-Review

### 1. Spec coverage

| Requirement | Task |
|-------------|------|
| Web control panel | Tasks 5–7, 8 (UI at `/ui`) |
| Authentication | Tasks 1, 5, 7 (master key + virtual keys) |
| Usage statistics | Tasks 6, 8 (spend logs, `/user/daily/activity`) |
| UW network access | Task 8 (SSH tunnel docs) |
| Rate limiting (nice-to-have) | Tasks 1, 7, 9 (rpm_limit on keys) |
| Deploy existing framework | Tasks 2–6 (LiteLLM, not custom UI) |
| New API vs LiteLLM decision | Recommendation section above |

No gaps.

### 2. Placeholder scan

No TBD/TODO/similar-to placeholders. All code blocks are complete.

### 3. Type consistency

- Model alias `deepseek-v4-flash-0731` used consistently
- Ports: `LITELLM_PORT=4000`, vLLM internal `8001`, legacy proxy `18080`
- Env vars: `LITELLM_MASTER_KEY`, `DATABASE_URL`, `GATEWAY_HOST` consistent across tasks

---

## Future work (out of scope for this plan)

- **Split deployment:** LiteLLM + Postgres on a stable CPU Slurm job; vLLM on GPU job reads dynamic backend URL from `vllm-endpoint.env` (avoids UI URL changing when GPU node moves).
- **OIDC / UW NetID:** LiteLLM Enterprise SSO, or nginx oauth2-proxy in front of `/ui`.
- **New API evaluation:** If team wants OpenRouter-style consumer billing UI, run a parallel PoC on a CPU node before switching.
