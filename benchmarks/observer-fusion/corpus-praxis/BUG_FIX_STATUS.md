# 🚀 PRAXIS — DEPLOYMENT & BUG FIX STATUS

**Last Updated:** March 28, 2026  
**Growth Sprint:** 100% Complete ✅

---

## 🔴 URGENT — Axiom brief & planner (added 2026-08-30)

Reported by the owner. Scoped in `docs/AXIOM_BRIEF_PLANNER_PLAN.md`.

**Status 2026-08-30 — U1 and U3 fixed, deployed and verified on pansa.
U2, U4, U5, U6 scoped, not built.**

### U1. Morning brief renders as raw JSON

**Where:** `client/src/features/dashboard/components/AxiomMorningBrief.tsx`,
`parseBrief()` around line 353.

**Why it happens:** three services write `axiom_daily_briefs` and they do not
agree on a shape — `AxiomScanService` emits `message`/`routine`/`resources`,
`AxiomBriefPoolService` emits `message` alone, `AxiomMorningBriefService` emits
`action`. `parseBrief()` already carries a legacy-shape branch and a
`JSON.parse` fallback that returns `{}` on failure. Any shape none of those
branches recognise falls through to `data ?? {}` and whatever the render does
with an unexpected object ends up on screen.

**FIXED 2026-08-30.** It took three passes and the first two were wrong in
instructive ways, so all of it is recorded.

**Server — where the JSON came from.**

1. `src/services/axiom/llmJson.ts` matched `/\{[\s\S]*\}/`: greedy from the
   first brace to the **last** one in the reply. Prose after the object, or a
   second object, produced a span that either threw or parsed into something
   nobody designed. Now a brace-depth scan that respects strings and escapes,
   prefers a fenced block, and returns null for an unterminated object rather
   than handing the rest of the buffer to `JSON.parse`. 14 unit tests.
2. `AxiomScanService` then sliced an unparseable reply to 500 characters and
   stored it as the user-facing `message` — and an unparseable reply is
   precisely one that _is_ JSON. `looksLikeJson()` now routes it to the
   algorithmic fallback. A model that nests JSON inside `message` parses
   cleanly and would still have leaked, so that is discarded too.

**Client — why fixing the server was not enough.** Four components read
`axiom_daily_briefs` from Supabase _directly_, so there is no single fetch to
normalize at. The first pass guarded only `AxiomMorningBrief`; the other three
kept rendering. `client/src/lib/axiomBrief.ts` is now the single reading
(`readBrief` / `isProse`), used by `AxiomMorningBrief`, `AxiomDailyProtocol`,
`AxiomNoteCard` and `DayDetailView`, with 13 tests.

**The JSON the owner was actually looking at was none of those.** It was a
`notebook_entries` row: `AxiomScanService` auto-saves each brief with
`content: axiomMessage` verbatim, so rows written before the guard hold the raw
payload, and `NotebookEntryCard` rendered it as prose. The server no longer
writes a non-prose entry (the structured payload is already in
`metadata.full_brief`), and the card shows one line for `axiom_brief` entries
that are not prose. Only that entry type is checked — the guard is for
machine-written rows, never for what a person typed.

**Still open (deliberate):** the three writer services still disagree on shape.
The `BriefView` + per-writer adapter normalization is the real repair and is
scoped in the plan; the above stops the user-visible symptom without a schema
migration.

**Existing rows:** `regen_pro_briefs.js` regenerates today's brief for pro/admin
users whose stored message is not prose, and only those — forcing all of them
would overwrite briefs people may have read to produce nothing better, which is
the same reason `AxiomProBriefCron` does not force.

### ⚠️ Why the first two fixes appeared not to work

Recorded because the failure was not in the code, and cost three rounds of "it
still does it".

`praxis-autoupdate.timer` runs every five minutes and **refuses to update while
the working tree is dirty** — and said so on every tick in
`logs/auto_update.log`:

```
WARNING: uncommitted changes in /home/gio/praxis_webapp — refusing to update.
```

Dirty: `client/src/AppRouter.tsx`, `client/src/features/home/MePage.tsx`,
`src/services/llm/LlmProviderRegistry.ts`, plus untracked Capacitor output and
`*.tmp.js` scratch files. So the _source_ on the host was current — the fix
commits were ancestors of deployed HEAD — while the **artifacts were not**:

| Artifact                            | Built        | Fix landed         |
| ----------------------------------- | ------------ | ------------------ |
| `dist/services/AxiomScanService.js` | Aug 29 20:29 | Aug 30 11:1x       |
| `client/dist/assets/*`              | Aug 30 10:21 | Aug 30 11:14–11:25 |

The compiled server contained zero occurrences of `looksLikeJson` and the
compiled `llmJson.js` was still the greedy regex. Both rebuilt by hand,
`praxis-backend` restarted, served bundle verified to contain the guard.

**Still open:** the updater is blocked by those same uncommitted files, so the
next fix will silently fail to deploy too. Commit or discard them, and gitignore
the Capacitor build output.

**Also:** the client ships a service worker (`[precache] 122 assets`), so a
stale bundle can survive a correct deploy until the worker updates. Hard-reload
before judging any UI fix.

### U2. Merge the daily planner into the weekly planner

**Where:** `client/src/features/planner/PlannerPage.tsx` (848 lines) and
`client/src/features/dashboard/components/AxiomDailyProtocol.tsx` (516).

Two planners maintained separately will keep drifting. Merge to one, with the
day as a view of the week rather than a second implementation of it.

**Note:** `client/src/__tests__/PlannerPage.test.tsx` has **one pre-existing
failure** — it expects a `Note for Axiom` placeholder that does not render for
`source='axiom'` blocks. Verified pre-existing (fails identically with the
2026-08-30 changes stashed). It is U4's territory: the feedback-note field that
test expects is exactly the signal U6 needs, and it is missing today.

### U3. Planner appears only outside the morning brief — ✅ FIXED 2026-08-30

The brief should say what today is _for_; planning is a different act and does
not belong inside a summary the user reads once.

**Fixed, in two parts.** The first pass only stopped the brief _mutating_
completion — tapping a slot now navigates to `/planner` and completion is still
_read_ from the same `localStorage` key, so nobody's ticks vanish.

But the visible half was different: `<UnifiedTimetable>` was rendered inside
`AxiomMorningBrief` **and** by `DayPlanCard` immediately below it on the
dashboard — same component, same `planner_blocks` rows, same date, two identical
20-slot timetables on one screen. Removed from the brief. One render site
remains (`DayPlanCard`), plus `/planner`.

The duplication is fed by `AxiomScanService`, which syncs the generated
`routine` into `planner_blocks` with `source: 'axiom'`. That sync is correct and
stays; the bug was rendering the result twice.

### U4. Comments and notes on the daily planner

Free-text boxes per day (and per slot) carried over from the weekly view. This
is also the input U6 needs: a note attached to a moved slot is the cheapest
signal of _why_ it moved.

### U5. Simpler editor

The current editing surface is heavier than the task. Target: change a slot in
one interaction, no dialog for the common case.

### U6. Remember edits; adapt proposals to the person

The system should learn from what the user actually does to a proposal, and
propose against their context: **place, age, sex, goals, timetable**.

Two things to keep apart when building this:

- **Personalization inputs.** Place, age, sex, goals and timetable are user
  data; sex and health-adjacent inferences are _special category_ data under
  GDPR art. 9, which needs explicit consent and a stated purpose — not a field
  quietly added to a profile. It also has to appear in the Play data-safety
  declaration (see `client/android/PUBLISHING.md`).
- **Edit memory.** Every accepted, moved or deleted proposal is evidence. Store
  the edit and the outcome, not a derived "preference score" — a score cannot be
  audited later and cannot be explained back to the user when the proposals
  change.

**Suggested order:** U1 (users see it daily) → U3 (removes surface before it is
merged) → U2 (merge with less to merge) → U4/U5 → U6 last, since it needs U4's
signal and a consent decision.

---

## ⚠️ PRE-EXISTING BUGS (Not from Growth Sprint)

These issues existed before the growth sprint work:

### 1. ShareDialog.tsx — MUI v7 API Changes

**File:** `client/src/components/common/ShareDialog.tsx`  
**Issue:** `ListItem` component API changed in MUI v7  
**Fix Needed:**

```tsx
// Old (broken)
<ListButton component="label" htmlFor="...">

// New (fixed)
<ListItem>
  <ListItemButton component="label" htmlFor="...">
```

**Estimated Fix:** 15 minutes

---

### 2. ShareButton.tsx — Missing Import

**File:** `client/src/components/common/ShareButton.tsx`  
**Issue:** Missing import statement  
**Fix:**

```tsx
import { useUser } from "../../hooks/useUser";
```

**Estimated Fix:** 2 minutes

---

### 3. GoalSelectionPage.tsx — Domain Enum Mismatch

**File:** `client/src/features/onboarding/GoalSelectionPage.tsx`  
**Issue:** Domain enum values don't match backend  
**Fix:** Update domain strings to match `Domain` enum in `src/models/Domain.ts`

**Estimated Fix:** 10 minutes

---

### 4. MarketplacePage.tsx — Missing Supabase Import

**File:** `client/src/features/marketplace/MarketplacePage.tsx`  
**Issue:** `supabase` used but not imported  
**Fix:**

```tsx
import { supabase } from "../../lib/supabase";
```

**Estimated Fix:** 2 minutes

---

## 📧 OPTIONAL: Email Triggers (Not Critical)

These email templates exist but aren't automatically triggered:

### 1. Weekly Digest Email

**Template:** ✅ Exists in `src/services/emailService.ts`  
**Trigger:** ❌ Not scheduled  
**Implementation:**

```typescript
// Add to src/utils/cron.ts or host cron (no Railway)
import EmailService from "../services/emailService";

// Run every Sunday at 9 AM
async function sendWeeklyDigests() {
  const { data: users } = await supabase
    .from("profiles")
    .select("id, email, name");

  for (const user of users) {
    // Calculate weekly stats
    const stats = await getUserWeeklyStats(user.id);
    await EmailService.sendWeeklyDigest(user, stats);
  }
}
```

**Priority:** Low (nice-to-have)  
**Estimated Time:** 1-2 hours

---

### 2. Re-engagement Email

**Template:** ✅ Exists in `src/services/emailService.ts`  
**Trigger:** ❌ Not scheduled  
**Implementation:**

```typescript
// Run daily at 10 AM
async function sendReEngagementEmails() {
  const sevenDaysAgo = new Date();
  sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);

  const { data: inactiveUsers } = await supabase
    .from("profiles")
    .select("id, email, name, last_activity_date")
    .lt("last_activity_date", sevenDaysAgo.toISOString())
    .eq("email_preferences.re_engagement", true);

  for (const user of inactiveUsers) {
    await EmailService.sendReEngagement(user);
  }
}
```

**Priority:** Low (nice-to-have)  
**Estimated Time:** 1 hour

---

## 🔧 OPTIONAL: Infrastructure Improvements

### 3. Webhook Retry Handling

**Issue:** If Stripe webhook fails to credit PP, no automatic retry  
**Current Behavior:** Manual intervention required  
**Fix Options:**

**Option A: Queue-based retry (Recommended)**

```typescript
// Use Supabase pg_cron or external queue (Upstash, SQS)
CREATE TABLE webhook_retry_queue (
  id UUID DEFAULT gen_random_uuid(),
  event_id TEXT,
  event_type TEXT,
  payload JSONB,
  attempts INT DEFAULT 0,
  max_attempts INT DEFAULT 3,
  next_retry_at TIMESTAMPTZ DEFAULT now()
);
```

**Option B: Simple logging + manual review**

```typescript
// Log failed webhooks to separate table for manual review
await supabase.from("failed_webhooks").insert({
  event_id: event.id,
  error: error.message,
  timestamp: new Date().toISOString(),
});
```

**Priority:** Medium (production readiness)  
**Estimated Time:** 3-4 hours (Option A) / 1 hour (Option B)

---

### 4. Elite Tier Implementation

**Issue:** "Elite" tier mentioned in streak rewards but not implemented  
**Current Tiers:**

- ✅ Pioneer (10 PP)
- ✅ Apprentice (20 PP)
- ✅ Achiever (50 PP)
- ✅ Mentor (80 PP)
- ✅ Legend (150 PP)
- ✅ Visionary (200 PP)
- ❌ Elite (not implemented)

**Fix:** Either:

1. Remove "Elite" references from docs/UI
2. OR implement as tier between Legend and Visionary (250 PP)

**Priority:** Low (cosmetic)  
**Estimated Time:** 30 minutes

---

## ✅ GROWTH SPRINT FEATURES (All Working)

### Week 1: Revenue + Viral (7/7 ✅)

- [x] Stripe billing portal
- [x] Success page verification
- [x] Annual pricing toggle
- [x] Achievement share modal
- [x] Leaderboard share button
- [x] Check-in share (+10 PP)
- [x] Goal completion share

### Week 2: Retention + Packaging (4/4 ✅)

- [x] Email service (5 templates)
- [x] Milestone celebration modal
- [x] Admin metrics dashboard
- [x] Acquisition packet documentation

---

## 📋 RECOMMENDED FIX ORDER

### Before Product Hunt Launch (Critical)

1. ✅ ShareButton.tsx — Missing import (2 min)
2. ✅ MarketplacePage.tsx — Missing supabase import (2 min)
3. ✅ GoalSelectionPage.tsx — Domain mismatch (10 min)
4. ✅ ShareDialog.tsx — MUI v7 API (15 min)

**Total:** ~30 minutes

### After Launch (Optional)

5. Weekly digest cron job (1-2 hours)
6. Re-engagement cron job (1 hour)
7. Webhook retry handling (1-4 hours)
8. Elite tier decision (30 min)

---

## 🎯 VERIFICATION CHECKLIST

After fixing pre-existing bugs:

**Frontend:**

- [ ] ShareDialog opens without errors
- [ ] ShareButton works on all pages
- [ ] GoalSelectionPage onboarding completes
- [ ] MarketplacePage loads without crashes

**Backend:**

- [ ] Stripe webhooks credit PP correctly
- [ ] Email service sends milestone emails
- [ ] Metrics endpoint returns data

**Growth Features:**

- [ ] Check-in share button visible (streak >= 3)
- [ ] Achievement share modal appears
- [ ] Leaderboard share button works
- [ ] Annual pricing toggle switches prices

---

## 📞 SUPPORT

For questions about fixes or implementation:

- Check `docs/ACQUISITION_PACKET.md` for business context
- Check `docs/METRICS_DASHBOARD_GUIDE.md` for metrics setup
- Review commit messages for feature details

---

**Status:** Growth Sprint Complete ✅  
**Pre-existing Bugs:** 4 (all minor, ~30 min total fix time)  
**Optional Enhancements:** 4 (can be done post-launch)

**Ready for Product Hunt launch after fixing 4 pre-existing bugs!** 🚀
