# Praxis — Business Plan

> Snapshot: 2026-08-26. Supersedes any earlier revenue assumptions.
> Pairs with `GTM_TARGETS.md` (who to sell to) and `PITCHES.md` (what to say).
> Update the metrics table on the first of each month. If the numbers do not
> move, the plan is wrong, not the execution.

---

## 1. Where we actually are

Measured on production (pansa), 2026-08-26. These are not projections.

| Metric                                    | Value                                                              |
| ----------------------------------------- | ------------------------------------------------------------------ |
| Registered users (`auth.users`)           | 47                                                                 |
| Signed up in last 30 days                 | 3                                                                  |
| Signed in in last 7 days                  | **1**                                                              |
| `profiles.is_premium = true`              | 5                                                                  |
| `profiles.premium_until > now()` (gifted) | 0                                                                  |
| Paying subscribers                        | 0 confirmed                                                        |
| MRR                                       | €0                                                                 |
| Commits                                   | 1,582 since 2026-02-06 (~6.5 months)                               |
| Backend                                   | 502 TS files · 95 route files · 90 controllers · 101 services      |
| Frontend                                  | 483 files · 51 feature areas                                       |
| Stripe                                    | `sk_live` + 5 price IDs + webhook secret — **live and functional** |

### The diagnosis

The payment rail is built, tested (`stripeWebhook.test.ts`, `premiumStatus.test.ts`,
`boostPrice.test.ts`) and running against live Stripe keys. The tier gate
(`src/middleware/requireTier.ts`) is applied to ~22 routes. Nothing about
collecting money is unbuilt.

Revenue is €0 because **47 people have ever seen the product and one of them
used it this week**. Engineering capacity was never the constraint. Six and a
half months of full-time output produced a platform with more feature areas (51)
than weekly active users (1).

Two structural errors follow from that, and both are addressable:

1. **The paywall sits behind the churn cliff.** Pro gates wiki search, advanced
   analytics, community comparison, sparring and cohorts. A consumer who churns
   on day 2 never reaches any of them. Meanwhile the daily LLM brief — the only
   feature with a genuine per-user-per-day cost — is free forever.
2. **The premium tier is mostly multiplayer.** Matching, sparring, duels,
   leaderboards and peer verification all require density. At 47 users there is
   none, so the paid tier is a paywall in front of empty rooms.

### The strategic consequence

Consumer subscription at €10/mo is the **slowest** available path to revenue from
this position. It needs thousands of top-of-funnel visitors against a category
(habit/journal apps) with roughly 5% 30-day retention, competing with free
incumbents. Current top-of-funnel is 3 signups per month.

**The pivot is not a rewrite. It is a change of buyer.** Sell to an organisation
that already has the users, instead of acquiring users one at a time.

---

## 2. Strategy: pilot-led B2B, consumer as byproduct

### The wedge

Praxis is not a course platform and should never be sold as one. It is the
**accountability layer between sessions**.

Every coach, trainer and cohort programme has the same unmanaged gap: the six
days between one session and the next. Today that gap is run on a WhatsApp group
and a spreadsheet. The operator has no visibility into who is actually doing the
work until someone quietly stops showing up.

Praxis already fills that gap with primitives that exist and run today.

### What is already built for this buyer

Verified against the codebase, not aspirational:

| Capability                                                   | Where it lives                              | State              |
| ------------------------------------------------------------ | ------------------------------------------- | ------------------ |
| Group/cohort container with roles (`admin`/`mod`/`member`)   | `src/controllers/groupController.ts:75,437` | Shipped            |
| Private groups with join-request approval/rejection          | `groupController.ts:88,398,420`             | Shipped            |
| Shared group goals                                           | `src/routes/groupRoutes.ts:61-64`           | Shipped            |
| Assignable group tasks with accept/decline                   | `groupRoutes.ts:66-70`                      | Shipped            |
| Group messaging                                              | `groupRoutes.ts:48-49`                      | Shipped            |
| AI group brief (weekly aggregate: tasks done, goal progress) | `src/routes/axiomRoutes.ts:402`             | Shipped, Pro-gated |
| Peer verification of completed goals                         | `completionController.ts`                   | Shipped            |
| Anti-collusion limits on verification                        | `src/services/VerificationLimitService.ts`  | Shipped            |
| Signed, publicly verifiable credentials + `/cv/:username`    | `src/routes/credentialRoutes.ts`            | Shipped            |
| Time-boxed cohort challenges                                 | `src/routes/cohortRoutes.ts`                | Shipped, Pro-gated |
| Daily AI brief, streaks, check-ins, 20+ trackers             | core                                        | Shipped            |
| Offline-capable installable PWA                              | `client/public/service-worker.js`           | Shipped            |
| Real-money commitment device (Stripe, €1–500)                | `src/controllers/realBetsController.ts`     | Shipped            |

This is a genuinely unusual bundle. The peer-verification-with-anti-collusion
plus signed credentials combination is the defensible part, and it is worth
nothing in a single-player consumer app.

### What is NOT built — do not promise it

This list is load-bearing. Every pitch in `PITCHES.md` is constrained by it.

| Gap                                                                                                                                       | Impact on a pilot                                                                                      | Effort to close |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | --------------- |
| **No organisation/tenant model.** A "group" is a chat room, not an account.                                                               | Acceptable for pilot: one room per cohort. Blocks multi-cohort customers.                              | 1–2 weeks       |
| **No org-admin dashboard.** Admin analytics are global superadmin only (`requireAdmin`). A coach cannot see their own cohort's adherence. | **This is the #1 gap.** It is the thing the buyer is paying for. Must be built during the first pilot. | 1–2 weeks       |
| **No seat-based billing.** Stripe sells individual Pro/Boost/lifetime only.                                                               | Invoice manually for the first 5 customers. Do not build this early.                                   | 1 week          |
| **No org-scoped data export** for the coach.                                                                                              | Manual CSV on request during pilot.                                                                    | 2–3 days        |
| **No per-org branding / white-label.**                                                                                                    | Do not sell to anyone who requires it yet.                                                             | 2+ weeks        |
| **No DPA / GDPR paperwork for processor role.** Selling B2B in the EU means handling employee or client data as a processor.              | **Blocks any corporate/HR deal.** Not required for a solo coach pilot with client consent.             | 1 week + review |
| **No SLA, no on-call, no status page.** Two self-hosted home machines, single operator.                                                   | Be explicit about this in the pilot agreement. Do not sign an SLA.                                     | Ongoing         |

**Rule: the first pilot's scope is the intersection of what is already shipped
and what one specific customer needs.** Build the org-admin dashboard only after
a named customer has agreed to a pilot, so it gets built against a real
requirement instead of an imagined one.

---

## 3. Pricing

### Pilot (first 3–5 customers)

- **Free for 6 weeks**, up to 25 seats.
- Not a discount — an explicit trade. In exchange: weekly 20-minute feedback
  call, permission to use anonymised adherence data, and a reference/testimonial
  if the pilot works.
- Ends with a yes/no conversation on a specific date agreed up front. A pilot
  without a decision date is a free forever plan.

### Paid (post-pilot)

| Plan        | Price        | Seats     | For                                 |
| ----------- | ------------ | --------- | ----------------------------------- |
| Cohort      | €99/mo       | up to 30  | Solo coach, PT, one class           |
| Studio      | €249/mo      | up to 100 | Multi-coach studio, training school |
| Institution | from €500/mo | 100+      | University programme, corporate     |

Rationale for the floor: benchmark cohort platforms sit at $36–259/mo
(Thinkific, Disco, ProProfs). Praxis is a **complement** to those, not a
replacement, so it must be cheap relative to programme revenue. At €99/mo across
a 20-person cohort priced at €300/head (€6,000 programme revenue), Praxis is
1.7% of revenue. That is an easy yes if it demonstrably reduces dropout.

Do not price per-seat below €99/mo total. Sub-€99 deals cost more in support than
they return, and at this stage every customer is a high-touch design partner.

### Consumer tier — the four changes

Keep it running, stop investing in it, and make these fixes because they are
cheap and they also improve the B2B story:

1. **Move the paywall to the daily brief.** Free for 14 days, then Pro. It is
   the habit-forming feature _and_ the per-user cost line — gate and margin
   finally align. Ungate wiki search and analytics; nobody pays for those.
2. **€5/mo, not $10.** And fix the currency inconsistency: the tier UI
   (`client/src/features/payments/premiumTiers.ts`) quotes USD while real-money
   commitments are in EUR. Pick one currency.
3. **Hide ~30 of the 51 feature areas from navigation.** Hide, not delete. The
   product needs one sentence and three screens.
4. **Drop lifetime from €249** or remove it. It is unpriceable for an unproven
   product and it sells future MRR at a discount. If used at all, use it as a
   capped founding-member instrument (see §5).

---

## 4. Unit economics

### B2B (target state)

| Line                          | Value                                                          |
| ----------------------------- | -------------------------------------------------------------- |
| ARPA                          | €99–249/mo                                                     |
| Gross margin                  | >90% (self-hosted; marginal cost is LLM inference)             |
| LLM cost per active seat      | ~€0.30–1.00/mo (daily brief, `InferenceBudgetService` caps it) |
| CAC                           | ~0 in cash, high in founder hours (direct outreach)            |
| Break-even on hosting + tools | ~1 Studio customer or ~3 Cohort customers                      |

### Real-money commitments — a warning

The 8% fee on €1–500 commitments does not clear Stripe's floor at the low end.
On a €5 commitment: gross fee €0.40, Stripe takes roughly €0.25 + 1.5% ≈ €0.33.
Net €0.07, while carrying the full regulatory and custody risk of holding user
funds.

**Actions:**

- Set a minimum commitment of €25 so the fee clears costs, or
- Reframe the fee as a flat handling charge, and
- **Get the commitment-device framing legally reviewed before any growth.** In
  the EU, taking custody of user money against a future outcome invites a
  gambling/payment-services question. This is the one risk on the list that ends
  the project rather than slowing it. It is not urgent at 47 users; it becomes
  urgent the moment it is marketed.

---

## 5. Cash bridge

Revenue from B2B realistically lands in 4–8 weeks. Two instruments cover the gap:

**A. Freelance / contract work (days to first invoice).** The demonstrable asset
is a deployed full-stack PWA with live Stripe, self-hosted Supabase, an offline
service worker, pgvector matching and a ~40-provider LLM routing layer. That is a
€300–600/day skill set in the EU market. `ForFreelancersPage.tsx` already exists
in the codebase — the idea was already there. Praxis becomes the portfolio piece.
This is the primary cashflow source and should be treated as such, not as a
failure.

**B. Founding-member lifetime, capped (1–2 weeks).** Offer €39–59 lifetime to the
existing 47 users and personal network. Twenty conversions ≈ €800–1,200 cash and,
more valuable, the first real willingness-to-pay signal.
**Cap it at 25 seats and close it.** Uncapped lifetime sales are a permanent
support liability funded by a one-off payment.

---

## 6. Ninety-day plan

Weeks are numbered from 2026-08-26.

### Weeks 1–2 — Outreach only. No code.

- [ ] Build a list of 40 qualified targets (see `GTM_TARGETS.md`)
- [ ] Send 40 personalised cold emails/DMs (see `PITCHES.md`)
- [ ] Book 6+ discovery calls
- [ ] Launch capped founding-member offer to the existing 47 users
- [ ] Start freelance outreach in parallel

**Hard rule for these two weeks: no feature commits.** The failure mode this plan
exists to correct is retreating into the codebase. If a call surfaces a missing
feature, write it down; do not build it.

### Weeks 3–4 — Convert to pilots

- [ ] Run discovery calls; qualify hard against the ICP checklist
- [ ] Sign 2–3 free pilots with named decision dates
- [ ] Build **only** the org-admin adherence dashboard, scoped to what pilot
      customers asked for on those calls
- [ ] Ship the consumer paywall move (brief after 14 days) — one day of work

### Weeks 5–10 — Run the pilots

- [ ] Weekly 20-minute call with each pilot customer, every week, no exceptions
- [ ] Instrument and report cohort adherence weekly — this report _is_ the product
- [ ] Close gaps that pilot customers actually hit, in the order they hit them
- [ ] Ask for the reference/testimonial at week 4 of the pilot, not week 6

### Weeks 11–13 — Convert to paid

- [ ] Conversion conversations on the agreed dates
- [ ] Target: **2 paying customers, €200–500 MRR**
- [ ] Build seat billing only if a customer is blocked on it
- [ ] Write the case study from the first conversion

### Success criteria at day 90

- ≥2 paying B2B customers, or
- ≥€500 MRR from any source, or
- ≥1 signed pilot with a written commitment to pay on conversion

### Kill criteria — decide in advance, honour them

If at day 90 **all** of the following are true, the B2B thesis is wrong and
Praxis should be reclassified as a portfolio project and personal tool, with
effort moving to freelance/contract work:

- 40+ qualified outbound contacts made, and
- fewer than 3 discovery calls booked, and
- zero pilots signed

Writing these down now is the point. The premortem's finding was not that the
product is bad; it was that six months of building happened without a single
falsification test. This section is that test.

---

## 7. Risk register

| #   | Risk                                                | Severity                                             | Mitigation                                                                                    |
| --- | --------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| 1   | **Retreating into the codebase instead of selling** | Fatal — this is the observed historical failure mode | Weeks 1–2 feature freeze; measure outbound sent, not commits                                  |
| 2   | No distribution channel ever emerges                | Fatal                                                | Direct outbound is the channel; kill criteria at day 90                                       |
| 3   | Real-money commitments attract regulatory attention | Fatal                                                | Legal review before any marketing; €25 floor; do not scale unreviewed                         |
| 4   | Single-operator infrastructure fails during a pilot | High                                                 | No SLA in pilot agreement; be explicit about beta status; document a restart runbook          |
| 5   | GDPR processor obligations block corporate deals    | High                                                 | Do not target corporate/HR until a DPA exists; solo coaches with client consent first         |
| 6   | Product surface confuses buyers                     | Medium                                               | Hide 30 feature areas; demo exactly three screens                                             |
| 7   | LLM cost outruns revenue on a large cohort          | Medium                                               | `InferenceBudgetService` caps already exist; verify caps apply per-org before a 100-seat deal |
| 8   | Pilot never converts (free forever)                 | Medium                                               | Decision date in writing at pilot signature                                                   |

---

## 8. What success looks like at 12 months

Not "a consumer app with 10,000 users". That path requires marketing spend and
retention mechanics that do not exist here.

Realistic: 8–15 paying organisations at €99–249/mo — €1,000–3,000 MRR — serving
200–800 end users who arrive through their coach rather than through a funnel.
The consumer app continues as the client surface those users log into, funded by
the B2B contracts rather than by its own subscriptions.

That is a business that one person can run on two self-hosted machines. Ten
thousand consumer users is not.
