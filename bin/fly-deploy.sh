#!/usr/bin/env bash
#
# End-to-end Fly.io deploy. Stands up:
#   - Fly Postgres cluster
#   - litellm-agents-web   (this repo, Dockerfile target=runner)
#   - litellm-agents-worker (this repo, Dockerfile target=worker)
#   - k3s-backed sandbox cluster (via bin/k3s-up.sh)
#
# Brings: nothing. Re-runnable.
#
# Usage:
#   FLY_API_TOKEN=fo1_... \
#   LITELLM_API_BASE=https://your-litellm-host \
#   LITELLM_API_KEY=sk-... \
#   K8S_HARNESS_IMAGE=registry.example.com/opencode-sandbox:latest \
#     bin/fly-deploy.sh
#
# Optional env:
#   FLY_REGION         default: iad
#   PG_APP             default: litellm-agents-pg
#   WEB_APP            default: litellm-agents-web
#   WORKER_APP         default: litellm-agents-worker

set -euo pipefail

FLY_REGION="${FLY_REGION:-iad}"
PG_APP="${PG_APP:-litellm-agents-pg}"
WEB_APP="${WEB_APP:-litellm-agents-web}"
WORKER_APP="${WORKER_APP:-litellm-agents-worker}"

err()  { printf "[fly-deploy] error: %s\n" "$*" >&2; exit 1; }
info() { printf "[fly-deploy] %s\n" "$*"; }

command -v flyctl   >/dev/null || err "flyctl not installed"
command -v openssl  >/dev/null || err "openssl not installed"

: "${FLY_API_TOKEN:?FLY_API_TOKEN required}"
: "${LITELLM_API_BASE:?LITELLM_API_BASE required}"
: "${LITELLM_API_KEY:?LITELLM_API_KEY required}"
export FLY_API_TOKEN

# When K8S_HARNESS_IMAGE is unset, we build the harness image locally and
# import it into the k3s machine's containerd via SSH. K3s nodes can't
# pull from private registries without imagePullSecrets, and pushing to a
# public registry adds a Docker Hub / GHCR auth step the user shouldn't
# need for a self-contained Fly deploy.
LOCAL_BUILD=0
if [ -z "${K8S_HARNESS_IMAGE:-}" ]; then
  LOCAL_BUILD=1
  K8S_HARNESS_IMAGE="docker.io/library/opencode-sandbox:fly"
fi
export K8S_HARNESS_IMAGE

flyctl auth whoami >/dev/null 2>&1 || err "flyctl token rejected"

# ---- 1. Postgres ---------------------------------------------------------
if flyctl apps list --json | grep -q "\"$PG_APP\""; then
  info "Postgres '$PG_APP' exists; reusing"
else
  info "creating Fly Postgres '$PG_APP'"
  flyctl postgres create \
    --name "$PG_APP" \
    --region "$FLY_REGION" \
    --initial-cluster-size 1 \
    --vm-size shared-cpu-1x \
    --volume-size 3
fi

# ---- 2. Sandbox cluster (k3s) -------------------------------------------
K3S_APP="${K3S_APP:-litellm-agents-k3s}"
if [ -z "${KUBE_CONFIG_B64:-}" ] || [ -z "${K8S_NODE_HOST:-}" ]; then
  info "spinning k3s sandbox cluster (bin/k3s-up.sh)"
  KUBE_CFG_FILE=$(mktemp)
  trap 'rm -f "$KUBE_CFG_FILE"' EXIT
  FLY_APP="$K3S_APP" bin/k3s-up.sh > "$KUBE_CFG_FILE"
  KUBE_CONFIG_B64=$(cat "$KUBE_CFG_FILE")
  K8S_NODE_HOST="$K3S_APP.fly.dev"
fi

# ---- 2b. Build + import harness image into k3s --------------------------
if [ "$LOCAL_BUILD" = 1 ]; then
  info "building harness image: $K8S_HARNESS_IMAGE"
  docker build --platform linux/amd64 \
    -t "$K8S_HARNESS_IMAGE" \
    harnesses/opencode/

  info "transferring image to k3s machine (~30s)"
  TAR=$(mktemp)
  docker save "$K8S_HARNESS_IMAGE" -o "$TAR"
  # k3s ships a bundled containerd; `k3s ctr` pipes load the image
  # directly into it. We stream the tar over stdin so we don't have to
  # land it on disk in the Fly machine first.
  flyctl ssh console -a "$K3S_APP" -C "k3s ctr images import -" < "$TAR"
  rm -f "$TAR"
fi

# ---- 3. Web app ----------------------------------------------------------
if flyctl apps list --json | grep -q "\"$WEB_APP\""; then
  info "web app '$WEB_APP' exists; reusing"
else
  info "creating Fly app '$WEB_APP'"
  flyctl apps create "$WEB_APP" --org personal
fi

info "attaching Postgres to web"
flyctl postgres attach "$PG_APP" --app "$WEB_APP" 2>&1 \
  | grep -v 'already attached' || true

MASTER_KEY=$(openssl rand -hex 16)
info "setting web secrets"
flyctl secrets set --app "$WEB_APP" --stage \
  MASTER_KEY="$MASTER_KEY" \
  UI_USERNAME="admin" \
  LITELLM_API_BASE="$LITELLM_API_BASE" \
  LITELLM_API_KEY="$LITELLM_API_KEY" \
  KUBE_CONFIG_B64="$KUBE_CONFIG_B64" \
  K8S_NODE_HOST="$K8S_NODE_HOST" \
  K8S_HARNESS_IMAGE="$K8S_HARNESS_IMAGE" >/dev/null

info "deploying web"
flyctl deploy --app "$WEB_APP" --config fly.toml --remote-only

# ---- 4. Worker app -------------------------------------------------------
if flyctl apps list --json | grep -q "\"$WORKER_APP\""; then
  info "worker app '$WORKER_APP' exists; reusing"
else
  info "creating Fly app '$WORKER_APP'"
  flyctl apps create "$WORKER_APP" --org personal
fi

info "attaching Postgres to worker"
flyctl postgres attach "$PG_APP" --app "$WORKER_APP" 2>&1 \
  | grep -v 'already attached' || true

info "setting worker secrets"
flyctl secrets set --app "$WORKER_APP" --stage \
  MASTER_KEY="$MASTER_KEY" \
  UI_USERNAME="admin" \
  LITELLM_API_BASE="$LITELLM_API_BASE" \
  LITELLM_API_KEY="$LITELLM_API_KEY" \
  KUBE_CONFIG_B64="$KUBE_CONFIG_B64" \
  K8S_NODE_HOST="$K8S_NODE_HOST" \
  K8S_HARNESS_IMAGE="$K8S_HARNESS_IMAGE" >/dev/null

info "deploying worker"
flyctl deploy --app "$WORKER_APP" --config fly.worker.toml --remote-only

info ""
info "=== READY ==="
info "web:      https://$WEB_APP.fly.dev"
info "login:    MASTER_KEY=$MASTER_KEY"
info "sandbox:  https://$K8S_NODE_HOST:6443 (k3s API)"
info ""
info "Tear down:"
info "  flyctl apps destroy $WEB_APP"
info "  flyctl apps destroy $WORKER_APP"
info "  flyctl apps destroy $PG_APP"
info "  flyctl apps destroy litellm-agents-k3s"
