# Hosting topology

> **Status (2026-10-04): the engine this document describes is RETIRED.**
> `physis-pro` is no longer a development target; `~/dev/physis-next` is the only
> Physis. The process still runs and praxis still proxies to it, so the topology
> below is accurate as an _operations_ record — but it is not the product spec,
> and nothing here should be extended or designed against. Do not delete these
> pages: they are the only record of how a live dependency is wired. See
> `AGENTS.md` for what this means for an agent.

> Verified on the machines themselves on **2026-08-27**. Everything below was
> read from `systemctl`, `docker ps`, `ss -tlnp`, `tailscale status` and
> `cloudflared tunnel info` on the hosts, not from prior documentation.
>
> Supersedes the host tables in `RUNBOOK.md` §1 and `docs/DR_RUNBOOK.md` §5b
> where they disagree. Those documents still describe `air` as the warm standby;
> that role moved to `omo`.

## Short version

Everything public runs on **one machine, `pansa`**. `pansa-ts` is not a second
host — it is the `~/.zshrc` SSH alias for `pansa` over Tailscale
(`ssh gio@100.65.59.18`). One box, two names.

> **2026-08-31 — both standbys are working again, and both were broken.** On
> 2026-08-30 the replication slots on pansa read `active=f, wal_status=lost`
> and had for over a week: omo ~2.0 days behind, air ~8.2 days. Two unrelated
> causes — omo aged past `max_slot_wal_keep_size`; air's `primary_conninfo`
> connected as `postgres` where `pg_hba.conf` grants replication only to
> `praxis_replicator`. Neither could catch up without a fresh `pg_basebackup`,
> and both have since been reseeded. Verified 2026-08-31: both slots active,
> both walreceivers streaming from their Tailscale addresses, `lag=0s` on omo
> and `1s` on air, and both carrying `blog_posts` and `planner_block_events`.
> omo's app tier — separately, 16 commits behind on a dirty tree — is also
> fixed. `DR_RUNBOOK.md` §5g has the causes and the reseed procedure; the check
> that distinguishes this state from the broken one is one query and nothing
> else reports it.

`omo` is a warm standby: it runs the app and a streaming database replica.
cloudflared **is installed and registered** there — binary, system unit and a
token in `/etc/cloudflared/token` — but the unit is `disabled` and `inactive`,
so omo carries no live connector and takes no public traffic today. Enabling it
as it stands would reproduce a known outage; see "omo's dormant connector"
below.

`air` — the standby every existing document names — is **not** offline. That
claim was wrong: on 2026-08-30 air had 26 days of uptime, a clean checkout
current with `origin/main`, and a passing auto-update, while omo's app tier was
16 commits stale. Its database replica was broken then and has since been
reseeded; as of 2026-08-31 air runs the app, a healthy streaming replica, and
its own `praxis-air` tunnel.

## The tailnet

Tailnet `tailfd9cf.ts.net`. Nine nodes, three of which matter for hosting.

| Node             | Tailscale IP     | LAN            | State (2026-08-30)         | Hosting role                                                                                                                                                                   |
| ---------------- | ---------------- | -------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pansa`          | `100.65.59.18`   | `192.168.1.15` | active — **this box**      | **Sole public origin.** Everything.                                                                                                                                            |
| `omo`            | `100.116.39.57`  | `192.168.1.7`  | active                     | Warm standby app + PG replica (reseeded 2026-08-31, `lag=0s`); colibri engine **stopped 2026-08-31**                                                                           |
| `feather-1`      | `100.94.69.0`    | —              | active via relay `fra`     | None. XFCE desktop, no services. **Hard rule (2026-08-31): NEVER a Praxis origin — client-only, out-of-building.**                                                             |
| `air`            | `100.66.51.47`   | `192.168.1.6`  | **active** via relay `fra` | App tier current. PG replica reseeded 2026-08-31 (`lag=1s`). Colibri **client** only — cannot host an engine                                                                   |
| `feather`        | `100.122.202.38` | —              | offline 1 day              | None                                                                                                                                                                           |
| `raspberrypi`    | `100.76.191.123` | —              | **active, serving**        | App node since 2026-09-29: backend `:3003`, own tunnel `praxis-pi`, pinned `pi.praxisweb.xyz`, gate. Artifact-fed (no build, no autoupdate). Also the `k1-ts` / `pi-ts` alias. |
| `administrator`  | `100.72.28.61`   | —              | offline 864 days           | None                                                                                                                                                                           |
| `oneplus-8t`     | `100.102.167.28` | —              | offline 23 days            | Client                                                                                                                                                                         |
| `realme-note-60` | `100.88.125.4`   | —              | offline 21 days            | Client                                                                                                                                                                         |

Aliases live in `~/.zshrc` (~line 723): `pansa`/`pansa-ts`, `air`/`air-ts`,
`omo`/`omo-ts`, `pi-ts`, `k1-ts`. The bare alias is the LAN address, the `-ts`
suffix the Tailscale one.

## pansa — the whole public surface

Debian 13 (trixie), x86-64, kernel 6.12.

### Public ingress

One `cloudflared` **system** service, running from
`/etc/cloudflared/config-pansa-edge.yml`. Tunnel `praxis-edge`, ID
`8ee0b50b-f6aa-445d-9fc5-db7035778e02`, one connector, origin IP
`176.206.189.159`.

> **2026-08-31 evening — §5h secret rotation applied.** Pansa now serves from
> `praxis-edge` (per-host tunnel, clean secret); the exposed `praxis-tunnel`
> connector is retired and its credential files destroyed. Its deletion is
> pending one manual step: it has account-level private network routes that the
> CLI cannot see (see bottom of §5h in DR_RUNBOOK) — clear them in the Cloudflare
> dashboard, then `cloudflared tunnel delete praxis-tunnel`. The failover
> watchdog's `PRIMARY_TUNNEL` now names `praxis-edge`. An extra ingress
> `ssh.praxisweb.xyz → ssh://localhost:22` is staged on the edge but NOT routed
> in DNS yet — completing it needs a Cloudflare Access application + policy on
> that hostname so no ungated public SSH port is exposed.

| Hostname                 | Origin                   |
| ------------------------ | ------------------------ |
| `praxisweb.xyz`          | `http://localhost:3001`  |
| `www.praxisweb.xyz`      | `http://localhost:3001`  |
| `pansa.praxisweb.xyz`    | `http://localhost:3001`  |
| `supabase.praxisweb.xyz` | `http://127.0.0.1:54321` |
| anything else            | `http_status:404`        |

> **pansa's own uplink, 2026-09-29.** All of the above hangs off pansa's single wired
> path, and that path is `enp7s0` on omo's `ethernet-shared` segment (`10.42.0.0/24`,
> gateway `10.42.0.1` = omo's `eno1`), with omo NATing pansa out. On 2026-09-29 that
> forwarding stopped while pansa kept its lease and DNS, so cloudflared could not reach
> the edge: the site was down for 13 minutes after a reboot and the tunnel flapped 252
> times before it, until `nmcli con up ethernet-shared` was run on omo by hand.
> `pansa-uplink-watch.timer` now measures that path every 30s and repairs it — including
> routing around omo to the ISP router, which pansa already shares a wire with. Full
> write-up, ladder and drill: **`docs/PANSA_UPLINK.md`**.

Other tunnels in the account, corrected 2026-08-30/31:

| Tunnel          | ID          | Connectors                                                                                                                                          |
| --------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `praxis-tunnel` | `087b548c…` | **Retired 2026-08-31.** Exposed secret; zero connectors, zero DNS; creds destroyed. Delete blocked by dashboard private routes — clear then delete. |
| `praxis-air`    | `0e74ce7a…` | **air**, live since 2026-08-30 14:19 UTC                                                                                                            |
| `praxis-edge`   | `8ee0b50b…` | **pansa**, connected live 2026-08-31 — the origin for every hostname                                                                                |
| `myserver`      | `6b43594b…` | none                                                                                                                                                |

An earlier version of this section said `praxis-air` had zero connectors. It
has not since air brought it up. `scripts/praxis-failover.sh` still names
`praxis-air` as `PRIMARY_TUNNEL`, but that script was **retired** on 2026-08-27
(`MESH_SERVING_PLAN.md` phase 0 item 5) and is not the failover path any more —
the adopted pattern is several connectors on one tunnel, where the edge drops a
dead node with no DNS change at all.

Health check is `https://praxisweb.xyz/health` — **not** `/api/health`, which
404s. The payload names the serving host, which is how you tell which node
answered:

```console
$ curl -s https://praxisweb.xyz/health
{"status":"ok","supabase_key_type":"service_role_ok","node":"pansa"}
```

### Praxis itself

| Piece          | Where                                                                                       |
| -------------- | ------------------------------------------------------------------------------------------- |
| Backend        | `praxis-backend.service` (user unit), `:3001`, `node /home/gio/praxis_webapp/dist/index.js` |
| Frontend       | Built into `client/dist`, served by the backend on the same origin                          |
| Checkout       | `/home/gio/praxis_webapp` — this is the live one                                            |
| Supabase stack | 14 Docker containers, `supabase_*_praxis_webapp`                                            |

Supabase ports: Kong `54321`, Postgres `54322`, Studio `54323`, Mailpit
`54324`, Logflare `54327`. Postgres image is `supabase/postgres:17.6.1.063`.

Timers on this host:

| Timer                      | Cadence        | Does                                                             |
| -------------------------- | -------------- | ---------------------------------------------------------------- |
| `praxis-autoupdate.timer`  | 5 min          | pull `main`, install, rebuild, restart backend                   |
| `praxis-tunnel-gate.timer` | 30 s           | withdraw this host's connector when `:3001` stops serving        |
| `pansa-uplink-watch.timer` | 30 s           | keep pansa's own path to the edge alive (`docs/PANSA_UPLINK.md`) |
| `praxis-backup.timer`      | nightly ~05:33 | `scripts/praxis-backup.sh`, fan out to replicas                  |
| `physis-autoupdate.timer`  | 15 min         | same treatment for the Physis engine                             |
| `email_watcher.timer`      | 15 min         | inbound mail triage                                              |
| `sales_lead_digest.timer`  | daily 09:00    | GTM digest                                                       |
| `pansa-update.timer`       | daily ~04:30   | OS package updates                                               |
| `minecraft-backup.timer`   | daily 04:00    | game world snapshots                                             |
| `aion-update.timer`        | daily ~04:47   | Aion self-update                                                 |

> **`ggml-rpc-server.service` is disabled (2026-10-21).** It was a user unit
> on pansa meant to be the "RandoMesh compute contributor" — the llama.cpp
> ggml RPC server on `:50052`, which would have sharded model layers across
> the mesh for split-model serving. The executable
> (`~/llama-mesh/rpc-server`) no longer exists on pansa, and omo never had
> it either, so the unit had been failing `203/EXEC` and restarting every
> three seconds since 2026-10-19 03:35 — 377,000 journal events, no
> working process. It was disabled rather than left crash-looping. The
> split-model claim it backed was never true in production, so the
> operator page was corrected to describe what actually runs: one
> OpenAI-compatible router in front of the fleet, GPU-first on omo, CPU
> fallback on pansa. **If the RPC contributor is wanted back, the binary
> has to be rebuilt first** — `systemctl --user enable --now
ggml-rpc-server.service` will just resume the loop.

### Everything else sharing this box

> **`0.0.0.0` is a violation, not a design.** The service row above is
> re-verified on 2026-10-04: `physis-pro-web` listens on `0.0.0.0:19876`, while
> every other service on this box binds loopback or its tailnet IP. The intended
> form is `PHYSIS_HOST=<tailnet-ip>`. It is recorded here rather than fixed
> because the engine is retired: the fix is to migrate praxis off this process,
> not to rebind it. Left as a live finding, deliberately visible.

Praxis is not alone on `pansa`. These compete for the same CPU, RAM, GPU and
disk, and any capacity planning has to count them:

| Service                  | Port(s)          | What                                                                                                                                                                                                                |
| ------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `llama-server`           | `127.0.0.1:8080` | llama.cpp, Gemma 4 E4B, Vulkan                                                                                                                                                                                      |
| `colibri-olmoe`          | `11435`          | OpenAI-compatible shim, Praxis local LLM tier                                                                                                                                                                       |
| `orproxy`                | `127.0.0.1:8455` | OpenRouter key-rotation proxy                                                                                                                                                                                       |
| `physis-pro-web`         | `0.0.0.0:19876`  | **RETIRED engine** (system unit `physis-engine.service`; omo mirrors it on `127.0.0.1:19876` via `physis-omo.service`). `0.0.0.0` is a known violation — it should bind the tailnet IP via `PHYSIS_HOST`; see below |
| `hermes-gateway`         | `127.0.0.1:8642` | Hermes agent gateway                                                                                                                                                                                                |
| `hermes-webui`           | `0.0.0.0:8648`   | Hermes Studio dashboard                                                                                                                                                                                             |
| `aion-node`              | `127.0.0.1:8765` | Aion headless fleet peer                                                                                                                                                                                            |
| `aion-web`               | `8742`/`8743`    | Aion web HUD                                                                                                                                                                                                        |
| `shop-web`               | `0.0.0.0:8090`   | 3D-print shop static catalog                                                                                                                                                                                        |
| `minecraft-gateway`      | `25565-25567`    | wake-on-connect for three MC servers                                                                                                                                                                                |
| `praxis-searxng` (Docker) | `127.0.0.1:8888` | metasearch — the Axiom wiki's grounding source (`SEARXNG_BASE_URL`). Container `--restart unless-stopped`, 512 MB cap, config `~/.config/searxng/settings.yml` (JSON format on) |
| `listmonk` + PG (Docker) | `127.0.0.1:9000` | newsletter                                                                                                                                                                                                          |
| `llmwiki-watch`          | —                | user unit; `llmwiki watch` reindexes `/home/gio/ai/wiki` on change. Binary `/home/gio/.cargo/bin/llmwiki` (built from `~/dev/physis-global/vendor/llmwiki`) — absent before 2026-11-19, so this unit crash-looped |
| `omo-tunnel`             | —                | reverse SSH, `omo:2222` → `pansa:22`                                                                                                                                                                                |

`tailscale serve` on `pansa` also proxies two tailnet-only ports to
`192.168.1.2` (the K1 printer): `:8080` → `:80` and `:7125` → Moonraker. This is
tailnet-only and never reaches the public tunnel.

### Physis — the commercial engine

Physis is a Rust engine (`physis` / `physis_pro` on crates.io, GitHub
`ilPez00/physis`) exposed as a product through this app. The engine runs on
**both** app hosts as `physis-pro-web` (pansa: system unit `physis-engine.service`
on `0.0.0.0:19876`; omo: user unit `physis-omo.service` on `127.0.0.1:19876`).
pansa's backend proxies the **public** surface through `/api/physis/*` to its own
local engine (see `src/routes/physisRoutes.ts`); the physis token in pansa's
`.env` (`PHYSIS_API_TOKEN`) is what the proxy uses to talk to it.

- **Public demo surface** — unauthenticated, rate-limited: `/api/physis/health`,
  `/stats`, `/machines`, `/machines/<name>`, `/classify`, `/grid`, and the
  allowlisted `/api/physis/v1/<engine-path>` proxy (OEE, maintenance, PDCA,
  coherence, dreams, semiotic grid). The allowlist is in `physisRoutes.ts`
  (`PROXY_ALLOW`) and is the public product surface.
- **Sales page** — `/physis` (`PhysisLandingPage`): live classify demo, machine
  reducers, API docs, cargo/GitHub download, pricing, and a pilot lead form
  (`POST /api/physis/lead` → `sales_leads` + listmonk mirror).
- **Checkout** — `POST /stripe/create-physis-checkout` (monthly/annual
  subscription, lifetime one-time), prices `STRIPE_PRICE_ID_PHYSIS_MONTHLY` /
  `_ANNUAL` / `_LIFETIME`. The Stripe webhook flips `profiles.physis_pro`
  (migration `2026-08-31-physis-pro.sql`, applied 2026-08-31) on payment and
  revokes it on cancel. `physis_pro` is read by the client like `is_premium`;
  RLS already exposes profiles to the owner.
- **Self-host** — `cargo install physis_pro`, or clone `ilPez00/physis` and
  `cargo build --release --features full`. Runs fully air-gapped (deterministic
  random-projection embedder by default; ONNX optional).

#### Preparing another host for Physis

`scripts/setup-host-physis.sh` provisions the engine on a host so its local
backend can serve `/api/physis/*` (pansa + omo already run it; air and pi are
the targets when back online):

```bash
# on the target host (or via ssh), from a host with the runtime build:
./scripts/setup-host-physis.sh --ref gio@100.65.59.18 [--host air] [--dry-run]
```

It copies the known-good licensed runtime build + licence from the reference
host, lays out `~/physis-runtime`, installs `physis-engine.service`, wires
`PHYSIS_API_URL`/`PHYSIS_API_TOKEN` into the app `.env`, starts the engine and
health-checks it. It refuses to run if an engine already answers on `:19876`
(pansa: `physis-engine.service`; omo: `physis-omo.service`).

**Engine update path (pansa):** `physis-autoupdate.timer` →
`physis-autoupdate.service` → `~/physis_pro/scripts/pansa-physis-auto-rebuild.sh`
(docker build of the detached commit, smoke test, swap, restart, health check,
fast-forward). **Fixed 2026-08-31:** the unit wrapped `ExecStart` in
`/usr/bin/sg docker -c "…"`, but `sg` does not exist on this box, so every run
failed with `203/EXEC`. Removed the wrapper — `gio` is already in the `docker`
group — and the unit now runs cleanly (it still correctly skips while the
`physis_pro` checkout is dirty with local edits).

## omo — warm standby

LAN `192.168.1.7`, Tailscale `100.116.39.57`. Reached over the tailnet, not the
LAN — the two hosts have no direct LAN route to each other, which is why the
replica stream is a tailnet stream.

| Piece                         | Detail                                                                                                                                                                                                                                                                                                                                                                                                        |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `praxis-backend.service`      | `:3002`, same build, `PORT=3002`. Its data layer is **pansa's** Kong over Tailscale — omo has no Supabase stack.                                                                                                                                                                                                                                                                                              |
| `praxis-replica.service`      | Physical streaming standby of pansa's Postgres on `:54332`. Vanilla PG 17 + pgvector from conda-forge, not the Supabase image (the primary's `pgaudit`/`pg_cron`/`pgsodium`/`supautils` do not exist in that build; replay needs none of them). Needs `LOCPATH=/home/gio/.locale` or it replays WAL perfectly and then refuses every connection with "database locale is incompatible with operating system". |
| `praxis-replica-health.timer` | 5 min. Three checks that must all hold: in recovery, live WAL receiver, replay lag under threshold. `systemctl is-active` says "active" for a standby that stopped replicating days ago — that is how air's replica died unnoticed.                                                                                                                                                                           |
| `praxis-autoupdate.timer`     | 5 min, same deploy path as pansa                                                                                                                                                                                                                                                                                                                                                                              |
| `k1-bridge.timer`             | 1 min, printer bridge                                                                                                                                                                                                                                                                                                                                                                                         |
| Also on the box               | `randomesh-caddy` (`:8088`, unified OpenAI gateway), system `caddy` (`:8080`, leftover — see below), `ollama` (`:11434`), `colibri-olmoe` (`:11435`, **stopped 2026-08-31**), `orproxy` (`:8455`), `llama-server` (`:8081`, GPU), `aion-mesh`, `physis-pro-web` (`:19876`), Tor (`:9050`)                                                                                                                     |

> **2026-08-31 — omo's LLM router stack reduced, colibri left down.** `omo` is a
> dev machine first and a hosting node second, and on 2026-08-31 it was being
> hammered by its own agent workload — a `physis-pro pack` pinning all 8 cores,
> a `cargo test`, three `opencode` and two `claude` sessions — with load ~20 and
> 3.7 GiB in swap. Two things were done about it:
>
> 1. **Killed and disabled the redundant LLM routers**: `fcm-daemon`,
>    `fcm-router`, `omniroute`, `openclaw-gateway`. All four were OpenAI-style
>    proxies that overlapped with what `orproxy` + the randomesh caddy already
>    do, and none of the praxis/randomesh serving paths used them. Verified
>    `inactive` + `disabled`.
> 2. **`colibri-olmoe` was left down** (killed `TERM` at 12:34, enabled but
>    inactive, 8 GB peak). Axiom is single-engine until it comes back, which is
>    the "prefer pansa" direction anyway. **Do not restart it while the box is
>    under agent load** — an 8 GB-peak process on a machine already in swap will
>    thrash. Restart when the load is clear if the two-node cluster is wanted:
>    `systemctl --user start colibri-olmoe`.
>
> Two leftovers from earlier migrations, both flagged rather than fixed (the
> first needs root): the **system `caddy`** at `:8080` is the old `air` config —
> it serves `/srv/praxis` and proxies `/api` to a `:3001` that nothing listens
> on — and a **second dev-started `physis-pro-web`** answers on `:8791`
> alongside the `physis-omo.service` instance on `:19876`.

### omo's dormant connector

> **Defused 2026-08-27** by `omo:~/scripts/cloudflared-cleanup.sh`. Both copies
> of the token are gone — `/etc/cloudflared/token` and the whole
> `/etc/cloudflared` directory with it, plus the stray `~/cf_token.txt` — so the
> unit's `--token-file` argument now names a path that does not exist and it
> cannot start a connector even if enabled. `cloudflared.service` itself is
> still present at `/etc/systemd/system/cloudflared.service` and still merely
> `disabled` rather than `masked`; the mask step was declined. Masking is the
> one outstanding piece:
>
> ```bash
> ssh omo-ts 'sudo systemctl mask cloudflared'
> ```
>
> The table and reasoning below record what was found before the cleanup, and
> why the collision it prevents is a real one. Blocker 1 no longer applies to
> omo as it now stands, but it applies in full to any host that is given this
> tunnel's credentials again.

omo **was** registered with Cloudflare. What was on the box before the cleanup:

| Thing                      | State (pre-cleanup)                                                       |
| -------------------------- | ------------------------------------------------------------------------- |
| `/usr/bin/cloudflared`     | installed 2026-07-23 (`/usr/local/bin/cloudflared` symlinks to it)        |
| `cloudflared.service`      | system unit, `ExecStart=… tunnel run --token-file /etc/cloudflared/token` |
| Unit state                 | **`disabled`, `inactive`** — no connector registered at the edge          |
| `/etc/cloudflared/token`   | present, root-only, written 2026-07-30 — **now removed**                  |
| `~/cf_token.txt`           | same token, `0600`, created 2026-08-27 — **now removed**                  |
| `~/.cloudflared/`          | does not exist                                                            |
| `cloudflared-update.timer` | disabled                                                                  |

It was dormant, not absent. That was the deliberate result of
`sudo systemctl disable --now cloudflared`, the fix from `DR_RUNBOOK.md` §5e —
omo had been running this connector on the **praxis tunnel token** from a
different public IP, with no Praxis app behind it, and Cloudflare was balancing
a share of every request onto it and returning 503. That is the 2026-08-11
incident.

**Do not simply `systemctl enable --now cloudflared` on omo.** One of the three
reasons it was disabled has since gone away; two have not:

1. **Same-token collision — confirmed 2026-08-27.** The token decodes to tunnel
   `087b548c-1357-429d-8a53-0c085de22831`, which is `praxis-tunnel` itself: the
   same tunnel pansa serves production from. Two cloudflared instances on one
   tunnel's credentials collide on connIndex 0-3 — the second displaces the
   first rather than adding to it, and stopping either then 503s the site for
   minutes (`DR_RUNBOOK.md` §5c, learned the hard way). This blocker stands.

   To re-check after any change (prints the IDs, not the secret — `cloudflared
tunnel info` takes a name or UUID and cannot read a token file):

   ```bash
   ssh omo-ts
   sudo /bin/cat /etc/cloudflared/token | python3 -c "import sys,base64,json;t=sys.stdin.read().strip();t+='='*(-len(t)%4);d=json.loads(base64.urlsafe_b64decode(t));print('tunnel:',d['t']);print('account:',d['a'])"
   ```

   Match against `cloudflared tunnel list` on pansa: `087b548c…` =
   `praxis-tunnel`, `6b43594b…` = `myserver`, `0e74ce7a…` = `praxis-air`.

   **The token is a production credential.** Anyone holding it can register a
   connector on `praxis-tunnel` and receive a share of live `praxisweb.xyz`
   traffic — which is a login page. Treat `/etc/cloudflared/token` on any host
   as secret-grade, never paste its contents anywhere, and if it is exposed,
   retire the tunnel rather than trying to rotate the secret in place:
   Cloudflare has no rotate operation for it. Retiring means creating a new
   named tunnel, moving both hosts and all four DNS records onto it, then
   `cloudflared tunnel delete 087b548c-…`.

2. **A token-run tunnel is remotely managed.** Ingress comes from the Cloudflare
   dashboard and any local `--config` is silently ignored — the trap that made
   the 2026-08-11 incident so hard to read, and the reason pansa was moved to a
   credentials file (see the comment at the top of
   `/etc/cloudflared/config-pansa.yml`). So what omo's connector would route
   cannot be determined from omo at all; it has to be read in the dashboard.
   Whatever it says, note that omo's app listens on **`:3002`**, not the `:3001`
   every existing ingress names, and that omo has no Supabase stack for
   `supabase.praxisweb.xyz` to land on.
3. **Nothing behind it** — this one _is_ fixed. omo had no Praxis app listening
   in August; it now runs the backend on `:3002`.

Doing it properly means a **separate named tunnel** for omo run from a
credentials file (as pansa does, so its ingress is local and authoritative),
pointing `praxisweb.xyz` at `localhost:3002` and proxying
`supabase.praxisweb.xyz` back to pansa's Kong over Tailscale — otherwise
database traffic lands on a host with no database roughly half the time. Plus a
`praxis-tunnel-gate` drop-in on omo, or a connector whose backend dies keeps
receiving its share of traffic.

## pi — app node (2026-09-29)

905 MB RAM, aarch64, node 20. Cannot build (`tsc` heap) and `npm install`
OOMs, so it is an **artifact consumer**: `dist/` + `client/dist/` tarballed
from pansa, `npm ci --omit=dev` once, plus four modules hand-copied
(`formidable`, `asap`, `dezalgo`, `cuid2`+`@noble` — `formidable` was
dev-only in the lock until `e0b2abad` promoted it). No `praxis-autoupdate.timer`;
refresh = re-tarball from pansa + restart. Own tunnel `praxis-pi`
(`118be0b9…`), ingress `pi.` + apex + `www` on `:3003`, `praxis-tunnel-gate`
adapted via `PORT` env. Disk 94% (`~/.hermes` 6.1 G belongs to another system —
not touched). No Supabase, no engine; colibri peers pansa+omo over tailnet.

## Axiom colibri: the inference cluster

`pansa` and `omo` run `colibri-olmoe` on `:11435`, bound to **both**
`127.0.0.1` and their Tailscale IP, so each box can reach the other's engine
over the tailnet (`pansa` `100.65.59.18`, `omo` `100.116.39.57`).

> **2026-08-31 — this is currently a one-node cluster.** omo's `colibri-olmoe`
> was stopped (TERM, 12:34) and deliberately left down while the dev box is
> under heavy agent load; see the note in the `omo` section above. Until it is
> restarted, pansa's `COLIBRI_NODES=omo=…` points at a dead peer — exactly the
> shape that used to hide in this cluster — so `leastLoaded` will skip it and
> Axiom's tier-4 calls land on `colibri_local` (pansa) with the GPU
> `llama-server` as `local_vllm` fallback. Restart order when load clears:
> `systemctl --user start colibri-olmoe`, then re-verify both engines with
> `GET /admin/axiom/colibri-health`.

**`air` is a client, not a host, and that is a hardware fact rather than a
preference.** It has 7.8 GB of RAM and sits ~4.4 GB into swap while serving the
app and a Postgres replica; the engine peaks at 4.8 GB. It reaches both engines
over Tailscale and carries `COLIBRI_LOCAL=off` so a dead `localhost:11435` is
not registered — see below for why that matters more than it looks.

| Host    | Engine | `COLIBRI_NODES`                       | Sees      |
| ------- | ------ | ------------------------------------- | --------- |
| `pansa` | yes    | `omo=…`                               | 2 engines |
| `omo`   | yes    | `pansa=…`                             | 2 engines |
| `air`   | **no** | `pansa=…,omo=…` + `COLIBRI_LOCAL=off` | 2 engines |

**`COLIBRI_NODES` is a list and every entry is used** (2026-08-30). It was
always documented as one, but only entry zero was ever read — so a third box
could be named, be running a healthy engine, and never receive a request, with
nothing in any log to say so. Each peer is now its own provider,
`colibri_<name>`. Duplicate names and duplicate URLs are dropped: provider
names key the in-flight map the scheduler sorts on, so a collision would have
two peers sharing one load counter, and one engine registered twice would be
scheduled as two.

**`COLIBRI_LOCAL=off` is not cosmetic.** `leastLoaded` breaks in-flight ties by
name, and `colibri_local` sorts ahead of `colibri_omo` and `colibri_pansa`. On
a host with no engine, every Axiom call would therefore open a refused
connection to a port nothing listens on before trying a real peer. The fallback
works, which is exactly why it would never have surfaced — it just pays for a
doomed connection first, every time.

### GPUs — and the second engine nobody had wired up

> **Corrected 2026-08-31.** `scripts/freetoken-cluster/README.md` asserts
> "omo: 14 GiB RAM, **no GPU**. pansa: 15 GiB RAM, **no GPU**." That is wrong,
> and it was repeated into this document earlier the same day without anyone
> checking `lspci`. Both boxes have AMD GPUs, and **omo's is doing useful work
> right now.**

| Host    | GPU                                      | VRAM    | ROCm state                                                        |
| ------- | ---------------------------------------- | ------- | ----------------------------------------------------------------- |
| `pansa` | RX 550/560 (Baffin, Polaris, `gfx803`)   | 4.0 GB  | `/dev/kfd` present, no `rocminfo` — gfx803 is dropped by ROCm     |
| `omo`   | **RX 6650M (Navi 23, RDNA2, `gfx1032`)** | 8.57 GB | Working — `rocminfo`/`rocm-smi` present, user in `video`+`render` |
| `air`   | none                                     | —       | —                                                                 |

**Both hosts already run a GPU `llama-server`** on `:8081` (`-ngl 99`, alias
`e2b`, `gemma4-e2b-heretic-Q4_K_M.gguf`), and until 2026-08-31 **Axiom used
neither**. The registry has had a `local_vllm` provider for exactly this since
2026-08-26, but it defaults to `localhost:8080` and model `e4b`, and both boxes
serve `:8081`/`e2b` — so it resolved to nothing on every host. Fixed by setting
`LOCAL_VLLM_BASE_URL` / `LOCAL_VLLM_MODEL` in each `.env`.

The speed difference is the reason to care, measured on identical prompts:

| Engine                   | Time to answer |
| ------------------------ | -------------- |
| omo `llama-server` (GPU) | **0.15 s**     |
| colibri olmoe (CPU)      | ~2–5 s         |
| pansa `llama-server`     | ~4 s           |

pansa's is slow because `gfx803` lost ROCm support, so `-ngl 99` falls back to
CPU. omo's RDNA2 is the real accelerator here — and its 8.57 GB of VRAM is
already **7.6 GB consumed** by that llama-server, so there is no room to move
colibri onto the GPU as well without evicting it.

**Watch out:** this model emits `reasoning_content` and reasons before
answering, so a small `max_tokens` returns `finish_reason: "length"` with an
**empty** `content`. That reads as a broken endpoint and is not one.

**FreeToken still cannot run here, but not for the reason its README gives.**
The blocker is not "no GPU" — it is that FreeToken is **CUDA-only** and these
are AMD cards, and that Qwen3.8-27B in bf16 wants ~55 GB against omo's 8.57 GB
of VRAM. `ft-node.service` will fail on start. The Caddyfile beside it has two
further problems if resurrected: it targets FreeToken's `:1919` rather than
`:11435`, and it uses LAN addresses (`192.168.1.7`, `192.168.1.15`) between
hosts with no LAN route to each other, which fails as a multi-second dial
timeout rather than a clean error. None of it is needed anyway: the
distribution it sketches is what `leastLoaded` already does in-process, without
the extra hop or a single point of failure in front of the cluster.

> **Corrected 2026-08-30.** This section used to say Axiom "fans out to both
> engines and takes whichever emits a token first". It does not, and has not
> since `leastLoaded()` landed — that would burn full CPU inference on both
> boxes for every call and discard one result. It also claimed both hosts were
> running the engine, which was false: omo had colibri **installed and never
> once started** (empty journal, unit `disabled`), so pansa's `COLIBRI_NODES`
> pointed at a dead peer and every local-tier call paid a failed round trip
> before falling back. Started 2026-08-30; both directions verified with real
> completions.

Axiom's local (tier-4) generation **distributes** work rather than racing it.
With two or more colibri endpoints enabled, `AICoachingService` orders them by
live in-flight count (`LlmProviderRegistry.leastLoaded`) and sends the task to
the least-loaded engine, falling back along that order if it fails. Under
concurrent load — a user's place scans, briefs and retro threads overlapping —
this keeps **both** boxes working in parallel instead of funnelling everything
to the local host while the standby idles.

`callRacingStream` is still the primitive underneath, but on this path it is
invoked with a **single** provider, so nothing is raced or aborted. The genuine
fan-out is `verifyColibriEndpoints()`, which probes every engine at boot for
the health line and is informational only.

That distinction matters for capacity planning: this is a two-node cluster with
roughly twice the throughput of one box, not a hedged pair with the latency of
the faster one.

- **Peer address**: `COLIBRI_NODES` in each box's `.env` — `pansa` has
  `omo=http://100.116.39.57:11435`, `omo` has `pansa=http://100.65.59.18:11435`.
  (`COLIBRI_REMOTE_BASE_URL` / `EP_COLIBRI_REMOTE` also work.) MagicDNS hostnames
  like `omo-ts` do **not** resolve here, so use the Tailscale IPs.
- `colibri_remote` is **auto-enabled only when a peer is configured** and
  disabled on a single-box deploy (an unconfigured remote must never fire a
  request at a relative URL).
- **Verify both engines answer** (not just the race winner):
  - `GET /admin/axiom/colibri-health` (admin) returns per-engine
    `{reachable, latencyMs}` for every enabled colibri endpoint.
  - Or at startup the backend logs `Colibri endpoints: colibri_local=ok(..)
colibri_remote=ok(..)` — `journalctl --user -u praxis-backend | grep Colibri`.
- The independent probe is `LlmProviderRegistry.verifyColibriEndpoints()`; the
  race itself is `runWithFallback → callRacingStream`.

## What this topology actually protects against

Stated plainly, because the documents it replaces were optimistic:

- **Praxis process dies on pansa** — covered. `Restart=always`, plus the tunnel
  gate withdraws the connector so the edge stops sending it traffic.
- **Bad deploy** — partly. Autoupdate rebuilds every 5 minutes on both hosts, so
  a bad commit reaches both. Rollback is a `git revert` and one more cycle.
- **pansa's disk dies** — data covered (nightly encrypted archives on `omo`,
  and the streaming replica), service **not**. Promoting omo's replica gives a
  database, not a Supabase stack: PostgREST, GoTrue, Kong and Storage all live
  only on pansa.
- **pansa is off / loses power / loses its uplink** — **not covered.** One
  connector, one origin, one house. The site is down.
- **The house loses power or internet** — not covered. Every node that hosts
  anything is in the same building on the same uplink.

## Where this is going — fragmenting across the mesh

Intent recorded 2026-08-27. **None of this is implemented.** It is written down
so the next change is made in a direction rather than at random.

The phased plan for splitting serving across `pansa`, `omo`, `air` and `pi` —
with the traps to clear first, per-node roles, and the invariants that keep an
absent node from degrading the site — is
[`MESH_SERVING_PLAN.md`](MESH_SERVING_PLAN.md). What follows is the summary of
what must be true for any of it to work.

The goal is to stop treating `pansa` as the machine and start treating the
tailnet as the substrate: services placed on whichever node can host them, with
the public edge able to reach more than one of them.

What has to be true before any of it works, in dependency order:

1. **More than one live connector.** Today one node is the origin for every
   hostname. Each hosting node needs its **own named tunnel run from a
   credentials file** — not a second instance on a shared token, and not a
   remotely-managed token tunnel whose ingress you cannot see from the host —
   plus its own per-host hostname (`pansa.praxisweb.xyz` is the pattern) so
   health can be asked of one node independently of where DNS currently points.
   omo is the nearest node to this: the client is already installed and
   registered, just deliberately disabled, and re-enabling it as-is is the wrong
   move for the reasons above.
2. **The data layer stops being co-located with the app tier.** Every standby
   today reaches back to pansa's Kong. Until `supabase.praxisweb.xyz` can be
   served by whichever node holds a promotable database, an app-tier failover
   moves the pages and not the data.
3. **Promotion has to produce a Supabase stack, not just a Postgres.** The
   replica on omo replays WAL correctly and is still not a substitute for the
   primary, because PostgREST/GoTrue/Kong/Storage exist only on pansa. Either
   those containers run on every candidate node, or promotion stays manual.
4. **Health has to be per-service, not per-host.** `praxis-tunnel-gate.sh`
   already has the right shape — measure whether the thing serves, not whether
   systemd calls it active. Extend that per service being placed.
5. **At least one node outside this building.** `feather-1` is the only node
   that currently reaches the tailnet over a relay rather than the LAN, and it
   hosts nothing. Everything else shares one door, one uplink and one breaker.
   **This is a gap, and `feather-1` is explicitly NOT the fix** — administrator
   decision 2026-08-31: feather stays a client-only out-of-building node and
   never serves Praxis. Out-of-building redundancy, if pursued, needs a NEW
   dedicated host (or a cloud VPS in a different region), never feather.

Sequencing note: steps 1 and 4 are worth doing on their own even if the rest
never happens — they turn a single-origin deployment into an observable one.
Steps 2 and 3 are the expensive half, and nothing in the mesh story works
without them.

## Verifying this document

```bash
tailscale status                                  # who is up
systemctl --user list-units --type=service --state=running
docker ps --format '{{.Names}}\t{{.Status}}'
ss -tlnp                                          # what listens where
cloudflared tunnel info praxis-tunnel             # connectors and origin IP
curl -s https://praxisweb.xyz/health              # which node answers
ssh omo 'systemctl --user list-timers --no-pager'
ssh omo 'systemctl is-enabled cloudflared; systemctl is-active cloudflared'
ssh omo 'ls -l /etc/cloudflared'                  # dormant != absent
```
