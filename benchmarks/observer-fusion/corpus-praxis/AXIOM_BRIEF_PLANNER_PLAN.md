# Plan — Axiom brief & planner (U1–U6)

Scoped 2026-08-30 from the owner's urgent list in `docs/BUG_FIX_STATUS.md`.
Each item below names the files, the actual root cause where one was found, and
what "done" means. Order is at the end and is not the order they were reported.

---

## U1 — The morning brief renders as raw JSON

**Severity: highest.** Every Pro user sees this every morning.

### What is actually wrong

Not one bug. Three, stacked, and the renderer is only where they surface.

**(a) The extractor takes the widest possible bite.**
`src/services/axiom/llmJson.ts`:

```ts
const m = text.match(/\{[\s\S]*\}/); // first '{' to the LAST '}'
```

Greedy from the first brace to the final one. A model that emits prose
containing braces, or a fenced block followed by commentary, or two objects,
yields a span that is either invalid (throws, caught, falls back — fine) or
_valid but wrong_ (parses into something nobody designed — not fine).

**(b) A non-JSON reply becomes the brief verbatim.**
`AxiomScanService.ts:1086`:

```ts
axiomMessage = rawText.slice(0, 500); // "LLM returned plain text"
```

If the model answers with JSON the regex failed to match, this stores 500
characters of raw JSON as the user-facing message. Nothing downstream checks.

**(c) Three writers, three shapes, no contract.**
`AxiomScanService` writes `message`/`recap`/`followup`/`action`/`routine`/
`resources`/`priority`/`suggestedActions`/`energy`/`meta`/`match`/`event`.
`AxiomBriefPoolService` writes `...pick.content` plus `daypart`/`source`.
`AxiomMorningBriefService` writes a third. The client compensates in
`parseBrief()` (`AxiomMorningBrief.tsx:353`) with a legacy branch, a
`JSON.parse` fallback returning `{}`, and a final `data ?? {}` — so an
unrecognised shape reaches the render as an arbitrary object.

A comment already in that file records a previous outage from the same
disagreement: reading `action` alone "printed 'no brief yet' under a fully
populated brief, every day, for every user."

### Fix

1. **Narrow the extractor.** Balanced-brace scan from the first `{`, or prefer
   a fenced ```json block when present. Return null rather than a wrong span.
2. **Refuse to store JSON as prose.** At `:1086`, if the text still looks like
   JSON (`looksLikeJson()`), do not use it as `message` — fall through to the
   algorithmic brief, which is what that path exists for.
3. **One shape at the boundary.** A `BriefView` type plus one adapter per
   writer, applied server-side in `dashboardController` so the client stops
   guessing. `parseBrief()` becomes a read of a known shape.
4. **An honest failure state.** When no adapter recognises the row, render
   "today's brief could not be read" with a regenerate button — never the
   object. A brief that cannot be understood must say so.

**Done when:** a stored brief whose `message` is a JSON string renders the
fallback state, not the JSON; and unit tests cover the three writer shapes plus
one unrecognised one.

---

## U3 — The planner appears only outside the brief

Small, and worth doing before U2 so there is less to merge.

`AxiomMorningBrief.tsx` renders `routine` slots with completion toggles
persisted to `localStorage` (`axiom_routine_done:{user}:{date}`) — an editing
surface inside a summary. The brief should say what today is _for_; planning is
a different act.

**Fix:** the brief shows the day's shape read-only (slot times and titles, no
toggles), with one link to the planner. Move toggle state to the planner's own
store, and keep reading the existing `localStorage` key for one release so
today's checkmarks do not vanish under the user.

**Done when:** the brief has no mutating control, and completion set in the
planner is what the brief reflects.

---

## U2 — Merge the daily planner into the weekly planner — ✅ shipped 2026-08-30

`client/src/features/planner/PlannerPage.tsx` (848 lines) and
`client/src/features/dashboard/components/AxiomDailyProtocol.tsx` (516) are two
implementations of the same idea. Two will keep drifting; they already disagree
about where completion lives.

**Fix:** one planner, with the day as a **view** of the week rather than a
second component — same data, same mutations, a `view: 'day' | 'week'` toggle.
`AxiomDailyProtocol` becomes a thin day-scoped render of the planner store.

**Risk:** this is the largest item and touches a tested page
(`__tests__/PlannerPage.test.tsx`). Do it after U1 and U3, and extract the
shared store first as its own commit so the merge is reviewable.

**Done when:** one component owns slot CRUD; the day view is a filter over the
week's data; `PlannerPage.test.tsx` still passes and gains day-view cases.

**Shipped**, but not as described. `AxiomDailyProtocol` — one of the two
implementations named above — was dead code; nothing had imported it since the
brief stopped rendering a timetable. Deleted, 571 lines.

The live duplication was _inside_ `PlannerPage`: the today strip and the week
grid were two copies of one card that had drifted, and the strip had quietly
lost the note box. Both now render one `BlockCard`. The day became a real view
— a `view: 'day' | 'week'` toggle over one already-loaded dataset — and a test
asserts switching makes no request, which is what makes it one planner rather
than two.

Both view containers needed `unmountOnExit`: a collapsed MUI `Collapse` keeps
its children mounted, so the hidden view's cards were still in the DOM with
duplicate note fields and duplicate focusable controls.

The plan's count of two implementations was wrong; there were three.
`UnifiedTimetable` (used by `DayPlanCard` on the dashboard) had its own fetch of
`/planner/day` and its own card markup. It now renders the same `BlockCard`, so
all three surfaces gained inline rename, inline re-time, delete and an
always-present note box at once — and `PlannerBlock` is declared once and
imported rather than copied into each file, which is how the three drifted in
the first place.

Three tests in `UnifiedTimetable.test.tsx` lock the parity, because what
regresses next is someone adding an affordance to the week planner and not to
the dashboard.

**Still separate:** the three surfaces share the card but not the data layer —
`PlannerPage` fetches a week, `UnifiedTimetable` fetches a day. That is a
defensible split (a dashboard card should not pull seven days), so it is left
alone rather than merged for symmetry.

---

## U4 — Comments and notes on the daily planner — ✅ shipped 2026-08-30

Free-text per day and per slot, carried over from the weekly view.

Worth doing for its own sake, and it is also the input U6 needs: a note attached
to a moved slot is the cheapest available signal of _why_ it moved. Store the
text next to the slot, not in a separate notes table — a comment that outlives
the thing it comments on is noise.

**Done when:** a note survives a reload, is visible in both day and week views,
and is included in the payload U6 records.

---

## U5 — Simpler editor — ✅ shipped 2026-08-30

The current surface is heavier than the task.

**Target:** change a slot in one interaction — inline edit of time and title, no
dialog for the common case. Keep a dialog only for the rare full edit.

**Done when:** re-timing a slot takes one click and one keystroke sequence, no
modal.

---

## U6 — Remember edits; adapt proposals to the person — ✅ shipped 2026-08-30

The largest and the last, because it needs U4's signal and a consent decision.

### Two halves, deliberately kept apart

**Edit memory.** Every accepted, moved, deleted or re-timed proposal is
evidence. Store _the edit and its outcome_ — proposal id, what was proposed,
what the user did, when, and U4's note if there is one. Do **not** store a
derived "preference score": a score cannot be audited later and cannot be
explained back to the user when their proposals change. Derive at read time.

**Personalization inputs.** Place, age, sex, goals and timetable.

> **Consent is a blocker, not a footnote.** Sex and health-adjacent inference
> are special-category data under GDPR art. 9: they need explicit, specific
> consent and a stated purpose, not a field quietly added to a profile. They
> must also appear in the Play data-safety declaration
> (`client/android/PUBLISHING.md`). Ship the consent flow in the same release as
> the first use of these fields, or the feature ships unlawful.

### Shape

- `axiom_proposal_edits` — append-only: `(user_id, proposal_id, kind, from, to, note, at)`.
- Retrieval at proposal time: the last N edits for this user, this daypart, this
  domain — fed to the generator as context, the same way the brief already feeds
  recent briefs back to avoid repetition.
- Adaptation must be **explainable**: the proposal says "moved to 18:00 because
  you have moved this three times" or it does not claim to have learned.

**Done when:** an edit changes the next proposal in a way the user can see the
reason for, and every input used is one they consented to.

**Shipped**, and the shape changed once the code was read.

**Edit memory needed no new table.** `planner_block_events` already is one:
append-only, storing the edit and its outcome rather than a derived score, with
`buildLedger` deriving verdicts at read time. That is exactly what this section
specified, written months earlier under another name. An
`axiom_proposal_edits` table was drafted and deleted before it shipped — two
ledgers disagreeing about the same week is the failure this codebase keeps
producing, not a fix for it.

What was actually missing there was the _why_: retime, retitle and status
events recorded what changed and lost U4's note. The note now travels with the
event, read at edit time because it may later change or vanish with the block.

**Consent was the real work.** `axiom_personalization_consent` is opt-in, per
category, default none. An absent row is a no, never "not asked yet". The
service fails closed, and — the part worth keeping — it selects **only the
granted columns** from `profiles`. Fetching the row and filtering afterwards
would leave ungranted values in the process, one careless edit away from a
prompt; a test asserts on the select string rather than on the output, because
only the select string proves the value was never fetched. Same reasoning as
the coach digest's `grantedScopes()`-before-fetch ordering.

`sex` is described in the UI as optional out loud. It is the category people
are most likely to feel pressured into and the one that legally needs the
clearest yes.

**Explainability** is a prompt rule plus a `basis` string returned with every
generated plan: "Planned using where you are, your goals." Empty when nothing
personal was used, so a plan built from no personal facts never claims
otherwise.

The Play data-safety declaration now covers these — `client/android/PUBLISHING.md`
lists approximate location, age and sex as optional, consent-gated, and shared
with an LLM processor when the plan generator runs. Sex is answered separately
from the rest rather than folded into a general "personalization" answer.

A granted category with no value behind it is reported back: allowing `age` when
the profile has no age was a silent no-op, where the toggle said yes and the
plan stayed identical with nothing on screen to explain why.

---

## Order

1. **U1** — daily, visible, and the root cause is known
2. **U3** — removes surface before it has to be merged
3. **U2** — the merge, with less to merge
4. **U4**, **U5** — the editor, once there is one planner to edit
5. **U6** — last: needs U4's signal and the consent decision

U1 and U3 shipped 2026-08-30 — see `docs/BUG_FIX_STATUS.md` for what each
actually turned out to be. Two things worth carrying into U2–U6:

- **The renderer is rarely the bug.** U1 was a greedy regex, a fallback that
  stored JSON as prose, four components each reading the table directly, and a
  notebook row written from the same bad value. Every pass that fixed only what
  was on screen left the cause in place.
- **A fix is not shipped until the artifact is rebuilt.** `praxis-autoupdate`
  refuses to run on a dirty tree and had been skipping every deploy; the code
  was on the host and the compiled output was a day old. Check
  `logs/auto_update.log` before believing a fix is live. U2 onward are separate, reviewable commits.

**Found while scoping:** `PlannerPage.test.tsx` fails on a missing `Note for
Axiom` field for `source='axiom'` blocks — pre-existing, verified by stashing.
That field is U4's, and its absence is why U6 has no signal to learn from today.
