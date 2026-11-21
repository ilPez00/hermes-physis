# Splitting serving across the mesh — pansa, omo, air, pi

> Written 2026-08-27 against the state recorded in `docs/HOSTING_TOPOLOGY.md`.
> A plan, not a description. Each phase is separately shippable and separately
> reversible, and no phase requires the next one to be worth doing.
>
> **Revised 2026-08-31.** The original chose several connectors on one tunnel
> _and_ per-host pinned hostnames. Those are mutually exclusive — see "The
> pattern to extend" — and the combination broke `pansa.praxisweb.xyz` in
> production when air was added that way. The plan now specifies **one tunnel
> per host**, and accepts the cost that comes with it: failover needs a DNS
> watchdog again, so phase 0 item 5's retirement of `praxis-failover.sh` is
> withdrawn and replaced by phase 1b.
>
> `randomesh` was not found — no such directory under `/home/gio` on pansa or
> omo, no matching remote on any local checkout, and `gh` is unauthenticated on
> this box so the account could not be searched. If it holds a design that
> contradicts this, point me at it and this document gets revised rather than
> rewritten from scratch.
>
> **Answered 2026-08-30: it exists now, and it is empty.** `~/randomesh`
> appeared on **omo only** that morning (06:48) —
> `accounts/ docs/ machines/ programs/ scripts/ services/ users/{air,feather,omo,pansa}`
> — containing **zero files**. No unit, no process, no binary on `PATH`,
> nothing at all on pansa. It specifies no topology, so nothing here conflicts
> with it and open question 4 below is closed.

## What "without blocking function" has to mean

Two readings, both of which this plan honours:

1. **The migration must not break the working system.** Praxis serves today. No
   phase below takes the site down as a step, and every phase can be reverted by
   stopping one unit.
2. **No node's absence may block function.** `air` and `pi` are laptops and
   single-board machines that sleep, move and lose power. A design where a
   sleeping node degrades the site is worse than not using that node at all —
   that is precisely how omo caused the 2026-08-11 outage and how air's
   connector spent weeks in rotation with a dead backend behind it.

The second reading is the demanding one. It means every node joins rotation
_conditionally_, on measured health, and leaves it automatically. Membership is
earned per 30-second poll, not configured once.

## The ceiling, stated before the plan

Splitting the app tier across four nodes does **not** make Praxis four-way
available, and no phase below claims it does.

Browsers talk to two origins, not one:

| Origin                   | Serves                               | Where it can run today  |
| ------------------------ | ------------------------------------ | ----------------------- |
| `praxisweb.xyz/api`      | Express backend                      | any node with the build |
| `supabase.praxisweb.xyz` | GoTrue, PostgREST, Realtime, Storage | **pansa only**          |

`client/.env` sets `VITE_SUPABASE_URL=https://supabase.praxisweb.xyz`, so login
itself is a direct browser call to GoTrue on pansa. Fan out the app tier to four
nodes and the login page still dies with pansa.

So the honest availability statement for phases 1–3 is: **they remove the app
tier as a single point of failure, and leave the data tier as one.** That is
worth doing — it covers process crashes, bad deploys, one machine's uplink, and
maintenance on any single app node — but it is not "the site survives losing
pansa". Phase 4 is where that changes, and phase 4 is the expensive one.

## The pattern to extend, and why not the other one

Two multi-node approaches are already documented, and one of them is a trap.

**Do not use:** a second `cloudflared` started from the same **token file**. That
is `DR_RUNBOOK.md` §5c — instances collide on connIndex 0-3, the second displaces
the first rather than adding to it, and stopping either 503s the site for
minutes. A token-run tunnel also takes its ingress from the Cloudflare dashboard
and silently ignores the local `--config`, so what it routes is invisible from
the host. omo's dormant `/etc/cloudflared/token` is exactly this: confirmed
2026-08-27 to be `praxis-tunnel`'s own token.

**Do use: one tunnel per host.** Each machine runs its own named tunnel from a
credentials file generated on that machine, and DNS decides which one the apex
points at.

> **Revised 2026-08-31, after this plan's original answer was tried and broke
> production.** The first version of this section said to put several
> connectors on **one** tunnel (the §5f pattern) _and_ to keep a per-host
> pinned hostname. **Those two are mutually exclusive**, and nothing in
> Cloudflare's model makes them compatible.
>
> A tunnel's ingress is per-connector, but DNS resolves a hostname to the
> _tunnel_, not to a connector. There is no per-hostname connector affinity: on
> a shared tunnel, **every connector answers every hostname**, and Cloudflare
> picks one arbitrarily.
>
> So a host that omits `pansa.praxisweb.xyz` from its ingress — which phase 1
> originally instructed — does not decline to answer it. It answers with its
> catch-all `http_status:404`. On 2026-08-31 air was found as a second
> connector on `praxis-tunnel` doing exactly that: the apex served
> `"node":"air"`, and `pansa.praxisweb.xyz` returned **404 roughly half the
> time**. That hostname is the watchdog's only way to ask "is pansa serving?",
> so the failure mode is a live pansa reported dead — the precise inversion of
> what invariant 5 exists to prevent.

Each host therefore:

- has **its own named tunnel**, from a **credentials file generated on that
  host** (`cloudflared tunnel create praxis-<host>`), so nothing secret is
  copied between machines and the local ingress file is authoritative;
- carries a **per-host pinned hostname** routed to _its own_ tunnel, so "is
  this node serving?" has exactly one possible answerer;
- runs `praxis-tunnel-gate.timer` (30 s) so a connector whose backend stops
  serving withdraws itself in ~90 s.

**What this costs, stated plainly.** With per-host tunnels the apex points at
one tunnel at a time, so a dead node needs a **DNS change** to move traffic —
the automatic "edge simply stops having that connector" property of a shared
tunnel is not available. That means a DNS watchdog is required again.
`scripts/praxis-failover.sh` was retired in phase 0 item 5 on the assumption
that the shared-tunnel pattern made it unnecessary; that assumption is now
withdrawn. Its reasoning about hysteresis and about running the watchdog on the
standby rather than the primary is exactly what a replacement needs — see
"Phase 1b" below.

**Health checking stays on the pinned public hostname — and that is now safe.**

An earlier revision of this section said to move liveness checks onto the
tailnet. That was wrong on two counts, and a dry run of
`scripts/praxis-dns-failover.sh` against the live fleet caught it before it
shipped:

1. **It does not work.** `praxis-backend` binds loopback only —
   `src/index.ts:34`, `HOST` defaults to `127.0.0.1` — so
   `http://100.65.59.18:3001/health` is unreachable from anywhere. The probe
   returned `000` and the script declared a healthy pansa dead.
2. **It solved a problem per-host tunnels already fix.** The public probe was
   untrustworthy because on a _shared_ tunnel any connector could answer
   `pansa.praxisweb.xyz`. With one tunnel per host that hostname has exactly
   one possible answerer, so a 200 from it does mean pansa is serving.

It is also the more honest question: a host whose connector is down is not
serving users, whatever its local socket reports.

Two rules the watchdog must follow regardless:

- **Verify the payload, not just the status code.** `/health` names the node,
  and a 200 from the wrong node is the exact failure this design prevents.
- **Never probe the apex.** The apex is what the watchdog moves, so probing it
  is self-referential — after a failover it would report the standby as healthy
  and never fail back. Probe the pinned hostname.

A tailnet probe is still worth having later, because it separates "the app is
down" from "the connector is down", which the public probe cannot. It needs a
second listener bound to the tailnet address (`app.listen` takes one address,
so this is not a matter of listing two). `HOST=0.0.0.0` would also work and
also exposes the API on the LAN, which `src/index.ts:30` deliberately avoided —
do not reach for it casually.

## Node roles

Capability differs enough that identical treatment is wrong.

| Node    | Tier             | Serves                                 | Runs the build?                      | Notes                                                                                                       |
| ------- | ---------------- | -------------------------------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| `pansa` | full             | apex, `www`, `pansa.`, **`supabase.`** | yes                                  | Data tier lives here. Always primary until phase 4.                                                         |
| `omo`   | app + replica    | apex, `www`, `omo.`                    | yes                                  | Already has backend `:3002` and a streaming PG replica `:54332`. The only node that is a phase-4 candidate. |
| `air`   | app              | apex, `www`, `air.`                    | yes (56 GB free, has done it before) | Laptop. Sleeps. Was the standby until 2026-08-24.                                                           |
| `pi`    | app, constrained | apex, `www`, `pi.`                     | **assume no**                        | Specs unverified — offline since 2026-08-26. Treat as artifact-consuming until measured.                    |

`pi` needs deciding on evidence, not assumption. Two facts to establish before
giving it any role: how much RAM it has (the backend build exports
`NODE_OPTIONS=--max-old-space-size=4096`, which an ARM board with 2 GB cannot
satisfy), and its architecture (`node`, `cloudflared` and any native module must
have builds for it). If it cannot build, it can still serve — see phase 3.

Note also that `pi-ts` and `k1-ts` are the same address (`100.76.191.123`) with
different users. The K1 printer itself is _not_ a tailnet node: it is reached by
`tailscale serve` bridges from pansa and omo pointed at `K1-2410.local` on the
LAN. Do not conflate the two when assigning roles.

## Phases

### Phase 0 — clear the traps that are already armed

None of this adds capability. All of it prevents the next phase from
reproducing a past outage. **Do this first.**

**Status 2026-08-27: items 1, 3 and 5 are done. Item 2 is deliberately not done.
Item 4 is a decision, still open.** Nothing done in this phase touched serving —
connector ID, origin IP and `/health` were identical before and after each step.

1. **Remove the duplicate connector unit on pansa.** — **DONE 2026-08-27.**
   There were two cloudflared units on this host for the same tunnel:

   | Unit                                | State              | Config                                                                   |
   | ----------------------------------- | ------------------ | ------------------------------------------------------------------------ |
   | `cloudflared.service` (system)      | **running**        | `/etc/cloudflared/config-pansa.yml` — apex, `www`, `pansa.`, `supabase.` |
   | `cloudflared-praxis.service` (user) | disabled, inactive | `~/.cloudflared/config-pansa.yml` — **apex and `www` only**              |

   `scripts/tunnel-health-gate.sh` defaults `TUNNEL_UNITS=cloudflared-praxis.service`
   and _starts_ it on the failing→healthy transition; the log shows it doing
   exactly that at 2026-08-25T14:26:19Z. So a backend blip on pansa ends with two
   connectors from one host, the second of which 404s `supabase.` and `pansa.`
   for its share of traffic. Delete the user unit and its config, or point it at
   the same file the system unit uses. Do not leave both.

   What was done: the unit file moved to
   `~/.config/systemd/user/retired-2026-08-27/` — moved rather than deleted,
   because it is the reference for what omo's unit should look like in phase 1 —
   followed by `systemctl --user daemon-reload`. The two stale ingress files in
   `~/.cloudflared/` were renamed `config-pansa.yml.retired` and
   `config.yml.retired`. The directory itself stays: `cert.pem` there is the
   account origin certificate that `cloudflared tunnel list` and
   `tunnel route dns` need, and `praxis-tunnel.json` is pansa's credentials
   file. `cert.pem` was `0664` and is now `0600` — it authorizes creating and
   deleting tunnels and DNS records across the zone.

   The gate was then run by hand: exit 0, no action, no new log line. With no
   user cloudflared unit present it skips instead of starting anything, which is
   the safe failure mode. Connector count stayed at 1 throughout.

2. **Make the gate manage the unit that actually exists.** With the user unit
   gone, `TUNNEL_UNITS` must name `cloudflared.service` — a **system** unit,
   which `systemctl --user` cannot start or stop. Either give the gate a
   polkit rule / sudoers entry for exactly those two verbs, or move pansa's
   connector to a user unit like every other Praxis unit on these hosts. The
   second is more consistent with the rest of the deployment. Until one of them
   is done, **pansa's health gate is inert** — it logs, and stops nothing.

   **Not done, deliberately.** Both routes need root on pansa, and the user-unit
   route restarts the live connector. Neither qualifies as a change that does
   not interfere with serving, so both wait for an explicit decision. The
   consequence of leaving it: pansa is currently the only node serving, so a
   dead backend there fails requests whether or not its connector withdraws —
   502/503 versus nothing reaches the user the same way. The gate starts
   mattering at phase 1, when withdrawing one node's connector means the others
   absorb its traffic.

3. **Neutralise omo's token.** — **DONE 2026-08-27, one step outstanding.**
   `/etc/cloudflared/token` on omo was `praxis-tunnel`'s own token, and the unit
   was one `systemctl enable` away from the §5c collision. Both copies are now
   gone — the token file, the whole `/etc/cloudflared` directory with it, and
   the stray `~/cf_token.txt` — so the unit's `--token-file` argument names a
   path that no longer exists and it cannot register a connector even if
   enabled. No cloudflared process runs on the host, and `praxis-backend` and
   `praxis-replica` were untouched.

   Done with `omo:~/scripts/cloudflared-cleanup.sh`, which refuses to run on any
   host but omo, prompts before each change, decodes the tunnel ID without
   printing the secret, and finishes by confirming `praxisweb.xyz/health` still
   answers `"node":"pansa"`. Re-runnable, and `--dry-run` inspects without sudo.

   **Outstanding:** the mask step was declined, so `cloudflared.service` still
   sits at `/etc/systemd/system/cloudflared.service`, merely `disabled`. With no
   token it is inert, but masking makes it stay that way across a package
   update:

   ```bash
   ssh omo-ts 'sudo systemctl mask cloudflared'
   ```

   Masking does not block phase 1 — that installs a _user_ unit under a
   different name.

   No copy of the token was kept, deliberately. It is recoverable from the
   account on a host holding `cert.pem`: `cloudflared tunnel token praxis-tunnel`.

4. **Treat the tunnel secret as exposed.** It was pasted into a chat transcript
   on 2026-08-27. Cloudflare cannot rotate a tunnel secret in place; retiring
   means creating a new named tunnel, moving all four DNS records, then
   `cloudflared tunnel delete 087b548c-…`. Anyone holding it can register a
   connector and receive live traffic for the login page. Sequence this before
   phase 1 if it is going to happen at all — doing it after means redoing every
   host's credentials.

5. **Fix `scripts/praxis-failover.sh`'s constants or retire it.** — **DONE
   2026-08-27, retired.** It named `PRIMARY_TUNNEL=praxis-air` and
   `STANDBY_TUNNEL=praxis-tunnel`, true for about two days in August; run as
   written today it would repoint the apex at a tunnel with zero connectors.
   Under the §5f pattern it has no job at all. Both its units were already
   `disabled`/`inactive`; the script now carries a RETIRED header saying why,
   and the stale "air serves, pansa stands by" comment is marked as no longer
   true. The code is kept rather than deleted for its reasoning about
   hysteresis, and about running a watchdog on the standby rather than the
   primary — both of which apply to whatever replaces it.

### Phase 1 — omo gets its own tunnel

The smallest change that removes the app tier as a single point of failure.
Rewritten 2026-08-31 for per-host tunnels; the previous version put omo on
`praxis-tunnel` as a second connector, which is the shape that broke
`pansa.praxisweb.xyz` when air did it.

**Sequencing note.** `praxis-tunnel`'s secret is exposed (phase 0 item 4) and
its replacement `praxis-edge` (`8ee0b50b…`) already exists with pansa's ingress
staged. Retire the secret **first** — `DR_RUNBOOK.md` §5h — or omo's
credentials get created against a tunnel that is about to be deleted.

1. **On omo**, create omo's own tunnel. Nothing secret is copied between
   machines; only the account origin certificate, which authorises creating
   tunnels and DNS records across the zone, has to be present:

   ```bash
   ssh omo-ts 'cloudflared tunnel create praxis-omo'
   ```

2. Write `~/.cloudflared/config-omo.yml`. Because this is omo's **own** tunnel,
   its ingress is authoritative for its own hostnames and no other host can be
   asked to answer them:

   ```yaml
   tunnel: <praxis-omo id>
   credentials-file: /home/gio/.cloudflared/<praxis-omo id>.json
   ingress:
     - hostname: omo.praxisweb.xyz
       service: http://localhost:3002
     # The apex and www are listed so a DNS flip to this tunnel serves them
     # immediately, with no config change during an incident.
     - hostname: praxisweb.xyz
       service: http://localhost:3002
     - hostname: www.praxisweb.xyz
       service: http://localhost:3002
     - service: http_status:404
   ```

   **Do not add `pansa.praxisweb.xyz`.** On a per-host tunnel this now works
   the way invariant 5 intends: that hostname routes to pansa's tunnel only, so
   omo can never answer it, by construction rather than by omission.

   **Do not add `supabase.praxisweb.xyz`** — phase 2, and it needs a Kong check
   the gate does not yet do (invariant 4).

3. Route only omo's own hostname. The apex stays where it is:

   ```bash
   ssh omo-ts 'cloudflared tunnel route dns praxis-omo omo.praxisweb.xyz'
   ```

4. Install `cloudflared-praxis.service` as a **user** unit on omo running that
   config — from the credentials file, never `--token` — plus
   `praxis-tunnel-gate.timer` with `TUNNEL_UNITS=cloudflared-praxis.service`.
   The gate reads `PORT` from `praxis-backend.service`'s `Environment=`, so it
   polls `:3002` correctly with no change.

5. **Rotation policy is no longer a free choice.** Active-active across hosts
   was a property of one shared tunnel; with per-host tunnels the apex resolves
   to exactly one tunnel, so omo is a **reserve** that a DNS flip promotes.
   Configure the gate accordingly (`STANDBY_UNITS` +
   `PRIMARY_HEALTH_URL`), and see phase 1b for what does the flipping.

   If active-active traffic sharing is worth more than per-node
   identifiability, that is the shared-tunnel trade — but then invariant 5 has
   to be dropped deliberately and the pinned hostnames removed, not left in
   place to answer 404s half the time.

**Verification, and read this before drilling:** verify on the pinned hostnames
`pansa.praxisweb.xyz` and `omo.praxisweb.xyz`, never by stopping things and
watching the apex. The 2026-08-13 drill took the apex down for a minute doing
exactly that. Check
`/home/gio/.local/state/praxis-failover/current_target` before assuming where
the apex resolves. Sample each pinned hostname several times — the 2026-08-31
fault was only visible as an _intermittent_ 404, and a single successful curl
would have missed it.

### Phase 1b — a DNS watchdog, because per-host tunnels need one

Per-host tunnels give up the shared tunnel's automatic connector drop-out, so
something must move the apex when the primary stops serving. This is the job
`scripts/praxis-failover.sh` did before phase 0 item 5 retired it on the
assumption that it was unnecessary. That assumption is withdrawn.

Whatever replaces it must keep the three properties the original got right:

- **it runs on the standby**, not the primary, because on the primary it dies
  with the thing it watches;
- **hysteresis is asymmetric** — fail over after ~3 consecutive failures, fail
  back only after ~10 consecutive successes, because flapping DNS is harder to
  diagnose than a few more minutes of downtime;
- **it refuses to fail over to a standby that is not itself healthy.** Both
  hosts down is an outage; moving DNS only changes which host returns the
  error.

Two things must change from the original:

- its constants named `praxis-air` as `PRIMARY_TUNNEL`, which was true for
  about two days in August. Primary is pansa's tunnel — `praxis-edge` after the
  §5h cutover.
- **the payload is verified, not just the status code.** `/health` names the
  node, and a 200 from the wrong node is the failure this whole design exists
  to prevent. The original checked `%{http_code}` alone.
- **it must never probe the apex** — the apex is what it moves, so probing it
  is self-referential and it would never fail back. Probe the primary's pinned
  hostname, which under one-tunnel-per-host only the primary can answer.

**A scaffold exists**: `scripts/praxis-dns-failover.sh`, with units in
`ops/systemd/praxis-dns-failover.{service,timer}`. It is written and dry-run
against the live fleet — healthy, failover-after-3, and both-down-stay-put all
behave — but it is **not enabled anywhere** and both prerequisites (§5h
rotation, omo's own tunnel) are still outstanding. Drill it with `DRY_RUN=1`,
which logs every decision and touches no DNS.

### Phase 2 — `supabase.` gains a second path

Still one database, but no longer one _route_ to it.

Add to omo's ingress: `supabase.praxisweb.xyz` → `http://100.65.59.18:54321`
(pansa's Kong over Tailscale — **not** `192.168.1.15`, the hosts have no LAN
route to each other and a LAN IP here fails as a multi-second dial timeout
rather than a clean error).

What this buys: if pansa's _backend_ dies and its gate withdraws its connector,
`supabase.` keeps answering through omo while Kong is still up. What it does not
buy: anything at all if pansa is off. It is a route-level redundancy, not a
data-level one.

What it costs, and why it is phase 2 rather than phase 1: every `supabase.`
request served by omo takes a Tailscale round trip. Measure it before and after
(`curl -w '%{time_total}'` against both connectors' pinned hostnames) and be
prepared to revert if realtime or storage latency moves materially.

**The failure mode to design against:** a node advertising `supabase.` that
cannot reach Kong. That routes auth traffic into a void for a share of every
request — the 2026-08-11 shape. So the gate on any node serving `supabase.` must
poll **Kong**, not just the local backend. That is a new check, not one the
current script does.

### Phase 3 — air and pi join, conditionally

Same recipe as phase 1, one node at a time, with the constraints their hardware
imposes.

**air** — full recipe, no modifications. It has done this before
(`cloudflared-praxis-replica.service` was air's unit). Two things to re-check on
first bring-up, both of which bit before: a stray dev server holding port 3001
(the standby is on `:3002` precisely because of this), and `data/dictionaries/`,
which is per-host state that nothing replicates — air ran for a day with no
dictionaries while docs claimed otherwise, and the lexicon degrades silently.

**pi** — only after measuring it. If it cannot run
`NODE_OPTIONS=--max-old-space-size=4096 npm run build`, it must not run
`praxis-autoupdate.timer`, because a host that cannot complete a build will
serve an ever-staler bundle silently and indefinitely.

> **DONE 2026-09-29 as artifact consumer.** Measured: 905 MB RAM, aarch64,
> node 20, disk 94%. Backend `:3003`, tunnel `praxis-pi`, gate, pinned
> `pi.praxisweb.xyz` — serving. No autoupdate; refresh by re-tarbaling
> `dist/`+`client/dist/` from pansa. Details in `HOSTING_TOPOLOGY.md`.
> Two options were considered:

- **Artifact consumer:** pansa publishes `dist/` + `client/dist/` after a
  successful build; pi pulls the artifact and restarts. Needs a small publish
  step and a fetch script, and pi's `node` must match the build target.
- **Static-only edge:** pi serves `client/dist` for the SPA shell and proxies
  `/api` to a node that has the backend. Cheapest, and it still removes a class
  of outage — but it makes pi's usefulness depend on another app node being up,
  which weakens the "no node blocks function" property rather than strengthening
  it. Prefer the artifact route if the hardware allows.

Either way pi gets the same gate, the same pinned hostname, and the same
conditional membership. A sleeping pi must be indistinguishable from a pi that
was never configured.

### Phase 4 — the data tier stops being singular

This is what actually makes losing pansa survivable, and it is a different
class of work from phases 1–3. Recorded here as scope, not as steps.

The replica on omo replays WAL correctly and is still not a substitute for the
primary: promotion yields a **Postgres**, while browsers need a **Supabase
stack** — PostgREST, GoTrue, Kong and Storage, all of which run only on pansa.
Three things have to be true:

1. Those containers run on omo too, pointed at the local replica, started (not
   promoted) only on failover.
2. Storage blobs are replicated, not just `storage.objects` rows. These are
   separate things that disagree silently, and a missing blob answers **500**,
   never 404 — see `CLAUDE.md` and `scripts/verify_storage_urls.sh`.
3. Promotion is one command with a documented, drilled rollback, because a
   promoted replica cannot re-join as a standby without a rebuild.

Until all three hold, `supabase.` failover is manual and the honest RTO is
"however long a human takes".

> **STAGED 2026-09-29, partially live the same day.** All three exist as
> artefacts:
>
> 1. `deploy/supabase-standby/docker-compose.yml` (exact pansa images, pansa
>    service names for kong.yml upstreams) + per-host bundles in
>    `~/supabase-standby/` on omo and air (envs, kong.yml, blobs).
> 2. Hourly blob sync `praxis-blobs-sync.timer` on pansa (target `all`).
> 3. `scripts/supabase-failover.sh` — guarded promote + stack up + backend
>    repoint + **ingress repoint** (added 2026-09-29, see below), with rollback
>    notes; procedure in `docs/DR_RUNBOOK.md`.
>
> **What changed 2026-09-29** (the day pansa's uplink broke and the fleet found
> out what that costs — `randomesh/docs/INCIDENT_2026-09-29_pansa_offline.md`):
>
> - The Docker **engine** is present and active on omo; the CLI was pointing at
>   a Docker Desktop socket that does not exist, so `docker compose` failed
>   everywhere until the context was switched back to `default`.
> - omo's standby stack is **created with images pulled**
>   (`docker compose create` in `~/supabase-standby/`), so promotion is
>   `docker compose up -d` and a `pg_ctl promote`, not a pull.
> - The gate on omo no longer polls pansa's Kong. It did, and on this day that
>   made things worse in a specific way: the watchdog moved the apex to
>   `praxis-omo` while the gate counted Kong failures and stopped
>   `cloudflared-praxis` — withdrawing the only connector still serving, i.e.
>   the gate converted "pansa is down" into "the site is down". The unit file's
>   own comment said the opposite of what its `Environment=` line did. The gate
>   now measures the local backend only.
> - `supabase-failover.sh` also repoints this host's tunnel ingress
>   (`~/.cloudflared/config-omo.yml`, `supabase.` → `127.0.0.1:54321`) and
>   restarts the connector. Without it promotion produced the worst state
>   available: a perfectly healthy standby Kong on loopback while the edge
>   still dialled a dead host.
>
> Blocking what is left: the replica was reseeded on 2026-09-29 (its slot
> `omo_standby` had gone, and `praxis-replica` had been reading its own death as
> `active` for days), and one real promotion **drill** is still undone — it
> destroys the standby until a fresh `pg_basebackup` completes, so it needs
> explicit approval. Known standby degradation: realtime/functions/studio/pg-meta
> answer 502 (no containers); /auth, /rest, /storage fully served.

## Invariants

Rules that hold across every phase. Breaking any one of them reproduces a
documented outage.

1. **Never two connectors on one host.** One cloudflared per machine. Broken on
   2026-08-31: air ran both `praxis-air` and a second connector on
   `praxis-tunnel`. Check with `pgrep -af cloudflared`, not by reading units —
   there were two units and only one was the one anybody remembered.
2. **Never a shared token file.** Credentials are generated on the host that
   uses them, from a credentials file, so local ingress is authoritative.
   2b. **One tunnel per host** (2026-08-31). Not several connectors on one tunnel:
   that arrangement cannot keep invariant 5, because DNS resolves a hostname to
   a _tunnel_ and every connector on it answers every hostname. The cost is
   that failover needs DNS again — phase 1b.
3. **Never a connector without a gate.** Rotation membership is earned every
   30 s by serving, not granted at install time.
4. **Never a connector advertising a hostname it cannot serve.** Especially
   `supabase.` from a node with no route to Kong.
5. **`pansa.praxisweb.xyz` is answered only by pansa.** It is the watchdog's
   question, and any other node answering it makes a dead pansa look alive. The
   same rule applies to each new per-host hostname.

   **Omitting a hostname from a config does not satisfy this** — that was the
   2026-08-31 fault. On a shared tunnel the omitting host still receives the
   request and answers `http_status:404`, so a live pansa read as dead ~50% of
   the time. Only a per-host tunnel makes the hostname unanswerable by anyone
   else, which is why invariant 2b exists. **Sample it repeatedly:** the fault
   was intermittent by construction and one successful `curl` hides it.

6. **Cross-host addresses are Tailscale addresses.** LAN is blocked between
   these machines; a LAN IP surfaces as unexplained slowness, not an error.
7. **Drill on pinned hostnames, never the apex**, and check
   `current_target` first.
8. **A host with uncommitted changes stops deploying, silently.**
   `auto_update.sh` refuses a dirty tree and fast-forwards only, and the only
   evidence is `logs/auto_update.log` in that host's checkout. Every node added
   is another checkout that can go stale without saying so. Check the log on
   each node after any deploy that matters.

## Open questions

Answer these before phase 3; none of them block phases 0–2.

- `pi` specs: RAM, architecture, disk. Offline since 2026-08-26 — unmeasurable
  from here. **Still open.**
- ~~Whether `air` is coming back at all~~ — **answered 2026-08-30: it never
  left.** 26 days of uptime, a clean checkout current with `origin/main`, and a
  passing auto-update, while omo's app tier was the stale one. The claim that
  air had been "offline since 2026-08-24" was wrong in every document that
  carried it. Note that air is now serving its **own** tunnel (`praxis-air`,
  since 14:19 UTC that day), which predates this plan's one-tunnel decision and
  should be folded onto the shared tunnel when convenient.
- ~~Whether the exposed tunnel secret is being retired~~ — **answered
  2026-08-30: yes.** Replacement tunnel `praxis-edge`
  (`8ee0b50b-f6aa-445d-9fc5-db7035778e02`) is created and pansa's ingress is
  staged. The ordered, zero-downtime cutover is `DR_RUNBOOK.md` §5h, and it
  runs **before** omo is provisioned, exactly as this item requires. Phase 1
  below therefore targets `praxis-edge`, not `praxis-tunnel`.
- ~~Whether `randomesh` specifies a topology this should conform to~~ —
  **answered: no.** See the header. It is an empty directory skeleton.

## The inference tier is already meshed (2026-08-31)

Worth stating because this plan is about the **app** tier and says nothing
about inference, and the two are independent: Axiom's local generation is
clustered across all three boxes today, with no tunnel, no DNS and no
connector involved.

| Host    | Engine | Role                                      |
| ------- | ------ | ----------------------------------------- |
| `pansa` | yes    | serves its own + omo's                    |
| `omo`   | yes    | serves its own + pansa's                  |
| `air`   | **no** | pure client of both (`COLIBRI_LOCAL=off`) |

Three things had to be fixed to get there, all of which failed silently:

1. omo's engine was installed, correctly configured, and **had never been
   started** — empty journal, unit `disabled`. pansa's `COLIBRI_NODES` pointed
   at it regardless, so every local-tier call paid a failed round trip.
2. `COLIBRI_NODES` was read as a single value despite being a list, capping
   the cluster at two engines however many were named.
3. A host with no engine still registered `colibri_local`, which sorts ahead
   of the peers on name — so it would open a refused connection before trying
   a real one, on every call.

This obeys the plan's second reading of "without blocking function": a sleeping
node degrades nothing, because routing is by live in-flight count and a peer
that stops answering is simply passed over. `air` is the case in point — it
contributes no capacity and costs nothing when absent.

**`air` cannot host an engine**, and that is hardware, not policy: 7.8 GB RAM,
already deep in swap, against a 4.8 GB peak, while serving the app and a
replica. Neither can anything else run FreeToken — it is CUDA-only and wants
≥64 GiB or a ≥70 GB GPU, and no box here has a GPU at all. Details and the
trap in the existing Caddyfile: `docs/HOSTING_TOPOLOGY.md`.

## Corrections found while executing this plan (2026-08-30)

- **Phase 4's premise did not hold on 2026-08-30, and does again as of
  2026-08-31.** It opens "the replica on omo replays WAL correctly". That day
  it did not: omo's slot was `wal_status=lost`, as was air's, and neither host
  was a standby. Both have since been reseeded. Verified 2026-08-31: both slots
  active and reserved, both walreceivers streaming from their Tailscale
  addresses, `replica-health.sh` reporting `healthy: lag=0s` on omo and
  `lag=1s` on air, and both carrying `blog_posts` and `planner_block_events` —
  so the WAL path demonstrably delivers schema again. The lesson in
  `DR_RUNBOOK.md` §5g stands regardless: the slot check is the only thing that
  distinguishes this state from the broken one, and nothing else reports it.
- **Invariant 8 caught a live instance.** omo had been stuck 16 commits behind
  with `auto_update.sh` refusing a dirty tree, silently, exactly as that
  invariant predicts. Cleaned 2026-08-30. The lesson generalises the way the
  invariant says it does: every node added is another checkout that can go
  stale without saying so.
- **A first attempt at phase 1 got the pattern wrong** and is worth recording
  so it is not repeated. It gave omo a _separate_ tunnel with its own hostname,
  which (a) served neither the apex nor `www`, so omo would have taken no
  production traffic at all, (b) generated credentials on pansa and copied them,
  breaking invariant 2, and (c) advertised `supabase.` without the Kong check
  invariant 4 requires. The separate-tunnel shape also needs the DNS-flipping
  watchdog that phase 0 item 5 deliberately retired. The tunnel was deleted and
  its config removed from both hosts.

## Sequencing summary

| Phase | Buys                           | Cost                                                 | Reversible by                                     |
| ----- | ------------------------------ | ---------------------------------------------------- | ------------------------------------------------- |
| 0     | Removes armed traps            | An afternoon                                         | n/a — pure removal                                |
| 1     | App tier no longer single-node | Small                                                | `systemctl --user stop cloudflared-praxis` on omo |
| 2     | Second route to `supabase.`    | Latency risk, new Kong check                         | Removing one ingress rule                         |
| 3     | Two more app nodes             | Per-node bring-up; pi may need an artifact pipeline  | Stopping that node's unit                         |
| 4     | Surviving loss of pansa        | Large — a second Supabase stack and blob replication | Not casually                                      |

Phases 0 and 1 are worth doing on their own even if 2–4 never happen. Phase 4 is
the only one that changes the availability story, and nothing in it works before
0 and 1.
