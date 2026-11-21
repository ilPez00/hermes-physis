# mirrord — local code, live cluster context

`mirrord` runs your local backend (or a local container) as if it were the
staging pod on omo: same env allowlist, same cluster DNS, a **copy** of live
traffic. The remote pod keeps serving — nothing is disrupted.

Staging lives on **omo only** (`praxis-staging` namespace, single-node k3s,
ClusterIP, no Ingress). pansa is never touched; pi is excluded (905 MB RAM,
artifact-fed). There is no prod mirrord target.

## Setup (once)

```bash
# 1. k3s staging on omo (refuses on pansa/pi):
ssh omo-ts 'bash -s' < scripts/setup-host-k3s.sh

# 2. locally: mirrord binary + kubeconfig pointing at omo k3s
mirrord --version  # 3.x
export KUBECONFIG=~/.kube/omo-staging.yaml
```

The `praxis-staging-secret` (service-role key, Stripe keys) is created by the
setup script from env vars on omo and **never committed**. `mirrord.json`
imports an allowlist only (`PORT`, `SUPABASE_URL`, …) and explicitly excludes
`*_SECRET*`, `*_TOKEN*`, salts and Stripe keys — a stolen laptop with a
kubeconfig still does not get prod secrets through mirrord.

## Recipe 1 — local dev with live context (default)

```bash
MIRRORD_LOCAL=1 mirrord exec --target deployment/praxis-backend \
  -n praxis-staging npm run dev
```

- Incoming traffic is **mirrored**: your local process sees a copy, the pod
  still answers. Responses from your machine are dropped — safe to run.
- `MIRRORD_LOCAL=1` skips the ~18 background crons locally (the pod already
  runs them; see `src/utils/backgroundGuard.ts`). Without it you double-send
  briefs and emails.
- Reach staging directly when needed:
  `kubectl -n praxis-staging port-forward svc/praxis-backend 3002:3002`.

## Recipe 2 — debug one request chain (steal, filtered)

```bash
MIRRORD_LOCAL=1 mirrord exec --target deployment/praxis-backend \
  -n praxis-staging --steal --filter "header:x-mirrord-debug=<your-name>" \
  npm run dev
```

Only requests carrying your header reach your machine; everyone else is
served by the pod. Unfiltered `--steal` on shared staging is banned (it
hijacks real traffic) — the mirrord Operator policy rejects it.

## Recipe 3 — no cluster / offline (local container)

```bash
mirrord container -- docker run --env-file .env praxis-backend:staging
```

Runs the local image with mirrord loaded, no k3s needed. Uses your local
`.env` (dev Supabase or pansa Kong over Tailscale), not the staging pod.
Use this on planes, on pi-class hardware, or before omo staging exists.

## Operator policy (team guardrails)

Once staging is shared, install the Operator on omo k3s
(`mirrord operator install`) and set: default incoming = `mirror`, `steal`
requires a header filter, env `exclude` = same deny-list as `mirrord.json`.
This is dashboard/API config on the cluster, not a repo file — nothing secret
belongs in git either way.

## Troubleshooting

| Symptom                                            | Cause / fix                                                                                                                                                                           |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MIRRORD_LOCAL` ignored, crons double-fire         | Envar must be `1`/`true`; checked in `src/index.ts` via `shouldStartBackgroundServices()`                                                                                             |
| Proxy/gateway connection error from container      | `mirrord` ≥3.232 auto-detects the host gateway; if it fails, set `container.host_gateway_detection=false` + `external_proxy.host_ip=0.0.0.0` in a local (uncommitted) config override |
| Pod resolves `localhost:54321` / LAN `192.168.x.x` | Wrong env: staging ConfigMap points at pansa Kong via Tailscale (`100.65.59.18`); LAN has no route between hosts                                                                      |
| `/health` 000 over Tailscale                       | Backend binds `127.0.0.1` on systemd hosts by design; in-cluster it runs `HOST=0.0.0.0` (Deployment). Never flip HOST on pansa/omo/air                                                |
| Lexicon empty (`data/dictionaries/` missing)       | Per-host state, nothing replicates; run `npm run dictionaries` on the machine serving, then restart                                                                                   |
| Nothing works, kubeconfig fine                     | `mirrord diagnose` first; kernel needs 4.20+, runtime Docker/containerd                                                                                                               |
