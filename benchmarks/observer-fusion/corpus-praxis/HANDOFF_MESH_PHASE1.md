# Handoff — mesh serving, phase 0→1

> Written 2026-08-31 for whoever picks this up next, human or agent.
>
> **Updated 2026-08-31: Jobs 0, 2, 3 are done.** omo runs its own tunnel
> serving `omo.praxisweb.xyz`, gated; the DNS watchdog is live **on the
> standby** (omo) and moves the apex to `praxis-omo` on a pansa outage; pansa's
> health gate is scope-aware and unit-correct, needing only a root-installed
> sudoers rule to gain real control.
>
> **The apex has not moved.** `praxisweb.xyz` is served by pansa, as it was.
> omo takes no production traffic until a failover event — which the watchdog
> now automates. The one remaining manual step is Job 1 (the secret rotation),
> plus one root command to complete Job 0.
>
> Read `docs/MESH_SERVING_PLAN.md` first for the design and its invariants.
> This file is the execution order, the artefacts, and the traps.

## Where you probably are

The development checkout `/home/gio/Praxis/praxis_webapp` **is on omo**. That
box is both the dev machine and the warm standby, and it holds two unrelated
checkouts:

| Path                             | What it is                                      |
| -------------------------------- | ----------------------------------------------- |
| `/home/gio/Praxis/praxis_webapp` | dev checkout — edit and push from here          |
| `/home/gio/praxis_webapp`        | **production** on omo — auto-pulls, do not edit |

Consequences that have caused confusion:

- `ssh omo-ts` from the dev checkout is omo connecting to itself. Harmless, but
  "remote" commands against omo are local ones.
- `hostname -s` is `omo`, so `scripts/setup-host-tunnel.sh --host omo` will
  pass its host guard **right here**. That is correct — omo is where omo's
  tunnel belongs — but it means Job 2 is runnable from the dev shell, which is
  not obvious.
- Anything you check with a bare path is the _dev_ checkout, not what omo
  serves. Deploy state is `/home/gio/praxis_webapp`, and it moves only via
  `git push` + the 5-minute timer.

## The one thing to understand before touching anything

DNS resolves a hostname to a **tunnel**, not to a connector. Cloudflare has no
per-hostname connector affinity, so on a shared tunnel **every connector
answers every hostname**.

The plan originally asked for both "several connectors on one tunnel" and "a
per-host pinned hostname". Those cannot both hold. It was tried: on 2026-08-31
air was running a second connector on pansa's tunnel, the apex was serving
`"node":"air"`, and `pansa.praxisweb.xyz` — the only way to ask whether pansa is
serving — returned **404 about half the time**. A healthy primary read as dead.

The plan is now **one tunnel per host** (invariant 2b). The cost is explicit:
failover needs a DNS watchdog again, because you give up the shared tunnel's
automatic connector drop-out.

## Order of work

```
Job 2  omo gets its own tunnel               DONE 2026-08-31
Job 3  Enable the DNS watchdog               DONE 2026-08-31 (on omo)
Job 0  Fix pansa's health gate               DONE 2026-08-31 (needs one root cmd)
Job 1  Rotate the exposed tunnel secret      needs: sudo on pansa, DNS — REMAINING
```

> **Correction, 2026-08-31.** An earlier version of this file said "Job 1
> before Job 2 is not stylistic", on the grounds that provisioning omo first
> would create credentials against a tunnel about to be deleted.
>
> **That dependency does not exist.** It was carried over from the
> shared-tunnel model, where omo would have taken credentials _for_
> `praxis-tunnel`. Under one-tunnel-per-host — which this plan now specifies —
> omo has its own tunnel (`praxis-omo`, `e513d1fb-3f97-41ad-8271-eab30e3d9efd`)
> and deleting `praxis-tunnel` cannot touch it. Job 2 was done first, without
> incident.
>
> The real remaining ordering is narrower: **Job 3 depends on Job 1** only
> because the watchdog's `PRIMARY_TUNNEL` must name a tunnel that still exists.
> Set it to `praxis-tunnel` and rotate afterwards, and the watchdog points at a
> deleted tunnel.
>
> **That dependency is handled for today, and must be re-checked after Job 1.**
> The watchdog was enabled pointing at **`praxis-tunnel`** (the live tunnel),
> NOT `praxis-edge`, because the rotation has not happened yet — `praxis-edge`
> exists but carries **no connector**, so a failback to it would route the apex
> to nothing. The moment §5h moves pansa's connector to `praxis-edge`, flip
> `PRIMARY_TUNNEL` to `praxis-edge` in **both** the running unit
> (`~/.config/systemd/user/praxis-dns-failover.service` on omo) and the repo
> template + script default. See Job 3 below.

**Job 2 is complete.** omo serves `omo.praxisweb.xyz` from its own tunnel, with
a health gate. It does **not** take apex traffic — that is now a watchdog
decision, and Job 3 automates it. See "State of the fleet" below.

## Artefacts in this repo

| Path                                      | State                                                                                                                                           |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/praxis-dns-failover.sh`          | **Live on omo** (standby). Default `PRIMARY_TUNNEL=praxis-tunnel`; flip to `praxis-edge` after Job 1.                                           |
| `scripts/setup-host-tunnel.sh`            | **Used** on omo 2026-08-31. Refuses wrong host / missing cert / dead backend.                                                                   |
| `ops/systemd/cloudflared-praxis.service`  | Template. `@CONFIG@` substituted. **Live on omo.**                                                                                              |
| `ops/systemd/praxis-dns-failover.service` | **Live on omo** as user unit. Roles via `Environment=`. `PRIMARY_TUNNEL=praxis-tunnel` (pre-Job-1).                                             |
| `ops/systemd/praxis-dns-failover.timer`   | **Live on omo**, 60s.                                                                                                                           |
| `ops/sudoers.d/praxis-tunnel-gate`        | **Ready to install on pansa** (root). NOPASSWD `start`/`stop` of `cloudflared.service` only.                                                    |
| `scripts/tunnel-health-gate.sh`           | **Scope-aware as of 2026-08-31.** Controls user units (`--user`) and system units (`sudo -n`). Live on omo; on pansa it needs the sudoers rule. |
| `scripts/praxis-failover.sh`              | **Retired.** Kept for its reasoning only. Do not enable.                                                                                        |

Already staged on the hosts, not in git:

| Path                                         | State                                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `pansa:~/.cloudflared/config-pansa-edge.yml` | Ingress for the replacement tunnel. Routing identical to live; only the tunnel id differs. |
| `pansa:~/.cloudflared/8ee0b50b-….json`       | `praxis-edge` credentials.                                                                 |

## Job 1 — rotate the exposed secret

`praxis-tunnel` (`087b548c-1357-429d-8a53-0c085de22831`) had its token pasted
into a chat transcript on 2026-08-27. Cloudflare **cannot rotate a tunnel
secret in place**. Anyone holding it can register a connector and receive live
traffic for the login page.

Replacement `praxis-edge` (`8ee0b50b-f6aa-445d-9fc5-db7035778e02`) exists and
pansa's ingress is staged. **Full procedure: `DR_RUNBOOK.md` §5h.**

The single thing to get right: **start the new connector before moving DNS.**
The obvious order — move the four records, then start it — points them at a
tunnel with no connectors, and the site is down for the gap.

Two cloudflared processes run on pansa during the cutover. That is safe and is
**not** the 2026-08-11 collision, which was two connectors on the _same_ tunnel
fighting over connIndex 0-3. Different tunnels do not interact. It transiently
bends invariant 1, which is why it is a window and not a resting state.

Rollback: re-run the `route dns` loop with `praxis-tunnel` until the old tunnel
is deleted. That deletion is what actually revokes the secret, so it is last.

**Blocked on:** `sudo` on pansa needs a password, its connector is a _system_
unit (`/etc/systemd/system/cloudflared.service`, config in `/etc/cloudflared/`),
and DNS record writes were not available to the agent that prepared this.

## Job 2 — omo gets its own tunnel

Run **on omo**. `scripts/setup-host-tunnel.sh` does the mechanical parts and
refuses several ways to get it wrong:

```bash
ssh omo-ts 'cd ~/praxis_webapp && ./scripts/setup-host-tunnel.sh --host omo --port 3002 --dry-run'
# read what it intends, then drop --dry-run
```

It refuses if `hostname -s` disagrees with `--host` (credentials must be
generated on the host that uses them — invariant 2), if `cert.pem` is missing,
or if nothing healthy answers on the given port. That last check is invariant
4: a connector in front of a dead backend is worse than no connector, because
it receives traffic and fails it.

It routes **only** `omo.praxisweb.xyz`. It does not touch the apex. It does not
add `supabase.praxisweb.xyz` (phase 2 — needs the gate to poll Kong first) and
it does not add another host's pinned hostname (invariant 5).

**omo's app is on `:3002`, not `:3001`.** Pointing the ingress at 3001
reproduces the exact mismatch that made the earlier token-tunnel attempt fail.

Then install the unit:

```bash
mkdir -p ~/.config/systemd/user
sed -e 's#@CONFIG@#/home/gio/.cloudflared/config-omo.yml#' \
  ops/systemd/cloudflared-praxis.service > ~/.config/systemd/user/cloudflared-praxis.service
systemctl --user daemon-reload
systemctl --user enable --now cloudflared-praxis praxis-tunnel-gate.timer
```

### Verify — and this is where the last person went wrong

```bash
for i in $(seq 5); do curl -s https://omo.praxisweb.xyz/health; echo; done
for i in $(seq 5); do curl -s https://pansa.praxisweb.xyz/health; echo; done
```

**Sample repeatedly.** The 2026-08-31 fault was intermittent by construction —
Cloudflare picked a connector per request — so a single successful `curl` hides
it completely. Every answer must name the host you asked for.

Do **not** drill by stopping things and watching the apex. The 2026-08-13 drill
took the apex down for a minute doing exactly that.

If any pinned hostname ever names the wrong node, a shared-tunnel arrangement
has crept back in. Check `pgrep -af cloudflared` **on every host** — not the
unit list. On 2026-08-31 air had two units and only one was the one anybody
remembered.

## Job 3 — enable the watchdog ✅ DONE 2026-08-31

`scripts/praxis-dns-failover.sh`, installed **on the standby, never the
primary**. A watchdog on the primary dies with the thing it watches; that is
the whole reason its answer can be trusted.

**Live on omo**: `~/.config/systemd/user/praxis-dns-failover.{service,timer}`
(enabled, 60s). `ExecStart` runs the production copy at
`/home/gio/praxis_webapp/scripts/praxis-dns-failover.sh`, which auto-updates
from the repo on the 5-minute timer. The unit file itself is a manual copy and
is not re-synced by `auto_update.sh`.

**State file:** `/home/gio/.local/state/praxis-failover/` (consecutive
failures/OKs + `current_target`). Reset to `praxis-tunnel` on enable, matching
the live apex.

**Watch the flip after Job 1:** `PRIMARY_TUNNEL` is `praxis-tunnel` because
`praxis-edge` has no connector yet. After §5h rotates pansa's connector onto
`praxis-edge`, update the unit's `Environment=PRIMARY_TUNNEL=...` (and the repo
template + script default) to `praxis-edge`. Until that flip, a failback will
correctly return the apex to pansa's live tunnel.

Behaviour, verified by dry run against the live fleet (both paths re-verified
2026-08-31 before enabling):

| Situation                          | Result                         |
| ---------------------------------- | ------------------------------ |
| Primary healthy                    | increments OK counter, no move |
| Primary down ×3, standby healthy   | moves apex + www together      |
| Primary down, standby also down    | stays put, logs why            |
| Primary healthy ×10 after failover | fails back                     |

Asymmetric hysteresis is deliberate: 3 failures to fail over (~3 min), 10
successes to fail back (~10 min). Flapping DNS is harder to diagnose than a few
more minutes of downtime.

```bash
# drill first — logs every decision, touches no DNS
DRY_RUN=1 bash scripts/praxis-dns-failover.sh
```

Two things it does that the retired version did not:

- **verifies the payload**, not just the status code. `/health` names the node,
  and a 200 from the wrong node is precisely the failure this design prevents.
- **moves every apex hostname together.** Moving `praxisweb.xyz` while leaving
  `www.praxisweb.xyz` behind splits the site across two origins; a partial
  failure is reported loudly rather than left half-applied.

### A correction worth reading before you change the probe

An earlier draft told the watchdog to probe the primary over Tailscale
(`http://100.65.59.18:3001/health`). **That does not work.** `praxis-backend`
binds loopback only — `src/index.ts:34`, `HOST` defaults to `127.0.0.1` — so
the probe returns `000` and the script declares a healthy pansa dead. The dry
run caught it.

It was also unnecessary: the public probe was only untrustworthy on a _shared_
tunnel. With one tunnel per host, `pansa.praxisweb.xyz` has exactly one
possible answerer.

A tailnet probe is still worth having eventually — it separates "the app is
down" from "the connector is down", which the public probe cannot. It needs a
second listener bound to the tailnet address; `app.listen` takes one address,
so this is not a matter of listing two. `HOST=0.0.0.0` would work and would
also expose the API on the LAN, which `src/index.ts:30` deliberately avoided.

**Never point `PRIMARY_HEALTH` at the apex.** The apex is what the watchdog
moves, so it would be self-referential: after a failover it would see the
standby answering and never fail back.

## Job 0 — pansa's health gate is inert (independent, and it matters more now) ✅ script done, one root step left

`scripts/tunnel-health-gate.sh` is supposed to withdraw a host's connector when
its backend stops serving. On pansa it could not: `TUNNEL_UNITS` named a
**system** unit and the gate ran as `systemctl --user`, which cannot start or
stop one. Inert since 2026-08-27 (`MESH_SERVING_PLAN.md` phase 0 item 2).

**Fixed 2026-08-31 (script side).** `tunnel-health-gate.sh` is now **scope
aware**:

- `unit_scope()` detects whether a unit lives in the user or system manager
  (`cloudflared-praxis.service` → user on omo; `cloudflared.service` → system
  on pansa).
- `unit_systemctl()` controls user units via `systemctl --user` and **system
  units via `sudo -n systemctl`**. Without privilege it returns non-zero and
  logs a clear, actionable error instead of silently claiming the connector
  was withdrawn.
- pansa's gate unit now sets `Environment=TUNNEL_UNITS=cloudflared.service`
  (the real unit name; previously it defaulted to a non-existent
  `cloudflared-praxis.service`).

**The one remaining step is root on pansa:** install the NOPASSWD sudoers
drop-in so `sudo -n systemctl start|stop cloudflared.service` works for the
gate. `ops/sudoers.d/praxis-tunnel-gate` is written and `visudo`-checked:

```bash
sudo install -m 440 -o root -g root \
  ops/sudoers.d/praxis-tunnel-gate /etc/sudoers.d/praxis-tunnel-gate
```

The rule is scoped to exactly one unit and two actions (no shell). Route (b) —
moving pansa's connector to a user unit — remains an alternative, but it
restarts the live connector and is unnecessary once the rule is in.

After the rule is installed, verify: stop a throwaway check by running the gate
with a bogus `PRAXIS_HEALTH_URL` and confirming it stops `cloudflared.service`
and restarts it on the next healthy run. Do not rehearse against the live
backend port.

## State of the fleet, 2026-08-31

Verified, not remembered:

```
apex            {"node":"pansa"}   5/5          (watchdog live on omo: praxis-tunnel ⇄ praxis-omo)
pansa.          200                5/5
omo.            200                5/5          (own tunnel, gated)
/api/blog       200
replication     omo lag=0s, air lag=0s, both slots active, lag_bytes=0
inference       pansa: colibri_local + colibri_omo
                omo:   colibri_local + colibri_pansa (engine STOPPED 2026-08-31 — see note)
                air:   colibri_pansa + colibri_omo (hosts no engine)
tunnels         praxis-tunnel  → pansa (secret exposed, being retired — Job 1)
                praxis-air     → air
                praxis-edge    → none yet (the replacement; flip PRIMARY_TUNNEL after Job 1)
                myserver       → none
watchdog        praxis-dns-failover.timer   active on omo, 60s
gate            omo: user unit, works. pansa: scope-aware, awaits sudoers rule.
```

Recent history a newcomer will otherwise trip over:

- Both database standbys were dead for over a week with every unit reporting
  `active`. The only check that catches it is `pg_replication_slots` on pansa;
  `f|lost` means reseed, not restart. `DR_RUNBOOK.md` §5g.
- omo's colibri engine was installed, correctly configured, and had **never
  been started** — empty journal, unit disabled — while pansa's config pointed
  at it, so every local-tier call paid a failed round trip.
- `COLIBRI_NODES` was documented as a list and only its first entry was read,
  capping the cluster at two engines however many were named.
- Both boxes have **AMD GPUs** and both already run a GPU `llama-server` on
  `:8081` that Axiom did not use, because the `local_vllm` provider defaults to
  `:8080`/`e4b`. The `freetoken-cluster/README.md` claim of "no GPU" is wrong.
  `docs/HOSTING_TOPOLOGY.md` has the measurements.
- **2026-08-31, omo LLM router cleanup:** `fcm-daemon`, `fcm-router`,
  `omniroute` and `openclaw-gateway` were killed and disabled on omo — four
  OpenAI-style proxies overlapping `orproxy` + the randomesh caddy, unused by
  any serving path. The same session left `colibri-olmoe` down (killed TERM at
  12:34; 8 GB peak, and omo was thrashing under its own agent load at ~load 20).
  The colibri cluster is therefore **single-engine until it is restarted** —
  the intended "prefer pansa" state. Restart with
  `systemctl --user start colibri-olmoe` when the box is quiet.

The pattern in all five: **a component reporting healthy while doing nothing
useful.** Prefer a check that asks the question directly over one that asks
whether the process is running.
