# Praxis — Go-to-Market Targets

> Snapshot: 2026-08-26. Companion to `BUSINESS_PLAN.md` (strategy) and
> `PITCHES.md` (messaging).
>
> **Status of the names below: research leads, not qualified prospects.** They
> were found by web search and have not been contacted, and their fit against
> the ICP has not been verified. Qualify each one against §2 before spending a
> personalised email on it.

---

## 1. What we are selling, restated

Not a course platform. Not a habit tracker.

> **The accountability layer between sessions.**

The buyer runs a recurring group programme and loses people in the gap between
one session and the next. Today that gap is a WhatsApp group and a spreadsheet,
and the operator only learns someone disengaged when they stop showing up.

Praxis makes the gap visible: shared cohort goals, assignable tasks with
accept/decline, daily check-ins, streaks, peer verification of completed work,
and a weekly AI-generated cohort brief. Plus signed, publicly verifiable
credentials at completion — which the buyer can hand to participants as proof.

---

## 2. Ideal Customer Profile — qualification checklist

A lead is qualified when **all five** are true. Score honestly; an unqualified
lead consumes a week and teaches nothing.

- [ ] **Runs recurring group programmes** of 8–100 people (cohorts, courses,
      class-based training, team programmes) — not purely 1:1
- [ ] **Charges more than €200 per participant**, so €99/mo is under ~2% of
      programme revenue and the buying decision is not a budget fight
- [ ] **Has stated dropout, adherence or "between-sessions" as a problem** — in
      their marketing, a podcast, a LinkedIn post, or on the discovery call.
      Do not sell them the problem; find someone who already has it.
- [ ] **Currently improvises the solution** — WhatsApp group, Google Sheet,
      Notion, manual check-in messages
- [ ] **One person can say yes.** Owner-operator or programme director. No
      procurement, no committee, no security review.

### Hard disqualifiers at this stage

- Requires white-label or custom branding → not built (`BUSINESS_PLAN.md` §2)
- Requires a signed DPA / GDPR processor agreement → not built. **This rules out
  corporate HR and enterprise wellness for now.**
- Requires an SLA or uptime guarantee → two self-hosted machines, one operator
- Requires SSO, SCIM, or an integration with an existing LMS/HRIS
- Wants to replace their course platform → wrong product, walk away
- Public-sector or university procurement track → months of process, revisit at
  10 paying customers

---

## 3. Segments, ranked

Ranked by expected time-to-first-euro, not by market size.

### Tier 1 — Start here

#### 1A. Independent coaches running group programmes ⭐ best first customer

Life, business, executive and health coaches who run cohort programmes rather
than only 1:1 sessions.

- **Why they fit:** owner-operator decides alone; dropout is a known,
  openly-discussed pain; no procurement; no DPA needed if participants consent
  directly; already paying for tools
- **Why they buy:** retention is their revenue. A participant who disengages in
  week 2 does not renew and does not refer.
- **Deal size:** the live product has **no per-cohort B2B tier yet**. Real tiers
  (see `client/src/features/payments/premiumTiers.ts`): Free, Pro $10/mo,
  Pro $80/yr, Lifetime $249 — and **groups/cohorts are a free feature** anyone
  can create. So the wedge is: sell the *operator* a Pro account ($10/mo or
  Lifetime $249) and have them invite their class as free members into a Praxis
  group. Do NOT quote "€99/mo Cohort" — that price does not exist. Pilot = free
  operator Pro for 4 weeks.
- **Reachable via:** the directories in §4

#### 1B. Personal trainers and small studios with group/online programmes

PTs running 8–12 week transformation programmes, small-group training, or online
coaching packages.

- **Why they fit:** adherence _is_ the product; results depend on client
  behaviour between sessions; small operations, fast decisions
- **Why they buy:** client results drive testimonials drive sales. Visibility
  into who skipped the week is directly monetisable.
- **Deal size:** operator Pro ($10/mo / $80/yr / $249 Lifetime); class joins free
  via the Groups feature. Multi-trainer studio = multiple Pro seats. Pilot = free
  operator Pro for 4 weeks.
- **Geographic advantage:** Verona is local. In-person demo beats a cold email by
  an order of magnitude. **Walk into three gyms before sending fifty emails.**

#### 1C. Coaching schools running certification cohorts

ICF/AICP-accredited schools run multi-month cohorts of 10–30 students who must
complete practice hours and log work between modules.

- **Why they fit:** structurally identical to the wedge — long programmes,
  scheduled sessions, months of unmanaged gap in between
- **Why they buy:** practice-hour logging and verification is a real
  administrative burden. Peer verification with anti-collusion limits
  (`VerificationLimitService.ts`) and signed credentials map onto it almost
  exactly. **This is the strongest product-market fit on the list.**
- **Deal size:** operator Pro ($10/mo / $80/yr / $249 Lifetime); students join the
  cohort group free. **This is the strongest product-market fit on the list.**
- **Caution:** slower decisions than 1A/1B; often two or three people involved

### Tier 2 — After the first pilot converts

#### 2A. Cohort-based online course creators

Runs 4–12 week live cohorts on Maven/Disco/Thinkific/Circle.

- **Fit:** high. Same gap; they talk about completion rates constantly.
- **Risk:** they may want Praxis to replace their platform. It does not.
  Position strictly as a complement.
- **Benchmark context:** Disco from $79/mo (50 students), Thinkific from
  $36/mo, one platform at $1.99/active learner/mo. Praxis at €99 must be framed
  against _programme revenue_, never against those tools' pricing.

#### 2B. University programme coordinators and student support services

Thesis cohorts, career services, student mentoring programmes.

- **Fit:** high on the problem, poor on the buying process.
- **Existing hook:** `univrRoutes.ts` and `campusExamRoutes.ts` already exist —
  there is a Verona campus surface with unauthenticated exam-date lookup.
- **Play:** ignore procurement. Find one professor or programme coordinator with
  discretionary budget or a pilot mandate and go direct.
- **Deal size:** €500+/mo, but 3–6 month sales cycle. Not a week-1 target.

### Tier 3 — Do not pursue yet

#### 3A. Corporate wellness and EAP providers

Italy corporate wellness is ~$2.0B in 2026 growing to ~$2.5B by 2031 (4.6%
CAGR), with typical programme spend around €500/employee/year. Named players
include Jointly, Health Italia S.p.A., LifeGate Way and Trainers Plus.

**Blocked, and the block is real:** a DPA and GDPR processor posture are
mandatory, procurement is a committee, and an SLA will be demanded. Revisit once
those three gaps in `BUSINESS_PLAN.md` §2 are closed and there are reference
customers. Attractive market, wrong stage.

#### 3B. Addiction recovery and behavioural health programmes

Excellent problem fit; health data is special-category under GDPR Art. 9.
Do not touch without legal review.

---

## 4. Lead sources — where to build the list of 40

These are directories and listings that produce many leads at once. This is the
highest-leverage hour of the week.

| Source                                            | Segment | URL                                                              | Notes                                                                                     |
| ------------------------------------------------- | ------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| ICF Italia — accredited schools list              | 1C      | `coachingfederation.it/scuole-accreditate-icf/`                  | Official directory of ICF-accredited Italian schools. **Best single source for Tier 1C.** |
| ICF Italia                                        | 1A / 1C | `coachingfederation.it`                                          | Italian chapter of ICF (30k+ members across 138 countries)                                |
| AICP — Associazione Italiana Coach Professionisti | 1A      | `associazionecoach.com`                                          | 800+ members, 17 regions, registered under Law 4/2013                                     |
| Elenco Italiano dei Coach                         | 1A      | `associazionecoachingitalia.it/elenco-dei-coach-professionisti/` | Public coach listing                                                                      |
| INCOACHING (Milan)                                | 1C      | `incoaching.it`                                                  | ICF Level 1 provider; 100-hour programme; Milan                                           |
| 4C School of Coaching                             | 1C      | Milan / Monza / Como                                             | Level 1; cohorts capped at 10 — small but a clean pilot shape                             |
| Life Coach Italy                                  | 1C      | `lifecoachitaly.it`                                              | ICF Level 2; also runs an ICF-certified **team** coaching track (AATC) — strong fit       |
| Learning Coach Solutions                          | 1C      | `learningcoachsolutions.it`                                      | ACTP / Level 2, AICP-recognised                                                           |
| NonSoloFitness — courses by city                  | 1B      | `nonsolofitness.it/sede/verona`                                  | PT and instructor courses in Verona; both the school _and_ its graduates are leads        |
| ASI Fitness Academy — Verona                      | 1B      | `asifitnessacademy.it/personal-trainer-verona/`                  | Issues PT diplomas, national register enrolment                                           |
| Fitnessway (NASM Italy)                           | 1B      | `fitnessway.it/corso/corso-personal-trainer-verona/`             | NASM-affiliated, EREPS/EuropeActive accredited                                            |
| Superprof — Verona sport coaches                  | 1B      | `superprof.it/lezione/coach-sportivo/verona/`                    | ~36 listed coaches in Verona; individually reachable                                      |
| The Coach Verona                                  | 1B      | `thecoachverona.club/`                                           | Local PT studio, San Massimo — **walk-in distance**                                       |
| ensun — corporate wellness Italy                  | 3A      | `ensun.io/search/corporate-wellness/italy`                       | 73 companies listed. Tier 3 — for later.                                                  |

### The Verona advantage

Roughly a third of the list is within travel distance. An in-person coffee
converts at a multiple of a cold email, and it produces the honest objections
that a cold email never surfaces. **Allocate week 1 to walking into local gyms
and studios**, and use email only for what cannot be reached in person.

---

## 5. Outreach cadence

Per lead, over ~12 days:

| Day | Action                                                 |
| --- | ------------------------------------------------------ |
| 0   | Personalised cold email (`PITCHES.md` §3)              |
| 3   | LinkedIn connection request, no pitch attached         |
| 5   | Follow-up email #1 — one new, concrete angle           |
| 12  | Follow-up email #2 — the breakup email                 |
| —   | Stop. Mark `closed-no-response`. Do not send a fourth. |

**Rules:**

- Every email names something specific about _their_ programme. A template that
  goes out unedited is worse than nothing — it burns the address and teaches you
  nothing.
- Sequence in batches of 10, not 40. Read the replies (and the silence) from
  batch 1 before sending batch 2, and rewrite the message between batches.
- Track reply rate per segment. If one segment replies at 3× another after 20
  sends, drop the others and go deep.

### Weekly scoreboard

Report these numbers, and only these numbers, every Friday. Commits are not on
the list on purpose.

| Metric                    | Week 1 target | Week 2 target   |
| ------------------------- | ------------- | --------------- |
| Qualified leads added     | 20            | 40 (cumulative) |
| Personalised emails sent  | 20            | 40 (cumulative) |
| In-person visits (Verona) | 3             | 6 (cumulative)  |
| Replies received          | —             | ≥4              |
| Discovery calls booked    | 2             | 6 (cumulative)  |
| Pilots signed             | 0             | 1               |

---

## 6. Tracking

Keep this in a sheet, not in the codebase. Minimum columns:

`name · organisation · segment (1A/1B/1C/2A/2B) · source · ICP score (0–5) ·
first contact date · last touch · stage (new → contacted → replied → call booked
→ call held → pilot → paid → closed-lost) · objection raised · next action ·
next action date`

The **objection raised** column is the most valuable one in the sheet. Twenty
recorded objections are worth more than the next twenty features — they are the
only real market research this project has ever collected.
