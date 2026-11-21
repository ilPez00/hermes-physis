# Disaster recovery and standby hosting

> Snapshot: 2026-08-10. Production runs on one Linux host (`pansa`) behind a
> Cloudflare tunnel. This document covers what happens when that host is gone,
> and what paid hosting can and cannot do about it.
>
> **Re-verified 2026-08-27 — two facts below have changed.** (1) The warm
> standby and the streaming replica moved from `air` to `omo`, so every sentence
> naming `air` as the standby now describes `omo`. (The rest of that sentence
> used to read "`air` has been offline since 2026-08-24"; **that was wrong** —
> see §5g. air was up throughout, with a current checkout. Its _replica_ was
> dead, and so, it turned out, was omo's.) (2) `omo`
> no longer runs a live connector — the §5e fix stuck and `cloudflared.service`
> is `disabled`/`inactive` there, though the client, unit and token are all
> still installed — which means there is
> exactly **one** connector in the account and the DNS-failover path in
> `scripts/praxis-failover.sh` has no second tunnel to move to. Current per-host
> inventory: `docs/HOSTING_TOPOLOGY.md`.

---

## 1. What is backed up, and how

`scripts/backup-praxis.sh`, run nightly at 03:30 by `praxis-backup.timer`.
Output: one encrypted archive per run in `~/praxis-backups`, 30-day retention,
~31 MB each.

| Artifact               | What it is                                                          | Used for                                                         |
| ---------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `praxis.portable.sql`  | `pg_dump` of `public`, `auth`, `storage` — DDL + data, no ownership | Rebuilding the application tables in any Supabase stack          |
| `praxis.auth-data.sql` | `pg_dump --data-only` of `auth` and `storage`                       | **The accounts.** See §2 — without this a restore has zero users |
| `praxis.cluster.sql`   | `pg_dumpall`, roles included                                        | Rebuilding this exact cluster into an empty Postgres             |
| `storage.tar.gz`       | The storage docker volume                                           | Avatars and chat media                                           |
| `config/`              | `.env`, `client/.env`, both systemd units, the deployed git commit  | The keys — this is why the archive is encrypted                  |
| `row-counts.csv`       | Counts at dump time                                                 | Checking a restore against what was actually backed up           |

Encryption is GPG symmetric AES-256, passphrase at
`~/.config/praxis/backup.passphrase` (mode 600).

**The passphrase is not in the backup.** Store a copy somewhere that survives
this machine — a password manager, or printed. An encrypted archive with a lost
passphrase is a deleted archive.

### Offsite

Not configured yet, and until it is, the backups sit on the same disk as
production. To enable:

```bash
sudo apt install rclone && rclone config          # add a remote, e.g. Backblaze B2
systemctl --user edit praxis-backup.service       # add:
#   [Service]
#   Environment=PRAXIS_BACKUP_REMOTE=b2:praxis-backups
```

Roughly €0.20/month at 31 MB × 30 days. This is the single highest-value item in
this document.

---

## 2. The trap: a restore with no accounts

Verified on 2026-08-10 against a throwaway `public.ecr.aws/supabase/postgres:17.6.1.063`
container.

Restoring `praxis.portable.sql` alone finishes with exit code 0 and looks
successful:

```
goal_trees 10 ✓   checkins 23 ✓   bets 12 ✓   auth.users 0 ✗
```

Every application row is present, and every one of them references a user that
no longer exists. Nobody can log in, and the foreign keys are dangling.

The cause: a Supabase image ships its own `auth` schema, created at initdb by
GoTrue. The dump's `CREATE TABLE auth.users` therefore fails, the following
`COPY auth.users` is skipped, and psql carries on. The same happens with
`praxis.cluster.sql` unless the target cluster is genuinely empty — and a
Supabase image never is.

`praxis.auth-data.sql` exists solely to fix this, and the backup script fails
hard if the database has accounts but that file has no `COPY auth.users` line.

---

## 3. Restore procedure (verified)

Restores **44 accounts, 10 goal trees, 23 check-ins, 12 bets, 50 storage object
rows** — matching `row-counts.csv`.

> **"Storage object rows" is not "storage objects".** `praxis.portable.sql`
> carries the `storage` schema, so step 4 restores the _metadata_ whether or not
> step 5 ever runs. On 2026-08-26 production was found with 50 rows in
> `storage.objects` and **2 blobs** in the volume: every avatar, banner, chat
> image and post image described by the database had no file behind it. Requests
> returned **500** (`ENOENT` on the version path), not 404, so nothing looked
> like a missing file. It went unnoticed for months because the URLs stored in
> `profiles`, `posts`, `messages` and friends still pointed at the retired
> hosted Supabase project, which was still serving them.
>
> A row-count check passes in that state. Step 7 below therefore verifies blobs,
> not rows.

```bash
# 1. Decrypt and unpack
gpg --decrypt --passphrase-file ~/.config/praxis/backup.passphrase \
    praxis-YYYYMMDD-HHMMSS.tar.gz.gpg | tar xz

# 2. Bring up an empty Supabase stack (same image tag as the backup)
cd praxis_webapp && supabase start

# 3. Application tables first
docker exec -i supabase_db_praxis_webapp \
  psql -U supabase_admin -d postgres < <stamp>/praxis.portable.sql

# 4. Then the accounts. Truncate first so a partially-seeded auth schema does
#    not collide — the image creates a few rows of its own.
docker exec -i supabase_db_praxis_webapp psql -U supabase_admin -d postgres -c \
  "TRUNCATE auth.users, auth.identities, auth.sessions, auth.refresh_tokens,
            storage.objects, storage.buckets CASCADE;"
docker exec -i supabase_db_praxis_webapp \
  psql -U supabase_admin -d postgres < <stamp>/praxis.auth-data.sql

# 5. Storage objects
docker run --rm -v supabase_storage_praxis_webapp:/data -v "$PWD/<stamp>":/b \
  alpine tar xzf /b/storage.tar.gz -C /data

# 6. Verify against the manifest — do not skip this step
docker exec supabase_db_praxis_webapp psql -U supabase_admin -d postgres -At -F',' -c "
  SELECT 'auth.users', count(*) FROM auth.users
  UNION ALL SELECT 'goal_trees', count(*) FROM goal_trees
  UNION ALL SELECT 'checkins', count(*) FROM checkins
  UNION ALL SELECT 'bets', count(*) FROM bets ORDER BY 1;"
diff <(...) <stamp>/row-counts.csv

# 7. Verify STORAGE — blobs, not rows. Do not skip this either: step 6 passes
#    on a restore where every image is a 500.
scripts/verify_storage_urls.sh
#    Checks that nothing still references the retired hosted project, that every
#    distinct stored URL serves bytes, and that storage.objects and the file
#    backend agree in both directions. Prints "RESULT: clean" or exits non-zero.

# If the blob count is short, the objects can be recovered from wherever they
# still exist (a previous host, another backup) with:
#    DRY=0 scripts/migrate_storage_blobs.sh   # copies blobs, reconciles rows
#    psql ... < scripts/rewrite_storage_urls.sql   # ONLY after blobs verify
# Order matters: rewriting URLs before the blobs land turns working images into
# broken ones with no way back.

# 8. Restore config, rebuild, start
cp <stamp>/config/.env praxis_webapp/.env
cp <stamp>/config/client_.env praxis_webapp/client/.env
npm ci && npm run build && (cd client && npm ci && npm run build)
systemctl --user start praxis-backend
```

If the restore is on a different host, `psql -U supabase_admin` may prompt for a
password you do not have. On a machine you control, prepend
`local all all trust` to `/etc/postgresql/pg_hba.conf` inside the container and
restart it.

---

## 4. Paid hosting as a standby: removed

> **2026-10-06: there is no Vercel and no Railway.** Everything is hosted
> locally on the fleet (`pansa` primary, `omo` warm standby, `air`) behind the
> Cloudflare tunnel — see `docs/HOSTING_TOPOLOGY.md`. The Vercel-frontend and
> Railway-backend standby proposals that used to live in this section were
> never the production path and have been deleted; `client/vercel.json` and
> `railway.toml` are vestigial files. Hosted-Supabase notes below still apply
> if a _copy_ is ever considered.

### Supabase hosted — a copy, not a replica.

Managed Supabase will not act as a physical standby of a self-hosted primary:
there is no superuser, no inbound replication slot, no `pg_basebackup` target.
The only route is a scheduled logical restore of `praxis.portable.sql` +
`praxis.auth-data.sql` into a hosted project — a cold copy whose freshness is the
dump interval.

Two costs to plan for:

- Free-tier projects pause after a week of inactivity, which is precisely what a
  standby is. A useful standby needs Pro, €25/month.
- Every project has its own JWT secret. Failing over invalidates every session —
  all users are logged out — and the Google OAuth redirect URIs for the standby
  domain must be registered **in advance**, or nobody can log back in at the
  moment you need them to.

---

## 5. Recommendation

Ordered by value per euro:

| Step                                               | Cost      | Protects against                                  | RPO | RTO     |
| -------------------------------------------------- | --------- | ------------------------------------------------- | --- | ------- |
| 1. Offsite the nightly archive                     | ~€0.20/mo | Disk loss, theft, fire, ransomware                | 24h | hours   |
| 2. Second tunnel connector (omo/air)               | €0        | Tunnel/host failure (local fleet, not paid cloud) | —   | minutes |
| 3. Nightly restore into hosted Supabase Pro        | €25/mo    | Machine loss with a warm database                 | 24h | ~1h     |
| 4. _(Vercel/Railway standby — removed 2026-10-06)_ | —         | —                                                 | —   | —       |

**Do 1 and 2 now.** Step 1 is where essentially all the risk actually lives: at
one active user, an hour of downtime costs nothing, and losing the database
costs everything.

**Skip 3 and 4 until there is a paying customer or a pilot with an SLA.** €30/mo
buys a faster recovery from an outage that currently has no one to inconvenience.
Once a university pilot exists, both become mandatory rather than optional — and
by then the hosting question is likely "move primary off this machine" rather
than "add a standby to it".

---

## 5b. Replicas on the local network

Added 2026-08-11. Every nightly archive is copied to the other machines here, so
the encrypted backup no longer lives only on the disk it protects.

| Host    | Reach           | Role                                           | Notes                                              |
| ------- | --------------- | ---------------------------------------------- | -------------------------------------------------- |
| `pansa` | —               | **primary**: app, database, tunnel             | Everything runs here                               |
| `air`   | LAN + Tailscale | backup replica + **warm app standby** on :3002 | 56 GB free                                         |
| `omo`   | Tailscale only  | backup replica                                 | 153 GB free; LAN down, wakes intermittently        |
| `pi`    | unreachable     | —                                              | Off on both LAN and Tailscale when this was set up |

**Roles as of 2026-08-27** (the table above is the 2026-08-11 state, kept because
the rest of this section reasons from it):

| Host    | Reach           | Role                                                                                                          |
| ------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `pansa` | LAN + Tailscale | **primary**: app `:3001`, Supabase stack, the only tunnel connector                                           |
| `omo`   | LAN + Tailscale | backup replica + **warm app standby** `:3002` + **streaming PG replica** `:54332`, health-checked every 5 min |
| `air`   | offline         | nothing — down since 2026-08-24                                                                               |
| `pi`    | offline         | nothing                                                                                                       |

Configured in `~/.config/systemd/user/praxis-backup.service.d/replicas.conf`
via `PRAXIS_BACKUP_HOSTS`. An unreachable replica logs a warning and is skipped
— a sleeping laptop must not fail the backup for everyone else. Retention is
applied on the replica too, or it grows unbounded on a machine nobody watches.

Verified: identical sha256 and byte size on pansa and both replicas.

**The passphrase now lives on all three hosts** (`~/.config/praxis/backup.passphrase`,
mode 600), copied 2026-08-12. Verified by decrypting an actual replica archive on
each host using only that host's own copy — not a checksum comparison, an actual
`gpg --decrypt | tar tz`.

The tradeoff, stated plainly: air and omo now hold both the ciphertext and the
key, so on those two machines the backup is effectively plaintext. What it buys
is the scenario that actually matters — pansa's disk dying — where the archives
are otherwise unopenable. What it does not survive is one of the replicas being
compromised.

An off-network copy (password manager, paper) is still worth having, because all
three machines sit on one LAN behind one door. That is now an improvement rather
than the single point of failure it was.

### The warm standby (on air then; on omo now)

> **2026-08-27:** this standby now lives on `omo` as `praxis-backend.service`
> (`PORT=3002`), and it reaches pansa's Kong **over Tailscale** rather than the
> LAN — the two hosts have no direct LAN route to each other. The unit is named
> `praxis-backend.service`, not `praxis-standby.service`, because
> `scripts/auto_update.sh` restarts it by that name and reads `PORT` out of its
> `Environment=` to health-check the right port after a deploy. Everything below
> about _why_ the standby is not on the tunnel still holds, unchanged.

`praxis-standby.service` on air runs the same build against pansa's database
over the LAN (`http://192.168.1.15:54321`, ~12 ms). It is deliberately _not_
connected to the Cloudflare tunnel, for a reason worth keeping in mind: the
tunnel's ingress also serves `supabase.praxisweb.xyz`, so a naive tunnel replica
on a host with no database would route database traffic into a void roughly half
the time.

To make it serve public traffic, the tunnel config on air would need
`supabase.praxisweb.xyz` proxied back to pansa. That is a real option, and it is
active-active for the app tier only — if pansa is down, the standby serves pages
whose data layer is gone.

Two things this standby does _not_ protect against, both by construction: loss
of pansa (the database is there) and loss of the LAN. It covers the app process
and that host's configuration, nothing more.

**Gotcha found while building it:** air already had a Praxis dev server
(`ts-node src/index.ts`) holding port 3001. The standby appeared to start
successfully and answered `/health` and `/plans` correctly — because those
requests were being served by the _other_ process. Every database read failed
while everything else looked healthy. The standby is on **3002** for that
reason. If it ever reports a healthy service with empty query results, check
what actually owns the port before anything else.

---

## 5c. Do not run a second cloudflared on the same tunnel token

Tried 2026-08-11. It took the site down for roughly ten minutes. Recorded here
so nobody repeats it.

**What was attempted.** Install cloudflared on air with the same tunnel token as
pansa, expecting Cloudflare to treat the two as replicas of one tunnel and
balance across them. Both registered four connections and reported healthy.

**What actually happened.** The two instances collide on connIndex 0-3, and the
second set displaces the first rather than adding to it. With both running the
site still answered, which made it look like it worked. Stopping either one then
produced a hard 503 from Cloudflare's edge — `server: cloudflare`, a cf-ray, and
no request ever reaching pansa's cloudflared. Re-registering four connections on
pansa did not fix it; the edge kept 503ing for several minutes.

**What restored it.** Stopping the tunnel service, leaving it down long enough
for Cloudflare's stale connection state to expire, then starting it again.
Roughly: `systemctl --user stop cloudflared-praxis`, wait, `start`. Two rapid
restarts did _not_ work — the gap is what mattered.

**Wrong turns worth skipping.** Moving the backend to port 3002 (theory: the
tunnel is remotely-managed and air's local ingress got pushed to the edge) —
made no difference, reverted. Running the tunnel as a named tunnel from
`~/.cloudflared/config.yml` — fails with "tunnel credentials file not found";
`cert.pem` alone is not enough, and there is no credentials JSON on this host.

**If app-tier redundancy is still wanted**, the supported routes are:

- a Cloudflare **Load Balancer** with two _separate_ tunnels, one per host, and
  health checks — this is what the replica behaviour was mistakenly assumed to
  be; or
- keep air as a manual standby: it already runs the app on :3002 against
  pansa's database, so failover is repointing the tunnel's ingress at it.

Either way it protects only the app process. The database is on pansa, and no
tunnel arrangement changes that.

---

## 5d. Streaming replica on air

> **2026-08-27:** the streaming replica now runs on **`omo`**
> (`praxis-replica.service`, port `:54332`), not air. Two omo-specific details
> that are not in the text below and will otherwise cost an afternoon: it uses
> vanilla Postgres 17 + pgvector from conda-forge rather than the Supabase image
> (the primary preloads `pgaudit`, `pg_cron`, `pgsodium` and `supautils`, which
> that build does not have; WAL replay needs none of them), and it needs
> `LOCPATH=/home/gio/.locale` or it replays WAL perfectly and then refuses every
> connection with "database locale is incompatible with operating system" —
> which reads as a broken replica and is not one. It is health-checked every 5
> minutes by `praxis-replica-health.timer`, because `systemctl is-active`
> reported air's replica as healthy for days after it stopped streaming.

Set up 2026-08-11. air now holds a live physical standby of pansa's database,
replaying WAL continuously. Verified: a row written on pansa was readable on air
within a round trip, `lag_bytes=0`, and `auth.users` / `goal_trees` counts match
(44 / 10).

|         |                                                                                               |
| ------- | --------------------------------------------------------------------------------------------- |
| Primary | pansa, Supabase Postgres 17.6, port 54322                                                     |
| Standby | air, `~/praxis-replica/data`, Postgres 17.10 (conda-forge), socket `/home/gio/praxis-replica` |
| Slot    | `air_standby` on the primary                                                                  |
| Service | `praxis-replica.service` (user unit on air)                                                   |

### Nothing on the primary required a restart

`wal_level` was already `logical` and `max_wal_senders` 10, so the only changes
were a `pg_hba.conf` replication rule and a reload. `max_slot_wal_keep_size` was
already 4 GB, which is the safety property that matters: if air is off for long
enough to fall 4 GB of WAL behind, Postgres **invalidates the slot** rather than
filling pansa's disk. A dead replica is recoverable by reseeding; a full primary
disk is not.

### Checking it

```bash
# From pansa — is it streaming, and how far behind?
docker exec supabase_db_praxis_webapp psql -U postgres -d postgres -c \
  "select application_name, state, pg_wal_lsn_diff(sent_lsn, replay_lsn) AS lag_bytes
     from pg_stat_replication;"

# From air — is it still a standby, and current?
ssh air 'LOCPATH=$HOME/.locale ~/micromamba/envs/pg17/bin/psql \
  -h /home/gio/praxis-replica -U postgres -d postgres \
  -c "select pg_is_in_recovery(), now() - pg_last_xact_replay_timestamp() as behind;"'
```

If the slot ever shows `active=false` for a long period, check
`safe_wal_size` in `pg_replication_slots` before assuming it will catch up.

### The replica died silently for two days (2026-08-16 → 08-18)

Found 1 day 20 hours behind, `pg_stat_wal_receiver` **empty**. Throughout,
`systemctl --user is-active praxis-replica.service` reported `active`: the
Postgres process was up and serving reads of stale data, and unit state says
nothing about the replication link. Two independent causes had to be fixed:

1. `primary_conninfo` named pansa as `host=192.168.1.15` — the **third** place
   that stale LAN address has hidden, after the tunnel ingress and air's `.env`.
   `FATAL: could not connect to the primary server: ... No route to host`,
   retried every 5s, logged only to the replica's own journal.
2. With the address corrected, the primary then refused it:
   `no pg_hba.conf entry for replication connection from host "172.21.0.1"`.
   The connection arrives **NATed as the Docker bridge gateway**, not as air's
   Tailscale address, so the existing `host replication all 100.64.0.0/10` rule
   never matches it. Added on the primary:

   ```
   host replication all 172.21.0.1/32 scram-sha-256
   ```

   **This lives in `/etc/postgresql/pg_hba.conf` inside the container, which is
   NOT the mounted data volume** — recreating the container silently reverts it
   and replication dies again. Re-apply it after any `supabase stop/start` that
   recreates the db container.

No re-seed was needed: the slot still held WAL (`wal_status=reserved`, 317 MB
behind, ~3.9 GB of headroom), so it caught up to sub-second on its own.

### Watching it: `praxis-replica-health.timer`

`scripts/replica-health.sh`, every 5 minutes on air. It asserts the three things
that must **all** hold for a standby to be one, because the outage above passed
any one of them individually:

1. the server is in recovery — a promoted replica is not a standby
2. a WAL receiver exists — the link is live, not merely configured
3. replay lag is under `MAX_LAG_SECONDS` (default 300)

Exit code is non-zero when degraded, so a promotion path or a drill can gate on
it rather than assume. It logs on **every** run to `logs/replica-health.log`,
including healthy ones — a log that goes quiet when things are fine cannot be
distinguished from a check that stopped running, which is exactly the ambiguity
that hid this for two days. Lag unknown (NULL replay timestamp) counts as
degraded, never as zero.

```bash
ssh air 'REPO_DIR=$HOME/praxis_webapp bash praxis_webapp/scripts/replica-health.sh; echo $?'
ssh air 'tail -5 praxis_webapp/logs/replica-health.log'
```

### What a full pansa outage still cannot survive

**air cannot serve the app if pansa is off, and promoting the replica does not
change that.** Verified 2026-08-18: air has **no Docker binary** and no GoTrue,
PostgREST or Kong process. It runs bare Postgres plus the Node app. So:

| Component           | pansa | air                     |
| ------------------- | ----- | ----------------------- |
| Postgres data       | yes   | yes (streaming replica) |
| PostgREST (`/rest`) | yes   | **no**                  |
| GoTrue (`/auth`)    | yes   | **no**                  |
| Kong (`:54321`)     | yes   | **no**                  |
| Storage             | yes   | **no**                  |

air's app is configured with `SUPABASE_URL=http://100.65.59.18:54321` — pansa's
Kong. The browser also talks to `supabase.praxisweb.xyz`, whose ingress on air
proxies to that same address. If pansa is off, both die, and a promoted replica
is a database with nothing in front of it that the app or the client can speak
to.

Closing this is mesh phase 4, **staged 2026-09-29, not yet live**: omo and
air both hold a standby bundle (`~/supabase-standby/`: exact images, envs
with pansa's JWT secret so sessions survive, Kong config, hourly-synced
blobs) plus `deploy/supabase-standby/docker-compose.yml`. What is missing
before it serves: a docker **engine** on omo/air (CLI only today — needs
human `apt install`), replica `listen_addresses` extended to the docker
bridge, and one real promotion drill. Until then:

| Failure                            | Covered?   | By what                                        |
| ---------------------------------- | ---------- | ---------------------------------------------- |
| pansa's app slow / overloaded      | yes        | DNS watchdog moves apex to omo (drilled 2m19s) |
| pansa unreachable from the edge    | yes        | same, verified live 2026-09-29                 |
| pansa's backend process down       | yes        | same; `supabase.` now rides along to omo/air   |
| **pansa host off / Supabase down** | **staged** | bundle + scripts ready, engine + drill pending |
| data loss on pansa                 | partial    | replicas (omo+air) + nightly encrypted dump    |

### The stream is TLS 1.3 since 2026-08-13

The pansa→air WAL stream used to be plaintext on the LAN. `ssl = on` (plus a
self-signed cert `server.crt`/`server.key` in the primary's data dir) was
enabled via `postgresql.auto.conf` and a reload — no restart. The standby's
`primary_conninfo` has `sslmode=prefer`, so after a replica restart the
walreceiver renegotiated and now shows `ssl=t TLSv1.3`. Verify with:

```bash
docker exec supabase_db_praxis_webapp psql -U postgres -d postgres -c \
  "select s.pid, a.application_name, a.client_addr, s.ssl, s.version
     from pg_stat_ssl s join pg_stat_replication a on s.pid = a.pid;"
```

The cert and auto.conf live on the docker data volume, so they survive a
container recreate. Reversible: delete the three `ssl*` lines from
`postgresql.auto.conf` and reload. It is self-signed — it defeats passive
sniffing, not an active MITM; acceptable on a home LAN, worth replacing with a
real CA cert if the LAN ever hosts untrusted devices.

### Promoting it (pansa is gone)

Promotion target is **omo first**, air second (same recipe, slower box).
Do not hand-run `pg_ctl promote` — it skips every guard:

```bash
ssh omo-ts 'REPO_DIR=/home/gio/praxis_webapp bash /home/gio/praxis_webapp/scripts/supabase-failover.sh'
```

The script refuses while pansa's Kong answers (split-brain guard), refuses on
an unhealthy/stale replica (uses `replica-health.sh`, not hope), promotes,
starts the standby stack, waits for Kong, and repoints the backend at local
Kong. DNS (`apex` + `supabase.`) is already on omo's tunnel via the watchdog.

**Read this before relying on it:**

- The replica is **vanilla Postgres with pgvector**, not the Supabase image. The
  primary preloads `pgaudit`, `pg_cron`, `pgsodium`, `pg_tle`, `plan_filter`,
  `supautils` and others that do not exist in this build; the standby's
  `shared_preload_libraries` is empty. Replay does not need them.
- Installed extensions on the primary are `pg_net`, `pg_stat_statements`,
  `pgcrypto`, `plpgsql`, `supabase_vault`, `uuid-ossp`, `vector`. Of those,
  **`pg_net` and `supabase_vault` have no binaries on air**. Their tables
  replicate; calling their functions after promotion will fail until the
  extensions are installed or the objects dropped.
- A promoted replica is a **database**, not a Supabase stack. PostgREST, GoTrue,
  Kong and Storage all still live on pansa. Promotion covers "the data
  survived", not "the product is up". For the second, restore the backup archive
  onto a full Supabase stack — section 3.

### Two host-specific gotchas, both already handled

`LOCPATH=/home/gio/.locale` is set in the unit. The data directory carries
`LC_COLLATE=en_US.UTF-8`, which air does not have; without the generated locale
Postgres replays WAL perfectly and then refuses **every** connection with
"database locale is incompatible with operating system". It looks like a broken
replica and is not one.

`lc_messages`/`lc_monetary`/`lc_numeric`/`lc_time` are set to `C` in the
standby's `postgresql.conf` for the same reason. These are formatting only —
`LC_COLLATE`, the one that affects indexes, comes from the primary and is
untouched.

---

## 5e. Two tunnels and DNS failover (replaces the paid Load Balancer)

Set up 2026-08-12. Cloudflare's Load Balancer needs a paid subscription and an
API token neither of which is available here, so the same outcome is achieved by
repointing DNS: `praxisweb.xyz` routes to exactly one tunnel, and a watchdog
moves it when that tunnel stops answering.

|                 |                                                                        |
| --------------- | ---------------------------------------------------------------------- |
| `praxis-tunnel` | pansa, id `087b548c…`                                                  |
| `praxis-air`    | air, id `0e74ce7a…`                                                    |
| Per-host health | `pansa.praxisweb.xyz`, `air.praxisweb.xyz` — each pinned to one tunnel |
| Watchdog        | `praxis-failover.timer` on **pansa**, every 60s                        |

Two _separate_ tunnels, not one token on two hosts. Sharing a token collides on
connIndex and takes the site down — §5c.

**The watchdog runs on the standby**, deliberately. On the primary it would die
with the thing it watches. It fails over after 3 consecutive failures (~4
minutes worst case) and fails back only after 10 consecutive successes, because
flapping DNS is harder to diagnose than a few more minutes of downtime. It
refuses to fail over to a standby that is not itself healthy: both hosts down is
an outage, and moving DNS would only change which host returns the error.

Verified by drill: with pansa's tunnel stopped, DNS moved to air in ~13 seconds
and the site served 10/10 including live API responses.

### Roles are currently reversed: air serves, pansa stands by

> **Reverted since. As of 2026-08-27 pansa serves and there is no second
> connector at all**: `cloudflared tunnel info praxis-tunnel` shows exactly one,
> from pansa's public IP, and the `praxis-air` tunnel has zero connections
> because air is off. `scripts/praxis-failover.sh` still has
> `PRIMARY_TUNNEL=praxis-air` / `STANDBY_TUNNEL=praxis-tunnel` — the constants
> the section below tells you to swap. Swapping them is now correct but not
> sufficient: with air down there is nothing on the other side to fail over to.
> Restoring a real failover path means giving `omo` its own tunnel, and that
> requires proxying `supabase.praxisweb.xyz` back to pansa from it (see §5b).

**Root cause of the 2026-08-11 outage: a third machine was on the tunnel.**

`cloudflared tunnel info praxis-tunnel` showed a second connector, `abe96f33`,
created 2026-08-11T17:10:12Z from origin IP `87.4.247.25` — not pansa's address.
It survived `cloudflared tunnel cleanup`, service restarts and a rebuild, which
is what made it look like stale edge state.

It was not stale. **omo** runs cloudflared as a system service
(`/usr/bin/cloudflared --no-autoupdate tunnel run --token-file /etc/cloudflared/token`)
on the _praxis tunnel token_, from a different network — its public IP is
`87.4.247.25`, and its process start time (19:10:06 CEST) matches the connector
to the second. omo has no Praxis app listening, so every request Cloudflare
balanced onto it hit nothing and returned 503.

That explains all of it: the intermittent 503s that predate any of this work,
why restarting pansa's tunnel never helped, and why the site looked healthy
while half the requests failed.

**omo must not serve Praxis.** Its role is a backup replica and a copy of the
encryption key, nothing more. To take it off the tunnel (needs root on omo):

```bash
ssh omo 'sudo systemctl disable --now cloudflared'
```

Until that is done, `praxis-tunnel` will keep 503ing a share of requests and
`pansa.praxisweb.xyz` will stay unreliable — which is why production currently
points at `praxis-air`.

omo also carries a pre-existing `~/praxis_webapp` checkout with its own `.env`.
Nothing runs from it and nothing here touches it, but it is worth knowing it is
there.

`praxisweb.xyz` and `supabase.praxisweb.xyz` therefore point at `praxis-air`,
which has only its own connectors and is clean. air proxies database traffic
back to pansa over the LAN.

**When the ghost connector finally expires** — check with
`cloudflared tunnel info praxis-tunnel` for a single connector — swap
`PRIMARY_TUNNEL` and `STANDBY_TUNNEL` (and the two health URLs) in
`~/.local/bin/praxis-failover.sh`, and move the watchdog to air.

### The primary tunnel no longer runs from a token

Run with `--token`, a tunnel is _remotely managed_: Cloudflare supplies the
ingress from the dashboard and the local `--config` is **silently ignored**.
That is why `pansa.praxisweb.xyz` returned 503 no matter what the local file
said, and why editing configs during the incident changed nothing — the file
being edited was never the file in force. pansa now runs from
`~/.cloudflared/praxis-tunnel.json` with `config-pansa.yml`, matching air, so
local ingress is authoritative on both hosts.

### Every tunnel restart costs minutes

Stopping and starting a tunnel leaves Cloudflare 503ing that hostname for
1-5 minutes while the edge converges. This is normal and not a fault. Do not
restart a tunnel to "fix" a 503 that a restart just caused — wait, and check
`cloudflared tunnel info` for stale connectors before touching anything.

---

## 5f. Replicas on one tunnel (2026-08-13), and the drill that found the flaw

Both hosts now run a connector for **the same tunnel** (`praxis-tunnel`), so
`praxisweb.xyz` is served by pansa and air simultaneously and losing one host
costs no DNS change at all.

|                 |                                                                          |
| --------------- | ------------------------------------------------------------------------ |
| pansa connector | `cloudflared-praxis.service`, config `config-pansa.yml`                  |
| air connector   | `cloudflared-praxis-replica.service`, config `config-praxis-replica.yml` |
| Health gate     | `praxis-tunnel-gate.timer` on **both**, every 30s                        |
| Node identity   | `GET /health` returns `node`, so you can tell which replica answered     |

This contradicts §5c only in appearance. What broke in August was a connector
with **nothing behind it** (omo), and a tunnel run from a _token_, where the
local ingress file is silently ignored. Both connectors here run from a
credentials file, so the ingress each applies is the one on disk, and each points
at a backend that actually serves. air's credentials were generated on air with
`cloudflared tunnel token --cred-file … praxis-tunnel`; nothing secret was copied
between hosts.

air's config deliberately omits `pansa.praxisweb.xyz`. That hostname means "is
pansa serving?", and answering it from air would blind the watchdog — a dead
pansa would look alive.

### The health gate is the health check the edge does not have

The edge picks a connector per connection and never checks whether the origin
behind it works. A connector whose backend is dead keeps taking its share of
traffic and failing it. `scripts/tunnel-health-gate.sh` polls the local backend
and stops the local cloudflared units after 3 consecutive failures at 30s, so a
genuinely dead backend leaves rotation in ~90s while a deploy restart never trips
it. It measures whether the app _serves_, not whether systemd calls it active —
a backend can be active and return 500 to everything.

### The drill took the apex down for about a minute. Read this before the next one.

Withdrawing air's connectors to test the gate produced `error code: 1033` (530)
on `praxisweb.xyz` — a real outage, roughly 04:46–04:47 UTC on 2026-08-13.

The cause was not the gate and not edge convergence. **The apex DNS pointed at
`praxis-air`, the air-only tunnel**, because the §5e watchdog had made air
primary and left it there. So the apex depended on exactly one host while
`www.praxisweb.xyz` — still on `praxis-tunnel` — served throughout without a
blip. Stopping air removed the only connector the apex had.

Two lessons worth more than the drill cost:

- **Check which tunnel each hostname resolves to before testing a failover.**
  `/home/gio/.local/state/praxis-failover/current_target` records what the
  watchdog last chose, and it is not necessarily what you assume.
- A drill that can fail user requests belongs on a hostname nobody uses, or
  behind a maintenance window. Verifying the two pinned hostnames
  (`pansa.` and `air.`) proves both connectors serve without touching the apex.

### The apex now points at the shared tunnel (completed 2026-08-13)

The apex was repointed at the shared tunnel on 2026-08-13 (~05:22 UTC) with:

```bash
cloudflared tunnel route dns --overwrite-dns praxis-tunnel praxisweb.xyz
```

The §5e watchdog (`praxis-failover.timer`) had already been disabled on pansa,
so nothing moves the apex back to `praxis-air`. Both `current_target` state files
(`~/.local/state/praxis-failover/current_target` on air and pansa) were synced to
`praxis-tunnel`. `praxisweb.xyz` is now served by both hosts' connectors on the
shared tunnel, and the pinned `air.`/`pansa.` hostnames still prove per-host
availability without touching the apex. `praxis-air` is kept running because the
pinned `air.` hostname (the watchdog's per-primary health URL) depends on it, and
it is what a future drill uses to rehearse a single-host withdrawal.

### systemd `Environment=` splits on whitespace

air's gate needs to stop two units, and the drop-in said:

```
Environment=TUNNEL_UNITS=cloudflared-praxis.service cloudflared-praxis-replica.service
```

systemd read that as two assignments and `TUNNEL_UNITS` came out holding only the
first unit, so the gate withdrew one connector and left the other serving
nothing — the exact failure it exists to prevent. Quote the whole thing:

```
Environment="TUNNEL_UNITS=cloudflared-praxis.service cloudflared-praxis-replica.service"
```

Verify with `systemctl --user show praxis-tunnel-gate.service -p Environment`
rather than by reading the file.

### air is a standby, not half of production (2026-08-18)

Two connectors on one tunnel means Cloudflare balances across both with **no
health awareness**, so air was serving roughly half of every request at all
times. The intent is a reserve that serves when pansa is overloaded or
unreachable, so the gate now distinguishes two classes of unit:

| Variable        | Meaning                                     |
| --------------- | ------------------------------------------- |
| `TUNNEL_UNITS`  | serve whenever **this host** is healthy     |
| `STANDBY_UNITS` | serve only while **the primary** is failing |

air's drop-in (`~/.config/systemd/user/praxis-tunnel-gate.service.d/units.conf`):

```
[Service]
Environment="TUNNEL_UNITS=cloudflared-praxis.service"
Environment="STANDBY_UNITS=cloudflared-praxis-replica.service"
```

air's own tunnel stays always-on: the pinned `air.` hostname is what proves the
replica is alive, and it is the DNS-failover target. Only the connector on the
**shared** tunnel is gated. pansa sets no `STANDBY_UNITS`, so its behaviour is
unchanged.

The primary is polled at `https://pansa.praxisweb.xyz/health` — through the
edge, not over Tailscale, because air cannot reach pansa's backend port (the app
binds locally) and the question that matters is whether the internet can get a
page from pansa. The reply carries `"node":"pansa"`; a 200 from anything else
counts as a failure, or the standby would stay withdrawn during exactly the
outage it exists for. "Overloaded" is response time (default 2s), because
`/health` is served by the same Node event loop as every real request — a loop
too busy to answer it is the overload.

Timings: the timer fires every 30s, takeover needs 3 consecutive failures
(~90s), stand-down needs 10 consecutive successes (~5min). Asymmetric on
purpose — flapping splits traffic unpredictably and makes every other symptom
unreadable.

**Rehearsed 2026-08-18**, all three paths, with production verified up
throughout (12/12 `/health` requests answered `node: pansa`):

```bash
# on air, with REPO_DIR / TUNNEL_UNITS / STANDBY_UNITS set as in the drop-in
rm -f /tmp/praxis-primary-gate.state

# 1. primary healthy -> stands down on the 10th check
for i in $(seq 1 10); do bash scripts/tunnel-health-gate.sh; done

# 2. primary unreachable -> takes over on the 3rd check, not the 1st
PRIMARY_HEALTH_URL=http://127.0.0.1:9/health bash scripts/tunnel-health-gate.sh

# 3. primary reachable but slow -> same takeover via the overload branch
PRIMARY_SLOW_SECONDS=0.001 bash scripts/tunnel-health-gate.sh
```

**Known gap.** air's app talks to _pansa's_ Supabase
(`SUPABASE_URL=http://100.65.59.18:54321`), so air covers pansa being overloaded
or unreachable _from the edge_, but not pansa being off — then air serves the
app and cannot reach the database. Closing that needs the replica promoted;
`scripts/praxis-failover.sh` and `praxis-standby.service` are the machinery, and
the failover timer is currently **not scheduled** on either host.

## 5g. Both standbys were dead at once (found 2026-08-30)

On 2026-08-30 neither host was a standby. Both replication slots on pansa read
`active=f, wal_status=lost`, and had for over a week. Every migration applied in
that window — including `2026-08-27-planner-learning.sql` — reached the primary
and nothing else, while `manual_actions.txt` recorded them as applied.

|     | Lag when found        | Frozen at    | Cause                                     |
| --- | --------------------- | ------------ | ----------------------------------------- |
| omo | 173,209 s (~2.0 days) | `2/D2B41D78` | fell past `max_slot_wal_keep_size` (4 GB) |
| air | 708,876 s (~8.2 days) | `1/67B0D70`  | `primary_conninfo` uses the wrong user    |

Two unrelated causes, one identical symptom. The primary was at `4/26959E40`.

**omo** — the WAL it needed had already been recycled:

```
FATAL: could not receive data from WAL stream: ERROR:
requested WAL segment 0000000100000002000000D2 has already been removed
```

This is `max_slot_wal_keep_size` working as designed (§5d): past 4 GB behind,
Postgres invalidates the slot rather than let an absent standby fill the
primary's disk. The consequence is the part worth internalising — **an
invalidated slot never catches up.** Restarting the replica cannot help. Only a
fresh base backup does.

**air** — rejected at the door, every 5 seconds, for 8 days:

```
FATAL: no pg_hba.conf entry for replication connection from
host "100.66.51.47", user "postgres", SSL encryption
```

Not the LAN-address bug of §5d, and not a host-matching problem: pansa's rule
is `host replication praxis_replicator 100.64.0.0/10`, and that CIDR **does**
cover air's Tailscale address. The mismatch is the **user** — air connects as
`postgres`, the rule grants only `praxis_replicator`. Note also that the rule
set no longer contains the hand-added `172.21.0.1/32` line from §5d, so the db
container was recreated at some point and reverted it, exactly as §5d warns.

### Why nothing surfaced it

`scripts/replica-health.sh` caught this correctly and immediately. It logged
`UNHEALTHY` to `logs/replica-health.log` every 5 minutes for the entire period
and left `praxis-replica-health.service` in a `failed` state on both hosts. The
check was never the problem. **Nothing was reading the check.** A monitor whose
only consumer is a human who has to remember to SSH in is a monitor that detects
outages and reports them to nobody.

The one live replication connection on pansa is `172.23.0.7` on slot
`cainophile_raw07ftl` — that is Supabase Realtime inside the Docker network. It
is easy to glance at `pg_stat_replication`, see a row in `streaming` state, and
conclude replication is fine. **Match on `client_addr` before believing it**: a
Docker-internal address is never a standby.

### The one command that answers it

```bash
ssh pansa-ts 'docker exec supabase_db_praxis_webapp psql -U postgres -At -F"|" \
  -c "select slot_name, active, wal_status from pg_replication_slots;"'
```

Any standby row reading `f|lost` means that host needs a reseed, not a restart.
Run this after any migration that matters, and after any period where a standby
was off. It costs one round trip and it is the only check that catches both
causes above without knowing either.

### Reseeding a standby

Both hosts need this; neither can recover on its own. On the standby:

```bash
# 1. stop the replica
systemctl --user stop praxis-replica.service

# 2. move the old data directory aside — do NOT delete it until the new one
#    is verified streaming. It is the only copy if the base backup fails.
mv ~/praxis-replica/data ~/praxis-replica/data.dead-$(date -u +%Y%m%dT%H%M%SZ)

# 3. re-seed from the primary. Use praxis_replicator, NOT postgres — that is
#    the user pansa's pg_hba.conf grants replication to.
~/micromamba/envs/pg17/bin/pg_basebackup \
  -h 100.65.59.18 -p 54322 -U praxis_replicator \
  -D ~/praxis-replica/data -R -P --slot=<omo_standby|air_standby> -C

# 4. air only: correct the user in the generated conninfo before starting,
#    if pg_basebackup did not carry it through.
grep primary_conninfo ~/praxis-replica/data/postgresql.auto.conf

# 5. omo only: LOCPATH is required or it replays fine and refuses connections
#    with "database locale is incompatible with operating system" (§5d).
systemctl --user start praxis-replica.service

# 6. verify — this must exit 0, not merely "active"
bash ~/praxis_webapp/scripts/replica-health.sh; echo $?
```

`-C` recreates the slot; the old invalidated one must be dropped on the primary
first if the name collides:

```bash
ssh pansa-ts 'docker exec supabase_db_praxis_webapp psql -U postgres \
  -c "select pg_drop_replication_slot('"'"'omo_standby'"'"');"'
```

Confirm `pg_stat_replication` shows the standby's **Tailscale** address, then
delete the `data.dead-*` directory.

### Second fault found the same day: omo's app tier was 16 commits behind

Unrelated to the database, same shape. `praxis-autoupdate.service` on omo had
been failing every 5 minutes:

```
New commits on origin/main. Updating e4f1323d -> 768872d6
Resolve by hand. Nothing was changed.
```

omo's tree held five untracked Capacitor build artifacts
(`client/android/app/capacitor.build.gradle`,
`client/android/app/src/main/assets/`, and three more) which commit `18905812`
had since begun tracking. `auto_update.sh` refuses a dirty tree rather than
stashing (see CLAUDE.md), so omo sat on `e4f1323d` indefinitely while air —
the host the docs called offline — stayed current.

The fix is to remove the untracked copies on omo; they are regenerable `cap
sync` output and the fast-forward then installs the tracked versions. **Do not
add them to `.gitignore`** — they are tracked deliberately as of `18905812`,
and ignoring them would mean `git rm --cached` on 300+ files.

```bash
ssh omo-ts 'cd ~/praxis_webapp && git clean -fd -- \
  client/android/app/capacitor.build.gradle \
  client/android/app/src/main/assets \
  client/android/app/src/main/res/xml/config.xml \
  client/android/capacitor-cordova-android-plugins \
  client/android/capacitor.settings.gradle'
```

## 5h. Retiring the exposed tunnel secret, then omo joins (2026-08-30)

> **This supersedes an earlier version of §5h that was wrong.** That version
> gave omo its own separate tunnel (`praxis-omo`) and told you to reach for
> `scripts/praxis-failover.sh`. Both contradict `docs/MESH_SERVING_PLAN.md`,
> which retired that script on 2026-08-27 and adopted the §5f pattern instead:
> several connectors on **one** tunnel, so the edge drops a dead node without
> any DNS change. `praxis-omo` has been deleted and its config removed from
> both hosts. Read the MESH plan's invariants before touching any of this.

Two jobs, and the **order is not optional**. The MESH plan (phase 0 item 4)
says to retire the secret _before_ provisioning a new host, because doing it
after means redoing that host's credentials.

### Why the secret is being retired

`praxis-tunnel` (`087b548c-1357-429d-8a53-0c085de22831`) had its token pasted
into a chat transcript on 2026-08-27. Cloudflare **cannot rotate a tunnel
secret in place**. Anyone holding it can register a connector and receive live
traffic for the login page.

Replacement tunnel already created: **`praxis-edge`,
`8ee0b50b-f6aa-445d-9fc5-db7035778e02`**, credentials at
`~/.cloudflared/8ee0b50b-….json` on pansa, ingress staged at
`~/.cloudflared/config-pansa-edge.yml` (routing identical to the live file;
only the tunnel identity differs).

### Why this needs you rather than an agent

`sudo` on pansa requires a password, and its connector is a **system** unit
(`/etc/systemd/system/cloudflared.service`, config `/etc/cloudflared/`). DNS
record writes are likewise not automatable here. Every step below is yours.

### Cutover — run the new connector BEFORE moving DNS

The naive order (move DNS, then start the connector) takes the site down for
the gap, because the records point at a tunnel with no connectors. Running the
new one first makes the DNS move seamless.

Two cloudflared processes exist on pansa during steps 2–5. That is safe here
and is **not** the §5c collision: that failure is two connectors on the _same_
tunnel colliding on connIndex 0-3. These are different tunnels. It does
transiently bend MESH invariant 1 ("never two connectors on one host"), which
is why it is a brief, deliberate cutover window and not a resting state.

```bash
# 1. Install the new credentials and ingress where root can read them.
sudo install -m 600 -o root -g root \
  ~/.cloudflared/8ee0b50b-f6aa-445d-9fc5-db7035778e02.json /etc/cloudflared/
sudo install -m 644 -o root -g root \
  ~/.cloudflared/config-pansa-edge.yml /etc/cloudflared/

# 2. Start the new connector alongside the old one, in the foreground first so
#    you can see it register. Ctrl-C once it reports connections.
sudo cloudflared tunnel --config /etc/cloudflared/config-pansa-edge.yml run

# 3. Confirm praxis-edge has connectors before touching a single DNS record.
cloudflared tunnel info praxis-edge     # must show 1+ connections

# 4. Move all four records. Each overwrites the existing CNAME in place.
for H in praxisweb.xyz www.praxisweb.xyz pansa.praxisweb.xyz supabase.praxisweb.xyz; do
  cloudflared tunnel route dns praxis-edge "$H"
done

# 5. Verify through the edge, not locally. The node field names the origin.
curl -s https://praxisweb.xyz/health          # {"node":"pansa"}
curl -s -o /dev/null -w '%{http_code}\n' https://supabase.praxisweb.xyz/auth/v1/health

# 6. Only now cut the old connector over. Point the system unit at the new
#    config permanently, then restart it.
sudo sed -i 's#config-pansa\.yml#config-pansa-edge.yml#' \
  /etc/systemd/system/cloudflared.service
sudo systemctl daemon-reload && sudo systemctl restart cloudflared

# 7. Old tunnel should now have zero connectors. Delete it — this is what
#    actually revokes the exposed secret.
cloudflared tunnel info praxis-tunnel        # expect no connections
cloudflared tunnel delete praxis-tunnel
```

**Rollback at any point before step 7:** the old tunnel still has its
connector and its credentials. Re-run step 4 with `praxis-tunnel` and the
records move straight back.

**`air` is on its own tunnel** (`praxis-air`, serving since 2026-08-30 14:19
UTC) and is unaffected by all of this. It also predates the MESH plan's
one-tunnel decision — fold it onto `praxis-edge` when convenient, by the same
recipe as omo below.

### Then, and only then — omo joins as a second connector

This is MESH plan phase 1, unchanged except that the tunnel is now
`praxis-edge`. **Credentials are generated on omo**; nothing secret is copied
between machines (MESH invariant 2). That does require the account origin
certificate on omo:

```bash
# cert.pem authorises creating/deleting tunnels and DNS across the whole zone.
# Treat it accordingly: 0600, and it is the reason ~/.cloudflared is 0700.
scp pansa-ts:~/.cloudflared/cert.pem omo-ts:~/.cloudflared/cert.pem

ssh omo-ts 'chmod 600 ~/.cloudflared/cert.pem
  cloudflared tunnel token --cred-file ~/.cloudflared/praxis-edge.json praxis-edge'
```

Then omo's ingress. **Omit `pansa.praxisweb.xyz`** — invariant 5, it is the
watchdog's question and omo answering it makes a dead pansa look alive.
**Omit `supabase.praxisweb.xyz`** — that is phase 2, and invariant 4 requires
the gate to poll Kong before any node advertises it. That check does not exist
yet.

```yaml
# ~/.cloudflared/config-omo.yml on omo
tunnel: 8ee0b50b-f6aa-445d-9fc5-db7035778e02
credentials-file: /home/gio/.cloudflared/praxis-edge.json

ingress:
  # omo's app listens on 3002, not 3001. Pointing this at 3001 reproduces the
  # exact mismatch that made the token-tunnel attempt fail.
  - hostname: praxisweb.xyz
    service: http://localhost:3002
  - hostname: www.praxisweb.xyz
    service: http://localhost:3002
  - hostname: omo.praxisweb.xyz
    service: http://localhost:3002
  - service: http_status:404
```

```bash
ssh pansa-ts 'cloudflared tunnel route dns praxis-edge omo.praxisweb.xyz'
# user unit, never a system one, and never --token
ssh omo-ts 'systemctl --user enable --now cloudflared-praxis praxis-tunnel-gate.timer'
```

**Verify on the pinned hostnames, never by stopping things and watching the
apex** — the 2026-08-13 drill took the apex down for a minute doing exactly
that (§5f):

```bash
curl -s https://pansa.praxisweb.xyz/health    # {"node":"pansa"}
curl -s https://omo.praxisweb.xyz/health      # {"node":"omo"}
```

A connector without `praxis-tunnel-gate.timer` is invariant 3 broken: rotation
membership is earned every 30 s by actually serving, not granted at install.

### Already done, and not part of the above

- omo's app tier: was 16 commits behind on a dirty tree (§5g), cleaned, current.
- omo's inference tier: `colibri-olmoe` was installed and had never been
  started. Started; Axiom now distributes across both boxes. See "Axiom
  colibri" in `docs/HOSTING_TOPOLOGY.md`.

## 6. GDPR and security posture

Snapshot 2026-08-13. The app has account deletion (hard delete with password
confirmation), anonymize (soft delete), and data export — `DELETE /users/me`,
`POST /users/me/anonymize`, `GET /users/me/export` in `userController.ts`. These
cover the GDPR right to erasure, right to be forgotten, and right to data
portability for the live database.

### What is encrypted, and where

| Layer             | State (2026-08-13)                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------ |
| Live DB at rest   | Plaintext on pansa and on air's replica disk — no LUKS. Only the backups are encrypted.                |
| Nightly backups   | GPG AES-256 symmetric, passphrase on all three hosts (`~/.config/praxis/backup.passphrase`, mode 600). |
| Replication WAL   | TLS 1.3 since 2026-08-13 (self-signed cert). Defeats passive sniffing on the LAN, not a MITM.          |
| Backend API       | Bound to 127.0.0.1 since 2026-08-13; only reachable through the Cloudflare tunnel.                     |
| Storage (avatars) | Inside the encrypted backup archive only.                                                              |

### Breach notification (72-hour clock)

If personal data (auth.users emails, goal journals, check-ins) is exposed or
believed exposed:

1. Stop the bleeding: `systemctl --user stop praxis-backend cloudflared-praxis`
   on pansa, or disconnect the host from the LAN/Tailscale. Do not destroy
   evidence — freeze the disk if a replica is compromised.
2. Decide within 72h whether it is a GDPR breach (unauthorised access to PII)
   vs. an outage. Only the former triggers the regulator; an outage does not.
3. If it is a breach, notify the relevant supervisory authority (in Italy, the
   Garante) via their portal, and affected users directly if the risk is high.
4. Record it in `logs/` regardless — breach or not, write down what happened,
   when it was detected, and what was exposed. This log is what a later
   authority request would want to see.
5. Rotate the exposed secrets: service-role key, Stripe keys, backup passphrase
   (only if it left the three hosts).

### Erasure and retention

- Deleting an account removes live rows; the encrypted backup archives keep the
  data up to 30 days (retention). A "delete everything" request must therefore
  also purge the archives: on each host run
  `gpg --decrypt --passphrase-file ~/.config/praxis/backup.passphrase <archive>`
  to confirm scope, then delete the `.gpg` files and let retention not apply.
  Do this before responding to the request.
- Backups are also copied to air and omo — purge all three copies, not just
  pansa's. See §5b for where they live.

### Open items

- Offsite remote not configured — backups are currently single-disk, on one LAN
  behind one door.
- No LUKS disk encryption on pansa or air; the live DB is plaintext at rest.
- No monthly restore drill. A backup is a belief until it is restored; §3 is
  verified as of 2026-08-10 and should be re-run after any Supabase image bump.
- `NODE_ENV=development` on the live instance.
- No data-retention policy for the Axiom/AI briefs or chat content beyond the
  backup window.
