# Plan — Managed accounts (gym · therapist · coach) and the AI on top

Scoped 2026-08-30. Supersedes nothing; sits beside
`docs/AXIOM_BRIEF_PLANNER_PLAN.md` (U1–U6), which stays the fix track.

---

## Where this actually starts

Less exists than the marketplace suggests.

| Thing                   | State                                                                           |
| ----------------------- | ------------------------------------------------------------------------------- |
| `coach_profiles`        | A public **listing**: bio, skills, domains, rate, rating. Anyone can read it    |
| `services` / `offers`   | Marketplace transactions                                                        |
| `coaching_briefs`       | An AI report **about a user, for that user**. Self-coaching, not a coach's view |
| `AICoachingService`     | Generates that report from the user's own context                               |
| **coach ↔ client link** | **Does not exist**                                                              |

So a coach can be _found_ and _paid_, and then has no way to see anything. There
is no relationship table, no scoped access, and therefore no consent mechanism —
which means "managed" is greenfield, and the first thing built has to be the
part that is hardest to retrofit.

## The decision everything else rests on

A gym, a therapist and a coach are all asking for the same thing: _another
person's data_. Praxis holds mood, energy, check-ins, notes and health-adjacent
trackers. Under GDPR that is special-category data (art. 9), and sharing it with
a professional is lawful only on **explicit, specific, revocable consent**.

That is not a compliance footnote to add later. It is the schema.

Five rules, and everything below follows from them:

1. **The client grants; the coach requests.** A coach can never add a client.
   The invitation flows one way and the acceptance is the consent record.
2. **Consent is per scope, not per relationship.** `goals`, `planner`,
   `trackers`, `checkins`, `notes`, `health` are separate grants. A running
   coach gets `planner` and `trackers`; a therapist may get `checkins` and
   `notes`; nobody gets everything by default.
3. **`health` is its own scope and is never bundled.** Bundling special-category
   data with ordinary data is precisely what art. 9 forbids.
4. **Every read is logged, and the client can see the log.** "Your therapist
   opened your mood log on Tuesday" is both the accountability record GDPR wants
   and a feature people will value.
5. **Revocation is immediate and honest.** Access stops at once; anything
   already exported is gone and the UI must say so rather than implying a recall
   that cannot happen.

## Build order

### G1 — `managed_relationships` + scoped access (foundation) — ✅ shipped 2026-08-30

Schema, RLS, and one server-side gate every coach-facing read must pass.

```
managed_relationships
  id, coach_id, client_id, kind('coach'|'therapist'|'gym'|'trainer')
  status('invited'|'active'|'paused'|'ended')
  scopes  text[]     -- granted by the client, never by the coach
  invited_at, accepted_at, ended_at
```

The gate is one function — `assertScope(coachId, clientId, scope)` — and it is
the only way coach-facing code reads client data. A second path is how a leak
happens.

**Done when:** a coach with `status='active'` and the right scope can read that
slice and nothing else; every other case is a 403; and the test suite proves the
403 for paused, ended, wrong-scope, and not-my-client.

### G2 — `managed_access_log` — ✅ shipped 2026-08-30 (with G1)

Append-only: who read what, for which client, when. Written by the gate itself,
so it cannot be forgotten at a call site. Exposed to the client, not only to
auditors.

### G3 — Invitation flow — ✅ shipped 2026-08-30

Coach sends an invite naming the scopes it wants and _why_; client sees the ask
in plain language and grants scope by scope. Declining is a first-class outcome,
not an error.

**Shipped:** `POST /managed/invite`, `GET /managed/invitations`,
`POST /managed/:id/{grant,decline,revoke,end}`, `GET /managed/caseload`,
`GET /managed/access-log`. 14 controller tests.

Three things the build settled that the plan had left implicit:

- **The ask needed its own column.** The RLS rightly forbids a coach writing
  `scopes`, so an invitation had nowhere to say what it wanted — the client
  would have seen "someone wants access" with no content. `requested_scopes` is
  the ask, `scopes` is the answer, and the gap between them is the consent.
- **A purpose is required, not optional.** A request with no stated reason is
  one a client cannot meaningfully consent to, so the endpoint refuses it.
- **The rules are enforced twice, deliberately.** RLS covers the browser's
  direct Supabase access; the API runs on the service-role key and bypasses it
  entirely, so the controller restates every rule. An endpoint that trusts
  policies it has already bypassed is one that grants consent nobody gave.

### G4 — Coach caseload view — ✅ shipped 2026-08-30

One screen: clients, last check-in, adherence to the plan the coach set,
anything flagged. Reads only through G1's gate, so an empty column means "not
shared" and says so rather than rendering zero.

**Shipped:** `/clients` (`CoachCaseloadPage`) and `/shared-access`
(`SharedAccessPage`), plus `GET /managed/client/:clientId/summary` and
`GET /managed/mine`. Both routes are in the ACCOUNT menu.

It turned into two screens, not one, and the reasons are worth keeping:

- **`/caseload` deliberately reads no client data.** Listing who shared what is
  not reading what they did, so opening the tab writes nothing to anyone's
  access log. The read happens when a coach expands a row. That keeps the log a
  record of attention rather than of page refreshes — a log that fires on every
  refresh is one clients learn to ignore, which destroys the feature that makes
  this whole arrangement defensible.
- **`summary` and `digest` are separate endpoints** because they are separate
  acts in that log: `action='view'` for a person opening a row, `'digest'` for
  the AI reading the same slice. `build()` takes the action rather than
  hardcoding it.
- **The client needs their own screen, or "revocable" is only true in the API.**
  `/shared-access` is where invitations are answered, individual scopes are
  revoked chip by chip, and the access log is read. It is in the menu
  unconditionally — a revocation control reachable only through an invitation
  is one nobody can reach in a hurry.
- **Nothing is pre-ticked on an invitation.** The coach's `requested_scopes`
  render as unchecked boxes labelled "asked for". A pre-selected box is not
  consent, and the accept button reads "Accept, share nothing" when nothing is
  ticked rather than nagging for a complete form.
- **The draft check-in has a Copy button and no Send button.** Deliberate, and
  the toast says so: "send it in your own words".

Not done: `/clients` is visible to everyone rather than gated on a coach
profile — the empty state explains itself, and a menu-time fetch would cost
every user a request to hide one row. Worth revisiting when G6 adds orgs.

### G5 — AI for the coach, on scoped data only — ✅ shipped 2026-08-30

This is where the AI features and the managed features meet, and where the
temptation to cheat is highest.

- **Client digest.** "What changed for this client this week" — generated from
  exactly the scopes granted, and **citing which** ("based on planner and
  check-ins; notes were not shared").
- **Drafted check-in.** A message the coach edits and sends. Never sent
  automatically: a coach's relationship is the product, and an autosent AI
  message damages it.
- **Risk flags** — a dropped rhythm, a mood trend, adherence collapse. Framed as
  _"worth asking about"_, never as an assessment. Praxis is not a diagnostic
  device and must not read like one, particularly for the therapist case.

**Hard constraint:** the prompt is built from scoped data only. A digest that
"happens to know" something outside the granted scopes is a breach that will
never appear in a log, because no read was recorded.

**Shipped:** `ManagedDigestService`, `GET /managed/client/:clientId/digest`,
19 tests. The order in `build()` is the design — ask `grantedScopes()` first,
fetch only those through `assertScope(..., 'digest')`, then build the prompt
from the fetched slices. `buildPrompt` takes slices and no client id, so it has
no route by which an ungranted slice could enter.

Four things the build settled:

- **The containment test is on the database, not the output.** Asserting the
  prompt lacks a client's notes proves little; asserting `notebook_entries` was
  never queried proves the slice could not have leaked by any path.
- **A personalization feature was the back door.** `runWithFallback` injects a
  per-user hook profile — tone learned from that user's own notes and planner
  outcomes. Passing the client's id would have folded ungranted data into the
  coach's prompt through a feature that has nothing to do with access control.
  The endpoint passes no `userId` at all, and that omission is load-bearing.
- **Clinical output is dropped, not sanitized.** A summary that needed editing
  to stop sounding like a diagnosis is not one to show a coach. The
  deterministic facts survive, so the digest stays useful without the prose.
- **Notes are counted, never quoted.** Noticing that someone wrote is a
  different act from reading what they wrote, and only the first belongs in a
  summary.

Left for G4's UI: the digest is generated on request, not on a schedule. A
nightly job would read every client's data whether or not their coach ever
looked — access without a reader, which the log would record and nobody could
justify.

### G6 — Org accounts (gym) — ✅ shipped 2026-08-30

A gym is several coaches and shared clients. Add `orgs` + membership, with the
relationship still coach↔client — a client consents to a _person_, and a gym
that reassigns a coach re-asks. Tempting to skip; skipping it means one leaving
trainer takes a roster's data with them.

**Shipped:** `migrations/orgs.sql` (`orgs`, `org_members`, nullable
`managed_relationships.org_id`), `OrgService`, `POST /api/orgs`,
`GET /api/orgs/mine`, `POST /api/orgs/:id/{invite,remove,handover}`,
`POST /api/orgs/members/:memberId/accept`, `GET /api/orgs/:id/roster`.
19 tests.

The design is defined by what it refuses:

- **Org membership grants nothing.** There is no policy giving admins SELECT on
  `managed_relationships`, the access log, or any client table. The cheap
  version — "org admins can see their org's clients" — is one policy line, and
  it is how a client who consented to one trainer becomes visible to eleven
  strangers at the front desk. A test asserts an org admin is refused by
  `assertScope` with the same reason as a total stranger, and that the refusal
  lands in the client's log.
- **The roster is numbers, never names.** A gym owner knowing "Sam has eleven
  clients" is roster management. Knowing who they are is a disclosure those
  eleven never agreed to. The test serialises the whole payload and asserts no
  client id appears in it under any key.
- **A private client is not the gym's business even in aggregate** — counts are
  scoped to relationships struck under that org, so a coach's own clients do
  not appear.
- **Handover re-asks.** It creates fresh invitations from the incoming coach at
  zero scopes, with the previous grant carried only as `requested_scopes`. The
  admin gets a count back, not a list. A purpose is required, exactly as for a
  normal invitation.
- **Removing a coach ends the employment, not the coaching.** Their grants
  survive; only the `org_id` label is cleared, because it has stopped being
  true. The endpoint says so in its response rather than letting an admin assume
  otherwise. An org that could revoke on a client's behalf could grant on their
  behalf too — the same door, facing the other way.

One trap found while building, and it is a data-loss one rather than a privacy
one: `managed_relationships` is unique on `(coach_id, client_id)`, so an
unguarded handover upsert would silently blank a live grant whenever the
incoming coach already worked with that client. Those clients are skipped and
counted separately.

Not done: no UI. The endpoints are usable but there is no org screen yet.

## AI features outside the managed track

Ordered by how much they need that nobody else can build:

1. **Explainable proposals** (U6's other half) — every suggestion states the
   evidence it came from. Also the prerequisite for a coach trusting the digest.
2. **Retro on demand** — "what actually happened last month", generated from the
   ledger rather than from vibes. `WeeklyReviewService` is the seed.
3. **Ask-my-history** — natural-language question over the user's own notes and
   trackers, grounded and cited. Physis already has this shape working over a
   corpus; the same contract applies here.

## What this plan deliberately does not do

- **No diagnosis, no clinical language, ever.** For the therapist case that is
  the line between a tool and a regulated device.
- **No coach-side write into a client's data.** A coach proposes; the client
  accepts. Otherwise the client's history stops being theirs.
- **No "anonymised" analytics across clients** until someone with a DPO hat has
  looked at it. Re-identification from a mood series and a city is not hard.
