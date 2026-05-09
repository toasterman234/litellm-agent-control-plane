# Deploy

**Recommended path: AWS EKS for the sandbox cluster, Render for web + worker.**
The codebase ships scripts and a Render Blueprint for exactly this combo.

## Architecture

```
┌──────────┐   ┌──────────┐   ┌──────────────┐   ┌──────────────┐
│   web    │──▶│ postgres │   │ litellm prox │──▶│  model API   │
│ (Render) │   │ (Render  │   │   (yours)    │   │              │
└────┬─────┘   │  or Neon)│   └──────▲───────┘   └──────────────┘
     │         └──────────┘          │
     │              ▲                │
┌────▼─────┐        │         ┌──────┴───────┐
│  worker  │────────┘         │   AWS EKS    │
│ (Render) │─── kube API ────▶│  + agent-    │
└──────────┘                  │   sandbox    │
                              │   CRD        │
                              └──────────────┘
```

## Steps

1. **Provision EKS** (~15 min, one-time):
   ```bash
   AWS_ACCESS_KEY_ID=AKIA... \
   AWS_SECRET_ACCESS_KEY=... \
   AWS_REGION=us-east-1 \
     bin/eks-up.sh > kube-config.b64
   ```
   Capture the `K8S_NODE_HOST` value the script prints to stderr.

2. **Push the harness image** to ECR (or any registry your cluster nodes
   can pull from):
   ```bash
   ECR="$(aws sts get-caller-identity --query Account --output text).dkr.ecr.$AWS_REGION.amazonaws.com"
   aws ecr create-repository --repository-name opencode-sandbox || true
   aws ecr get-login-password | docker login --username AWS --password-stdin "$ECR"
   docker build -t "$ECR/opencode-sandbox:latest" harnesses/opencode/
   docker push "$ECR/opencode-sandbox:latest"
   ```

3. **Deploy to Render** via the 1-click button in
   [`render/README.md`](render/README.md). Paste:
   - `KUBE_CONFIG_B64=$(cat kube-config.b64)`
   - `K8S_NODE_HOST=<from step 1>`
   - `K8S_HARNESS_IMAGE=$ECR/opencode-sandbox:latest`
   - `LITELLM_API_BASE`, `LITELLM_API_KEY`, `LITELLM_DEFAULT_MODEL`

That's it. Worker's first reconcile tick fills the warm pool; spawns
land in <2s once primed.

For a scripted / agent-driven deploy, see
[`render/AGENTS.md`](render/AGENTS.md).

## Required env

Set on both `web` and `worker` services. The 1-click flow handles
auto-generated values; everything else is paste-once.

```ini
DATABASE_URL=                     # Render-provisioned, or your Neon
MASTER_KEY=                       # auto-gen by render.yaml
UI_USERNAME=admin

LITELLM_API_BASE=
LITELLM_API_KEY=
LITELLM_DEFAULT_MODEL=anthropic/claude-sonnet-4-6

KUBE_CONFIG_B64=                  # bin/eks-up.sh output
K8S_NODE_HOST=                    # bin/eks-up.sh output
K8S_HARNESS_IMAGE=                # ECR path

K8S_NAMESPACE=default
K8S_NODEPORT_MIN=30000
K8S_NODEPORT_MAX=32767
K8S_IMAGE_PULL_POLICY=IfNotPresent

PREINSTALLED_GITHUB_REPO=https://github.com/BerriAI/litellm
WARM_POOL_SIZE=2
```

## Rotations

- **EKS service-account token** caps at 24h. Re-run `bin/eks-up.sh`
  on a daily cron, then `PUT /v1/services/<id>/env-vars/KUBE_CONFIG_B64`
  on both Render services. Or move the cluster path to IRSA — out of
  scope for this guide.
- **MASTER_KEY** rotates by setting a new value in Render dashboard.

## Other clouds / providers

The platform speaks vanilla Kubernetes via the
[kubernetes-sigs/agent-sandbox] CRD. GKE, AKS, on-prem, k3s — all work
if you bring a kubeconfig that the platform can dial. The recommended
path above is what we test against.

[kubernetes-sigs/agent-sandbox]: https://github.com/kubernetes-sigs/agent-sandbox
