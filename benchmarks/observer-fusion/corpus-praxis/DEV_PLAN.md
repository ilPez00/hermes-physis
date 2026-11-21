# Praxis — Development Plan

> Snapshot: 2026-07-02. Authoritative roadmap doc. Pairs with `claude_steps.txt` (session log) and `manual_actions.txt` (DB/env changes). Update this file when shipping, removing, or proposing major features.

> **Commercial direction (2026-08-26).** This file covers _what_ gets built. It
> no longer decides _whether_ something should be. Praxis has 47 registered
> users, 1 weekly active, and €0 revenue after 1,582 commits — engineering
> capacity was never the constraint. The strategy is now pilot-led B2B (sell one
> cohort programme, not fifty consumer subscriptions), and feature work is gated
> on a named customer asking for it.
>
> Read before proposing a feature:
>
> - `docs/BUSINESS_PLAN.md` — metrics, B2B capability/gap audit, pricing, 90-day plan, kill criteria
> - `docs/GTM_TARGETS.md` — ICP, segments, lead sources, outreach cadence
> - `docs/PITCHES.md` — outreach templates (IT/EN), call script, pilot terms, and the do-not-promise list
>
> The "what is NOT built" table in `BUSINESS_PLAN.md` §2 is what sales is allowed
> to promise. If a gap there is closed, update that table in the same commit.

## Backend: the service-role client was not service-role (2026-11-19, done)

Chasing a 500 on signup exposed a larger defect: `login` ran
`supabase.auth.signInWithPassword` **on the shared service-role client**.
supabase-js installs the session it returns onto whichever client ran it, so
the first login after a restart silently downgraded that singleton to the
user's token — for the life of the process. Every later PostgREST call ran with
that user's privileges, and service-role-only writes started failing
`row-level security`. The symptom was nowhere near the cause: signup returned
500 on the profile upsert for every account created after any login, while the
same code succeeded from a fresh process, or from a script.

- `src/lib/supabaseClient.ts` exports `createCredentialsClient()`: a throwaway
  client with its own storage key, for anything that must _test_ a session.
- `login` and `deleteMyAccount` use it. `deleteAccount` also stopped calling
  `supabase.auth.getUser()` (which reads the singleton's session — the wrong
  question, since `req.user` already says who is asking) and reads the account
  through `admin.getUserById` instead.
- `src/__tests__/sharedSupabaseClient.test.ts` pins it: login checks credentials
  on a throwaway client, never on the shared one, and a fresh client is minted
  per login.

Verified against the running process: login → **200**, then signup → **201**.
Before the fix that pair returned 200 then 500.

## Data: signup was creating half an account (2026-11-19, done)

`/auth/signup` returned 201 and relied on a `handle_new_user` trigger that no
migration creates. Every account made that way had **no `profiles` row**, so its
first page load produced 406 from PostgREST on `profiles`, 400 from
`/api/users/:id` and 404 from `/api/gamification/profile` — the "2 auth.users
with no profile row" in the ops backlog was never two anomalies, it was every
signup since the trigger stopped existing.

- `authController.signup` now upserts the row itself (id, email, name, age, bio,
  UTM attribution), retries without `utm_*` on hosts that have not run that
  migration, and **fails the request** when the row cannot be written. An account
  that cannot read itself is not a success.
- `migrations/2026-11-19-profile-backfill.sql` puts the function and trigger back
  in version control and backfills the orphans; the run command is in
  `manual_actions.txt`. Until an operator runs it, existing half-created accounts
  stay broken — the code fix only covers new ones.
- `src/__tests__/authSignupProfile.test.ts` pins the upsert, the attribution
  fallback and the failure path.

Found while executing `docs/UI_FOLLOWUP_PLAN_2026-11-19.md`, which also closed
the three UI follow-ups in the same pass: every authenticated route now has one
`h1`, alerts stopped rendering as cards inside cards, `/settings` prose is capped
at 62ch, and `/me` lost its 42 em-dashes.

## Backend: login's language sync was rejected 100% of the time (2026-11-19, done)

After a successful sign-in, `LoginForm` fires one fire-and-forget
`PUT /api/users/:id { language }` so the UI language follows the account. It
returned **400 on every login, for every user**, and the client only
`console.warn`'d it — the preference never reached the server.

This was not a race with session propagation, which is what it looked like: the
request was authenticated, and the handler rejected it. `validateBody` replaces
`req.body` with `schema.parse(req.body)`, and `updateProfileSchema` listed only
`name`, `bio`, `city`, `avatar_url` — zod drops keys it does not know, so the
body arrived as `{}` and `updateUserProfile` threw "No valid fields provided for
update." The controller's `UPDATABLE_PROFILE_FIELDS` whitelist already contained
`language`, and `profiles.language` is a nullable text column: only the schema
disagreed.

- `src/schemas/userSchemas.ts` gains `language: z.string().min(2).max(16).optional()`,
  with a comment saying why an omitted field is not harmless here.
- `src/__tests__/schemas.test.ts` pins both directions: the key survives
  validation, and a one-character code is still rejected.

Verified end to end on pansa with a throwaway account: nulled the column, logged
in through the browser, and the login's own PUT returned **200** with
`profiles.language` going `NULL` → `en` (it had never been written before). The
account was deleted afterwards; orphan profiles remain 0.

Observed while verifying, not fixed: the first profile write after a backend
restart stalled on a cold Supabase/PostgREST connection (incoming request logged,
no completion for >30s); later requests on the same process answer in tens of
milliseconds. Nothing in the repo bounds that wait.

## Reliability: the three defects that cycle exposed (2026-11-19, done)

1. **Nothing bounded a Supabase call.** `src/lib/supabaseClient.ts` now gives
   every sub-client (PostgREST, auth, storage) a `global.fetch` of
   `fetchWithTimeout`: a 15s deadline composed with the caller's own abort
   signal via `AbortSignal.any`, overridable with `SUPABASE_REQUEST_TIMEOUT_MS`.
   A stalled cold connection now fails instead of hanging the request forever.
   `src/__tests__/supabaseRequestTimeout.test.ts` (4 tests) pins that it aborts, honours a
   caller abort that already happened, and passes method/headers/body through
   untouched.
2. **Background syncs failed invisibly.** `LoginForm` had the app's only
   `.catch(e => console.warn(...))` on a network write. The sync moved to
   `client/src/lib/profileSync.ts`: it never blocks the login (still not awaited),
   logs under a stable `[profileSync]` tag, and raises a `react-hot-toast` error —
   the app already mounts a `Toaster` in `AppShell`, so the message survives the
   navigation that follows login. `client/src/__tests__/profileSync.test.ts`
   pins both the quiet success and the loud failure.
3. **A stripped field said nothing.** `validateBody` replaces `req.body` with the
   parsed object, and zod deletes undeclared keys — which is exactly how the
   language sync was rejected for months without a single log line. The
   middleware now compares what the client sent with what survived and logs
   `validateBody dropped fields the schema does not declare` with the path,
   method and key names. Behaviour is unchanged; the class of bug becomes
   visible everywhere at once instead of one 400 at a time.

The drift audit itself found exactly one mismatch: `updateProfileSchema` (now
fixed). `PUT /api/plan/items/:id` has an `allowedFields` whitelist but **no zod
schema at all**, so nothing is stripped there — it is unvalidated, which is a
different debt (`all_day: "banana"` reaches the DB). Left as found, recorded here.

## /me: the operator page, rebuilt around one question (2026-11-19, done)

`/me` was 1,487 lines and 11,064 px tall at 1440× (16,721 px on a phone), with a
joke for a subtitle, ten sections and no signposts. It answers exactly one
question — is this one person out of his depth? — for a coach and a hiring
company, so the pass was judged against that sentence.

- **The first line is now a fact, not an in-joke.** `GEMINI STOP CONFUSING ME
WITH OTHER GIOs PLEASE!` became `MECHATRONICS · PHILOSOPHY · BARE-METAL
INFRASTRUCTURE`, and the hero's primary action became "Available for contract
  work" instead of "FAQ".
- **Signposts.** An on-page nav (`The story · Curriculum · Machine room · What I
run · Also built · Hiring`) with a test that every anchor resolves to a real
  id. Section labels are now real `h2`s — the page had zero headings and a screen
  reader read the whole thing as one run of paragraphs.
- **Height.** CV into two columns (and its redundant intro removed), "Now"
  absorbed "Unusual true facts", the PDCA diagram moved beside the story it
  illustrates, and the four stacked product cards became a 2×2 portfolio driven
  by one `ProductCard`. Desktop 11,064 → 9,459 px; phone 16,721 → 15,450.
- **Drift the restructure exposed.** The four product cards carried
  `rgba(34,197,94,…)` washes — the pre-Sync-Rule green the token file retired —
  now `alpha(colors.green, 0.06)`. Nine copy-pasted link buttons became one
  `MeButton` with `tone`/`solid` (~150 lines gone). The footer's six-button row
  lost five; they already appear in the index above.
- **Phone rows.** The 2,650 px index and the repo list stacked name / kind /
  description; they wrap into two lines now, ~900 px off the phone version.
- The full idea list, shipped and deferred (sticky rail, reference quotes, a
  dated "last updated", the donation block on profile routes), lives in
  `docs/UI_BRAINSTORM_2026-11-19.md`.

Gates: client `tsc` clean, **374 vitest** (68 files, incl. `MePage` 4 and
`profileSync` 2); backend `tsc` clean, **1,584 jest**. Deployed to pansa by hand.

### Follow-up in the same day (2026-11-19, done)

- **The donation block is off the operator page.** `AppShell` renders
  `DonationSection` for every route it owns, which appended a payment ask and
  three crypto QR codes (~1,700 px on a phone) to the bottom of a CV. It is now
  skipped on `/me` and `/giovanni_pezzin`; `/wiki` still shows it, which was
  checked live rather than assumed. `/me` 9,459 → 8,825 px.
- **`[POST|PUT] /api/plan/items` got the schemas they never had.** The controller
  copied `req.body` through an `allowedFields` whitelist straight into Postgres,
  so `all_day: "banana"` reached a boolean column and `sort_order` was unbounded.
  `createPlanItemBodySchema` / `updatePlanItemBodySchema` in
  `src/schemas/planSchemas.ts` now validate the exact payload the plan UI sends
  (date `YYYY-MM-DD`, time `HH:MM[:SS]`, hex colour, boolean flags, bounded
  `sort_order`) and strip keys the row does not own, so a round-tripped item
  cannot rewrite `id`/`user_id`. 6 tests in `schemas.test.ts`.
- **`/me` a11y and motion.** The PDCA dot and the mesh packet ran forever; the
  page now carries one authored motion (the rule under the name) and both run a
  single pass. The copy-email button announces its state through a polite live
  region, and the machine-room claim carries a `/health` link — the claim is
  checkable in one click.

Gates after this follow-up: backend `tsc` clean + **1,590 jest**; client `tsc`
clean + **374 vitest**.

### Sticky was broken app-wide, and `/me` got its pinned rail (2026-11-19, done)

`/me`'s anchors now park under the header once their place in the flow scrolls
away, and the chip for the section you are reading is marked with
`aria-current="location"`.

Getting there uncovered a shell bug that outranked the rail itself.
**`AppShell` put page content in `<main class="flex-1 overflow-y-auto">`.** That
`main` is a flex item inside a `min-h-screen` column, so it is always exactly as
tall as its content and **never scrolls itself — the document does**. The
property therefore bought nothing and cost something real: it created a
scrollport that no descendant could stick to. Consequences, all of them live
until now:

- `LandingNav`'s `position: sticky` did nothing on every shell route. The rail
  measured `top: -3454` after a 3,000 px scroll: it simply left with the page.
- `TopBar`'s `sticky top-0 z-40` did nothing either, so the signed-in app had no
  persistent header despite asking for one.
- `.praxis-scroll-root`, the element `NotebookPage`'s pull-to-refresh looks for
  to decide whether the feed is at its scroll origin, **does not exist anywhere
  in the codebase** — the guard is dead and pull-to-refresh could fire mid-feed
  on touch. Fixed in the follow-up below.

Removing `overflow-y-auto` from `main` fixes the first two: the document is the
scroll container, which is also what `window.scrollTo` in Bioenergetics and the
landing sections already assume. Verified signed-in with a throwaway account on
`/dashboard` and `/notebook` (header at `top: 0` after 1,490 px of scroll, zero
horizontal overflow, fixed BottomNav/SpeedDial unaffected) and anonymously on
`/me`, `/physis`, `/wiki`, `/physics`.

Two consequences of stickiness starting to work, found by scanning every
`position: sticky` in the client for one that pins at `top: 0` **inside the
shell**: `/wiki`'s own toolbar (`sticky top-0 z-20`, 55px tall) now parked
itself behind the 48px `TopBar`, hiding all but 7px of it. It is `top-12` now,
so it stacks under the shell header (`top: 48`, verified after a 770px scroll).
The other `top: 0` bars (`/cyclops`, `/univr`, `/bioenergetics`) are standalone
routes with no `TopBar` above them and are correct as they are; `DiaryTimeline`'s
sticky date headers are the third case, but nothing imports that component.

The pull-to-refresh guard was fixed after this cycle: the gesture moved to the
page root, its origin is `window.scrollY` (the one scroller this app has), and
it refreshes the feed **and** today's brief, so the release visibly does
something. Its indicator now sits above the page header, where the pull starts,
instead of above the feed far down the page. Verified with CDP touch events on a
throwaway account at 390 px: a 260 px drag at `scrollY 0` showed "Release to
refresh", spun, and fired `GET /notebook/entries` + `GET /dashboard/summary`;
the same drag from `scrollY 900` showed no indicator, fired nothing, and simply
scrolled the page up to 655. A desktop pointer drag does nothing.

With a real scrollport in place, the rail went back to being plain CSS. The
first implementation was a fixed bar on a one-pixel sentinel with a scroll
listener and a spacer, because neither `sticky` nor an IntersectionObserver
sentinel could work; that machinery is gone again. The rail is
`position: sticky; top: 65px` (the header's measured height), desktop only — a
pinned bar on a 390 px screen would cost a fifth of the viewport to save a
thumb-flick — with `scroll-margin-top: 116px` on its six targets so an anchor
jump does not land under the two bars. `MePage.test.tsx` stubs the observer and
pins the `aria-current` contract.

Gates: client `tsc` clean, **375 vitest / 68 files**, eslint clean; backend
untouched.

## The wiki was empty for four separate reasons (2026-11-19, done)

`/wiki` rendered as a large blank panel for every account anyone tested it
with. It was not one bug, and none of them was the one the page's own copy
implied ("Axiom synthesizes 5 wiki pages nightly").

1. **The list could never render.** The mount effect was one `Promise.all` over
   `/wiki/pages`, `/trackers/my` and `/trackers/summary/today`, with a single
   `.catch` that ran `setPages([])`. **`/trackers/summary/today` does not exist
   — it answers 404.** `Promise.all` rejects on the first failure, so that 404
   cleared the page list on every load, for every account, even when the wiki
   API returned pages. Proven with a token: `/wiki/pages` 200 with two pages,
   `/trackers/summary/today` 404, UI empty. The wiki's own answer now decides
   the page; the tracker strip is decoration and may be absent (it reads
   `/trackers/activity-feed?days=1`, which exists).
2. **The nightly pass visited nobody.** `getActiveUserIds()` selected users with
   a check-in in the last 21 days; the newest check-in in production is
   **2026-03-17**, so the set was empty every night. The pass still acquired its
   lease at 03:23 daily and wrote nothing: `axiom_wiki_pages` held **10 rows
   worldwide**, last written **2026-08-25**, while 16 goal trees had never
   produced a page. The union is now check-ins ∪ goal trees ∪ notebook entries
   in the window — measured on the box: **0 → 19 users**.
3. **Goal pages were never materialized on demand.** They are pure math over
   the goal tree (no LLM, no sources), but only the nightly pass wrote them.
   `GET /wiki/pages` now renders them when an account has none and reads again,
   guarded to once a minute per user. Live proof: a fresh Pro account with two
   goals got `goals/run-a-marathon.md` and `goals/weekly-long-runs.md`
   (`[WikiGoalPages] Synced 2 goal page(s)`) on first load, and the list
   rendered them.
4. **A 403 was rendered as an empty library.** All three endpoints sit behind
   `requirePro`; 42 of 53 profiles are not Pro. The catch above turned that into
   "No pages yet. Axiom writes these nightly." — an empty knowledge base that
   would never fill, with no upgrade route. The gate is state now: a Pro panel
   listing what the wiki actually contains (one page per active goal, at most
   one researched page a night, search), an upgrade button, and a disabled
   search field.

Also in this pass, found by the tests and the live run rather than by reading:

- **Icons matched nothing.** `PAGE_ICONS` was keyed `goals.md`/`persona.md`, but
  the writers emit `goals/<slug>.md`, `generated/<slug>.md`, `patterns/<date>.md`,
  so every real page rendered as 📄. Prefix map now, tested.
- **A populated list still said "YOUR KNOWLEDGE BASE".** Nothing was selected,
  so the reading pane showed the empty state next to the list. The first page
  now opens itself, and the pane says `opening…` if it is slow.
- **Every page had two `<h1>`s.** The chrome renders the title as the page's
  heading and the body opens with `# <title>`; the body's headings are shifted
  down one level and its duplicate title line is dropped. The locked and empty
  states had **no** h1 at all — the panel and the empty heading are the h1s.
- **Search snippets were raw markdown** (`## State - **Domain:**`), now stripped
  for display.

**Follow-up (same day): the widgets, and the backfill.** `/trackers/summary/today`
had two more callers — `DesktopWidget` and `MobileWidget` — which wrap their
fetches in `Promise.allSettled`, so the 404 was swallowed and both widgets showed
`0 tracker entries today` permanently. Both now call `fetchTodayTrackerCount()`
(`client/src/lib/trackerStats.ts`), which reads `/trackers/activity-feed?days=1`
and counts `kind: 'tracker'` items stamped today — notes are not tracker entries.
The wiki sidebar uses the same helper rather than its own copy of that filter.

And the wiki was backfilled rather than left waiting for 04:23: the deterministic
goal-page sync ran once for every user the new nightly selection reaches —
**19 considered, 12 had active goals, 28 pages written, 0 failures**. The table
went from **10 pages / 3 users** (stale since 2026-08-25) to **35 pages /
13 users**. The `llmwiki` reindex step reported `spawn llmwiki ENOENT` on this box,
so the semantic index was skipped; search still answered from the snippet service.
That is fixed below.

Gates at the time of the backfill: backend `tsc` clean + **1,608 jest** (18 new in `wikiLazySync.test.ts` +
`wikiNightlyUsers.test.ts`); client `tsc` clean + **383 vitest** (8 new: 4 in
`WikiBrowserPage.test.tsx`, 3 in `trackerStats.test.ts`, plus one in
`MobileWidget.test.tsx`); eslint clean. Deployed by hand (ADR 007 — the
commits were made on pansa).
Live, with a throwaway Pro account seeded with two goals: list rendered both
`goals/*` pages with 🎯, `Weekly long runs` auto-opened, one h1, `Progress: 45%`,
search returned the other page, toolbar at `top: 48` under the shell header,
zero horizontal overflow at 390 px. With `is_premium` false: Pro panel, one h1,
disabled search, `/wiki/pages` 403. With the goals deleted: the empty state,
one h1, and its "Open your goals →" button keyboard-focusable and navigating to
`/goals`. Account deleted (`0` profiles, `0` pages, `0` trees left behind).

### Then the engine that writes those pages was missing too (2026-11-19, same evening)

The backfill only wrote the deterministic goal pages. The **grounded** pages
(`generated/<slug>.md`, written from cited web sources) are the ones the nightly
pass exists for, and the pass had never produced one on this box. Two pieces of
infrastructure were missing, not two bugs:

1. **`llmwiki` was never installed.** `AxiomWikiMaterializer` and
   `AxiomWikiSearchService` shell out to a binary at `~/.cargo/bin/llmwiki`
   (`LLMWIKI_BIN`), the user unit's `PATH` includes it, and there was no such
   file — so every reindex logged `spawn llmwiki ENOENT` and every search fell
   through to the Postgres/snippet path. The source only exists vendored at
   `/home/gio/dev/physis-global/vendor/llmwiki` (v0.2.1, edition 2024); built
   `cargo build --release`, installed the 39 MB binary, `llmwiki index` built the
   Tantivy + usearch index over `/home/gio/ai/wiki`. `llmwiki-watch.service` had
   been enabled and crash-looping on the same missing file since it was written;
   it now runs and reindexes on write.
2. **SearXNG was gone from `:8888`.** `AxiomSearchFetchers` grounds a page from
   SearXNG, then Brave; with neither reachable every generation is rejected for
   having zero sources (by design). The container is back as `praxis-searxng`
   (`127.0.0.1:8888`, `--restart unless-stopped`, 512 MB cap), settings in
   `~/.config/searxng/settings.yml` with the JSON format on — the older
   `/home/gio/searxng` symlink is a broken bind-mount leftover and stays unused.

One real bug surfaced once indexing began: **llmwiki requires `title`,
`description` and `tags` in frontmatter and skips a page that lacks them**, with
only a stderr warning. Three of one user's pages (`_index.md`, `plans/`,
`patterns/`) were being silently skipped on every reindex, i.e. unsearchable
forever. `normalizeFrontmatter()` in the materializer now backfills a missing
title from the file name and a missing description from the page's own first
prose line (ignoring headings, bullets, tables and an embedded YAML block), and
coerces a scalar tag into an array — it invents nothing.

Result of the real pass: `[WikiNightly] done: 19 user(s), 12 new page(s) in
1490s`, **12 of 13 wiki users now have a researched page**, `axiom_wiki_pages`
**10 pages / 3 users → 47 pages / 13 users**, index `46 files, 0 skipped`, and
`llmwiki search "therapy"` returns `c5ff90bc…/generated/therapy.md` as hit 1.

**So it cannot go quiet again.** `src/services/WikiInfraHealth.ts` probes both
(the binary with `--help`, SearXNG on `/healthz` — not a search, so the check
costs an engine nothing), cached for a minute. `/health/ready` now carries a
`wiki` field and a top-level `warnings[]` when something is missing, and boot
logs one `[WikiInfra]` line naming the broken half and pointing at
`manual_actions.txt`. Readiness still says `ready`: one Pro feature must not
take the API down. Verified live by stopping the container — `"wiki":"SearXNG
unreachable at http://127.0.0.1:8888 …"` plus a warning, then `ok` again once
it was back.

Gates: backend `tsc` clean, `wikiMaterializerFrontmatter.test.ts` (6 tests) and
`wikiInfraHealth.test.ts` (5); client untouched.

### Section reveals on /me — and the shell that was eating every one of them (2026-11-19)

`/me` is 8,818 px tall and every block below the hero was painted before the
viewport reached it, so scrolling moved a page that had already happened. Each
of the eight sections now rises once as its top clears the last tenth of the
viewport, through the shared `Reveal` (so the whole app has one scroll
gesture, one curve, one duration).

Verifying it in a browser turned up the real bug: **no scroll reveal in the
authenticated app had ever animated.** `AppShell` renders every route inside
`<AnimatePresence mode="wait" initial={false}>`, and that presence context
disables *initial* states for the whole subtree, not just its direct children.
So `initial={{ opacity: 0 }}` was dropped, the motion wrapper was never written
a style at all, and the blocks simply sat there at opacity 1. The landing page
is shell-less, which is why its reveals looked right and hid the problem.
Measured before the fix: on `/me` the section wrappers had no inline style and
read `opacity: 1` at load, while the same component on `/` read `0`.

A second, sharper reason to stop leaning on framer here: its in-view feature
creates its `IntersectionObserver` in `mount()` without checking the API
exists, and throws where it does not — jsdom, and any WebView old enough to
predate it (the Android app *is* a WebView). `Reveal` is now an
IntersectionObserver plus a CSS transition with the same 16 px / 550 ms /
exponential ease-out shape: the hidden state is ours, the trigger is ours, and
a missing observer means "show it", never "crash" or "stay invisible".

A CV gets printed. A motion block starts at `opacity: 0` inline, so printing or
saving `/me` as PDF before scrolling that far came out blank below the fold —
now `@media print` in the theme forces the resting state (`opacity: 1`,
`transform: none`, no animation), which beats inline styles with `!important`.

Checked in Chromium against the built bundle, not just in jsdom:

- at load, all eight sections read `opacity: 0`; each reaches `1` on scroll and
  none is left behind at the bottom (8,818 px page);
- mid-flight samples on the first reveal: `0.47 → 0.84 → 0.96 → 0.99 → 1.00`,
  so it is a real 550 ms transition, not a jump;
- clicking the sticky nav's “Machine room” lands the section at `top: 116` with
  `opacity: 1` — the `scroll-margin-top` and the reveal agree;
- a 1600 px-tall viewport reveals only the section actually inside it;
- `prefers-reduced-motion: reduce` still arrives (150 ms after the scroll the
  section reads `1`) — the global rule collapses the transition, not the content;
- print media: all eight sections read `opacity: 1` without scrolling;
- 390 px wide: no horizontal overflow, every section arrives;
- no page errors, and the landing page still hides 18 blocks at load and reveals
  all 18 on scroll, so the shared primitive did not regress the one place it
  already worked.

Tests: new `client/src/__tests__/motion.test.tsx` (no-observer fallback renders
the content; hidden → arrives on intersection) and a `MePage` test that stubs
the observer and requires all sections hidden at first, then visible. Both were
run against the pre-fix code first and failed — the fallback test threw the
`IntersectionObserver is not defined` the guard exists to prevent, and the
`/me` test found the wrappers already at `opacity: 1`.

Gates: client `tsc` clean, **387 vitest**, eslint clean, bundle **1497.4 KiB /
158 assets** (budget 1.8 MiB).

Known gaps this exposed, not fixed here:

- A cold load of `/me#machine-room` does not scroll: the browser resolves the
  fragment before the SPA renders the target (pre-existing, unrelated to the
  reveal). Same-document nav clicks work.
- `Rise` / `Cascade` / `Item` suffer the same presence-context suppression
  inside `AppShell` — their mount entrances are silent no-ops there today, so
  `/for/freelancers` and every authenticated page's block entrances do nothing.
  Same fix applies if we want them back; the landing (shell-less) is unaffected.

### Deep links, in-shell entrances, and a print sheet (2026-11-20)

The three gaps the reveal work opened, closed together because they are the
same kind of defect: a thing that works in the one place it was tested and does
nothing in the place users actually arrive.

**Fragments now land.** The browser resolves `#machine-room` before this SPA has
rendered anything, so inbound `/me#machine-room` used to open on the hero
(measured: the section sits 3,109px down). `lib/hashScroll.ts` looks for the
target and keeps looking for up to 3s while a lazy route mounts — a Suspense
fallback in the shell, or nine lazy sections on `/` — then stops. Same-document
links animate; a cold deep link lands instantly, because a three-thousand-pixel
animated scroll under a reader who just arrived reads as the page scrolling
itself. It is mounted once, on the router subscription in `AppRouter`, so the
landing, the public pages and the authenticated shell all get it and no page
keeps a private copy of the retry loop (the one on `/` was removed).

Verified cold in Chromium: `/me#machine-room` → `scrollY 2993`, target `top 116`
(the section's `scroll-margin-top`), content opacity 1; `/me#hire` → `top 116`;
`/#features` and `/#pricing` → `top 80`; on a 390px phone `#hire` → `top 84`,
no horizontal overflow. Unknown fragments expire silently rather than scrolling
somewhere arbitrary.

**Mount entrances run on the first page too.** `Rise` / `Cascade` / `Item` were
framer components with `initial={{opacity: 0}}`, and framer's `use-visual-state`
resolves `presenceContext.initial === false` before it looks at the component's
own props — so inside `AppShell`'s `<AnimatePresence initial={false}>` the
hidden state was dropped for the whole subtree. The first route of a session
had no entrances; every navigation after it animated. All three are now CSS,
built on the same `Enter` primitive as `Reveal`: the hidden state is ours, two
frames give the browser something to transition from, and the theme's global
reduced-motion rule collapses the duration. Verified on a cold load of
`/for/freelancers`: `0 → 0.18 → 0.34 → 0.48 → 0.60 → 0.69 → 0.76 → 0.82 → 1`
across ~680ms, where it used to be 1 from the first paint. `AppShell` now carries
a warning at the `AnimatePresence` so the next framer entrance added there is
not silently dead.

**`/me` prints on white paper.** The page is drawn for a dark screen, so
printing it produced a black sheet with the text legible only where the
cartridge gave up. The page's own `@media print` block now sets white ground,
drops the header / footer / on-page nav, hides the hero banner (a background
image browsers do not print) and the copy button, remaps every token tone at
once — luminance carried the hierarchy, so on paper size and weight do — keeps
sections from splitting across a page break, and puts the address and
availability on page one, where the fixed nav and the copy button used to be the
only way to find them. The portrait prints greyscale. Twelve A4 pages, and the
index section prints as a catalogue.

Checked with `emulateMedia({ media: 'print' })` plus a generated PDF: root is
`rgb(255,255,255)`, `h1` black, body copy `rgb(26,26,26)`, `header`/`footer`/`nav`
and all three `[data-print-hide]` blocks `display: none`, section
`break-inside: avoid`, and the PDF's page 1 opens with name, dateline, contact
and the start of Curriculum. Screen rendering is unchanged (dark ground, cover
image, buttons).

Tests: `hashScroll.test.ts` ×6 (no fragment, immediate smooth jump, lazy target
found then instant, cancel, expiry, reduced motion), `motion.test.tsx` +3 (Rise
settles on the app curve, Cascade staggers 50ms apart, non-`Item` children are
not cloned), `MePage.test.tsx` +1 (print block, white ground, banner marker,
print-only contact line). Client: `tsc` clean, **397 vitest / 72 files**,
eslint clean, bundle **1497.9 KiB / 159 assets** under the 1.8 MiB budget.

### Follow-up, same day: focus, print palette, pagination, and the nav band (2026-11-20)

The four gaps the first pass left, closed the same day.

**A fragment now moves focus, not just the scroll.** The browser moves the
*sequential focus navigation start point* to the target when it resolves a
fragment; it never resolves one here, so a keyboard reader following
`/me#machine-room` landed mid-page with Tab still starting at the top of the
document. `jumpTo` gives a plain element `tabindex="-1"` (never a natively
focusable one — that would take it *out* of the tab order) and calls
`focus({ preventScroll: true })` after the scroll so it cannot fight it.
Verified at 1440 and 390 px: `document.activeElement.id === 'machine-room'`,
`tabindex="-1"`, section at `top 116` / `84`.

**The diagrams have a print palette.** They are SVG `fill` / `stroke`
attributes, which a CSS `color` cannot reach, and a blanket `fill: #1a1a1a`
would have blackened the labels inside the PDCA circles. Each mark that has to
survive is tagged in the markup instead: `data-print-ink` (becomes text-dark),
`data-print-stroke` (rule-dark), `data-print-paper` (filled white, so what sits
inside it still reads). The mesh now prints as line art — white-ringed nodes on
a dark rail, dashed overlay and node names in black — where the screen version
was amber-on-white at 2.2:1.

**Pagination follows the content.** `break-inside: avoid` on whole sections
bought whole sections and left three pages ending a third of the way down.
The guard moved down: `h2`/`h3` get `break-after: avoid` (a title never closes a
page), short panels and `figure`s opt out of breaking with `data-print-keep`, and
long lists flow. **12 pages → 9, evenly filled** (20–26 lines per page, the last
one short), and no card is split away from its heading — the failure that
started this was the Storage-tier body continuing on the next sheet with no
title above it.

**The on-page nav measures its band.** `rootMargin: '-112px …'` was the desktop
header (65) plus the sticky rail (47); on a phone the header is shorter and the
rail does not stick at all, so the highlight sat ~50px behind the section under
the reader's eye. The band is now the real header height plus the rail height
only when the rail is actually sticky, re-measured on the `(min-width: 900px)`
breakpoint. Verified: after scrolling to "What I run" the active entry is
"What I run" at both 1440 px (header 65, rail sticky) and 390 px (header 65,
rail static).

### Third pass: the paper, the primitive, and the ring (2026-11-20)

**The sheet is laid out for A4, not for a phone.** Paper is ~793px wide, under
the 900px breakpoint, so the first print sheet was the *phone* layout: portrait
above the statement, Education above Work, cards stacked. The rows that carry
the layout now ask for their wide arrangement on paper (`data-print-row`), and
the container's 15mm screen padding and the section gaps are tightened
(0 padding, 8mm between sections) because paper has no scroll to spend them on.
`Stack`'s spacing is a margin on the axis its props asked for, so forcing a row
produced a 0px gutter — measured, then fixed with an explicit `columnGap: 6mm`.
Page 1 is now a CV front: name, contact and availability, portrait beside the
statement, and the whole two-column Curriculum.

**Pagination settles at 8 pages.** Whole sections avoided breaks and left pages
a third empty; whole panels did the same thing one level down. The guard now
sits where the reader needs it: `h2`/`h3` and every card title
(`data-print-heading`) never close a page, CV entries and `figure`s move whole,
and everything else flows. **12 → 9 → 8 pages**, 21–31 lines on each, no card
orphaned from its heading. The one card that still breaks at the page edge is
the long RandoMesh panel, and it breaks between a paragraph and a figure —
which is what typography does.

**One motion primitive.** `Rise`, `Item` and `Reveal` were three components with
the same hidden state written three times; they are now `Enter` with a
`trigger` (`mount` or `view`), a duration and an offset — 400/12 for `Rise`,
350/10 for `Item`, 550/16 for `Reveal`, stagger still 50ms through `Cascade`.
Same behaviour, one place to change the curve, and the presence-context story is
told once.

**The deep-link budget is 5s.** Three seconds was measured against a warm chunk;
a cold lazy route on a slow phone can take longer. `GIVE_UP_MS` is 5s, pinned by
a test that appends the target at 4.2s and still lands.

**A landed section does not get a ring.** The skip-link rule gives every
`:focus-visible` a 2px amber outline; a deep link focuses the section it lands
on (so Tab continues from there), which drew that outline around a 4,000px
region under the sticky header — it reads as a rendering fault, not as "you are
here". `[tabindex="-1"]:focus-visible` now carries no outline. Verified:
the section still matches `:focus-visible` with `outline: none`, and the first
Tab stop (the skip link) still shows its 2px ring.

Gates: client `tsc` clean, eslint clean, **399 vitest / 72 files**, bundle
**1498.5 KiB / 159 assets**. Print verified by generating the PDF and reading
the pages; deep links verified at 1440 and 390 px.

### Fourth pass: the paper has margins, says whose it is, and counts its pages (2026-11-20)

**The eight-page sheet was printing into the unprintable area.** With no `@page`
rule the content box was the whole sheet: page 1 put the h1 two millimetres
from the paper edge, inside the region every printer crops. `@page { margin:
10mm }` fixes it — and costs pages, honestly: 8 → 10. A little print-only type
scaling (`html { font-size: 15.2px }`) brings it back to **9 pages**, each
19–32 lines, none of them closer than 10mm to an edge.

**Every sheet now names itself and its place in the pile.** `@page me-sheet`
carries a `@bottom-left` footer (`Giovanni Pezzin · praxisweb.xyz/me`) and a
`@bottom-right` counter (`4 / 9`); `/me` opts in by tagging its root `page:
me-sheet` for print, so no other surface prints a footer. Margin boxes live
*outside* the content box, which is exactly what a `position: fixed` footer
cannot do:

| approach | measured result |
| --- | --- |
| `position: fixed; bottom: 0` | repeats per page, but laid out **inside** the content area → overlaps the last line |
| same, with the bottom margin reserved for it | 8 → **11 pages** of paper |
| same, negative `bottom` to reach the paper margin | dropped on page 1, then printed **on top of a line of text** at the top of the last page |
| `@page` margin boxes + `counter(page)` | **works**, costs no paper, page count stays 9 |

An earlier pass of this same work recorded margin boxes as "not implemented in
Chromium" and shipped an in-flow end note instead. That was wrong: Chromium 145
(pw 1208) implements them, and the probe that proves it is three lines of CSS.
The end note was not wasted — it became the document's **colophon**, `Printed
20 November 2026`, right-aligned above the footer: the footer can say *who*, but
only the rendered page knows *when*, and a CV's facts age.

**Two things paper was printing that paper cannot use.** The mesh diagram's
packet is a `<circle>` whose position comes from a CSS animation; the global
print rule cancels animations, so it fell back to its untransformed position,
x=0 — a stray arrow floating left of the first node. It is `data-print-hide`d
now: a still of a moving packet is not a fact about the mesh. The footer's
"all repositories on GitHub" pill was still printing as an outlined control next
to the index; it is hidden for the same reason the hero row already was — the
address is already in the print contact line under the name.

### Fifth pass: the sheet stops printing its phone layout and its holes (2026-11-20)

The fourth pass got the geometry right; reading all nine rasterised pages then
showed four places where the *content* was still laid out for a screen that
scrolls. Each fix is one rule, and each was checked on the page it changed.

- **A row of unequal columns leaves half a sheet white.** `NOW` pairs two short
  paragraphs with a six-item list, and forcing the row on paper left the left
  column empty from mid-page down. The section now carries `data-print-flow`:
  on paper the two blocks flow through `column-count: 2` and fill the page top
  to bottom. Each fact is `data-print-keep` — without it the first fact split
  mid-sentence at the column break ("…drawn in a" / "ruined paper notebook…"),
  which reads as two unrelated sentences — and the list's heading wrapper gets a
  3mm print top margin, since in one flow it butts against the paragraph above.
- **The PDCA figure squeezed its own prose.** `The first Praxis was paper` was a
  print row whose figure keeps its intrinsic width: the text column came out
  250px wide, **eleven words a line**, running the length of page 2. Dropping
  `data-print-row` there gives the prose the full measure and puts the loop
  under it — shorter than the squeezed version, not longer.
- **A flex row fragmented across sheets.** On the hire section the prose stayed
  on page 7 in a quarter-width column while the contact card jumped to page 8,
  because Chromium fragments a forced row badly. Stacked, page 7 reads down the
  page: statement, four services, then the card — and the email, previously
  broken as `…@gmail.co` / `m`, now fits on one line.
- **A card's border printed alone at the foot of a sheet.** Product cards had no
  break guard, so Chromium painted the top border at the bottom of page 5 and
  put the content on page 6 — an empty card followed by a card with no top. They
  are `data-print-keep` now, and their CTA pills (`OPEN PRAXIS ↗`, `PHYSIS API
  ↗`, …) are `data-print-hide`d: paper carries no controls, and every product is
  named in the index at the end of the sheet.

Verified by reading all nine rasterised pages again. **Still 9 pages** — the
moves rearrange paper rather than spend it. Screen output untouched: every rule
involved is print-only.

Tests: `MePage`'s print test now requires `page:me-sheet` and the dated
colophon instead of the retired end note.

**Print parity is a gate now.** `client/src/__tests__/printSheet.test.tsx` (run
as its own CI step, in front of the suite) pins the three things a refactor
drops silently, because none of them is visible on screen: the 10mm page box,
`@page me-sheet` with its identity line and `counter(page)` / `counter(pages)`,
and `/me`'s print opt-in. It also pins "paper carries no controls" from both
ends: `MeButton` marks its output `data-print-control`, and the print block
hides any marked control that did not mark itself hidden — so a button added
later is silent on paper even if nobody remembers it is a button. Each half was
mutation-tested before it was trusted: renaming the named page, deleting the
opt-in, and dropping the marker each turn the check red.

**The audit gate had been red for weeks.** `scripts/ci-local.sh` found it on its
first run: 20 high/critical advisories across the three trees, hidden because CI
has not started a job since the account's billing failed. `npm audit fix`
cleared 19 of them with **lockfile-only** changes (root: `body-parser`,
`follow-redirects`, `zod`; client: `follow-redirects`, `undici` ×8; physis-mcp:
`@modelcontextprotocol/sdk`, `axios`, `body-parser`, `follow-redirects`, and a
critical `proxy-addr`). Each tree was re-verified: backend 1624 jest + tsc +
build, client 403 vitest + tsc + build + `print:check`, physis-mcp 7 tests +
typecheck + build.

The twentieth was recorded as an exception and then found to be fixable, which
is worth keeping in the record: `zod` GHSA-mrhx-c7xc-m3w5 (memory exhaustion on
oversized constrained collections) needs zod 4.6.6+, and I first wrote it off as
"pinned by the MCP SDK". It is not — `@modelcontextprotocol/sdk@1.32.2` declares
`zod: ^3.25 || ^4.0` as both peer and dependency, and npm had hoisted zod 3 only
because physis-mcp's own range said `^3.23`. Bumping to `zod@^4.6.7` dedupes to a
single copy and the schemas convert cleanly. Verified by handshake, not by
trusting the typechecker: `initialize` answers, `tools/list` returns **14 tools**,
and each input schema still arrives as draft-07 JSON Schema with its
descriptions. The exception file is back to its single nodemon/braces entry.

**The lint ratchet and the dependency gate, both green — one honestly, one by
accident.** The ratchet had drifted 1272 → 1295 since 2026-11-02. A scripted
sweep of unused imports (338 specifiers, 91 files) was reverted: it mangled a
multi-line MUI import carrying an inline comment, and a 78-file reformat hides
more than it fixes. Hand-editing the two worst files (14 dead imports in
`GoalTreePage`, 13 in `SettingsPage`) reached 1268, and the ratchet lowered its
own `.lint-baseline`. The ~434 other unused-variable and 712 `no-explicit-any`
warnings stay: typing those is a review, not a sweep.

`scripts/no-new-deps.sh` had been passing without ever looking at anything. It
read only the root `package.json` — so `client/` and `physis-mcp/` carried
dependency trees nobody watched — and it compared `git diff`, which is empty in
a fresh CI checkout, so in CI it could not fail whatever was pushed. On top of
that its line-grep counted a new npm *script* as a dependency. It now parses the
dependency maps of all three manifests and reports additions and re-ranges by
name, against `github.event.before` in CI (the frontend checkout keeps
`fetch-depth: 2` so that commit exists) or `origin/main` in `ci-local.sh`; the
PASS line says which comparison it made. Mutation-tested five ways: a new dep in
`client`, a re-ranged `zod` in `physis-mcp`, and a committed-but-unpushed dep
all fail and name the package; a script-only edit passes; and the same committed
dep is invisible without a base ref, which is exactly the blindness being fixed.

The same blindness sat in `no-weakened-tests.sh`, in the half that is supposed
to be the hard failure. Its marker ratchet reads the working tree and so works
in CI, but the "no test file was deleted" check compared `git diff` and
`git diff --cached`: empty in a fresh checkout, and empty for a deletion already
committed locally. A deleted suite passed the gate whose entire purpose is to
notice that. Deletions now come from a base ref, resolved by
`scripts/lib/base-ref.sh` — one helper sourced by both diff-based gates, so they
cannot drift on which change they inspect — with `event.before` in CI and
`origin/main` in the local runner, and the PASS line naming the comparison.
Demonstrated rather than assumed: deleting a suite and committing it gives PASS
without a base and FAIL (`1 test file(s) deleted`, path listed) with one; an
extra `.skip` still fails the ratchet; an uncommitted deletion fails either way.

One lesson from doing this: a `git reset --hard` in a mutation test reverted the
uncommitted gate twice. Commit the thing before testing that it fails.

### Every gate, falsified (2026-11-20)

Two gates had already been caught passing without looking at anything, so the
rest were audited the same way: not "does it print PASS" but "what makes it
print FAIL, and did that happen on purpose?" A gate nobody can make fail is
decoration with a green label.

| Gate | Verdict before | What falsifies it (run, not assumed) |
| --- | --- | --- |
| `lint-ratchet.sh` | **fail-open** | A command printing no warnings (`--cmd "echo ok"`) read as 0 warnings and **wrote 0 into the baseline with a PASS**, destroying the ceiling. Now: 0 against a non-zero ceiling is `FAIL` exit 2, baseline untouched. A real rise still fails (`22 -> 23` style), a drop still lowers it. |
| `no-new-deps.sh` | **could not fail in CI** | Fixed earlier this day (see above): manifest scope, empty diff, script-vs-dependency false positive. |
| `no-weakened-tests.sh` | **half-blind** | Fixed earlier this day: the deletion check compared an empty diff. Committed deletion now fails, `1 test file(s) deleted` with the path. |
| `npm-audit-gate.sh` | sound, silently permissive | Fails closed on unknown high/critical, missing exception, or expired exception *when matched*. An exception that matched nothing printed nothing, so a stale entry rotted: the `braces` one had been expired since 2026-11-08. Unmatched/expired entries now print as `note —`, which immediately proved the point; it was deleted and the file is now `[]`. |
| `check-bundle-budget.js` | sound | 700 KB of `/dev/urandom` in `dist/assets` → `total 2182.7 KiB > 1843 KiB budget`, exit 1. Clean build: 1498.9 KiB / 159 assets, PASS. |
| coverage thresholds | sound | `--coverage.thresholds.lines=99` → `ERROR: Coverage for lines (61.81%) does not meet global threshold (99%)`. |
| `print:check` | sound | Deleting `@page me-sheet` from the built CSS produced 18 failures, exit 1 (earlier this day). |
| pre-commit (`lint-staged`) | **checked nothing for client work** | Its one glob was `src/**/*.ts`, backend only: every client commit printed "could not find any staged files" and exited 0. Replaced with `lint-staged.config.js` covering `client/src/**/*.{ts,tsx}` from `client/`. Two traps found by running it: lint-staged spawns without a shell (so `cd client && …` is looked up as a binary named `cd`), and it does **not** append filenames to a function task's returned string — the first version linted the whole repo (4225 problems for a one-line edit). Now: a broken staged file gives `✖ 1 problem (1 error, 0 warnings)` and exit 1 in either tree; a clean one exits 0. No `--fix`: a gate reports, it does not rewrite what was tested. |
| `npm run security:audit` | **could only fail** | Pointed at `scripts/security-audit.js`, which has never existed in this repo. Repointed at `npm-audit-gate.sh .`; `npm run security:audit` now prints the same PASS as the gate. |
| `ci-local.sh frontend` | **red for an environment reason** | This shell exports `NODE_ENV=production`; React 19 ships no `act` outside test builds, so every component test died as `TypeError: React.act is not a function` — a runner that is red on a clean tree teaches people to ignore it. The three vitest-bearing steps now set `NODE_ENV=test`. Full job: **12 steps, exit 0**. |
| `npx eslint src --quiet` (ci.yml) | redundant, not blind | Fails only on errors, and the ratchet above already hard-fails on a non-zero exit. Kept as a named step so a lint error reports as itself rather than as "warnings rose". |
| nightly `e2e` / `load` / `print` / `compliance` | **cannot run anywhere right now** | Actions refuses to start jobs (billing). `print` is mirrored by `ci-local.sh` and runs here. `e2e` discovers 265 tests in 5 files across 5 projects, but only chromium is installed (firefox/webkit/Mobile Safari would fail at launch) and the suite drives the real `:3001`, which on pansa is **production Supabase** — running it here would write into live data. `load` needs k6, which is not installed. `compliance` needs `npx license-checker` over the network. These four are unverified, and saying so is the point of this table. |

Not run in this pass, by resource discipline: the backend job (load average was
13 on 12 cores). Backend `tsc` + 1624 jest were green earlier the same day after
the dependency bump, and nothing since touched backend sources.

What would make this table stay true: run each gate's falsification after
changing it, and never record a gate as verified because it printed PASS.

**And parity on paper.** `npm run print:check` in `client/` renders `/me` to a
PDF with no scrolling and reads it back: every sheet must carry the footer, must
say its own number, and must print at least ten lines (that last one is what
catches an entrance wrapper printing invisible); the document must land between
7 and 12 pages, carry the colophon on its last sheet only, and contain no control
labels. Mutation-tested both ways — deleting `@page me-sheet` from the built CSS
produces 18 failures, removing the control guard produces 5, both exit 1. It is a
job in `nightly.yml` and a step in `scripts/ci-local.sh`, and needs
poppler-utils + a Playwright chromium.

The test reads `src/index.css` as text rather than importing it (`?raw` returns
post-Tailwind CSS, in which the named page has been compiled away) — that is
also why it cannot verify that Chromium *paints* the margin boxes, only that the
declaration is still there. Firefox and Safari print neither.

Still open:

1. **Firefox and Safari do not print margin boxes** (or not reliably). They get
   the margins and the colophon, no running footer — the hero still carries the
   name, address, URL and GitHub handle on page one, so no sheet is anonymous.
2. **The last sheet is a quarter full** (five index rows, the colophon, and not
   much else). Squeezing it onto page 8 would mean retuning the print type scale
   for one page; a CV's last page ending early is ordinary, so it stays.
3. **`.physis-next/` embed cache stays untracked** — unrelated to this work, but
   it is the one thing `git status` keeps showing.

### /me's diagrams were dead, and the commit that killed them was about something else (2026-11-19)

Two loops on `/me` — the PDCA dot orbiting the cycle diagram and the packet
travelling the mesh rail — were changed from `infinite` to a single pass by
`add58089`, whose subject is *"validate plan items, and keep the donation ask
off the CV"*. Nothing in that diff's shape suggested motion.

It read as broken rather than restrained, for two reasons: a **cycle** diagram
with a frozen dot argues against the thing it draws, and the packet's last
keyframe is `opacity: 0` — so it did not stop moving, it **disappeared** for
good 4.5 s after load. Both loop again (9 s orbit, 6 s packet, transform-only,
and the global reduced-motion kill-switch still flattens them). The page's
entrance gesture is still a single one: the rule under the name draws itself
once. `docs/UI_BRAINSTORM_2026-11-19.md` carried the "single pass" decision as
deliberate; it now records the reason it was reverted.

Pinned by a new `MePage` test that reads the CSS emotion emits and requires
`infinite` on both names — checked that it fails on the old source and passes
on the fix, since jsdom runs no animations itself. Measured in Chromium against
the built bundle: `iterations: Infinity` on both, packet wrapping `34 → 234 →
102 → 35 px` with `opacity` back to 1 each lap, no console errors. Client
`tsc` clean, `MePage.test.tsx` 6/6, bundle 1497.1 KiB / 158 assets under budget.

### Half the goal trees were invisible to the wiki (2026-11-19, same night)

After the pass, `axiom_wiki_pages` showed **four users with active goal trees and
zero pages**, and the pass had visited all of them without a word.

`goal_trees.nodes` holds two different shapes. `client/src/models/GoalNode.ts`
is the flat model — `name`, `progress ∈ [0,1]`, `parentId` — and every wiki
reader assumed it. Trees written by the onboarding/goal editor are **nested**
and use `title`, `children: [...]` objects, and `progress` already in percent. So
`n?.name?.trim()` was empty for those users: no goal pages, no nightly
generation (`topGoalFor` returned null), no log line either. `eca6962d`'s tree
is a `name` tree and got a page; the four `title` trees got nothing.

`normalizeGoalNodes()` accepts both: `name ?? title`, children walked
recursively with `parentId` derived from the nesting, `customDetails` read as
the description, a percent-valued `progress` rescaled to [0,1], and a JSON
string parsed (with junk yielding `[]`, not a throw). Both writers — goal pages
and the nightly top-goal pick — now share it. And a user who ends the night
without a page is counted and named: the pass logs one line per skip
(`no active goal` / `no sources` / failure) and the summary now reads
`19 user(s), N new page(s), M without a page (no active goal: a, no sources: b,
failed: c)`.

Second real pass: **19 user(s), 5 new page(s) in 508s**, `axiom_wiki_pages`
**47 pages / 13 users → 71 pages / 17 users**, `goals/` **28 → 48** (children are
pages too now), `generated/` **13 → 17**, index `70 files, 0 skipped`. Every
user who has a goal tree is covered; the three who have none (check-in or
notebook activity only) have nothing for a goal-anchored wiki to write about,
which the summary now states explicitly instead of hiding.

Gates: backend `tsc` clean, `wikiGoalNodeShapes.test.ts` (5 tests) + existing
wiki suites.

## Design: the signed-in app, audited for real (2026-11-19, done)

Every earlier design pass verified the **public** pages in a browser. The
signed-in app had never been scanned, because there was no account to scan it
with. This cycle created one throwaway user (the pattern the e2e suite already
uses), drove Playwright through `/login`, captured 20 authenticated routes, and
ran the parsed-DOM ruleset plus a live contrast walk against each.

What that changed, in the order it mattered:

- **The whole app was a heading skip.** `PraxisHeader` renders `h4` and
  `PraxisSection` renders `h6`, so a page title was followed by a card title
  three levels down; MUI also maps `subtitle1/2` to `h6`, putting labels like
  "Screen time (RescueTime)" in the outline. The theme's `variantMapping` now
  renders `h6` as `h2`, `h5` and both subtitle variants as `p`, and the two
  shared components declare `h1`/`h2` explicitly.
- **308 sub-12px font sizes had survived the sweep** because they sit inside
  ternaries (`isSm ? '0.6rem' : '0.7rem'`), which the line-anchored pass never
  matched; `0.7rem` was also outside its range. Pages that read the design
  system (`/trackers` reported 25 warnings) were the tell.
- **`/settings` rendered the handwriting preview in notebook ink** (`#2b2620`)
  on the dark page: 1.3:1, literally invisible. It uses the foreground now.
- **`rgba(255,255,255,0.3–0.5)` text** measured 3.1–3.8:1 on `#080808`; 175
  foreground colours across 60 files are at the 0.55 floor.
- **`/marketplace` highlighted tier button** was white on the amber→violet
  blend (2.5:1) and is now the amber lamp with dark ink; the "PRAXIS PRO"
  tracked all-caps eyebrow became a badge in mixed case, the ⚡ left the
  "Buy Praxis Points" heading, and the price figure stopped being a heading.
- **`/ai` dropped 0.06em tracking** from its mono labels; `/analytics` got the
  `h1` it never had.
- **A real signup bug, found by trying to sign up:** `passwordRegex` was
  `^(?=.*[A-Za-z])(?=.*\d)[A-Za-z\d]{8,}$`, so every symbol a password manager
  generates was rejected — answered with "must contain at least one letter and
  one number", which was true and therefore useless. Any printable character is
  accepted now, pinned by `schemas.test.ts`.

Measured, static scan before → after: `/trackers` 25→1, `/notebook` 2→0,
`/analytics` 5→1, `/marketplace` 5→1, `/ai` 1→0. What is left is `inter`
(the brand face) and, on `/leaderboard` and `/commitments`, error-state
`Alert`s sitting inside a card — only visible when a request fails, so it is
recorded here rather than refactored at 23 sites.

Gates: client `tsc` + eslint + 371 vitest, backend `tsc` + 1569 jest.

## Revenue strategy: sell services first, benchmark second (2026-10-25, active)

Cash flow beats traffic. Per the analysis in `docs/SEO_NICHE_EVALUATION.md` and `docs/GUIDE_SEARCH_CONSOLE_AND_PLANS.md`:

- **Priority 1 (70%):** Remote AI/systems contracts (€500–€800/day). Target Rust, MCP, agents, retrieval, evaluation. Apply to OpenTrain, Alignerr, direct outreach.
- **Priority 2 (20%):** Industrial AI pilots for machine builders in Veneto/Lombardy (€1.5k–€3k). Narrow offer: searchable docs/BOMs/manuals with citations and evaluation.
- **Priority 3 (10%):** Physis commercial proof. Only one defensible content asset: a reproducible codebase retrieval benchmark page (zero farms in SERP). Build this after getting baselines, before making token-savings claims.

**Do not build:** calculator/generator directories, 30-page content farms. They compete with AI spam or affiliates with no edge.

## `/me` as the index, and a sitemap that exists (2026-10-25, shipped)

Shipped: `/me` now indexes every public surface, `/me` carries its own metadata,
and the domain has a `/sitemap.xml`.

- **`/me` lost its three half-lists.** Four product cards, a GitHub list and six
  "Elsewhere" pills became one `SURFACES` array and one **"Everything, indexed"**
  section: Praxis, Physis + workbench, the AI terminal, goal plans, Bioenergetics,
  Univr Campus, Cyclops, Verona K, NietzscheDice, both blogs, the freelancer
  page, `/physics/`, `/agliardi/` (labelled **client work** — it is a shipped site
  for an earthworks company, not a personal project) and the GitHub profile.
  Rows rather than pills because a pill has room for a name and no room for what
  the thing is. `MePage.test.tsx` pins the href set, so a lost link is a red test.
- **`/me` previewed as the app itself.** It had no entry in `src/utils/pageMetadata.ts`,
  and every share — into a hiring thread, into a pilot conversation — rendered
  Praxis's own title and description. Added `ME` (title, description, canonical,
  `Person` schema) and extended the `/ai` metadata interceptor in `src/app.ts` to
  cover `/me`; the `/physis` tree keeps its own handler.
- **`/sitemap.xml` (new).** `robots.txt` pointed only at `/plans/sitemap.xml`,
  which covers the plan pages: `site:praxisweb.xyz` returned **one** result. New
  `src/controllers/sitemapController.ts` lists the 16 public surfaces; plan slugs
  stay in their own sitemap and `robots.txt` lists both. The path list is literal
  rather than derived from the router — the route tables live in the client
  bundle and a sitemap that guesses would publish login-walled routes. A test
  asserts both directions: public paths present, private paths absent.
- **The sitemap was publishing sixteen copies of one page.** Only four paths had
  an entry in `pageMetadata.ts`, so the other twelve served the app's own title,
  description and canonical: a crawler that followed the new sitemap found
  sixteen near-duplicates. Ten more entries (`/bioenergetics`, `/univr`,
  `/cyclops`, `/veronak`, `/nietzsche`, `/blog`, `/helloworld`,
  `/for/freelancers`, `/terms`, `/privacy`) give each its own title, canonical
  and description. `PAGES` now carries a `path` instead of a `matches` closure
  and the interceptor's route pattern (`METADATA_ROUTE`) is derived from that
  list, so a page cannot be added to the sitemap and silently keep the app's
  title. `pageMetadata.test.ts` reads `/sitemap.xml` and fails unless every URL
  it lists has a distinct title.

Gates: backend `tsc` clean + **1527 jest** (incl. 2 sitemap tests, 6 metadata
tests); client untouched this round (**365 vitest** / 64 files at the previous
commit); bundle **1385.7 KiB** across 152 assets, under the 1.8 MiB budget.

Why this and not thirty niche pages: see `docs/SEO_NICHE_EVALUATION.md`. Eleven
SERP probes across two rounds: every niche worth entering has **zero** AI farms
in its top 10, every niche that has farms is one a template can produce, and the
two populations never overlap. The recommended calculator niches are 80–100 %
occupied; the domain was not indexed at all, so content was never the bottleneck.

Remaining, in `manual_actions.txt` and `docs/GUIDE_SEARCH_CONSOLE_AND_PLANS.md`:
verify the Search Console property (needs the operator's login), clean the 34
e2e debris rows and grow the plan corpus, then one benchmark page carrying its
own failures — the only content asset here that cannot be copied.

## Physis Context MCP — launch closure (2026-10-04, proposed)

2026-10-05 documentation update: [sales runbook](PHYSIS_CONTEXT_MCP_SALES.md)
now links current 1.1.1 listing/scope, evidence-continuity direction, whole-task
TokenSave gates and the proposed primitive library. Future features remain
proposed; old deployment/payment snapshots are dated history. No engine feature,
public page, checkout or runtime change is completed by this update.

Owner runbook: [sales and launch](PHYSIS_CONTEXT_MCP_SALES.md). Physis engine
work stays in `/home/gio/dev/physis-next/tasks/plan.md`. Public artifact exists;
payment and licensed buyer outcomes remain open. No runtime change shipped here.

- [ ] Source now accepts €79/€49 through an explicit allowlist; verify effective
      configuration (legacy single-price override included) and prove actual
      provider test events for both prices deliver usable licenses.
- [x] Source repair2026-10-05: webhook uses documented `PHYSIS_CONTEXT_SIGNING_SEED_HEX`.
      Signed HTTP regressions for both prices fail before/pass after; missing signer
      and invalid signature refuse issuance.36 focused tests and backend typecheck
      pass with isolated fixtures. Deployment and real delivery remain open.
- [ ] Configure Creem checkout/webhook and reliable delivery; prove writable
      spool, duplicate delivery handling, recoverable email failure and reissue.
- [ ] Prove clean-buyer install and useful MCP query against exact public archive;
      align platform, absolute repo root, 12-tool docs, grant and budget claims.
- [ ] Use Rust/Linux demos, PulseMCP, current Claude Code showcase and tested
      Cursor Showcase first; article/newsletter and runnable Show HN follow.
- [ ] Track first useful query, paid licenses, refunds/support and source;
      paid advertising waits for conversion evidence.
- [x] 2026-10-13 page refresh: `/physis` published in praxis_webapp commit
      `425c3df1`; live response and deployed page asset verified. It explains
      Physis structural context for coding agents, publishes bounded evidence,
      and requests team-preview access.
      It preserves the Physis Context 1.2.0 archive link for existing users; no
      unverified price or new-release promise. Browser sample remains sign-in and
      fixture-only. See `docs/physis/PRODUCT.md` and `docs/physis/DESIGN.md`.
- [x] 2026-10-13 demo promotion: `/physis` now leads with the real 23-second
      highlight and embeds the 89-second walkthrough beside a plain-language
      explanation. Keeps scope notes and measured token results visible; removes
      the unlinked full recording that showed an operator network address and
      replaces internal run notes with a visitor-safe walkthrough. Validated
      with TypeScript, 294 client tests, production build, and Chromium at 1440,
      390, and 320px. The 1.2.0 archive, fixture-only browser sample, and
      preview-only offer remain unchanged.
- [ ] 2026-10-13 conversion follow-up (prepared, not yet deployed): add the
      existing €49/€79 Creem checkout path beside the demos, keep team-preview
      access, link the released 1.2.0 archive and checksum, and replace the
      incorrect development-CLI MCP snippet with the shipped command and paths.
      Add Physis-specific HTML share metadata. Page tests, client suite, root
      typecheck/build, and responsive browser checks pass. The full backend
      suite remains red in this environment (47 failures across six suites;
      includes missing test listeners/services). Production publication and
      post-deploy crawl/browser checks remain pending.

---

## 1. Currently Shipped (live in main)

### Impeccable design refinement — every surface (2026-10-21, `8bb4e3aa`)

A full design pass over the whole product, foundations first.

**Foundations.** The Sync Rule was broken: `green` had three values across the
three token sources. All three now read `#10B981`, and the amber/violet
light+dark steps exist in every source. The four shared primitives now match
DESIGN.md instead of drifting from it — `PraxisCard` is a surface with a 12px
radius, `PraxisChip` is a full pill, `PraxisButton` is 12px with ramp text
colours, `PraxisHeader` marks a tier word with a tinted pill rather than
gradient text. New `alpha(hex, a)` helper replaces eight hand-written
hex-to-rgba conversions.

**Public surfaces.** `/veronak` was bare MUI with a scale that started
disabled; it now leads with an authored white-horse mark (`components/brand`),
a signal diagram that opens once when the beacon leaves, and a number scale
that can be chosen before the location is known. `/physis` was corrected
against its own sources — three demo descriptions did not match the recordings
they label, and the caller-resolution figure contradicted the published
`physis-computer` README. `/physics` now says plainly which of the two
products named "Physis" it documents, and frames the 2026 paper as a research
record whose enterprise price list does not apply to anything sold today.
`/cyclops` gained a drawn product figure. `/blog` can carry images in
markdown. `/nietzsche` gained an empty state. `/univr` replaced twelve emoji
in its navigation with drawn icons. `/bioenergetics` keeps its mint
sub-brand and gained a drawn masthead rule. The landing funnel, which had
zero motion across all eleven files, now opens with a staged cascade.

**Interior.** The retired 3px side-tab stripe is gone from every card and
list item that carried one; status reads as a tint plus a dot. Titles that
fell back to the body face are in the display face. The eight competing
neutral backgrounds are back onto the ramp. One authored entrance per page,
not an identical one on every section.

**Also shipped.** `llms.txt` — the product is for coding agents and had no
agent-facing index. Eight megabytes of md5-identical duplicates of the retired
physis-pro dashboard and paper removed. The CSP hash for the rewritten
`/agliardi` inline script regenerated and verified against the file it
guards.

Gates: backend tsc + 1513 jest, client tsc + 356 vitest, eslint clean, bundle
1383.7 KiB / 1.8 MiB, impeccable detect clean.

**Still open.** The Physis release archives (20 MB + 2.7 MB) are still
untracked in git — they are served from `client/public/downloads/` in
production but a fresh clone does not contain them. The site publishes 1.4.0
while `github.com/ilPez00/physis-computer` is at 1.7.0. `og:image`,
`og:url` and `canonical` on `/agliardi` are root-relative because the
production domain for that client is not decided.

### The /me page told a story that was not true (2026-10-21)

`/me` is the operator's page and the one a prospect reads. Its claims were
re-verified against the running box, the GitHub API and
`docs/HOSTING_TOPOLOGY.md` — not against the page's own prose.

**Corrected.** The database backbone was described as "reveNas (Revenant)",
a zero-trust active-active multi-site engine that exists nowhere; it is
self-hosted Supabase Postgres on pansa with streaming standbys on omo and
air. The split-model inference fleet was described as sharding model layers
over ggml RPC with "no node owns a GPU"; the RPC contributor's binary does
not exist on either node, the unit had been crash-looping every three
seconds for two days, and omo runs a GPU server. aion was described as the
runtime orchestrator that owns rolling updates and health probes; it is a
desktop shell and stats HUD. The repository count was 45 and is 53, and one
of the six linked repositories was a **404**.

**Added.** The PDCA loop the origin story keeps claiming is now drawn, with a
dot going round it. The contact email has a copy button that says what it
did. A quiet "Elsewhere" strip points at the other public surfaces.

**Also shipped.** `ggml-rpc-server.service` — a unit with no executable,
failing `203/EXEC` and writing 377,000 journal events since 2026-10-19 —
was **disabled**, and `docs/HOSTING_TOPOLOGY.md` records why and what to do
if the contributor is wanted back.

**Interior sweep, app-wide.** `shadows.glow.none` was not none: it carried
a 32px shadow, so every card in the authenticated app was a ghost card (1px
seam + wide soft shadow). It now carries only the inset top-light. The
`#111827` chat and group headers, the `#1F2937` menus, the 5px accent bars
rebuilt from positioned `Box`es and the remaining 3px blockquote stripes are
all gone; the titles that were falling back to Inter are in the display face.
Twelve pages that swallowed their fetch errors and rendered the **empty**
state — "No entries yet", "No friends yet", "No matches found" — now show an
in-place error with a retry instead.

Gates: backend tsc + 1513 jest, client tsc + 356 vitest, eslint clean, bundle
1386.8 KiB / 1.8 MiB, impeccable detect clean.

**Shipped 2026-11-20.** 3.7.1 (`versionCode 20`, 13.0 MB, `apksigner` verifies)
was rebuilt and signed on this box with the cloud-free web bundle, using
`/opt/android-sdk` + JDK 17, and published twice: as GitHub release `v3.7.1`
(signed APK + AAB, the artifact record) and as the public download
`https://praxisweb.xyz/downloads/praxis-3.7.1-release.apk`, which is what /me's
"Android app" button uses. **The release page cannot be the public link — this
repository is private, so `releases/latest` returns 404 to a signed-out
visitor.** See `client/android/BUILD_NOTES.md` "v3.7.1", including the trap that
`dist/downloads/` must be deleted from the copied assets or the APK grows to
281 MB.

### NietzscheDice — philosophical chat + corpus OCR pipeline (2026-10-16)

`/nietzsche` page (chat / notebook / library / search tabs) backed by a corpus
ingested from the local PDF library, plus the OCR pipeline that made image-only
scans ingestable.

**NietzscheDice refactor shipped (2026-10-18, commits `04de7b05..0bd91c6a`).**
Eight tasks per `docs/features/NIETZSCHE_REFACTOR.md`: routes auth-gated;
fabricated endpoints deleted; nested notebook notes (4 levels) on Supabase with
real embeddings; one honest physis adapter instead of live :19876 calls; hybrid
vector search (`match_nietzsche_chunks` RPC) replacing the `ilike` cascade;
evidence-based `/compare` + nearest-neighbour `/alike`; new `NoteEditor`
(markdown preview, tag chips, citation sidecar, draft autosave); the ~1350-line
page split into `useNietzscheApi` hook + five tab components with the
design-system palette (amber primaries, pink domain data). Operator follow-ups
in `manual_actions.txt`: apply
`migrations/2026-10-18-nietzsche-search-functions.sql`, run
`scripts/backfill-nietzsche-chunks.mjs` on pansa.

**Redesign (2026-10-16, in progress):** Local-first with three modes — Notebook
(Gemini-style chat + upload), Quote Finder (semantic search), Brainstorm (Socratic
dialogue with uncensored model). See `docs/NIETZSCHEDICE_REDESIGN.md` for full spec.
Key: `qwen3.5-9b-uncensored-aggressive` on omo for Brainstorm, local SQLite + FS
for corpus/notebook, no Supabase dependency for core features.

**The corpus bug this fixed.** The ingest originally used `pdf-parse` and then
stripped every non-ASCII byte (`replace(/[^\x20-\x7E]/g, '')`). Two silent
failures followed:

- pdf-parse returns **0 chars** for image-only scans, and the length check then
  skipped the file. _The Birth of Tragedy_ (234p), _The Genealogy of Morals_
  (234p), _World as Will and Representation_ (558p) and _The World as Will and
  Idea_ were all absent from the corpus despite being on disk.
- The ASCII strip destroyed accents across the entire corpus (`Adelphi`, `verrà`,
  `società`).

**The fix — two-pass extraction.** `pdftotext` per page for the text layer,
`pdftoppm`+`tesseract` for image-only pages, then merge in page order. No page is
dropped for being image-only, and a fully-text PDF is still emitted.

**Engine choice is measured, not assumed.** Ground truth was generated by
rendering KNOWN text to a page image and scoring error rates:

| variant                            | CER       | WER       | s/page   |
| ---------------------------------- | --------- | --------- | -------- |
| **tesseract `-l ita`**             | **1.14%** | **4.93%** | **0.91** |
| rapidocr (PP-OCR ONNX)             | 7.20%     | 11.33%    | 8.14     |
| tesseract + dictionary word-repair | 7.51%     | 34.48%    | —        |

RapidOCR was rejected because it strips accents (`verrà`→`verra`), merges words
(`avvicinamentoalla`), and drops lines. Dictionary repair was rejected because it
destroys proper nouns (`Protagora`→`Pitagora`, `Apollodoro`→`Pomodoro`,
`Democrito`→`Demolito`) — a wrong repair costs more than a missed one. Multi-PSM
voting was rejected because PSM 6 reorders the page, so the "majority" is just
PSM 3 again.

**Two real bugs the new test found in the page itself:**

- `activeTab` was a string (`'chat'`) but MUI `<Tabs>` matches its value against
  child **indices**, so no tab ever selected and the content area was blank.
- `scrollIntoView` was called unguarded; jsdom and some embedded WebViews lack it,
  so the page threw on mount and the whole tree unmounted.

Migration adds a `UNIQUE (title, author)` constraint so re-ingest upserts instead
of duplicating rows; the ingest clears stale chunks on re-ingest.

### "Share into Praxis" — canonical objects → affinity → event rooms (2026-10-03)

Sharing a link from another app is an implicit act of interest, and Praxis now
treats it as one: share → canonical object → user→object edge → (private
notebook entry) → event entity → public room → user-user affinity → ranking.

Design and the reasoning behind each conservative choice:
**[`docs/SHARED_OBJECTS.md`](SHARED_OBJECTS.md)**.

- **Canonicalization** (`sharedObjectCanonicalize.ts`) — deterministic, no LLM,
  no network, no redirect-following. Tracking stripped, content params kept,
  dedup enforced by a UNIQUE `canonical_hash`.
- **Edges** (`object_shares`) — intent `saved|interested|going`, visibility
  `private|public`, owner-only RLS. Private saves may influence ranking and are
  never made visible.
- **Behavioral affinity** (`sharedObjectAffinity.ts` + `match_users_by_shared_objects`)
  — intent strength × object rarity × recency decay, saturating so volume cannot
  dominate. Deliberately **not** folded into `profile_embeddings`.
- **Event identity** — `sha256(title|date|location)` fingerprint, **time
  excluded** so re-advertised times don't split an event; conservative match
  ladder that never auto-merges across different venues.
- **Rooms** — ordinary `chat_rooms` (`type='event'`) + `chat_room_members`,
  created only when someone says "I'm going". One event, one room, N members is
  enforced by unique indexes, not by application timing.
- **Matching** — one new axis (`SHARED_OBJECT_WEIGHT = 0.06`) on the existing
  composite. Weights/blend extracted to `utils/matchingComposite.ts`, which was
  previously untestable. Missing axes still drop rather than score zero.
- **`/share`** — public route, PWA `share_target` in the manifest, three
  options (save privately / share publicly / I'm going), one tap, no composer.
- Native clients are not implemented and are not required: Android is an
  `ACTION_SEND` filter + one API call, iOS needs a Share Extension (iOS never
  delivers share content to a PWA). Both target the same intake endpoint.

Not built yet (deliberately, see the doc): room archiving after an event, and
the candidate-pool truncation fix in matching. 143 new tests across 5 backend
suites + 1 client suite.

### App-wide motion system + page choreography (2026-09-24)

Motion grounded in Jitter UI-element templates + Dribbble web-app animation
conventions (Awesome-GUI-Agents was surveyed and correctly skipped — it is a
GUI-_agent_ research index, not UI animation).

- **Choreography kit** `client/src/lib/motion.tsx`: one easing curve
  (`[0.22,1,0.36,1]`) + one spring app-wide; `Rise` (sequenced section
  entrances), `Cascade`/`Item` (staggered list/grid entrances), `page`
  variants. All primitives honour `<MotionConfig reducedMotion="user">`.
- **Page transitions** (`AppShell`): `AnimatePresence mode="wait"` around the
  Outlet — outgoing route exits (0.16s fade/rise-out) before the incoming page
  enters (0.3s rise-in). Exit faster than enter so navigation never feels
  blocked. Honours reduced motion (instant swap).
- **Theme-level CSS motion** (every page inherits, zero per-page code):
  `praxisRiseIn` keyframe + 6-sibling 40ms stagger on MUI Card entrances
  (capped so long lists don't queue), 0.97 press-scale on `MuiButtonBase`
  (springy tap feedback on every button/IconButton/Tab), dark-tuned
  `MuiSkeleton`, card hover-lift (clickable cards only), BottomNav active-pill
  glide via `layoutId`.
- **Hand-tuned pages**: Today (header → subtitle → action-card cascade),
  Notebook (header → Axiom brief → delegated tasks → retro card), Leaderboard
  (header → rank card → filters → list), Profile hero, Analytics header,
  Achievements header + progress bar.

Budget impact: bundle 1351 KiB gzipped total (< 1.8 MiB budget gate), no new
dependencies (framer-motion was already shipped).

> **2026-09-12 — Agent platform increment.** As an action-based AI+human layer:
>
> - **Agent registry (Pro).** `user_agents` table + `POST/GET/PATCH/DELETE /api/agent/registry`
>   (Pro-gated). PAT mint (`POST /api/agent/keys/personal`) is now Pro and
>   auto-registers the agent (name/slug/capabilities/model) best-effort.
>   `src/controllers/agentRegistryController.ts`, `src/routes/agentRegistryRoutes.ts`.
> - **UberWiki = knowledge AND reasoning (Pro).** `GET /api/uberwiki/reasoning?q=`
>   groups the physis coherence store into what worked (+1) / inert (0) / didn't (−1),
>   optional semantic recall over a question; exposed to agents as the MCP data tool
>   `get_uberwiki_reasoning`. `src/controllers/uberWikiController.ts`, `src/mcp/praxisDataTools.ts`.
> - **Physis is paid.** `/api/physis/*` is Pro-gated by default
>   (`PHYSIS_PRO_GATED=false` restores the public rate-limited mount). `src/app.ts`.

Core product surface as of this snapshot. Each bullet names the feature and where it lives in the codebase.

### /ai sandbox really runs commands + /ai + /me restyle (2026-09-28)

**The bug.** `/ai` "Shell" mode sent every line to Hermes — an LLM turn, ~30s,
reworded or skipped at the model's discretion, and dependent on a provider key.
A shell that cannot run `ls` is not a shell.

- **New exec path.** `POST /api/ai/terminal/exec` runs the typed line verbatim
  in the same bubblewrap jail (`src/services/aiSandboxService.ts` →
  `runSandboxExec`, `buildExecArgv`): no agent, no prompt, `stdout+stderr+exit
code` returned as data (a non-zero exit is a result, not an error), 90s cap,
  16k output cap with an honest truncation note. `ls -la` went from ~30s to
  ~17ms in the live check.
- **Agent path kept.** `POST /api/ai/terminal/run` (Hermes session with tools,
  memory and the user's Praxis context) is unchanged; the page now has three
  explicit modes — Shell / Agent / Chat — instead of one ambiguous box.
- **Uploads land where the shell can see them.** The attach button posted to
  `/ai/workspace/upload`, which writes into the _docker_ workspace container —
  a different jail, so `cat notes.txt` in the terminal could never find it. New
  `POST /api/ai/terminal/upload` writes into the bwrap user home
  (`/home/sandbox`), filename basenamed + sanitized, 1MB cap.
- **Rate-limit double count fixed.** `use('/ai')` also matches `/ai/terminal/…`,
  so both mounts counted each terminal request and the effective budget was 5
  commands/minute. The terminal router now mounts first (app.ts).
- **Debt:** `use('/admin/cli')` under `use('/admin')` has the same double-count.
- Tests: `src/__tests__/aiSandbox.test.ts` +5 (jail contract, exec argv carries
  no agent wrapper, filename sanitizer, sandbox-home write, input caps);
  `client/src/features/ai/AiTerminalPage.test.tsx` +5 (exec vs agent vs chat
  routing, quick chip, failure + retry); backend 1256 jest, client 260 vitest.

**Restyle (impeccable, design authority `DESIGN.md`).** Both pages were carrying
their own colour ramps and habits the system had already retired.

- `/ai`: header + status strip (dot carries state, not a thick edge), a mode
  segmented control, a terminal chrome bar over the scrollback (`bash ·
/home/sandbox`, `clear`), per-turn identity (amber `sandbox@praxis:~$`,
  violet `hermes` / `praxis-ai`), an exit-code dot + tabular mono duration, copy
  per output, a failure card that names the problem _and_ retries the exact
  line, pills for one-tap commands. Browser surfaces themed: amber caret, dim
  placeholder at 5.8:1, 8px scrollbar on the muted tier.
- `/me`: the page now reads the shared token ramp (it had its own `#101215`
  family and a 5:1 body grey), cards take the system 12px radius (cover 16px),
  tags are pills on the raised tier, the retired 2px `border-left` quote is an
  editorial pull-quote in the display face with the synthesised italic removed,
  CV dates/notes are tabular, links carry a 3px underline offset and the
  `#beginnings` anchor clears the fixed nav. Nine 11px uppercase kickers naming
  cards became `CardName` titles (display face, 800 weight); the hero's location
  eyebrow moved under the name as a dateline, with the page's single authored
  motion — a rule that draws itself on load (transform-only, flattened by the
  reduced-motion kill-switch).
- Verified: `tsc --noEmit` clean both sides, `impeccable detect` clean on both
  files, bundle 1357 KiB total (< 1.8 MiB budget).

### physis-pro retired; physis-next is the only Physis (2026-10-04)

Operator directive. `~/dev/physis-pro` is retired: not a development target, not
the product. `~/dev/physis-next` is the only Physis — the only CLI, crates, MCP
server, and the commercial Context MCP binary. Recorded in `AGENTS.md`, and in
`~/AGENTS.md` for the machine.

**The migration is not done, and this is why the note is not "delete it".**
praxis proxies `PHYSIS_API_URL` to a live `physis-pro-web` on `0.0.0.0:19876`,
and the 8 demo machines exist only in that engine; physis-next has no machines.
So the retirement creates the open work of migrating praxis off that process, and
an agent must not "clean up" `physisProClient.ts`, the `/api/physis` proxy, the
roster or the pages that describe them — they are live output today.

Documentation touched, all of it verified rather than assumed:

- `HOSTING_TOPOLOGY.md`, `PHYSIS_PUBLIC_ARCHITECTURE.md`,
  `PRAXIS_LOCAL_SB_AND_PHYSIS.md`, `PHYSIS_ATTENTION_ARCHITECTURE.md` each got a
  status banner: the engine they describe is retired, the topology stays as an
  operations record, and none of it should be extended or designed against.
- `PRAXIS_LOCAL_SB_AND_PHYSIS.md` pointed at `ilPez00/physis-pro` on GitHub.
  That URL is **404**; the live repo is `ilPez00/physis`. Corrected.
- `HOSTING_TOPOLOGY.md` now records, in the doc instead of only in a fleet file,
  that the engine binding `0.0.0.0:19876` is a known violation of the
  loopback-or-tailnet rule. Deliberately left unfixed: the fix is migration, not
  rebinding a retired process.

`VIS_ROADMAP.md` still proposes work "in the `physis_pro` Rust core". Left as-is
on purpose — it is a historical proposal list, and rewriting a roadmap to remove
its content would erase the record of what was planned. Flagged here instead.

### Physis claims audit against the real engines (2026-10-04)

Every Physis claim in the client checked against the two local checkouts,
`~/dev/physis-pro` (crates.io `physis_pro` 0.1.21) and `~/dev/physis-next`.
Two real defects, both user-visible falsehoods; several claims verified correct.

**1. The page advertised 10 machines that 404.** `GET /api/v1/machines` is the
ENGINE's roster and physis-pro marks all 18 machines `status: "live"`
(src/web/machines.rs has 18 literal `status: "live"` entries). praxis proxies
only 8 — `LIVE_MACHINES` in src/routes/physisRoutes.ts — and answers
`404 machine 'x' is not a live reducer` for the rest. The SPA trusted the status
field, so `live` is now the intersection with that set. The static fallback had
hard-coded `live: true` on all 17 rows, which is how the drift got in; the flag is
now derived from one list so it cannot drift again. `apodeixis` (the 18th) was
missing from the fallback entirely. Two tests pin the intersection.

**2. Apache 2.0 was advertised for proprietary software.** physis-pro's
LICENSE.txt is an EULA — "This software is proprietary and confidential. It is
licensed, not sold" — and Cargo.toml points `license-file` at it. Four places
claimed otherwise: MePage's project body, MePage's prose, an `Apache 2.0` badge,
and DonationSection's "Physis is indie and open-source". All corrected.

**3. "70-cell semiotic grid", stated six times, was wrong.** physis-pro
DEFAULT_DOMAINS is 5 and MODES is **9** (reworked 2026-09-19), so 45 cells. The
"70" survives only inside a historical measurement table for the superseded 5×14
grid, and the file's own doc-comment says "the count is a default, not a law".
Rewritten to state the mechanism and the real names — five domains (Heal,
Construct, Fabricate, Bond, Study) × nine modes (Create, Maintain, Motion,
Cognition, Communication, Social, Possess, Rest, Stative) — instead of a cell
total that the next remeasure would falsify again. Also "630+ ontology domains"
became "730+ ontology entries": semiotic_grid.rs cites 731, and those are ontology
entries, not domains.

**Verified correct, left alone:** "18 reducers · 8 live in demo" (NAMES in
src/machines/run.rs has exactly 18; LIVE_MACHINES has exactly 8); `cargo install
physis_pro` (HTTP 200 on crates.io); "published to crates.io"; the air-gap claim
(src/license.rs: "Physis ships to air-gapped sites", with offline licence tooling
in src/bin/license_tool.rs); `github.com/ilPez00/physis` (HTTP 200).

**Found next to it, not ours to fix:** crates.io lists physis_pro's repository as
`github.com/ilPez00/physis-pro`, which is **404** — the crate metadata points at a
dead URL. Also praxis' own LICENSE file says MIT while package.json says ISC.

Delegation note: this was scoped for hermes and hermes is unusable —
`Gemini HTTP 400 (INVALID_ARGUMENT): API key not valid` — so the sweep ran as
greps against the two checkouts instead.

Gates: client 264 → 266 vitest (53 files), tsc 1 pre-existing web-vitals error
and 0 new. Backend 1234 passing, 0 failing; the 8 suites that fail to load
(nodemailer) fail identically on a stashed tree.

### praxisweb.xyz/physis — the Context MCP panel already existed (2026-10-04)

Correction. This entry originally recorded adding a "Physis Context MCP — the local
binary" €50 panel. It was redundant: `a4615551` had already added it, wired to
`POST /sales/checkout/creem`, with the licence signed by
`PHYSIS_CONTEXT_SIGNING_SEED_HEX`. The panel and its env documentation are
Creem's and were kept; the duplicate was dropped.

Two lessons worth keeping, because both cost real work:

- **The local checkout was 24 commits behind `origin/main`.** That is why the
  earlier note in this file claimed the Creem webhook "does not exist anywhere in
  the repo". It does exist — `src/routes/creemWebhook.ts` with 167 tests. Absence
  was asserted from a stale tree, which is the same error this file keeps catching
  in prose. Check `git fetch` and `origin/main` before making any claim about what
  the repo does or does not contain.
- The page work also assumed the licence file had no emitter. It does:
  `PHYSIS_CONTEXT_SIGNING_SEED_HEX` signs the envelopes. The "praxis structurally
  cannot fulfil this" note was wrong for the same reason.

### Colibri registry: unique provider names (2026-09-28, follow-up)

Found while fixing two suites that were failing on `LlmProviderRegistry`
internals — they were the symptom, this was the disease.

- `buildProviderRegistry` registered every config it was handed, so a name could
  appear twice. pansa's own `.env` triggers it:
  `COLIBRI_NODES=local=http://127.0.0.1:11435` names **this box** as a peer, and
  the peer provider is named `colibri_<node>` → a second `colibri_local` beside
  the static local engine.
- Consequence: provider names key the `inflight` map `leastLoaded` sorts on, so
  the two records shared one counter while `AICoachingService` counted
  `colibriProviders.length >= 2` and took its delegated/racing path — believing
  it was load-balancing a two-box cluster when both "engines" were one machine.
- Fix: one record per name. The explicit node address wins the URL (mirrors
  `EP_COLIBRI_REMOTE` for `colibri_remote`), a record with no address is switched
  on by the node's, and the collapse is logged. Live: 42 → 40 providers, one
  `colibri_local` at the node address, all names unique.
- Tests: `colibriCluster.test.ts` +3 (unique names invariant), and the two suites
  are now hermetic — `COLIBRI_*` neutralised per test, `LlmResponseCache` mocked
  (a real cached answer in `axiom_response_cache` had been answering a unit test),
  and `global.fetch` fails loudly if a test reaches for a live engine.

### Identity & Onboarding

- **Supabase JWT auth** — backend service-role key, frontend anon key (`client/src/hooks/useUser.ts`)
- **@username at onboarding** — format + uniqueness validation, persisted in `profiles.username`
- **Interactive onboarding wizard (2026-08-16)** — `/onboarding` (`OnboardingPageV2.tsx`) rebuilt as a 5-step guided flow that **actually creates the profile** instead of a bare name+goal form: profile (name, @username with live availability check, age, avatar, bio) → goals (14-domain picker with starter steps or free-text with auto domain inference, ≤3, via the bulk `POST /goals` — which is free on first run; the old per-node path charged 150 PP and broke new users) → connection profile (home base + life stage + connection intents, previously stranded in the now-deleted `GettingStartedPage`) → notebook identity (handwriting + signature) → done (starts Day-1 streak +10 PP, celebrates, hands off to the tour). Goal presets moved to `features/goals/goalPresets.ts` (shared with `GoalSelectionPage`).
- **Interactive usage guide (2026-08-16)** — `OnboardingTour.tsx` replaced the static Italian marketing slides with a shell overlay that spotlights the **real** bottom-nav tabs and the ⚡ FAB (`data-tour` hooks on `BottomNav`/`SpeedDial`). Users either tap the highlighted control (guide advances on the real click) or press the CTA, which performs the action (navigate, or dispatch `praxis:quicklog` to open the actual log sheet). Shown only after `onboarding_completed`; gate bumps `TOUR_VERSION` (now `2026.08.16.v2`) so existing users get the new guide once.
- **Profile public view** + reliability score + location detect (Nominatim reverse geocode)

### Goal System (The Notebook)

- **Goal tree / Notebook** — JSONB `goal_trees.nodes`, recursive parent/child, weighted progress roll-up
- **Goal metadata** — `completionMetric`, `targetDate`, `customDetails` (description)
- **Peer verification** — DM card "Verify/Reject" → auto-achievement on approve (`completionController.ts`)
- **Premium gate** — `goal_tree_edit_count` enforces 1 free re-edit, admin bypass
- **Marketplace slot expansion** — buy extra root-goal slots (200 PP each via `goal_slot`)
- **Suspend goal** — 50 PP action sets `node.status='suspended'` (renders greyed + ⏸ badge)
- **Visual overhaul** — dot-grid bg, radial gradients, gradient branches, domain emoji icons
- **Notebook waypoints** — time + place bound notes. Calendar shows goal deadlines (🚩, click → goal tree) and future-dated notes (📌); click a future day or drop a map pin to add one. Geotag from map pin, GPS, typed place (Nominatim), or LLM-extracted from the note text. `notebook_entries.{occurred_at,latitude,longitude,location_name}` (`NotebookPage.tsx` calendar/map, `AddWaypointDialog.tsx`, `POST /notebook/entries` + `/notebook/geocode` + `/notebook/extract-place`)

### Tracker System

- **13 tracker types** — lift, cardio, meal, steps, sleep, meditation, study, reading, expense, investment, music, journal, gaming
- **Domain auto-activation** — FITNESS → lift/cardio/meal/steps etc.
- **Dual-write architecture** — JSONB log + 13 structured Postgres tables with `GENERATED ALWAYS AS ... STORED` computed metrics (volume_kg, pace_min_per_km, total_eur)
- **Exercise library** (50 entries grouped by muscle) + **food library** (51 entries with kcal/100g + Open Food Facts fallback) — both render as MUI Autocomplete
- **Library expansion (2026-08-14)** — exercise library → 500+ entries (secondary muscles + compound/isolation tags, new Neck group); food library → 230+ entries with full P/C/F macros per 100g + typical servings; cardio library → 110+ activities with MET + steps/min cadence. The meal form now auto-computes calories+macros from grams, and cardio sessions estimate step-equivalents; both are persisted into the structured tracker tables.
- **Nutrition dashboard (2026-08-14)** — `nutrition_targets` table + `GET/PUT /nutrition/targets` + `GET /nutrition/totals?days=N` aggregate `tracker_meals` per day. `NutritionDashboard` (rendered in `TrackerSection` when a meal tracker is active) shows today's kcal ring + P/C/F progress vs editable targets, a 7-day calorie bar chart, and today's meal-slot split. Web-app meal logs now dual-write `tracker_meals` directly so the dashboard reads the same typed rows Axiom uses.
- **Strength trends (2026-08-14)** — `LiftProgressionService` computes Epley e1RM + weekly best-e1RM curves from `tracker_lifts`; `GET /trackers/lifts/progression` returns per-exercise trends + a deterministic next-session suggestion (add ≤5% when reps top the range, else +1 rep, hold on decline). `StrengthTrends` widget in the lift card shows sparklines + `→ kg × reps` chips and one-tap schedules the next session onto the weekly planner.
- **Planner intelligence (2026-08-14)** — the weekly generator now learns from the previous 3 weeks of `planner_blocks`: habits completed ≥50% keep their full schedule, 20–50% get thinned to alternate days, <20% collapse to a single weekly probe (with explanatory notes). Pure helpers (`adherenceRates`/`gradeAdherence`/`applyAdherenceToDays`) are unit-tested.
- **Schedule matching (2026-08-14)** — shared `AvailabilityService` (overlap score, common meet-windows, clock-time availability) now backs `matchingController`'s schedule scoring and the daily `AxiomScheduleService` partner pick (evening/weekend availability bonuses). The matches API returns `scheduleWindows` ("Mon 09:00" — when you can both meet) shown as "Free together" chips on MatchesPage.
- **Match filters + explainer (2026-08-14)** — MatchesPage gains a "Prefer opposite sex" toggle (hard-filters binary same-sex matches) and the score badge explains itself on hover (Goal/Vibe/Ontology/Place/Schedule/Sex/Reliability weighted points); `useUser`/`User` now surface `sex`/`preferred_sex`.
- **Food barcode scan (2026-08-14)** — `MealBrowserDialog` gained a barcode box (paste EAN/UPC → Open Food Facts lookup → click to add with real macros + serving weight). `foodLibrary.fetchFoodByBarcode`/`parseWeightLabel` are unit-tested.
- **Per-domain progress plans (2026-08-14)** — `DomainProgressService` proposes a baseline-aware weekly target for **all 14 Maslow domains** and grades you against it (ahead/on-track/behind/plan-it). `user_baselines` (height, weights, body-fat %, fitness level + 3 lifts, activity, savings rate, knowledge level, reading wpm) calibrate everything: **Mifflin–St Jeor computed calories/macros** (no more hardcoded 2000 kcal), level-scaled lifting volume + e1RM progression rate, reading pace, and savings targets. `GET /domains/progress` + `GET/PUT /baselines`; `NutritionDashboard` now honors diet-**break days** (`nutrition_targets.break_days`) and shows "auto from your body". `DomainPlanSection` (top of TrackerSection) renders the full plan with a baselines editor.
- **Structured summary reader** — `getStructuredSummary()` powers Axiom prompts (AICoaching + notebookAxiom)

### Check-ins & Streaks

- **Daily check-in** — one-tap, mood + win_of_the_day, +10 PP base
- **Energy self-report is 1–5, and the field is `energy` (2026-08-26)** — the
  Axiom brief's check-in posted `energy_level` on a 1–10 slider; `checkinController`
  reads `energy` and nulls anything outside 1–5, so every reading that widget
  collected was discarded in silence. That starved `lowEnergy` (which scales the
  day's action down and leads with recovery trackers), `computeEnergySignal`,
  `deriveCharge` and `AxiomPersonaService`. The widget now uses the same 1–5
  stepper and `depleted / steady / charged` wording as `CheckInWidget`; the
  controller accepts `energy_level` as an alias and **logs a warning** instead of
  nulling an out-of-range value, because the silent null is what hid this.
- **Streak multipliers** — ×1 (base 10), ×2 @ 7d (+20), ×3 @ 30d (+30) stacked → 30–70 PP/day
- **Reliability score** `R = 0.65·C + 0.25·V + 0.10·S` (check-in rate, verification rate, streak stability)
- **Balance widget** — Master Roshi nudge when streak ≥ 14d and a domain averages 0% progress

### AI (Axiom) — LLM-gated

- **Morning daily brief** (once/day, LLM, fallback to heuristic on failure)
- **Every Pro user gets a brief every day (2026-08-27)** — nothing scheduled the
  real brief at all. `AxiomScanService.runGlobalScan` is the only producer of the
  full brief and was reachable only from the admin routes, so in practice a brief
  appeared when the client opened the app and noticed today's row was missing.
  `runGlobalScan` then filtered to users whose `last_activity_date` was today —
  the same circularity one layer down. The brief was gated on the behaviour it
  exists to cause. Now: `startProBriefCron` (`src/services/AxiomProBriefCron.ts`)
  runs daily at 05:10 under a `runExclusive` lease (so pansa and air cannot
  double-fire), generates for every premium/admin account that does not already
  have today's row, then **re-reads the table and logs anyone still missing** —
  "the job ran" and "every Pro user has a brief" are different claims. It does
  not pass `force`: `generateDailyBrief` early-returns on an existing row, so a
  user who already read today's brief is skipped rather than having it
  overwritten. Sequential on purpose (see the local-tier note below).
  `isEligibleForDailyBrief` is the extracted rule, covered by
  `src/__tests__/services/dailyBriefEligibility.test.ts`.
- **Local models get a timeout sized to measurement (2026-08-26)** — the default
  was 25s for every provider except `colibri_local`, which had 300s only because
  it was the one that had been hit by this before. `ollama_local` sat on 25s and
  could not finish even a short answer, so every local generation failed as a
  408 — indistinguishable in the error string from an endpoint being down, while
  both local endpoints answered `/api/tags` in 5ms. Measured on pansa
  (`gemma4:e2b`): 74 prompt tokens → 1249 completion tokens in 67s, ~18.6 tok/s;
  the brief's 17-slot JSON is ~1000–1500 output tokens, so minutes. Budget is now
  keyed on `isLocalEndpoint(baseUrl)`, not a provider name, at 240s. Verified end
  to end: real briefs now come back `source: llm` with a full 17-slot routine,
  ~500–700s per user. **Open question:** the nightly scan uses `CONCURRENCY = 3`,
  a number chosen for hosted API rate limits. On the local tier the constraint is
  CPU, so three concurrent generations contend rather than finish sooner — which
  is what pushes them back over the timeout. Not changed: it needs measuring, not
  guessing.
- **The two briefs are merged at the data layer (2026-08-26)** — there were two
  unrelated things both called "the daily brief". `AxiomMorningBriefService`
  (`GET /axiom/morning-brief`) computed an urgency ranking and emitted
  `action` / `why` / `secondary` / `patternAlert`, and only the goals-page strip
  called it. `AxiomScanService` wrote `axiom_daily_briefs` — every row in the
  database — with `message` / `routine` / `resources` and **no `action` at all**,
  and that is what the notebook and dashboard dropdown reads. `AxiomScanService`
  now computes the same signals (`scoreUserGoals`, `computeEnergySignal`,
  `computeContinuitySignal`, `buildSuggestedActions`) and writes `action`, `why`,
  `secondary`, `patternAlert`, `dreamProposal`, `priority`, `suggestedActions`,
  `energy`, `needsCheckin`, `meta` alongside its own fields. **Still one LLM call
  per day** — the scoring rides into the existing prompt as a PRIORITY GOAL block
  rather than firing a second generation. `patternAlert` is computed
  arithmetically, never asked of the model, and `action` / `why` / `secondary`
  have local fallbacks so an LLM failure still yields a concrete action.
- **Brief continuity (2026-08-26)** — `computeContinuitySignal` feeds yesterday's
  ACTION, whether the user checked in that day, and any actions they marked "not
  relevant" in the last 14 days into the next morning's prompt, with an explicit
  do-not-repeat rule and a scale-down rule after an unlogged day. Prompt
  conditioning, not a model of the user — the brief is still written fresh from
  the goal data each morning. The verdict is captured by two chips on the action
  and stored through the existing `POST /axiom/feedback` (`like` / `irrelevant`),
  so no schema change was needed.
- **Brief fields that were generated but never rendered (2026-08-26)** —
  `patternAlert` (stagnation / deadline inside a week), `secondary` (second
  goal's action) and `dreamProposal` (DreamEngine variation for stalled goals)
  now appear in the brief dropdown. All three had been produced and stored for
  months with no surface.
- **Ordered, metric-anchored brief (2026-09-28)** — the morning action now reads
  like a concrete order: the 06:00 routine slot is pinned to the exact PRIORITY
  GOAL name (`Set intentions — review "<goal>" targets`); every routine task
  and resource cites a real goal name + its precomputed progress % / days-since-
  check-in, never `#1` or invented metrics. Enforced by a CRITICAL RULES block in
  the prompt (overrides Radical Personalization) + a deterministic repair pass
  on `AxiomScanService.ts` that rewrites the routine first slot and rebuilds any
  resource whose `goal` is not a known goal, and goal-aware fallback generators
  (`generateRoutineFromArchetype`/`generateResourcesFromGoals` now accept the goal
  nodes). Backend `npm run typecheck` clean.
- **Whole-brief thumbs + comments (2026-09-28)** — `BriefThumb` (`client/src/
components/common/BriefThumb.tsx`) renders 👍/👎 on the MorningBrief header,
  backed by three endpoints: `POST /axiom/feedback` (existing), `GET
/axiom/feedback/mine?promptHash=` (own votes, for restore) and `DELETE
/axiom/feedback?promptHash=&feedbackType=` (un-vote). Vote hash is
  `brief:<YYYY-MM-DD>` so it is stable across reloads; thumbs-down opens a one-
  line comment field whose text feeds the next morning's generator as a
  "MARKED NOT RELEVANT" note (already consumed by
  `AxiomMorningBriefService.ts:213` reading `rejectedActions`). Scope = whole
  brief; per-item thumbs on the routine preview in `AxiomDailyProtocol` left for
  a follow-up when the plan card ships its own tickable blocks.
- **Notebook Signals collapse (2026-09-28)** — the six secondary always-on cards
  (DelegatedTasks, Retro, Places, Connections, TrainingBalance, KnowledgeReview)
  are folded into one default-closed `Signals` toggle on `/notebook`, leaving
  Brief + Planner + Calendar/Map + Journal visible by default. Badge-less for
  now; counts backstop behind each card's own signal and can be layered in once
  the per-card pending signals are plumbed.
- **Notebook query** — POST `/notebook/axiom-query` answers free-form questions on logged data (50 PP, premium free)
- **Progress estimation service** — LLM-assisted grading fed structured tracker aggregates
- **Coaching chat surface** — dedicated `/coaching` route with unlock CTA for non-premium
- **Standardized structured Markdown output (2026-08-29)** — Axiom responses across the entire system (coaching chat, Ask Axiom dialog, notebook queries, agent responses, fallback templates) now adhere to a consistent 4-part Markdown section format: `### 👁️ Observation` (grounded in user data), `### ⚡ The Undercurrent` (strategic psychological insight / avoidance vs. true will), `### 🎯 Next Action` (single concrete, measurable step <25 words), and `### 💬 To Sit With` (probing reflective question). Rendered in the client via `AxiomMarkdown.tsx` into themed visual cards with icon headers and inline markdown parsing.

### Points Economy (PP)

- **Catalogue** (`pointsController`) — boost_visibility/150, goal_slot/200, coaching_session/500, super_match/300, custom_icon/100, skip_grading/80, bet_stake/50, suspend_goal/50
- **Earn paths** — daily check-in streak (30–70), peer-verified goal completion (`+50 × weight`), streak sharing
- **PP purchase via Stripe** — pp_500 (€4.99), pp_1100 (€9.99), pp_3000 (€24.99) with idempotency on `stripe_session_id`

### Social & Network

- **Weekly planner (2026-08-14)** — `planner_blocks` 7-day grid; **`POST /planner/generate`** auto-fills a week from `goal_plan_configs` cadences + active tracker templates (source `'auto'`, idempotent, all blocks editable/deletable); copy-to-next-week; adherence summary.
- **Matching** — pure rule-based (domain overlap + progress similarity); no LLM
- **Matching v2 (2026-08-14)** — composite score now blends goal 30% / text 25% / ontology 12% / place 12% (geo + same-city) / **weekly schedule overlap 10%** (profiles.availability) / **opposite-sex preference 6%** (profiles.sex + preferred_sex) / reliability 5%; returns `scheduleScore`, `placeScore`, `sexScore`, `scheduleOverlapDays`. Profile editor gained a weekly availability grid + "prefer to spar with".
- **Geolocation + nearby users** — Haversine query, "Near You" widget on Dashboard
- **Personalized smart feed** — `/posts/feed` weighted score (goal overlap 30%, proximity 25%, honor 20%, reliability 15%, recency 10%, ×1.3 if shared board)
- **Groups = Reddit-style boards** — Posts tab (titled threads) + Chat tab per room
- **Chat** — text, media (Supabase Storage), completion-request cards, system messages, **rich reference linking** (goals/services/posts/groups/events inside messages via `metadata.reference`)
- **Video calls (WebRTC)** — Supabase Broadcast signaling (`webrtc_{sortedIds}`), Google STUN
- **Live streaming** + **donations** (PP tips to streamer)
- **Referral program** — unique code, +100 PP to both parties on claim

### Betting (Goal Staking)

- **Place/cancel bets** on goals with PP slider; 2× payout on peer-verified completion
- **Opponent betting** — bet against another user's claim; anonymous fails
- **Accountability posts** — placing a stake auto-posts to feed
- **Active/history tabs** with progress rings

### Marketplace

- **Services/jobs/gigs** CRUD (`/services`) — type filter, Browse + My Listings tabs
- **Premium (Pro)** — Stripe subscription; unlocks unlimited Axiom, extra goal slots, PDF full-history export
- **PP purchase** tiers (see Points Economy)
- **Betting entry point** (moved here from standalone nav)

### Notebook Export (PDF)

- **Free tier** — GET `/diary/export/plain` streams a curated 365-day PDF (cover, metrics grid, goals tree, tracker tables, life-log timeline, footer pagination)
- **Pro / 500 PP** — POST `/diary/export/notes` streams full-history (5-year) PDF
- **Renderer** — `NotebookPdfRenderer.ts` uses `pdfkit`, zebra-striped tables, auto-pagination with header repeat, brand palette

### Notebook Rhymes (Echoes — make the unseen seen)

- **Concept** — surface when free notes "rhyme": latent semantic / shared-tag / cyclical-rhythm / same-feeling echoes a user didn't consciously notice
- **Backbone** — per-entry `notebook_entries.embedding vector(768)` (Gemini `text-embedding-004`, same model as `profile_embeddings`); pgvector cosine via `<=>`
- **Embed pipeline** — `NotebookRhymeService.embedEntry()`, fired async on entry create + edit; backfill via `scripts/backfill-notebook-embeddings.ts`
- **Per-entry** — GET `/notebook/entries/:id/rhymes` → `match_notebook_entries()` RPC + JS enrich (tag/cyclical/feeling) → `{ rhymes, dream_echoes, pdca }`. Physis enrichment seeds the note + its top rhymes (per-user tenant). Shown via ✨ echo toggle → constellation panel in `NotebookPage` (rhymes + ✦ dream echoes + ⟳ PDCA). NOTE: ingests on every tap → tenant goals accumulate (eviction is an open physis item)
- **Echoes at rewrite** — POST `/notebook/rhymes/digest` { from, to } → `match_notebook_pairs()` RPC + union-find clustering in `NotebookRhymeService.digest()`. Clusters labeled by strongest shared signal (tag > feeling > weekday-rhythm). Rendered in `DiaryExportDialog` rewrite tier: cluster = batch digest, expand = per-note members. The reflective PDCA "Check" pass
- **Auto-tag (physis)** — `PhysisAutoTagService` calls physis `/api/v1/classify` (stateless ontology classifier) on entry create; fills `domain` when blank + appends high-confidence category tags without clobbering user tags. Fire-and-forget, fails soft when physis is down

### Physis Per-User Tenancy

- **physis_pro** web server is now **per-user**: session state (goals / PDCA / dreams / coherence) keyed by `X-Physis-User` header; absent → shared `_default` tenant (back-compat). Holarchy + stateless endpoints stay global. See physis `AGENTS.md` → Multi-Tenancy
- **`physisProClient.ts`** stateful fns (`getPdcaPlan`, `pdcaAct`, `generateDreams`, `registerCoherence`, `getCoherenceSnapshot`, `getStats`) take optional `userId` → sent as the tenant header via `tenant()`
- Unblocks the gated rhyme tier (PDCA loop / Dream echoes / coherence-weighted echoes) — those can now run per-user without cross-contamination

### UberWiki Anonymised Behaviour Grid (2026-08-15)

- **`GET /api/uberwiki/cells?kind=month|week&period=YYYY-MM-DD`** — population behaviour as physis **semiotic cells** (DOMAIN × MODE, fixed 5×14 = 70). Per cell: distinct users, events, mean progress, completion rate
- **`uber_wiki_cell_events`** has **no owner column**. Goals are classified via physis `/api/v1/classify`, then counted under two per-period HMACs (`member_hash` for distinct-user counting, `event_hash` for idempotency). The period is part of the hashed message, so the same user is unlinkable across periods. No secret → nothing recorded (fail closed)
- **k-anonymity floor (5 distinct users)** with **complementary suppression**: a lone sub-floor cell in a domain is derivable from the domain total, so the next-smallest goes with it. Withheld cells fold into a per-domain bucket carrying no user count
- Cells chosen over coherence-node labels deliberately: the classifier's codomain is fixed before the first user exists, so a suppression floor is meaningful. Free-text labels have unbounded cardinality and cannot be anonymised. Values outside the grid are dropped, never coerced
- **Leak closed**: `seedFromSystem` used to write every profile as a `privacy: 'global'` node with the real name, with `compressEntity` attaching that user's goal-domain distribution. The "anonymised" read masked names only when `privacy !== 'global'` — unreachable, since the query already filtered to global. User nodes are now `private`; migration fixes existing rows (prod had none — the seeder had never been run there, so the leak was latent rather than live)
- Both RPCs are `SECURITY DEFINER`, so RLS does not apply to them and Postgres' default `EXECUTE` grant to `PUBLIC` would have let any anon-key holder poison the aggregate and read the **unsuppressed** grid. Revoked from `PUBLIC`/`anon`/`authenticated`, granted to `service_role` only
- **Backfill (2026-08-16)** — `npm run backfill-cells [-- --dry-run] [-- --limit N]` replays existing `goal_trees` nodes through the same RPC the live hooks use. Idempotent (upsert on `event_hash`). Each node is stamped with **its own `created_at`**, not with the current period: suppression is a one-way door, because a period first published suppressed and later published unsuppressed discloses the original small cohort by **differencing** the two releases. Thin months therefore stay hidden permanently, by design
- **Honest scale note**: the grid needs 5 distinct users _per cell_ across 70 cells. Prod currently holds 10 goal trees / 25 nodes spread over three months, so every cell is below the floor and the page publishes withheld buckets only. That is the privacy guarantee working, not a defect — the surface becomes informative at roughly two orders of magnitude more users, and should be read as built-ahead-of-scale until then

### Blank dashboard — the SPA catch-all was serving HTML for missing chunks (2026-08-17)

Reported as "dashboard not loading, spinner forever" in production. The server
was healthy throughout: `/` and `/dashboard` 200, all 103 current assets 200,
Supabase up, `praxis-backend` running, no errors in the journal.

The fault: `app.ts`'s SPA catch-all answered **every** unmatched GET with
`index.html`, including `/assets/*`. A missing hashed chunk therefore came back
as **200 `text/html`** instead of 404. The browser had asked for a JavaScript
module, got an HTML document, `import()` failed on the MIME type, `React.lazy`
rejected and `<Suspense>` never resolved — a blank screen. Because the response
was a 200, **nothing appeared in the logs**, which is why the backend looked
healthy while the app was unusable.

Reached on every deploy: `client/dist` is rebuilt from scratch (verified — one
hash per chunk, all mtimes from the same build), so any client holding an older
shell asks for filenames that no longer exist. The old cache-first
`index.html` (fixed the same day, above) guaranteed shells went stale.

- `isBuildAssetRequest` (`src/utils/spaFallback.ts`) — anything under
  `/assets/` or carrying a build extension 404s instead of falling through.
  Extracted as a predicate so it is testable without booting the app; 5 tests,
  including that real client routes (`/dashboard`, `/u/:name`, `/goals/:id`)
  still reach the SPA, and that a slug containing `.js` is not mistaken for an
  asset. Mutation-verified.
- `lazyWithReload` (`client/src/lib/lazyWithReload.ts`) — all 57 route chunks
  now load through it. A failed chunk clears the `praxis-shell-*` caches and
  reloads once, guarded by a sessionStorage flag so a broken build surfaces as
  a real error rather than a refresh loop. Without this a stale tab cannot
  recover on its own: the shell is already loaded, so nothing re-fetches it.

Diagnosis note for next time: a blank page with **200s** in the access log is
this class of bug. The absence of 404s is the symptom, not the exculpation.

### Offline app shell — notebook / dashboard / home work with no network (2026-08-17)

The app was a PWA that did not survive losing the network. Three separate
faults, all in `client/public/service-worker.js`:

- **Lazy chunks were never precached.** The worker precached `/`,
  `/index.html`, `/manifest.json` and nothing else, while all 56 routes are
  `React.lazy`. Any page not already opened in the installed version had no
  chunk offline, so the import rejected and the screen went blank. This is why
  "downloaded" did not mean "works" — the shell was there, the app was not
- **`index.html` was cache-first.** Users were pinned to the shell they first
  installed, which references hashed chunks a later deploy has already deleted.
  Navigations are now network-first with the cached shell as fallback; hashed
  assets stay cache-first, which is safe precisely because a content-hashed URL
  can never serve stale bytes
- **The API whitelist named endpoints the app does not call.** It listed
  `/api/dashboard` while the page calls `/api/dashboard/summary`, and omitted
  the notebook's diary and tracker reads entirely — everything unlisted fell
  through to the 503 `OFFLINE` stub. Rewritten against the endpoints the three
  surfaces actually request

**`precacheManifest()`** (`client/vite.config.ts`, ~40 lines, no new
dependency) prepends `self.__PRECACHE_REV__` / `self.__PRECACHE_ASSETS__` to
`dist/service-worker.js` after the bundle is written — content-hashed filenames
are unknowable before that. Currently 102 assets, **4.0 MB**, one-time on
install. The source worker reads both off `self` with `|| []` fallbacks, so it
stays valid unprocessed for `npm run dev`. Chosen over vite-plugin-pwa/Workbox
because the worker is hand-written and carries behaviour worth keeping (the
cross-origin bail-out, the push handlers); only the manifest was missing.

**Two caches, deliberately different lifetimes.** `praxis-shell-<rev>` is keyed
by a hash of the asset list, so a deploy installs new assets and `activate`
drops the old. `praxis-api-v3` is **not** versioned — keying user data by build
would wipe everything held offline on every deploy. Both directions are pinned
by tests.

`OfflineBanner` (in the app shell, above the fold) says the screen is showing
saved data and how many queued writes are pending. It announces reconnection
rather than acting on it: `navigator.onLine` only means an interface exists — a
captive portal reports true — and auto-reloading would discard unsaved input.

Tested in `client/src/__tests__/serviceWorker.test.ts` (12 tests): the real
worker file is loaded into a fabricated worker global and its `fetch` handler
driven directly, so there is no second copy of the routing rules to drift.
Verified by mutation — versioning the API cache, narrowing the whitelist back,
and reverting navigations to cache-first each fail the suite.

Unchanged and still correct: auth already survives offline (`useUser` reads
`getSession()` locally and falls back to an authenticated stub rather than
ejecting to `/login`).

### Offline Lexicon + Lexical Matching (2026-08-16)

- **Six offline dictionaries** — English, Italian, Spanish, French, German, Russian. Hunspell wordlists (wooorm/dictionaries) for "is this a real word", Princeton WordNet 3.0 + Open Multilingual WordNet for synonyms and synset ids. Downloaded and compiled per host with **`npm run dictionaries`** into a gitignored `data/dictionaries/` (~21 MB compiled, ~65 MB raw). No model, no API key, no network at request time
- **`SortedIndex`** — the compiled files are plain sorted text, binary-searched in place. A `Set<string>` of ~470k base forms costs ~100 MB resident; string-plus-offset-table costs ~57 MB for all six languages including synonyms and concepts, and loads in ~350 ms. Fuzzy search scans without allocating (2.6 s → 112 ms across five Latin wordlists)
- **`LexiconService`** — `lookup`, `search` (exact → prefix → fuzzy), `lemma`, `synonyms`, `concepts`, `detectLanguage`, `analyze`. Language detection blends dictionary hit rate with function-word rate, because hit rate alone is biased by wordlist size (Russian's is 3× English's). Refuses to guess below a confidence floor rather than mislabelling
- **`user_vocabulary_profiles`** — per-user sparse vector of ~120 dictionary lemmas + ~160 WordNet synset ids with log-damped, L2-normalised weights, built from the **same corpus as `profile_embeddings`** (bio + non-private diary + non-private notebook). Private writing never enters it. Rebuilt nightly by `embeddingRefreshCron` (third pass, no quota so a 300-user batch), or on demand
- **Cross-language matching** — OMW keys every language's lemmas by Princeton synset offsets, so `disciplina` (it/es), `Disziplin` (de), `дисциплина` (ru) and `discipline` (en) share concept ids. Two people journalling in different languages can score as compatible, which their embeddings cannot do
- **New matching axis** — `LEXICON_WEIGHT = 0.12` in `matchingController`, blended as 40% concepts / 35% lemmas / 25% language overlap. Composite weights rebalanced (goal .28, text .20, lexicon .12, ontology .10, place .11, schedule .09, sex .05, reliability .05). **Optional axes now renormalise instead of scoring zero** — previously a candidate without a profile embedding was silently penalised for having written less
- **Explainable** — the API returns `lexicalAffinity`, `lexicalBreakdown` and `sharedVocabulary` (the actual words both people write), which an embedding cosine cannot produce
- **`/api/lexicon`** — `status`, `search`, `word/:word`, `analyze` (read-only, stores nothing, safe to point at private text), `vocabulary`, `vocabulary/refresh`. All authenticated; no `:userId` param exists, so no path-param identity to get wrong
- Degrades to nothing if the dictionaries were never built on a host: the axis drops out exactly like text affinity does without embeddings
- **Shipped to production 2026-08-16/17** — `user_vocabulary_profiles` applied on pansa (RLS on, owner-read policy, service-role writes); dictionaries compiled on both pansa and air, six languages with concept ids on each; `/api/lexicon/*` live and auth-guarded; profiles populate via the nightly 04:30 UTC pass. Air was asleep when the DDL ran and takes it over WAL on its next wake — worth confirming with `to_regclass` there before ever relying on a failover
- **Operational trap found during that rollout** — building the corpora under a _running_ server is a silent no-op. Indices are memoised per process and a missing file loads as an empty index, not an error, so pansa served empty concept indices (cross-language matching scoring zero, no error anywhere) until it was restarted. The boot log now prints the concept-entry count per language so the zero is visible

### Admin

- **Multi-tab panel** — Users, Groups/Boards, Stats, Network Diagram, Challenges, Axiom
- **Axiom tab** — usage stats, system prompt editor, key pool strategy, API key usage, LLM provider registry with enable/disable toggles, priority reorder, and health checks
- **Admin bypass** on edit gates (goal tree, locked briefs)
- **Demo-user seeder** — `POST /admin/seed-demo-users` creates 20 demo accounts with varied goals, streaks, tracker history, and diary entries. Idempotent (skips existing). Each demo user gets: auth account, profile with `username`/`praxis_points`/`checkin_streak`, goal tree, checkins (streak-length), tracker entries, and diary entries.

### Dev/Ops

- **Sentry** error tracking
- **Zod validation** on critical POST endpoints
- **Stripe idempotency + RLS audit + E2E smoke**
- **i18n** — 100+ translation keys
- **Praxis CLI** — internal automation binary
- **Self-hosted deployment** — pansa (app + self-hosted Supabase + the only Cloudflare connector) and, since 2026-08-27, omo (warm standby `:3002` + streaming DB replica `:54332`, no connector) in place of air, which is offline. `git push origin main` is the deploy: both hosts pull, rebuild and restart on the `praxis-autoupdate.timer` 5-minute cycle. The backend runs as the `praxis-backend.service` **user** unit, so `systemctl --user` needs `XDG_RUNTIME_DIR` set when driving it over SSH. (This line previously described a Railway + Vercel split; that has not been the deployment for some time.)
- **@swc/jest backend tests** (2026-07-02) — replaced ts-jest; 47 suites / 432 tests in ~40s on default heap (no more `NODE_OPTIONS` 4GB hack)
- **Login outage: stale LAN address in air's tunnel ingress** (2026-08-17) — `supabase.praxisweb.xyz` served 530 / `error code: 1033` while `praxisweb.xyz` served 200, so the site looked healthy and nobody could log in. The record points at air's tunnel **by design** (air proxies the database hostname through to pansa for failover); the fault was that air's ingress named pansa by LAN IP, `192.168.1.15:54321`, which air cannot reach — `no route to host` in ~3.0s despite both hosts being on `192.168.1.0/24`, while pansa's Tailscale address answers in 0.19s. That dial timeout was the "slow OAuth" phase; when air went offline the same fault turned into a fast 1033, which made it look like two problems. Fixed by pointing `config-air.yml` / `config-praxis-replica.yml` at `100.65.59.18`. **Never address one host from the other by LAN IP** — Tailscale is the only proven path between them
- **The app-wide slowdown of 15–17 Aug: two global regressions, neither in a page** (2026-08-18) — social, profile and notebook all degraded together, which ruled out any single screen. Both causes were in files every route loads. (1) `219f81c` (17 Aug) gave the service worker a real offline guarantee by precaching the whole build, and paid for it at install: ~102 assets, 4.0 MB raw / 1.2 MB compressed, fetched **all at once** with `cache: 'reload'` on the first page load after every deploy. `reload` bypasses the HTTP cache by design, so it also defeated the `immutable` headers — chunks unchanged since the last deploy were re-downloaded anyway — and ~102 parallel requests monopolise the browser's connection pool, starving the page the user is waiting for. Fixed by copying unchanged hashed assets from the previous shell cache (a hashed URL can only ever have one body), keeping `reload` for `index.html` alone, and bounding precache concurrency to 4. The offline guarantee is unchanged. (2) `acd4f67` (15 Aug) added six handwriting families for the Physical Notebook appearance as a second **render-blocking** Google Fonts stylesheet in `index.html` — paid by every visitor on every route, including those who never enable that appearance. Now fetched at low priority via `media="print"` and applied on load. **Neither fault was visible from any page's own code**, which is why per-page profiling kept coming up empty
- **Serving is single-node, and two traps are armed** (2026-08-27, verified on the hosts) — the whole public surface is `pansa`: one cloudflared connector, one Supabase stack, one origin for both `praxisweb.xyz` and `supabase.praxisweb.xyz`. `air` has been offline since 2026-08-24 and its standby role moved to `omo` (backend `:3002`, streaming PG replica `:54332`) without any document recording it. Two faults found while mapping this, neither yet fixed: (1) **pansa carries two cloudflared units for the same tunnel** — the running system unit with the full ingress, and a disabled user unit `cloudflared-praxis.service` whose config lists only apex + `www`; `tunnel-health-gate.sh` defaults `TUNNEL_UNITS` to that user unit and _starts_ it on the failing→healthy transition (it did, 2026-08-25T14:26:19Z), which ends with two connectors from one host, the second 404ing `supabase.` and `pansa.` for its share of traffic. (2) **the gate is inert on pansa** — it manages `systemctl --user` units while pansa's live connector is a system unit, so it logs and stops nothing. Also: omo still holds `praxis-tunnel`'s own token at `/etc/cloudflared/token` (unit disabled), one `systemctl enable` from the §5c collision. Inventory in `docs/HOSTING_TOPOLOGY.md`; phased plan for splitting serving across pansa / omo / air / pi in `docs/MESH_SERVING_PLAN.md`
- **air is active-active, not standby** (2026-08-18, confirmed; **no longer true as of 2026-08-27** — air is offline, its connectors are gone, and `cloudflared tunnel info praxis-tunnel` shows a single connector on pansa. The standby role moved to omo, which runs no cloudflared at all. See `docs/HOSTING_TOPOLOGY.md`.) — air runs a second connector on the _same_ production tunnel `087b548c`, so Cloudflare balances real `praxisweb.xyz` traffic across both hosts; `supabase.praxisweb.xyz` additionally routes through air by design, proxied on to pansa over Tailscale. This is intentional (see the `praxis-tunnel-gate.service` drop-in, which withdraws _both_ of air's connectors when its backend stops serving) but it is worth stating plainly, because "warm standby" in older notes reads as passive. Measured during the slowdown investigation and cleared as a cause: pansa answers `/health` in ~1 ms, air in ~2–10 ms, air idle at load 0.13. If active-active is not wanted, stopping `cloudflared-praxis-replica.service` on air leaves pansa as the only connector without breaking Supabase, which pansa's own ingress also serves
- **/dashboard slowness: a chunk group nobody asked for** (2026-08-18) — the notebook screen was the slowest in the app and the cause was not the backend (every endpoint under 121 ms except the once-daily `/morning-brief` LLM call) nor the network (supabase 190 ms, app root 90 ms). It was the bundle: the `charts` **manualChunks group** was a static import of the entry chunk, so every visitor downloaded 133 KB brotli of `@mui/x-charts` + `d3` to look at the landing page. A build-time probe over the entry's module graph proved no eager module imports x-charts — the grouping alone created the edge, by sweeping shared internals into a chunk that then had to be fetched whenever anything in it was touched. On top of that `NotebookPage` statically imported its collapsed analytics section, the map (Leaflet), two below-the-fold chart cards, and — via `NotebookEntryCard` — the whole charting library for a 24 px sparkline shown only on tracker entries. Eager set 2,121,323 → 1,831,416 bytes raw (634 → 549 KB gzip); notebook route chunk 76,651 → 60,174 raw with zero chart/leaflet/map static deps. **`<Collapse>` mounts its children unless given `mountOnEnter`**, so "default closed" was saving nothing
- **`/assets/*` served `immutable`** (2026-08-18) — the origin sent no `Cache-Control` for build artefacts, so Cloudflare applied its 4-hour default and every visit after four hours revalidated ~20 content-hashed files before the app could start. Hashed names cannot go stale by construction; `index.html` and `service-worker.js` are excluded because they are the two fixed names that must go stale for a deploy to reach anyone
- **Weekly planner embedded in the notebook** (2026-08-18) — `PlannerPage` gained an `embedded` prop that swaps its page `Container` for a plain `Box`, and the notebook renders it between Analytics and the calendar/map. The `/planner` route is unchanged. The shell is chosen from two module-scope components rather than one defined inline: an inline wrapper is a new component type every render, which would remount the subtree and drop dialog state and input focus mid-edit
- **Onboarding bounce: a guess rendered as a fact** (2026-08-18) — onboarded users were repeatedly redirected to `/onboarding`. Only one line in the client redirects there (`PrivateRoute.tsx`), so the flag had to be falsy at guard time — yet the reporting account's `profiles.onboarding_completed` was `true`. The gap: `useUser` falls back to an auth-metadata stub when the `profiles` fetch fails, and `auth.users.raw_user_meta_data` carries the flag for only 15 of 24 onboarded accounts, so the stub asserted `false` for the other 9. A failed fetch and a genuine new signup were **the same state**. Fixed by making the difference representable: `profileResolved` is set only when the row was actually read, and the guard requires it. Not knowing is now a reason to let someone through rather than reset them. The infrastructure was ruled out first and stayed ruled out — supabase 190 ms, app root 90 ms, air→pansa Kong hop 65 ms, both hosts serving byte-identical bundles
- **Shared-cache hazards from the one-fetch `useUser` rewrite** (2026-08-18) — moving the profile fetch to module scope removed 50 duplicate queries per page load but introduced two faults that per-instance state could not have had. **Identity**: the snapshot outlives every component, so signing in as a different account kept the previous profile visible to all subscribers for the length of the fetch — now cleared the moment the session resolves to a different id. **Ordering**: a forced refetch overtaking an in-flight load, or a sign-out landing mid-fetch, could apply a stale result over a fresh one — every publish is now gated on a monotonic generation token, and `SIGNED_OUT` bumps it so a departing session cannot be resurrected. Any module-scope cache keyed to a logged-in user needs both guards; neither is optional
- **530 diagnosis in `verify-auth.sh`** (2026-08-16, corrected 2026-08-17) — a 530 with a healthy local Kong prints the Cloudflare inner code and now distinguishes the two faults that look identical from the origin: a connector that is up but cannot reach its upstream (slow, then failing) versus no connector at all (fast 1033). The first version of this hint asserted "routing, not the database" and pointed at a DNS repoint, which would have been the wrong fix. The script's own "Google provider may be disabled" line is a downstream symptom — that check queries the public hostname while the callback check beside it queries local Kong
- **`auto_update.sh` installs dependencies** (2026-08-17) — `git push` deployed code but never `node_modules`, so any commit adding a runtime dependency built cleanly and then crash-looped on `require()`. air served nothing for a day on `Cannot find module 'snowball-stemmers'`, after a build the log had reported as successful — `tsc` happily emits code requiring a package that is not installed, so the build step structurally cannot catch it. Now runs `npm ci` for backend and client when the lockfile blob hash differs from a stamp inside `node_modules`, which also covers the re-exec path and a host whose `node_modules` was wiped
- **Cron leader guard** (2026-07-02) — every node-cron job runs through `runExclusive` (`utils/cronLease.ts`), which wins an atomic DB lease (`try_acquire_cron_lease` RPC, migration `2026-07-02-cron-leases.sql`) so 2+ replicas never double-fire. Fail-open until the migration is applied.
- **air's stale LAN addresses** (2026-08-17) — air's `.env` still had `SUPABASE_URL=http://192.168.1.15:54321` and `PHYSIS_API_URL=http://192.168.1.15:19876`. The two hosts cannot reach each other on `192.168.1.0/24`; Tailscale (`100.65.59.18`) is the only working path, and the identical fault had already been fixed in air's _tunnel ingress_ without anyone checking the application config. Two consequences, neither of which looks like an outage: the cron lease RPC failed every tick, so `runExclusive` fell open and **both hosts ran every cron unguarded**, double-firing `dream_tick` and `brief_pool_promotion`; and every DB-backed API call failed for whatever share of traffic air's connector served, which presents to a user as "the dashboard doesn't render, the site feels slow". Fixed by pointing both at `100.65.59.18` and restarting `praxis-backend.service`. **Grep every config for `192\.168\.` — fixing only the file that produced today's symptom is what left this one behind.**
- **Env precedence** — `src/lib/envLoader.ts` loads `/home/gio/.env` first and neither it nor dotenv overwrites an already-set key, so the master file silently wins for any key it defines. Check it before editing a repo `.env`, or the edit does nothing.
- **Disabled a crash-looping quick tunnel on air** (2026-08-17) — a user-scope `cloudflared.service` running an account-less `trycloudflare` tunnel to Caddy on `:8080` was exiting 1 and restarting every ~4s, which got the quick-tunnel API to rate-limit it (`429`, error 1015), so it could never recover on its own. Stopped and disabled; production uses named tunnels and was unaffected. Restore with `systemctl --user enable --now cloudflared.service` after re-creating it as a named tunnel.
- **Betting anti-mint guards** (2026-07-02) — `createBet` rejects bets on goals with a pending/approved completion claim; `resolveBetsOnGoalCompletion` voids (stake-refunds, no 1.8×) bets placed <48h before verified completion. Needs `voided` in the bets status CHECK (migration `2026-07-02-bet-voided-status.sql`). Also fixed: goal_trees progress update filtered on `userId` instead of `user_id`, so peer-verified goals never actually hit progress 1.0.

### Marketing / Acquisition (Phase 1, 2026-08-14)

- **UTM source attribution on signup** — `registerSchema` accepts `utm_source/medium/campaign`; `/auth/signup` stores them in user metadata and writes `profiles.{utm_source,utm_medium,utm_campaign}` (no-op if the column is missing — migration `2026-08-14-signup-attribution.sql`). Turns "twitter visitors convert" from a funnel guess into a customer fact.
- **Referral auto-claim on signup** — the invite URL's `?ref=CODE` is now auto-claimed at signup (shared logic extracted to `applyReferralClaim` in `referralController.ts`), so the share link works end-to-end without the manual Claim step. `referralCode` was previously sent by the client but silently stripped by zod.
- **Referral share upgrades** — `ReferralWidget` now builds the invite link from `window.location.origin` (was a stale hardcoded Vercel URL), embeds `ref` + UTM tags, and adds one-tap WhatsApp + email share buttons.
- **Freelancer hero variant** — `/` reads `?v=freelancers` or `?utm_campaign=for-freelancers` and swaps the hero copy for the solo-operator angle; default copy untouched for cold traffic.
- **`/for/freelancers` landing subpage** — standalone marketing page (own nav/footer, shell hidden), pitch blocks for billable hours / body / craft / stakes, CTA links carry `utm_campaign=for-freelancers`. Linked from the landing footer.
- **`first_duel_created` funnel event** — added to client type + backend allowlist, fired on duel creation.

### Onboarding + Agent Connectivity (2026-08-14)

- **Version-gated onboarding** — new `client/src/lib/onboardingGate.ts` stores the _version_ a user last saw (`TOUR_VERSION` / `SLIDES_VERSION`), not a boolean. The dialog tour and the dashboard Getting-Started slides show once per version, and again only when a future release bumps the constant. Legacy `has_completed_onboarding='true'` dismissals still count as seen, so existing accounts aren't re-toured by accident. Slides' "Start tracking" CTA now points at `/notebook` (was `/notes`).
- **`McpConfigCard`** (`client/src/components/common/McpConfigCard.tsx`) — reusable streamable-HTTP MCP setup with a per-client (Claude/Cursor/opencode/generic) config builder, copy + download (as the client's expected filename), and an inline one-tap key minting flow. Embeds a `pk_live_…` PAT as the bearer token.
- **MCP surfaced everywhere** — landing page `AgentAccess` section (pre-login, downloadable config with placeholder token, linked from footer `#for-your-ai`), `Settings → AI Assistants`, the private `/mcp` page, and `/connect`.
- **MCP defaults fixed for self-hosting** — `praxis-mcp-server` `PRAXIS_API_URL` defaults, `railway.toml`, `praxis-mcp.json`, README, plus client `Settings`/`AxiomProSection`/`ConnectPraxis` all updated from the dead `*.up.railway.app` URLs to `https://praxisweb.xyz`.

### Agent Tools & Scoped PATs (2026-08-29)

The agent surface grew from "log trackers / read goals" to a scoped, audit-friendly toolset an external agent (Aura, ChatGPT Actions, a script) can call on the user's behalf. Backend commit `a3dee04`.

- **Scoped Personal Access Tokens** — `POST /agent/keys/personal` now accepts an optional `scope` array. Scopes: `read` (always granted), `write` (tracker logs, check-ins, goals), `boards` (group kanban), `messages` (DMs). `normalizeScope` defaults `['read']` → `['read','write']`; a token can be `boards`-only without full `write`. `authenticateApiKey` returns `scope`; every tool call is gated by `enforceScope` in `src/mcp/scope.ts`.
- **`get_context`** — a one-shot snapshot tool (scope `read`): returns the user's memory, goals (active + goal-keyed trackers), recent activity, streak, and boards. Cheaper than N separate reads; the intended entry point for an agent starting a session.
- **Group kanban board tools** (scope `boards`) — `list_board_tasks`, `move_board_task`, `complete_board_task`. Operate on `group_tasks` rows, membership enforced via `group_members` (`assertBoardMember` throws if the token owner isn't in the room). `status` validated against `todo | in_progress | done`.
- **UI** — `Settings → AI Assistants` `AgentsPanel` gained a scope picker (read/write/boards/messages toggles) on PAT creation, scope chips on each key, and a "Connect to ChatGPT (Actions)" card linking the OpenAPI spec (`/api/openapi.json`) + MCP URL.
- **OpenAPI** — `src/openapi.ts` documents every tool via `POST /api/mcp/action` (`callAgentTool`) with an `operation` enum and a `403` Forbidden (scope) response, so ChatGPT Actions / Gemini function-calling can surface the full tool list.
- **Tests** — `src/__tests__/mcpDataTools.test.ts` covers the scope model and the four new tools through `routeDataAction`.

---

## 2. Implemented → Removed / Hidden

Features that shipped and were later removed, hidden, or replaced. Kept here for historical context and so we don't re-invent something we already tried.

| Feature                                                         | Status  | Commit                   | Reason / note                                                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Events (Discover + feature)**                                 | Hidden  | `95f9eac` + this session | Removed from Discover filter/markers/compact feed; awaiting reimplementation as seasonal/community milestones (see §3)                                                                                                                                                                                                                                                                              |
| **Identity verification page (`/verify-identity`)**             | Removed | `e95ca11`                | Route deleted; plan was face-api liveness, deprecated with face-api                                                                                                                                                                                                                                                                                                                                 |
| **face-api.js**                                                 | Removed | `952f0a7`                | Bundle size + accuracy; replaced by peer verification flow                                                                                                                                                                                                                                                                                                                                          |
| **Dashboard tracker chips + accountability buddy widget**       | Removed | `21debb0`                | Cluttered dashboard; trackers now live in notebook, buddy surfaced via matches                                                                                                                                                                                                                                                                                                                      |
| **Axiom goal-strategy + network-leverage blocks (daily brief)** | Removed | `88e81cd`                | Noisy; daily brief now focuses on one actionable nudge                                                                                                                                                                                                                                                                                                                                              |
| **Axiom's Take** (inline commentary)                            | Removed | `9d60a33`                | Redundant with notebook query + morning brief                                                                                                                                                                                                                                                                                                                                                       |
| **Welcome State (Notebook empty-state)**                        | Removed | `83aba9d`                | Onboarding already walks new users through first entry                                                                                                                                                                                                                                                                                                                                              |
| **Axiom training sources + knowledge base UI**                  | Removed | `3d74e21`                | Leaked internals; not user-facing value                                                                                                                                                                                                                                                                                                                                                             |
| **EmbeddingService**                                            | Removed | `f47e68a`                | Semantic search not materially better than rule-based match for current data size                                                                                                                                                                                                                                                                                                                   |
| **HabitCalendar**                                               | Removed | `09d013f`                | Replaced with check-in streak + tracker aggregates                                                                                                                                                                                                                                                                                                                                                  |
| **Old SVG goal-tree visualization**                             | Removed | `3287ddc`                | Replaced by gradient/dot-grid canvas viz                                                                                                                                                                                                                                                                                                                                                            |
| **More dropdown (nav)**                                         | Removed | `ea50180`                | Flattened nav; surface everything directly                                                                                                                                                                                                                                                                                                                                                          |
| **Friends (nav entry)**                                         | Removed | `2f65327`                | Matches + DMs cover this; "Friends" implied a social graph we don't maintain                                                                                                                                                                                                                                                                                                                        |
| **Betting (standalone nav)**                                    | Removed | `1cf2780`                | Moved under Marketplace                                                                                                                                                                                                                                                                                                                                                                             |
| **Post feed in marketplace**                                    | Removed | `d1a93f8`                | Marketplace is transactional; feed belongs on Dashboard                                                                                                                                                                                                                                                                                                                                             |
| **Archetype references**                                        | Removed | `be16712`                | Feature pivot; domains replaced archetypes                                                                                                                                                                                                                                                                                                                                                          |
| **Goal/subgoal nomenclature**                                   | Removed | `a5648ee`                | Everything is a "node" in the tree — uniform vocab                                                                                                                                                                                                                                                                                                                                                  |
| **Habit heatmap**                                               | Removed | `9cd77b9`                | Replaced with line chart (cleaner for streaks)                                                                                                                                                                                                                                                                                                                                                      |
| **react-leaflet**                                               | Removed | `9fe4a0f`                | Switched map stack                                                                                                                                                                                                                                                                                                                                                                                  |
| **Google Maps SDK**                                             | Removed | `82491ce`                | API-key-gated; deferred until we have budget / need                                                                                                                                                                                                                                                                                                                                                 |
| **Goal-tree → Notebook rename (old name deprecated)**           | Renamed | `43b4be4`                | Product identity: it's a journal, not a tree                                                                                                                                                                                                                                                                                                                                                        |
| **Trend / metrics button (Notebook)**                           | Removed | this session             | User-requested simplification                                                                                                                                                                                                                                                                                                                                                                       |
| **`minimal_ai_mode`**                                           | Removed | `8cdc20d`                | Collapsed into premium/PP gating                                                                                                                                                                                                                                                                                                                                                                    |
| **Social-density features**                                     | Removed | `0471182`                | Overengineered for current scale                                                                                                                                                                                                                                                                                                                                                                    |
| **Dead-code purge (2026-07-02)**                                | Removed | `5589b74`                | Deleted unreferenced/unrouted code: legacy `App.tsx`/`Navbar.tsx` shell, `coaching`, `communication`, `services`, `identity`, `streaming` features, `EventCheckinPage`, stale playwright artifacts (−4.8k lines). Links that pointed at dead routes repointed (`/coaching`→`/notebook`, `/communication`→`/groups`). UberWiki graph moved to `/uberwiki-graph` (was shadowed by a duplicate route). |

Pattern takeaway: most removals are **consolidation** (two surfaces → one) or **deferred infra** (face-api, maps, embeddings) — not product-direction reversals. Keep the list when considering a rebuild; most of these will come back in a cleaner form.

---

## 3. Proposed Future Features

Elaborations for each. Grouped by theme. Priority tags: 🔴 next up, 🟡 planned, 🟢 speculative.

### 3.1 Social & Accountability

**🟡 WhatsApp-style Groups v2** — governance core SHIPPED (backend, 2026-06-22)  
Current groups are Reddit-style boards (posts + chat). **Shipped (backend):** admin/mod/member roles + private rooms + join-approval + rules — migration `2026-06-22-groups-v2.sql` (`chat_room_members.role`/`.status`, `chat_rooms.is_private`/`.rules`, creator→admin backfill); `utils/roomMembership.ts` pure authz predicates (`canModerate`/`canAdminister`/`outranks`/`joinStatusFor`, tested 7); `createRoom` (private+rules, creator=admin), `joinRoom` (private→pending), message read/post gated on `status='active'`, and new admin/mod endpoints `PATCH /:roomId`, `GET /:roomId/requests`, `POST /:roomId/requests/:userId/approve|reject`, `PATCH /:roomId/members/:userId/role` (last-admin demotion blocked, double-approve guard). **Shipped (frontend, 2026-06-22):** `GroupRoom` now derives the caller's role from members; admin sees a ⚙ settings gear → `GroupSettingsDialog` (edit name/desc/rules + public↔private toggle, `PATCH /groups/:id`); the members icon carries a pending-count badge for mods; `GroupMemberDrawer` shows role chips, an admin per-member role selector (enabled only where they outrank), and a moderators-only pending-requests section with approve/reject. Migration applied to prod. **Pinned posts SHIPPED (2026-06-22):** `posts.pinned_at` (migration `2026-06-22-pinned-posts.sql`); `getPosts` orders pinned-first; `POST /posts/:id/pin` gated on board-room mod/admin via `getMembership(post.context)`; PostFeed shows a 📌 badge + a mod-only pin/unpin toggle (`canModerate` passed from GroupRoom's Posts tab). **Mute UI SHIPPED (2026-06-22):** GroupRoom header bell toggles room mute via existing `POST/DELETE /mutes/room/:id` (initial state from `GET /mutes`, optimistic + revert); `pushNotification`'s room-mute check was already live, so muting now actually silences a group. **@mentions SHIPPED (2026-06-22):** `utils/mentions.extractHandles` (word-boundary, email-safe, deduped, capped 10; tested 6); `sendRoomMessage` resolves `@username` against active members' `profiles.username`, stores `metadata.mentions`, and sends mentioned users a distinct `mention` ping instead of `group_message`; `getRoomMembers` now returns `username`; GroupRoom composer has an `@`-autocomplete popup (member avatar + name + @handle, click to complete). No migration (uses existing columns). **Still open:** Events pin area (see 3.6) — the last Groups v2 item, tied to the Events rebuild.

**🟡 Accountability streaks with peer verification**  
Two users pair up ("sparring partners"). Each logs daily; every 7d both must peer-verify each other to keep the joint streak. Joint streak unlocks bigger PP multipliers than solo. Builds on existing `completion_requests` table + matches. DB: `partnerships(user_a, user_b, status, started_at, current_streak)`. Surface: "Sparring" tab in Matches.

**🟡 Bet auto-grading via peer verification** — ✅ SHIPPED (final piece 2026-08-24)  
Bets already auto-resolved on peer approval (1.8× payout, <48h anti-mint void) — this cycle closed the two remaining gaps. **Provenance**: `bets.verification_request_id` (migration `2026-08-24-bet-honor-pool.sql`) is stamped by `resolveBetsOnGoalCompletion` when a verification discharges the bet, so every auto-pay traces to the approval that caused it. **Honor pool**: a rejected claim's escrowed bond no longer pays the rejecting verifier — _rejecting must not pay_ — it enters `honor_pool_ledger` and is redistributed evenly to up to 5 distinct verifiers with approvals in the last 30 days (claimer and rejector excluded; integer dust stays pooled; failures degrade to "stays pooled", never lost). Split math in pure `src/utils/honorPool.ts` (tested); the old forfeit contract test now pins the new one. Single approval still discharges goal + bets + achievement simultaneously.

**🟢 Federated sparring-partner matching**  
Opt-in cross-instance matching using domain overlap + progress similarity. Users from partnered Praxis installations appear in each other's match pool. Privacy: only share `{domain_vector, progress_hash, pseudonym}`. Needs protocol spec (ActivityPub? custom?).

**🟢 Reliability-score leaderboard** — ✅ SHIPPED (2026-08-24)  
Weekly board frozen from `reliability_snapshots` ((user_id, week_start) unique, RLS service-role-only; migration `2026-08-24-reliability-snapshots.sql`). Cron Monday 00:10 UTC (`reliabilitySnapshotCron`, lease-guarded, idempotent upsert of every opt-in profile's live `reliability_score`) + manual trigger `POST /admin/cron/reliability-snapshot`. `GET /gamification/leaderboard/reliability?week=YYYY-MM-DD` ranks snapshots via pure `rankByScore` (competition ranking "1224", tested) and **falls back to live opt-in scores** until the first snapshot lands, so the surface is never empty. LeaderboardPage gained a Points ⇄ Reliability toggle. Drive-by privacy fix: `/gamification/leaderboard` — the board the page actually consumes — now filters `leaderboard_opt_in = true` with the same legacy fallback as `/users/leaderboard` (it had been ignoring the opt-in entirely) and its select now carries `reliability_score`/`honor_score`, which the UI chips were reading but the endpoint never sent.

### 3.2 Axiom / AI

**🟡 Axiom "Boost" tier**  
Premium-within-premium. Unlocks: unlimited notebook queries, faster model (we currently gate to `fast`), weekly personalized "state of you" report (PDF, emailed), voice interface. Stripe new price ID. Gate in `aiCoachingService.generateCoachingResponse` on `profile.axiom_tier`.

**🟡 Multi-language coach persona**  
Respect `profile.language` not just in UI strings but in Axiom prompt + response. Tone guide already exists in English (`AXIOM_TONE_GUIDE.md`); translate + adapt for it/es/fr/de.

**🟢 Structured-tracker history visualizations**  
Per-metric sparklines (volume_kg over 90d, avg pace, kcal in/out, net spend trend) on each tracker's detail page. Data is in structured tables; just need SVG sparkline component + `GET /trackers/:type/history`.

**🟢 AI narrative export refinements**  
Notebook PDF currently lists raw data. Add opt-in "Axiom narrative" page at the front: 400-word human-readable summary of the export period, themes, concerns, wins. LLM prompt templated on existing notebookAxiom structure.

**🟢 Voice-first check-in**  
Speak your win_of_the_day + mood; Whisper transcribes; same check-in endpoint.

**🟢 Subconscious "Shadow" Audit (30-Day Behavioral Synthesis)**  
Deep behavioral analysis cross-referencing journal entries, timestamps, and tracker logs over 30 days to uncover subconscious avoidance patterns, productive procrastination (e.g. over-polishing design details to avoid sales outreach), and discrepancies between declared identity and lived habits.

**🟢 Bioenergetic Adaptive Load Balancing (Contextual Pacing)**  
Automatic context-sensitive pacing when user logs traumatic/exhausting events (relationship tension, bereavement, illness, sleep deficit). Shifts Axiom protocols from "Push & Execute" to "Restoration & Smallest Winnable Action" without breaking streaks or inducing guilt. Shipped: `BioenergeticPacingService` computes load from check-ins / notebook distress markers / metabolic biomarkers; `GET /axiom/pacing` exposes it; the daily-brief prompt injects a `BIOENERGETIC PACING` override block (intensity multiplier, streak-protected, no push-through framing); `PacingCard` renders it in Analytics; a compact banner surfaces it inline on the dashboard daily-protocol card and in the Goals-page `AxiomBriefStrip` (RECOVERY/RESTORATIVE chip + expanded directive, intensity multiplier) and in the mobile `MobileWidget` in restorative/recovery modes — the pacing state is now visible on every daily-brief surface (web dashboard, goals, analytics, mobile widget).

**🟢 Ambient Urban Scouting & Offline Anchors**  
Contextualized third-place physical recommendations mapped to user's city and immediate psychological state (e.g. recommending a specific quiet library or lively café in Milan/Verona to break an isolated rumination loop).

**🟢 Titan Archetype Dialectic (Historical Master Framing)**  
Dynamic psychological framing drawing on historical thinkers and strategists (e.g. Marcus Aurelius for resilience, Steve Jobs for product courage, Viktor Frankl for meaning through adversity) tailored to current friction points.

**🟢 Axiom Audio Brief (60-Second Daily Voice Brief)**  
Synthesized morning audio note delivered at 07:00 daily, spoken in Axiom's unhurried, intimate voice that directly articulates the user's priority action and daily reflective question.

### 3.3 Notebook & Trackers

**✅ Tracker backfill script (JSONB → structured)** — SHIPPED (`50cd9aa`/`2026-04-27`; tested `2026-06-22`)  
`scripts/backfill-trackers.ts` replays legacy `tracker_entries` JSONB into the typed per-category tables via `StructuredTrackerWriter`. Idempotent (dedupe by `tracker_entry_id`), paginated, `--type`/`--dry-run` flags, wired as `praxis backfill-trackers --user-id=all`. Now covered by `backfillTrackers.test.ts` (5): write+ctx forwarding, idempotent skip, unsupported-type skip, dry-run no-write, writer-error tolerance. So structured summaries/PDF reflect full history, not just post-2026-04-19 logs.

**✅ Enumerate+Increment tracker model v2** — SHIPPED (`8d175834`, 2026-08-29)  
Count-based trackers shipped (`3760b98`) earlier; this cycle added the **templated enumerations** slice end-to-end (no schema — reuses `goal` JSONB via `goal.enumeration` + `goal.options`): `trackerTypes.ts` templates now ship predefined `select` fields with option lists; `EditableTrackerForm` renders the "Predefined options" chips; `useTrackerForm` consumes both the static templates and the dynamic `PUT /trackers/:type/enumeration` + `GET /trackers/:type/suggestions` endpoints and logs a selected option via `POST /trackers/log { data: { option: label, value: 1 } }`. Backend `saveEnumeration`/`getTemplateSuggestions` registered in `trackerRoutes.ts` with `enumerationSchema` validation. Plus (same session, no-schema) **quick-add from history** — `axiomSuggestions` chips from `GET /trackers/:type/suggestions`; **"Carry forward from yesterday"** in `useTrackerForm` (seeds today's rows from the most recent prior-day entry, distinct from "Repeat last"). Tests: `client/src/__tests__/useTrackerForm.test.tsx` (carry-forward derivation + replay). All sub-slices now shipped; nothing remains open.

**🟢 Goal-dependency DAG (beyond tree)** — ✅ MOSTLY SHIPPED (audit 2026-08-24)  
Prerequisites live as `prerequisiteGoalIds[]` JSONB on goal nodes (`GoalNode.ts`), editable in `GoalWorkspaceSheet` (prereq chips), rendered by `GoalCard`/`GoalCardTree`: a node whose prerequisite is under 100% shows 🔒, dims to 55% opacity and labels "LOCKED — complete first: …". Remaining if wanted: dashed connector overlay on the indent tree and Axiom unlock-chain sequencing (speculative). The `goal_dependencies` table upgrade stays P3 per its own note in `migrations/goal-dependencies-pending.sql` — JSONB suffices until users hold 10+ inter-dependent goals.

**✅ Offline notebook logging with sync** — SHIPPED (write-side, 2026-09-23)
Offline reads shipped (`8132b8f`, 2026-08-17, "Offline app shell" below). The
write side is now live too: `api.ts` response interceptor enqueues allowlisted
mutating writes (`OFFLINE_WRITABLE`: `/notebook/entries`, `/diary/entries`,
`/checkins`, `/trackers/log`, `/journal/entries`, `/planner`) to `db.outbox` on
network failure with an idempotency key and returns a `202 { queued:true }`
placeholder so the screen treats the write as accepted (optimistic UI) without
queueing money paths. `createOutboxSyncer` (`lib/outboxSync.ts`) is now
**mounted in `AppShell`** — it drains the outbox on mount, on browser `online`,
on tab `visibilitychange`, and on a 5-min safety-net interval, replays oldest-
-first reusing the idempotency key, drops 4xx/rejected entries, and caps retries
at `MAX_ATTEMPTS` (5) — one replay guard (`_replay`) so a drain failure can't
re-enqueue on every network flicker. `AppShell` swapped the dead legacy
`hooks/useOfflineSync` (drained `trackerEntries`/`journalEntries` tables nothing
writes to) for this drainer; that hook is deleted. Tests:
`client/src/__tests__/outboxSync.test.ts` (7 — mount/online/visibility/interval/
cleanup/onSynced/re-entrancy guard); existing `outbox.test.ts` pins enqueue/drain
ordering + the money-path allowlist + 4xx-drop + idempotency behavior.

What is **not** done (deferred, product decision, not a correctness gap): conflict
resolution rules for concurrent offline edits remain last-write-wins at the server
row level (tracked by `X-Idempotency-Key` dedup); and the global `OfflineBanner`
counts the older `offlineDB`/`OfflineEntry` store (still used by
`GoalNotesPanel` via `saveForLater`) but not `db.outbox`, so queued
api.ts writes don't yet surface a count in banner text — surfaced via the
"Synced N offline changes" toast instead. Revisit the banner count if the
journal/tracker offline volume justifies it.

### 3.4 Maps & Location

**🟡 Google Maps upgrade** — ❌ DECLINED (user decision, 2026-08-24): Praxis stays on **OpenStreetMap / Leaflet**. No API-key-gated maps, no per-load billing, self-hosters keep full functionality for free. The notebook atlas already swaps CARTO tiles by appearance mode, which is the right amount of map polish. Revisit only if turn-by-turn or traffic ever becomes a real requirement.

**🟡 Map overlay for smart-feed / discover** — ✅ SHIPPED (2026-08-24)  
`posts.latitude/longitude` (migration `2026-08-24-post-locations.sql`, partial index on geotagged rows; OSM stays per the Maps decision). Opt-in only: the composer's place-pin button attaches browser geolocation on explicit tap — never silent, removable before send, validated/clamped server-side by `src/utils/geo.ts` (~1m precision, tested). `GET /posts/geo` serves the last 30 days of geotagged posts as pin payloads; DiscoverPage gains a green **Posts** layer on the map (pin → thread) plus a Posts list tab; feed/thread selects carry coordinates.

**🟢 Commute-based matching**  
"Users on your route" — if user opts in sharing home+work, compute potential meeting points near both commutes.

### 3.5 Monetization

**🔴 Stripe production hardening**  
Production `STRIPE_PRICE_ID`s for subscription + PP tiers; register production webhook endpoint; end-to-end test via Stripe CLI. Blockers are config-only.

**🟡 Annual plan + founding-member pricing** — ✅ CODE-COMPLETE (audit + build 2026-08-24)  
Annual billing was already wired end-to-end: `interval:'year'` checkout on `STRIPE_PRICE_ID_ANNUAL` (`createCheckoutSession`), boot-time env audit flags it when missing, client `TIERS.pro_annual` + UpgradePage handler. Built this cycle: the **founding-member badge** — `profiles.badges jsonb` array (migration `2026-08-24-founding-members.sql`; `badge` stays the single marketplace-owned display slot), pure eligibility logic in `src/utils/foundingMembers.ts` (tested): the webhook counts _distinct_ earlier `user_subscriptions` users before inserting, so cancel-and-resubscribe can't spend two slots and webhook retries are idempotent via slug append. First `FOUNDING_MEMBER_LIMIT` (default 100; ≤0 disables) subscribers get `founding_member` + display badge "Founding Member" when free; profile hero renders a 🏛 chip. **Remaining is config-only**: create the annual Price in the Stripe dashboard, set `STRIPE_PRICE_ID_ANNUAL`, register the production webhook endpoint.

**🟡 Referral tier upgrade** — ✅ TIER 1 SHIPPED (2026-08-24)  
5 successful referrals gift the referrer **1 month of Pro**, implemented on new gifted-premium infrastructure: `profiles.premium_until` (migration `2026-08-24-premium-until.sql`) + single-source `isPremiumActive` (`src/utils/premiumStatus.ts`, client mirror `client/src/lib/premium.ts`) wired into the three central gates — `requirePro` (`requireTier.ts`), `LlmAudienceResolver` (model tiering), and `hasFreeAxiom` — plus the client's `useUser` mapping, which folds an unexpired gift into `user.is_premium` so every existing UI gate lights up untouched. Admins grant/revoke via `POST /admin/users/:id/grant-premium-days {days}` (stacking extension, days=0 revokes, audience cache invalidated). Stripe subscriptions keep owning the boolean; gifts expire on their own. Widget shows "N more referrals → 1 month Pro". The lifetime ×1.2 earn multiplier stays deferred: multiplying inside `increment_pp` would also multiply refunds (bet/bounty/challenge escrow returns), so it must land at explicit award sites only.

**🟢 In-product ads (first-party only)**  
Surface featured `services` listings as promoted cards in smart feed for PP cost. No third-party ad network.

### 3.6 Events (reintroduction)

**🟡 Seasonal / community milestones** — mostly SHIPPED; progress wiring completed 2026-06-22  
Time-bounded challenges (Dry January, NaNoWriMo, SummerBody2026). **Already live:** `seasonal_events` + `seasonal_event_participants` tables (migration `2026-04-02-seasonal-events.sql` + seed), `seasonalEventController` (active/by-slug/join/my-progress/update/claim/**leaderboard**) mounted at `/seasonal-events`, and `SeasonalEventCard` on the Dashboard (join/progress/claim + detail dialog). **Fixed 2026-06-22:** only `streak_challenge` events auto-advanced — `goal_crusher`/`participation`/`betting` events never moved. New `services/SeasonalEventProgress.incrementSeasonalMetric` (+1 per action, capped, single completed_at stamp; tested 5) wired fire-and-forget into goal completion (`goals_completed`), check-in (`checkins`), and bet win (`bets_won`). **Discover page + leaderboard SHIPPED (2026-06-23):** `/seasonal` route → `SeasonalEventsPage` — discover grid of active challenges (join/progress bar/claim) with a per-event collapsible **leaderboard** (consumes the previously-unused `GET /seasonal-events/:id/leaderboard`, ranks by progress, highlights "(you)"); dashboard `SeasonalEventCard` header now links there ("Leaderboards →"). Seasonal events feature is now complete end-to-end.

**🟢 Group-hosted events** — ✅ SHIPPED as Group Sprints (2026-08-24)  
`seasonal_events.group_id` (migration `2026-08-24-group-sprints.sql`, nullable FK → `chat_rooms`; NULL = platform-wide exactly as before). Any room member hosts via `POST /seasonal/group/:roomId` — metric (checkins/goals_completed/bets_won), target 1–100, 1–30 days, prize 0–500 PP, bounds validated in pure `src/utils/groupSprint.ts` (tested); the host auto-joins. Join/progress/claim/leaderboard reuse the seasonal endpoints untouched since they key on event id, and `getActiveEvents` now filters `group_id IS NULL` so room sprints never leak to global discover. UI: `GroupSprintsCard` in the group header — progress bars, participant counts, join/claim inline, host dialog.

### 3.7 Journal consolidation (one notebook, one name) 🔴

Three surfaces currently claim the "journal" identity, and the word **Notebook** means a different thing on each:

| Route                              | Component      | Data                                                                              | Reached from                           |
| ---------------------------------- | -------------- | --------------------------------------------------------------------------------- | -------------------------------------- |
| `/notebook` (+ `/dashboard` alias) | `NotebookPage` | `notebook_entries` (user journal: notes, waypoints, Axiom briefs)                 | bottom tab **NOTEBOOK**                |
| `/notes`                           | `NotesPage`    | goal tree + `notebook_entries` hybrid (goal cards, per-goal notes, calendar, map) | drawer **Notes**                       |
| `/diary`                           | `DiaryPage`    | `diary_entries` (system log: Axiom captures/scans, words)                         | **nothing** — orphaned, deep-link only |

Proposal (incremental, each step shippable):

1. **✅ SHIPPED (2026-07-03). Kill `/diary` as a surface.** `NotebookPage` gained a "🗂️ Captures" option in the Type filter that reads `diary_entries` via `/diary/entries` (SpeedDial quick-notes + Axiom captures land there, so it's a captures log, not purely system). `/diary` → redirect `/notebook`; `DiaryPage` deleted; `DiaryExportDialog`/`GoalProgressBarChart` moved into `features/notebook/`. Follow-up shipped same day: capture rows are tagged `source_table: 'diary_entries'` and pin/edit/delete route to the mirrored `/diary/entries/:id` endpoints (`NoteEditDialog` gained an `apiBase` prop; its id-prefix strip no longer mangles plain UUIDs).
2. **✅ SHIPPED (2026-07-03). Rename `/notes` drawer entry.** Drawer label "Notes" → "Goals" (🎯); route unchanged for old links. `NodeJournalDrawer`/`GoalNotesPanel` stay — per-goal notes are a goal-tree feature, not a journal.
3. **✅ SHIPPED (2026-07-03, env-gated). One table.** Migration `2026-07-03-captures-into-notebook.sql` adds `notebook_entries.source` (`user`|`capture`), backfill-copies `diary_entries` (idempotent, keeps ids), drops the entry_type CHECK, and creates `get_capture_entries` (mirror of `get_diary_entries` over the unified table). All 9 backend call sites (diaryRoutes CRUD/RPC, exportRoutes, axiomCapture, words, TextAnalysis, both Axiom scans, AxiomSchedule, demoSeed) route through `utils/captures.ts` (`CAPTURE_TABLE`/`CAPTURE_LIST_RPC`/`captureRow`/`captureSelect`, 7 tests). Cutover = run migration + set `CAPTURES_IN_NOTEBOOK=1` on Railway; flag off = byte-identical legacy behavior. Drop `diary_entries` after the unified path proves out (SQL in manual_actions).
4. **✅ SHIPPED (2026-07-03). Naming sweep.** User-facing "Diary" is gone: "Save to Diary" tooltips (Matches/Profile/Events/Places), ShareDialog tab + button, camera/SpeedDial toasts now say Notebook; SearchPage note results navigate to /notebook (was /notes) and the section header reads Notebook. Onboarding tour + i18n were already on the final naming.

### 3.8 Platform & Infra

**🟡 Native iOS companion**  
Swift shell around PWA for notification reliability + Apple Pay + Shortcuts (one-tap check-in from home screen). React Native or Capacitor; Capacitor preferred for code reuse. Full plan: `docs/plans/2026-08-31-ios-companion.md` (stack = Capacitor bundled shell; phased M0–M5; blockers listed — needs macOS/Xcode, no scaffold exists yet).

**✅ Praxis CLI expansion** (DONE 2026-08-29)  
`praxis` binary gained user-facing commands for terminal power users: `login` (Supabase email/password → stores JWT at `~/.praxis/session.json`), `streak` (prints `profile.current_streak` via `GET /gamification/profile`), `log-tracker` (`POST /trackers/log` with `--type` + `--name/--value` or raw `--data` JSON), `export` (pdfkit PDF of the profile). Backed by `src/cli/praxisUserClient.ts`; 7 unit tests for the pure helpers + PDF generation.

**🟢 Self-host pack**  
Docker compose (Postgres + Express + React static) with seeded demo data. Needed for prosumer self-hosters; widens TAM.

**🟢 Webhooks for third-party integrations** — ✅ SHIPPED (2026-08-24)  
`outbound_webhooks` table ((user_id,url) unique, per-hook secret, event subscription array; migration `2026-08-24-outbound-webhooks.sql`, owner RLS). `GET/POST/PATCH/DELETE /settings/webhooks` — the secret is returned exactly once at registration; URLs are masked on read. Deliveries fire-and-forget from the three seeded events (`goal_completed` on peer approval, `streak_broken` from the daily fails cron, `bet_won` in bet resolution): JSON envelope `{event, created_at, data}`, 5s timeout, never throws into the caller, signed with `X-Praxis-Signature` (HMAC-SHA256 over the exact bytes — pure helpers tested). UI: Settings → Outgoing Webhooks card (register/toggle/delete, one-time secret copy). Distinct from the Pro brief webhook (`axiom_webhook_url`), which pushes Axiom briefs to chat platforms rather than structured events to automation tools.

### 3.9 Growth & Virality

**🟡 Shareable widgets**  
Streak badge + goal-progress card as `<iframe>` or raw SVG (`/widget/streak/:userId.svg`). Already have embeddable streak widget per MEMORY; needs polish + open-graph card for preview. ✅ this session: refactored `publicWidgetController` to a shared `buildWidgetData` gate (visibility + escaping in one place); the HTML streak widget now emits Open Graph + Twitter Card meta (`og:image` → the SVG), and `GET /public/widget/:userId.svg` returns a matching raw-SVG image (CORS-open, visibility-gated, 5-min cache). Added a dedicated **goal-progress ring widget**: `GET /public/widget/goal/:userId` (HTML conic-gradient ring) + `/goal/:userId.svg` (SVG stroke ring), both visibility-gated, reusing `buildWidgetData`. Route ordering keeps `/:userId` (dot-greedy) and the `.svg`/`/goal/*` routes from shadowing each other. Tests: `src/__tests__/publicWidget.test.ts` (8).

**🟡 Celebration moments v2** — ✅ SHIPPED (2026-08-24)  
`c22a243` shipped Better Celebrations. This cycle added the generative share card: every milestone modal (streaks, level tiers) now offers **Share Card**, rendering a branded 1200×630 PNG client-side (`client/src/lib/shareCard.ts` — accent aura, hero number, wrapped title, reward chips; spec layer clamps hostile input and is unit-tested). Uses the native share sheet with the image where `navigator.canShare({files})` exists (iOS/Android), downloads the PNG elsewhere, and degrades to a text share when canvas is unavailable. Feed auto-post deliberately deferred: it needs a context choice (which room?) — revisit as "post this card" on the composer's media path.

**🟢 Annual "Praxis Wrapped"**  
Spotify-style year-in-review: biggest goal, total PP, top domain, streak record, sparring partner — as swipeable cards + PDF + shareable. Run mid-December via cron.

### 3.10 Physical Notebook ("light mode") 🔴

The "light mode" for Praxis is **not a flat light UI** — it is an actual notebook page. An
appearance toggle (🌙 Dark ⇄ 📒 Notebook, persisted to `localStorage`, `data-appearance` on
`<html>`) flips the whole app to a paper look: cream sheet, blue ruled lines, a red margin rule,
spiral binding, and the user's own words rendered in a **handwriting font** they pick at
onboarding (editable in Settings). Implemented 2026-08-15 as an MVP:

- **Engine** — `client/src/theme/appearance.tsx` (`AppearanceProvider` + `useAppearance`),
  `client/src/theme/notebookTheme.ts` (paper MUI theme), `[data-appearance="notebook"]` token
  overrides in `client/src/index.css`, handwriting Google fonts in `client/index.html`,
  `--font-handwriting` CSS var driven by `client/src/theme/handwriting.tsx`
  (`HandwritingProvider` + `useHandwriting`). `profiles.handwriting_font` + `profiles.signature`
  (migration `2026-08-15-notebook-appearance.sql`).
- **Component library** — `client/src/components/notebook/`: `PaperSheet` (spiral page),
  `Polaroid` (photo + caption), `PostIt` (sticky note), `Letter` (folded message),
  `Wallet` (bets bifold), `DoodleSlot` (editable margin doodle), `TotalsBars` + `VariationSpark`.
- **Ported metaphors (MVP)** — Notebook page gains a `NotebookExtras` side panel (month
  calendar of check-ins + week plan + totals/variation graphs + doodle slots); feed posts render
  as `PostIt`; avatars render as `Polaroid` via a shared `AvatarPolaroid` primitive (used on the
  profile hero, friends page + dialog, matches, leaderboard and search); betting page shows a
  `Wallet`; group chat messages render as `Letter`. Handwriting captured at onboarding
  (`OnboardingPageV2` step 2) + editable in Settings.
- **1:1 DMs + group rooms ported to paper (2026-08-16)** — `MessageList` (personal chat) and
  `GroupRoom` (group spaces) now render text messages as `Letter`s (sent = wax seal, received =
  handwriting signature) and shared photos as `Polaroid`s in notebook mode; dark mode keeps the
  flat chat bubbles. Chat/message avatars get the paper-photo `MuiAvatar` treatment.
- **Doodles** — `DoodleSlot`s surface which goals are lacking logs (nodes at 0 progress). The
  user said _"I will generate them"_: a periodic job / manual pass should drop generated doodles
  (e.g. SVG sketches) into these slots; for now they are editable placeholders.
- **Small avatars app-wide (2026-08-16)** — every remaining tiny inline avatar (chat threads,
  group walls, challenge rosters, marketplace creators) now gets a paper-photo treatment via a
  `MuiAvatar` override in `notebookTheme.ts` (cream photo, white rim, soft sepia shadow) instead
  of per-site swaps — readable at 24–34 px where a caption polaroid would clutter the row.
  `AvatarPolaroid` (large, handwritten caption) is unaffected.
- **Map as a hand-drawn atlas (2026-08-16)** — `NotebookMap` + `LocationMap` are now
  appearance-aware: notebook mode swaps the dark CARTO tiles for the light `light_all` atlas,
  turns entry markers into tilted cream paper pins with ink rings, styles popups as paper
  notes (ink text on cream), and the zoom/attribution chrome becomes a paper control (`index.css`
  leaflet overrides). Dark mode is pixel-identical.
- **Generated doodles (2026-08-16)** — `DoodleLibrary` (`DoodleSvg`, `kindForLabel`,
  `doodleForGoals`) draws loose hand-sketched SVGs (wobble stroke, no fills) per goal domain.
  `DoodleSlot`s on the notebook margin now auto-fill: "Goals lacking logs" gets the sketch for
  the most common unlogged domain (or a cloud when fully logged), and the margin slot a
  **daily-rotating** sketch (gallery of 12 kinds, e.g. fire, sleep, compass, mountain, music,
  mail, plant, building) — still editable inline so the user can scribble over any sketch.
- **Week planner as a paper spread (2026-08-16)** — the WEEK page renders as an open two-page
  spread in notebook mode: ruled cream sheet with a dashed purple centre fold
  (`::after`), handwritten day headers and block titles, and sticky-tab blocks with a slight
  alternating tilt. Dark mode untouched.
- **Notebook-styled PDF export (2026-08-16)** — the free "full notebook" PDF (`POST /diary/export`)
  accepts `theme: 'notebook'` and renders as a physical notebook page: cream sheets, sepia ink,
  ruled lines and a red margin rule painted on every page (including auto-paginated ones via an
  `addPage` wrapper), while the default stays the branded dark-on-white look. Also fixed the
  export UI drift: the "RAW EXPORT" tier was labelled "Full notebook PDF" but actually downloaded
  JSON/MD client-side and never called the endpoint — it now downloads the real notebook PDF
  (theme-aware, honouring the optional date range). `theme` threads from `useAppearance`.
  Structural tests in `src/__tests__/notebookPdf.test.ts` (valid PDF, both themes, multi-page,
  page-count parity); the paper look itself still deserves an eyeball pass.
- **Notebook map pins actually render (2026-08-16)** — the "Map · N pins" atlas (and the NotesPage
  map) never showed a single pin: geo writes from the log-location flows (TrackerSection,
  QuickLogDialog, NoteGoalDetail, GoalNotesPanel, ShareDialog/ShareButton, UnifiedGoalWidget) POSTed
  `location_lat`/`location_lng`, which `POST /notebook/entries` silently dropped, so zero rows in
  `notebook_entries` carried coordinates. The route now normalises both column schemes through
  `utils/notebookGeo.ts` (`normaliseEntryGeo`, 6 tests) and writes BOTH pairs — canonical
  `latitude/longitude` (read by the NotebookPage map + physis topos) and legacy
  `location_lat/location_lng` (read by the NotesPage NotebookMap); `PATCH` keeps both pairs in sync
  (clearing one clears the other) and `POST /calendar/plan` mirrors the same columns. Client readers
  fall back to either scheme. No DB migration needed — the live `get_notebook_entries` RPC (the
  `p_semantic_entry_ids` overload the route actually calls) already returns
  `latitude/longitude/location_name`.

- **Notebook appearance bug-fix pass (2026-08-16)** — surfaces that stayed dark or broke on
  paper are now appearance-aware: `PraxisCard`/`GlassCard` (the discover/social profile cards
  that never flipped and made profiles unreadable on the light theme), the dashboard physis
  cards (`ConnectionsCard`, `PlacesCard`, `TrainingBalanceCard`, `RhythmsCard`), and
  `NotebookEntryCard`, whose note text was hardcoded white — **white on white** on paper — and
  now renders in ink.
- **Avatar photo hardening (2026-08-16)** — user photos on the social tab (and everywhere
  `AvatarPolaroid` is used) now render as a real `<img>` with a graceful initial fallback
  instead of a silent `background-image`, so a reachable photo always shows and a broken one
  degrades to a labelled blank rather than an empty square. The discover map's user markers
  use the same real-`<img>` approach, and the client mappers accept both `avatarUrl` and
  `avatar_url` to survive backend deploy skew. Verified that stored `avatar_url`s are valid
  public URLs against the live storage.

**Still to port (follow-ups):** the marketplace sections consistently
(offer/bounty cards now flip to paper but chip accents can be tuned); optional: a periodic pass
that records which doodle kinds have been drawn.

---

## Next-Session Plan (written 2026-06-12, end of day)

1. ~~Verify OAuth-loop fix~~ ✅ user-confirmed working (2026-06-12 late).
2. ~~Usage→oracle attestation~~ ✅ SHIPPED (see changelog): `praxis_mobile` provider (trust 0.7 self-device tier, providers now declare `trustWeight`), `usage_samples` table (applied to prod) + `POST /oracle/usage-sample` (today/yesterday only — no backfill), pure `evaluateUsageWindow` (4 tests), detox proof-source picker (RescueTime | this phone), housekeeping + bounties honor it, dashboard native card auto-submits the daily sample. Future hardening: Play Integrity attestation.
3. **APK build + on-device test** (user: Android Studio steps in manual_actions.txt) → then native push via @capacitor/push-notifications.
4. ~~Admin Economy UI~~ ✅ SHIPPED: `EconomySection` in Admin → Stats (faucets/sinks/daily net over `/admin/economy`).
5. ~~**Money-path controller tests** (bounty fund/claim/payout, detox commit)~~ ✅ SHIPPED (2026-06-22): `bountyMoneyPath.test.ts` (24) + `detoxCommit.test.ts` (8), pointsSpend/bettingResolution mock pattern.
6. **God components** — NotebookPage 1164→939 (Analytics + Calendar sections), EditableTrackerForm 998→847 (TrackerRowFields) ✅ first slices SHIPPED 2026-06-22; deeper splits still open.
7. Standing user actions: GitHub Actions billing, Strava API app env, optional demo-user leaderboard re-listing.

## Improvement Roadmap (2026-06-12 brainstorm)

Tiered; sequence chosen by leverage/effort. Status updated as slices ship.

- **T1.1 Done-for-today terminal state** — ✅ v1 SHIPPED (this session): after check-in the dashboard feed column defaults to a "you're done here, go live" card; feed one explicit tap away. Deeper version later: collapse quests/widgets too, show session-time trend going down.
- **T1.2 Streak-at-risk push** — already live (`streakPushCron`, 20:00 UTC); backlog entry was stale.
- **T1.3 Oracle housekeeping cron** — ✅ SHIPPED (this session, see changelog).
- **T1.4 Auto-verify (Pro lever)** — ✅ shipped inside T1.3 for all users; optional future: Pro-only gating.
- **T2.5 Praxis CV** — ✅ v1 SHIPPED (this session): public `/cv/:username` (shell-skipped) renders the verified-achievement résumé from new public `GET /credentials/cv/:username` (profile basics + stats + credential list, each linking `/verify/:id`); "Praxis CV →" link on the profile proof wall; Web Share button. **OG card SHIPPED (2026-08-24)**: `GET /cv/:username` is now served ahead of the SPA catch-all — social crawlers (`isSocialCrawler`: facebookexternalhit/linkedin/twitter/slack/discord/whatsapp/telegram…) get a server-rendered OG-tagged summary page (`utils/cvOgPage.ts`, escaping tested), every other agent falls through to the React route untouched.
- **T2.6 Strava oracle** — ✅ code SHIPPED (this session), env-gated: `strava` provider (OAuth-only — 6h tokens, rotating refresh persisted in new `oracle_connections.refresh_token`, 401→refresh→retry in verify), params `{min, sinceDays, sport, minDistanceKm, startDate}`; OAuth routes `/oracle/strava/auth|callback` mirror GitHub; Settings card section (hidden until configured); goal-dialog auto-verify button; bounty provider option with sport/min-km fields. Tests: `stravaOracle.test.ts` (6). **User action to go live**: register Strava API app + set `STRAVA_CLIENT_ID/_SECRET/_REDIRECT_URI` on Railway (see manual_actions.txt).
- **T2.7 RescueTime mobile honesty** — ✅ SHIPPED (this session): detox dialog now states the iOS limitation (desktop+Android strong, iPhone weak) + mentions nightly auto-verify. Real fix remains native Screen Time via Capacitor (§3.7).
- **T3.8 Sparring partnerships** — ✅ SHIPPED (this session): `partnerships` table (one non-ended per user), invite-by-username → accept → joint streak that advances only when BOTH check in same day (hooked fire-and-forget into `checkinController`; conditional update guards the simultaneous-check-in race, NULL-safe), missed day resets, +5 PP each/day (`sparring_bonus`, ×2 every 7th) so pairing strictly beats solo. `PartnershipService` (named to avoid collision with the older per-goal `sparring_requests` feature), `/partnerships` routes, `SparringCard` on the dashboard sidebar (invite/pending/active states). ✅ this session: `SparringCard` active state gained an "Open thread" button (routes to the existing `/chat/:me/:partner` thread) plus a partner-individual-streak chip, closing the old "threads UI only shows pending badges" gap. Migration applied to prod.
- **T3.9 Rank opt-in default** — ✅ SHIPPED: `profiles.leaderboard_opt_in` (default false), `getLeaderboard` filters to opt-ins (legacy fallback while unmigrated), Settings → Privacy "Appear on Leaderboard" toggle (client-side supabase update). Everyone is off the board until they choose comparison.
- **T3.10 Feed cap** — ✅ SHIPPED: `PostFeed` hard-caps at 25 posts with a terminal "That's everything from your people — no infinite scroll here" card.
- **T4 debt** — PP economy report ✅ backend SHIPPED: pure `buildEconomyReport` (`src/utils/economyReport.ts`, tested) + admin `GET /admin/economy?days=N` (pages pp_ledger, faucets/sinks/daily-net). ✅ admin UI SHIPPED: `EconomyTab` (totals, faucet/sink bars, daily-net sparkline, 7/30/90d selector) wired into `AdminPage` as the "Economy" tab. While there, fixed an off-by-one in `AdminPage` tab panels (duplicate `index={4}` had shifted Challenges→Debug onto the wrong panels). ✅ controller-level money-path tests (bounty/detox flows) SHIPPED 2026-06-22; ✅ god-component refactors first slices SHIPPED 2026-06-22 (NotebookPage −225, EditableTrackerForm −151) — deeper splits still open.

## Local Self-Hosting & Launch Hardening (2026-08-09/10)

The app moved off Vercel + Railway + hosted Supabase onto a single machine ("pansa"). This section is the authoritative description of that topology and of the launch-blocking work around it.

### Topology

- Cloudflare Tunnel `praxis-tunnel` (`087b548c-1357-429d-8a53-0c085de22831`) terminates TLS and routes:
  - `praxisweb.xyz` → `localhost:3001` (Express, serving `client/dist` via the SPA catch-all)
  - `supabase.praxisweb.xyz` → `127.0.0.1:54321` (local Supabase, Docker containers `supabase_*_praxis_webapp`)
- systemd **user** units: `praxis-backend.service`, `cloudflared-praxis.service`
- `loadMasterEnv()` (`src/app.ts:4`) loads `/home/gio/.env` _before_ `dotenv`, which is where the ~19 keyed LLM providers live. The project `.env` alone is not a complete picture of what Axiom can reach.
- Migrations run against local Postgres now; there is no hosted Supabase SQL editor step any more.

### Security fixes (all shipped)

- **Stub oracle auto-approve** (`src/services/OracleService.ts`) — `isConfigured` was `ALLOW_STUB_ORACLE === 'true' || NODE_ENV !== 'production'`, so any deploy that had not explicitly set `NODE_ENV=production` auto-approved every goal on the same code path `BountyService` uses to settle money. Now `ALLOW_STUB_ORACLE === 'true'` only. **The code fix alone was inert** — `.env` had `ALLOW_STUB_ORACLE=true` set explicitly; it is now `false`. Audit of live data found no exploitation: `credentials` 0, `oracle_connections` 0, `bounties`/`bounty_claims`/`open_bets` 0, `bets` 12 (all `is_real_money=false`, no `stripe_payment_intent_id`).
- **Stack traces on 404** (`src/middleware/errorHandler.ts`) — stacks now only ship on 5xx outside production.
- **Service worker breaking auth** (`client/public/service-worker.js`) — it intercepted _cross-origin_ requests, so every Supabase auth/REST call to `supabase.praxisweb.xyz` went through the cache layer. Now returns early unless `url.origin === self.location.origin`; caches bumped to `praxis-cache-v2` / `praxis-api-cache-v2`.
- **Anonymous LLM relays** — 17 bare-IP unauthenticated Ollama endpoints over plain HTTP were reachable in the provider cascade. Deleted `ollama1`–`ollama4` outright; the remaining anonymous relays now require `AXIOM_ALLOW_ANON_PROXIES=true` and log as disabled at startup.
- **Database dumps must never enter the repo.** `.gitignore` now covers `backups/`, `*.dump.sql`, `praxis-remote-*.sql`. Dumps belong outside the repo tree.

### Landing page + funnel

- `client/src/features/home/sections/` — `LandingNav`, `Hero`, `DailyLoop`, `FeatureGrid` (12 tiles, all features), `AgentAccess` (MCP), `PricingSection` (imports `TIERS` from `features/payments/premiumTiers` so pricing cannot drift), `FaqSection`, `ClosingCta`, `LandingFooter`. `TestimonialStrip` exists but is **deliberately not rendered** — its quotes are fabricated and the site takes payments.
- `client/index.html` — canonical/OG/Twitter/JSON-LD meta; eruda gated behind `?debug=1`; the cache-reset routine now preserves `sb-*-auth-token` keys instead of logging everyone out.
- `client/src/i18n.ts` — English default (`nonExplicitSupportedLngs`, detection order querystring → localStorage; no navigator sniffing).
- Funnel: `client/src/lib/funnel.ts` (+ `lib/apiBase.ts` to break a circular import), public `POST /api/funnel` (`src/routes/funnelRoutes.ts`, allowlisted events, always 204), `migrations/2026-08-09-funnel-events.sql` applied. `first_checkin` fires from the axios response interceptor — one instrumentation point instead of ~9 call sites.

### Axiom model tiering (Hermes-style)

- `src/services/llm/LlmProviderRegistry.ts` — providers sorted into tiers: keyed direct (0), `hermes` gateway (1), proxies (2), anonymous relays (3), local/`lfm2*` (4). `providersForAudience(providers, audience)` gives Pro the whole cascade and **free users local models only** (falling back to the full list if no local provider is configured).
- `src/services/llm/LlmAudienceResolver.ts` — 5-minute TTL cache over `profiles.is_premium/is_admin`, fails closed to `'free'`.
- `AICoachingService.runWithFallback(prompt, { userId })`; cache key is audience-scoped (`[${audience}] ${prompt}`) so a Pro answer is never served to a free user. `userId` threaded through the daily-summary / schedule / morning-brief / wiki-writer / wiki-generator / monthly-summary / brief-pool services and `CoachingContext.userId`. Progress-estimation and scan stay on `'system'` (local model).
- `HERMES_GATEWAY` provider added at `localhost:8642/v1`, keyed on `HERMES_API_KEY`; `callOpenAICompat` sends `X-Hermes-Session-Key: praxis:${userId}` so Hermes' memory system keys per user.
- Local inference: Ollama on `:11434` with `lfm2.5:latest` (2.69B, 131K context, ~113 tok/s on the Ryzen 5 5600X) and `gemma4:e2b`. LFM Open License v1.0 — free commercially under $10M revenue.

### Goal-keyed retrieval

`src/services/axiom/GoalContextService.ts` replaced the flat "here are your goals" bullet list with a budgeted pack: **ANCHOR / TRAJECTORY / EVIDENCE / LEVERS / SOCIAL** (700/300/200 chars per goal, max 3 goals, max 4 evidence items). Scoring is `goal_id` link +10, same domain +4, saturating lexical `3·hits/(hits+1.5)`, recency as a _multiplier_ `max(0.35, 0.5^(age/21))` rather than a term — so relevance beats recency instead of being drowned by it. Two bugs found and fixed during development, both worth remembering:

1. Windowing candidates to 60 days in SQL dropped 2851 of 2857 entries on the densest account and returned empty packs. Fixed with a 3-pass fetch (recent 30d / linked at any age / same-domain 365d), deduped.
2. A single lexical hit qualified an entry — the goal "weed" matched a website-launch note that mentioned it in passing. Fixed with a structural gate: `linked || (domainMatched && hits >= 1) || hits >= 2`.

Axiom-authored entry types (`axiom_brief`, `axiom_monthly_summary`, 174 rows) are excluded from evidence — otherwise Axiom cites itself as proof.

**Domain vocabulary drift**: exactly **1 of ~2800** notebook entries carried a domain string matching a canonical `Domain` value verbatim ("Craft & Career" vs "Career & Craft", "Money & Assets" vs "Wealth & Assets", …), which had silently disabled every domain comparison in the codebase. `src/models/domainNormalize.ts` resolves exact → token-set → alias and returns `null` rather than guessing; `sameDomain` never matches two unmappables. 10 tests pinned to real live values. Read-time normalization ships; normalizing the stored data is still open.

### Embeddings — the whole vector layer was dead (fixed 2026-08-10)

`text-embedding-004` was retired by Google and returns 404, so every embedding call had been failing silently. Live counts before the fix: `notebook_entries.embedding` **0 of 3049**, `profile_embeddings` **0 rows**, `goal_embeddings` **0 rows**.

The consequence is wider than the empty tables suggest. `match_users_by_goals` is a pgvector cosine query over `goal_embeddings` and is read by **eight** call sites — `matchingController`, `bettingController`, `axiomAgentController` (×2), `AxiomScanService`, `AxiomActionExecutor`, `AxiomUnifiedScanService`, `AxiomScheduleService`. Every one returned an empty list and logged nothing, so a dead feature looked exactly like "no matches found". Notebook rhymes and the text-affinity half of matching were out the same way.

- `TextAnalysisService` → `gemini-embedding-001` with `outputDimensionality: 768` (the width `004` produced and every `vector(768)` column expects), width-validated before storing, unit-normalized. **Changing the dimension requires migrating the columns _and_ re-embedding everything** — pgvector will compute a meaningless distance between two models' vectors rather than error.
- It also reads `GEMINI_API_KEY`, `AI_GEMINI_KEY`, `PRAXIS_GEMINI_KEY` and `GOOGLE_API_KEY` (22 distinct keys on this host, deduped, random start offset). Widening the pool got the backfill from 177 to 799 entries — and then exposed why the hosted path is a dead end (below).

**Then the hosted path turned out not to exist.** Probing all 22 keys directly returned **403 "Your API key was reported as leaked" on 20 of them**, 400 "API key not valid" on one, and 429 quota-exceeded on the single survivor. Google has revoked almost the entire pool. This is not only an embedding problem: `AICoachingService` logs "Gemini: 22 keys" at startup, which reads as depth it does not have — the Gemini leg of the Axiom cascade is one rate-limited key. **Every key in `/home/gio/.env` should be treated as compromised** and the leak surface found, or the replacements go the same way.

So embedding moved local: `EMBEDDING_PROVIDER` (default `ollama`), `OLLAMA_BASE_URL`, `OLLAMA_EMBEDDING_MODEL` (default `nomic-embed-text`, **768-dimensional natively** — exactly the schema's width). No quota, no key, and notebook text stops leaving the host. `migrations/2026-08-10-reembed-local-vectors.sql` clears the 976 Gemini vectors first: pgvector returns a confident, meaningless cosine distance between two models' spaces rather than an error, so a half-migrated table is worse than an empty one. **That migration and the two backfills are pending — they need running by hand** (a bulk write against the live database was blocked by a safety guard, correctly).

- `src/services/GoalEmbeddingService.ts` — diff sync of `goal_trees.nodes` → `goal_embeddings`: unchanged nodes cost no API call, renamed ones re-embed, deleted ones have their rows removed, anything over 30 days refreshes anyway (a `customDetails` edit is invisible to a name/domain comparison).
- `src/services/embeddingRefreshCron.ts` — nightly 04:30 UTC, goal + profile passes, started from `src/index.ts`. The tables emptied out because nothing but a manual backfill ever filled them.
- `scripts/backfill-goal-embeddings.ts` — `[--limit N] [--user UUID] [--force]`.

### Hermes gateway (enabled 2026-08-10)

Running on `127.0.0.1:8642`; `HERMES_GATEWAY` is now live in the Axiom cascade at tier 1 (Pro only — free users are pinned to local models). Verified end-to-end: an authenticated `POST /v1/chat/completions` returns a completion, an unauthenticated one returns 401.

**The toolset is pinned deliberately.** The Hermes API server dispatches the full agent, whose toolsets include terminal, file, `code_execution` and browser. Praxis prompts carry user-authored notebook text straight into it, so anything enabled there is reachable by prompt injection from any signed-up user. `platform_toolsets.api_server` is set to `[todo]` — a side-effect-free scratchpad. An **empty** list does not work: the resolver does not treat `[]` as explicit configuration, falls through to its default branch and auto-enables `x_search` because xAI credentials exist on this host. Re-check `GET /v1/toolsets` after ever editing that line.

Note the cost shape: a gateway call carries ~6.5K prompt tokens of Hermes system prompt and memory context before the Praxis prompt is even added. That is the per-user tailoring, and it is why the provider sits behind the Pro gate.

### Open items

- **Terms of Service + Privacy Policy** — SHIPPED at `/terms` and `/privacy` (`client/src/features/legal/`), linked from the landing footer and the signup form, lazy-loaded. **Three placeholders still need filling before this is live: `[OPERATOR LEGAL NAME]`, `[CONTACT EMAIL]`, `[POSTAL ADDRESS]`** — they are listed in a comment at the top of each page, and each page carries a visible notice that it is an unreviewed template. Signup age minimum moved 13 → 16 to match. Delegation to opencode was abandoned after three runs (~75 minutes) produced nothing; see the routing note below.
- **Browser login verification end-to-end** — not reproducible from the agent sandbox (navigation to `supabase.praxisweb.xyz` returns 200 but `fetch()` fails there even with service workers disabled), so this needs a human on a real browser.
- **`notebook_entries` backfill** — running; 3049 entries at roughly 120/minute. Rhyme detection stays partial until it finishes.
- Optional: ask GitHub Support to GC unreachable dump blobs (the dumps themselves now live outside the repo at `/home/gio/praxis-backups`, mode 700); normalize stored domain values; flip `NODE_ENV=development` off the public host (safe now that the oracle no longer keys off it).

### Note on delegated model routing

`grok`, `kilo`, `nex-agi` and the paid DeepSeek endpoints are all reachable through the local `opencode` CLI but currently return credit errors (`Add credits to continue`, `Insufficient Balance`, `requires more credits`). Only the free tier answers — `opencode/deepseek-v4-flash-free`, `opencode/nemotron-3-ultra-free` and the other `opencode/*-free` models. Claude Code's own `Agent` tool cannot route to any of them; it only takes `sonnet | opus | haiku | fable`. Anything delegated to those models has to go through `opencode run -m <provider/model>`.

Two things to know before delegating there again:

- **`opencode run` defaults to `agent=plan`, which is read-only.** The first delegated run spent 15 minutes explaining it could not write files. Pass `--agent build`.
- **The free models did not complete a real task.** A single-file write probe succeeded in about 30 seconds. Three attempts at the legal pages — 30, 30 and 15 minutes, `--agent build --auto`, the third narrowed to "write two Markdown files, touch nothing else, no typecheck" — each produced no files and no assistant output at all. Budget for that failure mode, or put credits on one of the paid providers.

## Active Roadmap (2026-08-25 — approved order)

Factory cycles, each shipped independently behind tests:

- [x] **A. Weekly review ritual** — Sat 18:00 UTC cron aggregates the week's block outcomes, thumbs/notes and retro threads → LLM distills headline/wins/frictions + ≤3 review questions → vault page `patterns/<week-start>.md` + notebook mirror; next Sunday's plan prompt seeds itself with the review. (`WeeklyReviewService`)
- [x] **B. Web push** — VAPID configured; brief-ready / question-waiting / streak nudges wired (860b3e4). Client subscribe UI pre-existed.
- [x] **C. Voice check-ins** — Web Speech dictation prefills the note dialog (80bcbc4).
- [x] **D. ESLint flat config** — backend+client configs, lint gates in CI (f2b4f18).
- [x] **E. Retroprompt upsell** — free weekly sample + /upgrade CTA on the card (50769a4).
- [x] **F. Automated backups** — nightly pg_dump + vault tar, 14-day retention, user timer live (verified restorable).
- [x] **G. Brief recap resilience** — 72h fallback window so recap/followup populate for sporadic users.
- [x] **H. Notebook render performance** — derivations memoized, feed capped at 10 days with show-more (b2671af).
- [x] **I. Comprehensive App Optimization Pass**
  - **1. Frontend bundle trimming**: Lazy-load `DesktopWidget` & `MobileWidget` in `client/src/config/routes.tsx` to remove unused widgets from the critical landing/auth path.
  - **2. Chunk isolation**: Add `cytoscape` to `manualChunks` in `client/vite.config.ts` so graph rendering loads strictly on demand.
  - **3. HTTP API response compression**: Add GZIP/Deflate `compression` middleware to Express in `src/app.ts` to cut JSON payloads by 70–85%.
  - **4. Postgres composite performance indexes**: Add `migrations/2026-08-29-perf-indexes.sql` targeting `planner_blocks`, `notebook_entries`, `axiom_daily_briefs`, and `goals`.
  - **5. Native Android WorkManager battery constraints**: Ensure `WidgetRefreshWorker` sets `setRequiresBatteryNotLow(true)` to prevent background wake-lock drain.
- [x] **J. Subconscious "Shadow" Audit (30-Day Behavioral Synthesis)**
  - **1. Behavioral analysis engine**: Cross-references declared goal priorities against actual logged entries and planner blocks over 30 days to measure Avoidance Index, Alignment Fidelity, and Productive Procrastination rate.
  - **2. Archetype detection**: Classifies deflection patterns (`The Eternal Architect`, `The Comfort Sprinter`, `The Solitary Overthinker`, `The Friction Evader`, `The Aligned Sovereign`).
  - **3. Endpoint & UI Integration**: Added `GET /axiom/shadow-audit` and interactive `<ShadowAuditCard />` on `/analytics` rendering 4-part Axiom glass synthesis (Observation, Undercurrent, Antidote, Reflection) with collapsible domain divergence breakdown.

## Active Roadmap (2026-08-30 — notebook, /me, /helloworld)

Requested 2026-08-30. `/helloworld` is built and shipped in the same commit as
this entry; everything else below is approved and specified, not written.

Read the notebook items together: 1–5 are one coherent change to how
`/notebook` is navigated, and doing 4 without 1–3 produces a sidebar competing
with a layout that has no room for it.

### N1. The by-hour schedule scrolls horizontally — [ ]

`NotebookPage.tsx`'s selected-day panel currently stacks the day's clock
vertically. On a phone that is a long scroll for a day with four entries.

Make the hour track a horizontal scroller, matching the pattern `/planner`
already uses (the today-timeline shipped 2026-08-25 — reuse it rather than
writing a second one). Hours with nothing in them collapse to a narrow tick so
an empty afternoon costs a thumb-flick, not a screenful.

- Snap-scroll per hour (`scroll-snap-type: x mandatory`), so it lands on hour
  boundaries rather than mid-block.
- The current hour is the initial scroll position, not 00:00.
- **Wide content must scroll inside its own container**, never the page body.

### N2. Map and calendar side by side on phone — [ ]

Today they stack on `xs`. Put them side by side at every width, splitting the
viewport, with the map given the smaller share on narrow screens.

The real constraint is that both are tall widgets; side-by-side on a phone
means each gets ~45vw. That is only usable in combination with N3 — decide the
two together, and do not ship N2 alone.

### N3. Tap to zoom — [ ]

Tapping either the map or the calendar expands it to fill the viewport;
tapping again (or a close affordance) returns it to the split. This is what
makes N2 usable: the split is for orientation, the zoom is for work.

- Zoom state belongs in the URL (`?focus=map`), so the back button leaves the
  zoom rather than the page. A phone user's first instinct after zooming is
  the back gesture.
- The map must invalidate its size on transition or Leaflet renders grey tiles
  into the new box.

### N4. Per-goal chapter markers — [ ]

A fixed rail down the side of `/notebook`, one marker per root goal, that jumps
to that goal's material. Each marker opens to three destinations:

| Destination    | Source                                                        |
| -------------- | ------------------------------------------------------------- |
| Free notes     | `notebook_entries` with no `goal_node_id`                     |
| Per-goal notes | `notebook_entries` filtered to that goal's subtree            |
| Tracker data   | the 13 structured `tracker_*` tables, filtered by goal domain |

The join is the open question. `notebook_entries` has no goal column today —
adding `goal_node_id UUID` is the direct route, but tracker rows relate to
goals only through their **domain**, and that mapping is many-to-many
(`FITNESS` feeds several goals). Resolve before building: either a
`goal_node_id` on the tracker tables too, or an explicit
`goal_tracker_links` join table. Do not infer the link from the domain at read
time — that is the "five domain lists" mistake in a new place.

**Use the dictionary.** Any domain touched here goes through
`resolveDomain()`; never hardcode a list (CLAUDE.md → Domains).

### N5. Sort and categorise data per goal — [ ]

The read side of N4. Given a goal, return its notes, entries and tracker rows
in one shape, sortable by time / domain / tracker type. Belongs in a service
(`NotebookGoalIndexService`), not in the page, so `/analytics` and Axiom can
use the same grouping.

Deterministic — this is filtering and sorting, no LLM.

### N6. Share into the notebook from Android internal storage — [ ]

Register the app as a share target for images, text and PDFs so "Share →
Praxis" from a file manager creates a notebook entry with the file attached.

`docs/DEV_PLAN.md` already records a share-to path (2026-08-24, "notebook
share-to"); check what landed before writing new code. What is missing is the
**internal storage** case — `content://` URIs from the system file picker,
which need `takePersistableUriPermission` or the URI is dead by the time the
upload runs.

Uploads go to self-hosted storage. **The storage API rejects the
`sb_secret_*` key** and needs the legacy JWT (CLAUDE.md → Architecture); a new
upload path that skips that will 403 in production while passing locally.

### M1. Real bio, education and employment on /me — [x] (2026-08-30)

Shipped with this entry. The Curriculum section had been generated filler —
it claimed a corpus spanning "Rust, Python, Go, and Rust ecosystems" (Rust
twice) and named no school, employer or date, on a page linked from outreach
and from a GitHub profile.

Replaced with the real record: maturità scientifica (Liceo Cavanis, Possagno,
2018, 80/100), ITS Meccatronico Veneto diploma (2021, 73/100), technical
draughtsman 2021–2024, philosophy student and self-employed 3D-printing and
repair work 2025–present.

Italian credential names are kept as proper nouns with an English gloss —
"maturità scientifica" names a specific Italian qualification and translating
it would describe a different thing.

### M2. Retroprompt the operator when /me goes stale — [ ]

`/me` is hand-written and drifts silently; the fabricated CV above sat public
for an unknown period. Add a dated `reviewedAt` constant beside the CV data
and surface a reminder when it is more than ~6 months old.

Deliberately **not** an LLM: an agent that rewrites a CV on its own is exactly
how the fabricated version happened. The prompt asks the operator; it never
fills the answer in.

### B1. /helloworld — public single-author blog — [x] (2026-08-30)

Shipped. Everyone reads, one account writes, nobody comments.

| Piece    | Where                                                           |
| -------- | --------------------------------------------------------------- |
| Schema   | `migrations/2026-08-30-helloworld-blog.sql` (`blog_posts`)      |
| API      | `src/controllers/blogController.ts`, `src/routes/blogRoutes.ts` |
| Page     | `client/src/features/blog/HelloWorldPage.tsx`                   |
| Renderer | `client/src/features/blog/markdown.tsx`                         |
| Tests    | `src/__tests__/blogController.test.ts` (12)                     |

Three decisions worth keeping:

- **Authorship is `BLOG_AUTHOR_USER_ID`, not `is_admin`.** Production has more
  than one admin account, so gating on the role would have handed a stranger
  posting rights. Unset env → nobody can post, never everybody.
- **No comments table and no endpoint that accepts reader text.** "Nobody can
  comment" is the absence of a place to put it, not a disabled button, so
  adding comments later takes a migration rather than a config flag.
- **Markdown renders to React nodes, never HTML.** No sanitizer is needed
  because nothing is ever parsed as markup — a `<script>` in a post body is a
  text node whatever it says.

Open: an RSS feed, and `/helloworld` is not linked from any nav yet.

## Change Log

- **2026-08-29 (shadow audit synthesis)** — **Subconscious "Shadow" Audit: 30-Day Behavioral Synthesis, Avoidance Index & Deflection Archetype Engine.**
  1. **Behavioral Synthesis Engine (`ShadowAuditService.ts`)**: Pure deterministic arithmetic computing declared vs. lived domain distribution divergence, Avoidance Index (% of high-weight commitments neglected >14 days), Productive Procrastination ratio (ratio of planning/setup vs direct execution), and primary bottleneck identification, backed by structured 4-part Axiom synthesis (Observation, Undercurrent, Antidote, Reflection).
  2. **API Endpoint (`GET /axiom/shadow-audit`)**: Added authenticated endpoint in `src/routes/axiomRoutes.ts` serving 30-day behavioral metrics with robust LLM-assisted or deterministic fallback synthesis.
  3. **Analytics Visualization (`ShadowAuditCard.tsx`)**: Created interactive dark-glass UI card in `AnalyticsPage.tsx` with alignment/avoidance gauges, bottleneck warning banner, 4-section color-coded cards, and collapsible domain divergence table. Covered by 4 backend jest tests and 2 client vitest tests.

- **2026-08-29 (performance optimization pass)** — **Full-Stack App Optimizations: Bundle Trimming, GZIP Compression, Database Indexes & Mobile Battery Guard.**
  1. **Frontend Bundle Splitting**: Lazy-loaded `DesktopWidget` and `MobileWidget` in `client/src/config/routes.tsx`, trimming critical-path JS from the initial shell entry; isolated `cytoscape` (433 KB) into a dedicated on-demand manual chunk in `client/vite.config.ts`.
  2. **API Response Compression**: Integrated `compression` middleware in `src/app.ts` with 1KB threshold, compressing JSON payloads (goal trees, timeline histories, analytics) by 70–85% across all endpoints.
  3. **Database Composite Indexes**: Added `migrations/2026-08-29-perf-indexes.sql` creating composite indexes on `planner_blocks(user_id, scheduled_at)`, `notebook_entries(user_id, created_at DESC)`, `axiom_daily_briefs(user_id, date DESC)`, `goals(user_id, status)`, and `friendships(sender_id, receiver_id, status)`.
  4. **Android Battery Optimization**: Configured `WidgetRefreshWorker` and `SyncScheduler` with `setRequiresBatteryNotLow(true)` constraints to eliminate background wake-lock battery drain on low battery.

- **2026-08-29 (open source foundation)** — **Open-Source Readiness, Root Docker Compose, Local AI & Self-Hosting Kit.**
  1. **Self-Hosting Quickstart**: Added root `docker-compose.yml` orchestrating PostgreSQL (with auto-migrated schema from `/docker-entrypoint-initdb.d`), production Node.js Express backend (`Dockerfile`), and React PWA served via Nginx with SPA routing proxy (`Dockerfile.client`).
  2. **100% Private Local AI Profile**: Integrated Ollama service (`--profile ai`) in `docker-compose.yml` allowing users to run local LLMs (e.g. `llama3.2:3b`, `mistral:7b`) for offline Axiom daily briefs with zero cloud dependencies or external API keys.
  3. **Comprehensive Self-Hosting & Publishing Guides**: Created `docs/SELF_HOSTING.md` (covering 1-click Docker, local AI setup, bare-metal deployment, Android app connection, and backup/restore workflows), `.env.example` (sanitized environment template), and `docs/PLAY_STORE_PUBLISHING_GUIDE.md` (complete Google Play Console metadata, Data Safety questionnaire answers, and Health Connect permissions breakdown). Transferred all store assets, icons, feature banners, and docs to `feather` for AAB publishing.

- **2026-08-29 (later)** — **Unified Daily & Weekly Timetable, Concise Brief Messages, SpeedDial Action Reordering.**
  1. **Unified Timetable Generation & Visualization**: Unified the daily brief timetable and weekly planner timetable into a single shared service layer (`planner_blocks`, `WeeklyPlanGeneratorService`, `plannerController` with `GET /planner/day` / `GET /planner/today`) and a single shared frontend visualization (`UnifiedTimetable.tsx`). `AxiomScanService` now directly materializes generated routine blocks into `planner_blocks` on brief generation. The legacy 17-item localStorage-bound checklist in `AxiomMorningBrief` and the static dummy block mock `DayPlanCard` are replaced with `UnifiedTimetable`, providing real status updates (`done`/`skipped`/`planned`), live adherence progress, and direct feedback synchronization.
  2. **Concise, Goal/Health-Focused Daily Brief Messages**: Updated `AxiomScanService`, `AxiomBriefPoolService`, and `metricBrief` prompts and templates so that morning brief messages are ultra-concise (1-2 sentences, strictly under 35 words) and razor-focused on the user's active goals and physical/mental health and energy state (recovery, workouts, nutrition, sleep, focus), eliminating generic fluff and pleasantries.
  3. **SpeedDial Actions Order**: Moved BET below CHAT (`LOG` → `PLAN` → `POST` → `CHAT` → `BET`) in `SpeedDial.tsx`.

- **2026-08-29** — **Standardized structured Markdown output across Axiom.** Axiom's output across all endpoints (coaching chat, Ask Axiom dialog, notebook queries, agent responses, fallback templates) now consistently emits four structured Markdown sections: `### 👁️ Observation` (grounded in actual goals, streak, and logs), `### ⚡ The Undercurrent` (psychological and strategic depth, avoidance vs true will), `### 🎯 Next Action` (single high-leverage step <25 words with concrete metrics/duration), and `### 💬 To Sit With` (reflective question to linger on). The frontend introduces `AxiomMarkdown.tsx` which parses and renders these sections into visually distinct, color-coded glass cards with accent icons, full inline formatting (bold, italic, code, quotes), and bullet/numbered lists. Integrated across `AxiomDialog`, `AxiomQueryDialog`, `MessageList`, `AxiomMorningBrief`, and `AxiomNoteCard`. Unit tested with 192 frontend and 1004 backend tests passing.

- **2026-08-28 (later)** — **Matching honours `match_visibility`.** `getMatchesForUser` now enforces the Settings → Privacy → "Profile Visibility" choice: candidates with `match_visibility = 'friends'` are returned only to callers who are accepted friends (via the `friendships` table); `'all'` (the default) and any unset/unknown value stay visible (fail-open on bad data). The gate is a pure, unit-tested helper `src/utils/matchVisibility.ts` (`isVisibleForMatching`) so the rule is isolated from the controller and the caller's own friendship lookup fails closed — friends-only candidates are hidden rather than leaked if that query errors. No migration (both columns/tables already exist). Tests: `src/__tests__/matchVisibility.test.ts` (6). The other two visibility fields are already handled: `share_reflections` is a write-time training-consent opt-in enforced in `reflectionTrainingController.submitReflection` (the only reflections read, `GET /reflections/patterns`, returns anonymous aggregates, so nothing personal to gate); `is_public` ("Make Profile Public") is **already read-gated on the embedded streak widget** — `publicWidgetController.getWidget`/`getWidgetData` return 404 when `is_public === false` (confirmed 2026-08-28). The Praxis CV is a deliberately shareable résumé and is intentionally not gated by `is_public`. **The settings visibility pass is now complete** (matching gated this loop; the other two pre-existing).

- **2026-08-27** — **The planner learns, and the daily plan moved into the week.** Three "daily planners" existed: `brief.routine` (17 LLM slots rendered as a read-only strip on the dashboard), `AxiomScheduleService` time slots (`/api/schedule/*`, zero client callers), and `PlannerPage`'s today strip — and only the last recorded outcomes, so only the last could be learned from. `POST /planner/adopt-routine` turns the brief's routine into real `planner_blocks` (`source='routine'`), and the dashboard strip became a four-chip preview plus **Add to my week**. A plan with no tick box generates no evidence; that is why nothing downstream had ever found out which parts of the routine worked.

  **`RoutineLearningService`** is the measurement layer under both planners: pure arithmetic folding blocks + interaction events into one verdict per recurring routine — `working` / `fading` / `stale` / `refused` / `new` — with the evidence stated in counts and a deterministic fix (move to the hour it actually happens at, shorten to the length actually finished, halve the occurrences, or stop). `migrations/2026-08-27-planner-learning.sql` adds `planner_block_events`, because an edit used to be a pure overwrite and a delete removed the row: "I move Reading to 21:00 every single time" and "I deleted this four times" were the two clearest signals a user can give and the only two that left no trace. `renderLedgerForPrompt` states verdicts as settled fact to `WeeklyPlanAiService` and `AxiomScanService`, and `applyLedger` **enforces** refusals by dropping them from proposals in both the AI and the deterministic generator — re-proposing something the user explicitly deleted is the most corrosive thing this feature can do, so it does not depend on the model complying. Ledger surfaces to the user as a "What's working" panel on `/planner` and `routineChanges` cards on the dashboard.

  **`NotebookSignalService`** makes notebook relevance measured rather than hoped for. The brief already pasted 50 entries in and asked the model to "MAKE IT OBVIOUS you read their notes" — nothing checked whether it did, and the algorithmic fallback had no notebook content at all. Threads are now counted (term appearing in N distinct entries, per-entry set so one long rant cannot outrank a six-week thread; Axiom's own `axiom_brief`/`planner_feedback` entries excluded so it does not learn from its own output), named in the prompt as a mandate, verified afterwards with `verifyReference`, and backed by `deterministicAnchor` — a notebook line built with no model, so the brief is notebook-anchored on a truncated response and on the fallback path too.

  Match/event/place stay **rule-based picks** per the AI policy; what changed is that their `reason` was a hardcoded constant (every user was told their match had "Aligned goals in your active domains"), and the model now writes that one sentence about the already-chosen candidate. Bug found on the way: `GET /planner` never selected `feedback`/`feedback_note`, so every thumbs vote came back null on reload — a saved vote looked unsaved, and users re-voted. 33 new unit tests; 975 backend + 184 frontend green.

- **2026-08-25** — **Planner today-first + delivery war room.** `/planner` opens on a horizontal today-timeline (tap cards, full button row after the first version read as "buttons vanished"); the 7-day spread hides behind a "Whole week" chevron. The notebook calendar's selected-day panel became one clock: planner blocks ⏰/✅, check-ins ☀️ (mood/streak/win) and entries 📝 merged chronologically — notes, tracks, logs, plans in place and time. Settings gained an **Axiom section**: Minimal AI mode (default off; the notebook works offline regardless — this only gates Axiom's generation features) and the daily-question toggle for retroprompting. Two delivery root-causes fixed: `profiles.minimal_ai_mode` shipped DEFAULT TRUE and nothing ever wrote it — all 45 accounts silently opted out of every AI brief since it landed (default flipped to false, rows reset); and node-cron evaluates in system-local time, so every axiom cron documented as UTC actually fired CEST+2 — schedules now pin `timezone: 'UTC'`, and the weekly plan sweep runs an idempotent boot catch-up when its window passes during deploy churn. Weekly-plan generation hardened against cascade reality: fenced JSON stripped before brace-slicing, weekday-name days accepted ("Monday" → 0), truncated ~240-char replies retried with distinct cache keys (runWithFallback caches by prompt hash; a poisoned entry served the truncation forever). Live-eval scripts: `scripts/force-week.ts`, `force-brief.ts`, `eval-retro.ts`, `eval-colibri.ts`. Retroprompting's follow-up slot was unreachable (first answer closed the thread before the follow-up could be answered) — found by the live eval, fixed. Memory formalized: `vaultTaxonomy.ts` namespaces + generated `_index.md` MOC (Obsidian-ready vault, see docs/OBSIDIAN_VAULT.md), physis auto-tags blank domains one-way. CI workflow repaired (dead eslint step removed, concurrency added). Morning brief dropdown restored to hero substance: greeting prose rendered at last, mood chips + energy slider AS the check-in, tap-to-toggle DAILY PROTOCOL with n/17 bar, GOAL MOVES cards.

- **2026-08-13** — **The UberWiki finally contains something.** `attachToDomain` existed to hang real records off the 32-node scaffold and was never called: exported, unit-tested, dead. The graph was a diagram of the ontology with no user content in it. `UberWikiGoalSync` now upserts a node per goal and attaches it to its domain on create, update and delete (fire-and-forget — a read model must never fail the write the user asked for), idempotent on `metadata.goalNodeId` so a rename updates rather than forks and a re-sync adds no second edge (the relationships table has no unique constraint to catch that). `scripts/backfill-uberwiki-goals.ts` brought the existing 25 goals in, 25 attached. This only became possible once the domain vocabularies were unified — the same join matched 8 of 25 before. Privacy is the load-bearing part: goal nodes are `private` with `owner_id`, and `getGraph` now requires **both** ends of an edge to be visible, because PostgREST's `or` returns an edge when either end is, and every private goal attaches to a global domain node — without the filter each user would receive a stub edge per stranger's goal (no titles, but an accurate count of other people's goals per life domain). `migrations/2026-08-13-uberwiki-owner-cascade.sql`: `owner_id` was `ON DELETE SET NULL`, so deleting an account left every goal title behind as an ownerless private row — invisible to every reader, therefore unnoticeable, and undeletable through any product surface. My own smoke runs produced 4 within minutes. Now CASCADE, plus the `owner_id`/`user_id` mismatch that made the admin manual-cascade list miss the table. 17 new tests; e2e 37/37.

- **2026-08-12** — **One bilingual domain vocabulary instead of five.** `models/Domain.ts` (canonical 14), `schemas/goalSchemas.ts` (legacy 10), `client/models/Domain.ts` (a different legacy 10), physis's ontology names written straight through by `PhysisAutoTagService` (`Craft & Career`, `Body & Health`, `Culture & Hobbies`, `Mind & Learning`), and OSM place types were all live at once. `createGoalSchema` validated against a list sharing **no value** with the app's own domains, so the API rejected the names its own client sent, and every join on a domain name compared strings from different lists — no errors, just wrong numbers that looked plausible. `models/DomainDictionary.ts` is now the single resolver: 14 domains, English + Italian labels, every legacy and colloquial alias in both languages, case/accent/punctuation-insensitive, reversed pairs handled as a class, `null` rather than a guess (`Axiom` was never a life domain). Validation normalises instead of refusing; junk still bounces. `domainNormalize` — a second, smaller resolver covering part of the same ground — delegates. The client mirror is duplicated (separate rootDirs) with a test that parses it and fails on drift. Data: 965 notebook rows, 899 places, 7 goal trees, 1 proficiency map; dry run now reports 0 changes. Found along the way: my own migration silently fixed only the first 1000 rows (PostgREST caps unbounded selects, and the symptom reads as phantom writes); `inferDomain` sent every study goal to Spirit & Purpose; logging prompts had business copy under romance; OSM filed clinics with gyms and spas under romance. Goal nodes attaching to the scaffold: 8 of 25 → 25 of 25.

- **2026-08-10 (later)** — **Legal pages, local embeddings, Hermes gateway.** `/terms` and `/privacy` shipped (`client/src/features/legal/`, shared `LegalPage` rendering structured sections — no Markdown dependency added for two static pages), linked from the landing footer and signup form, controller details real, age minimum 14 per the Italian derogation. The embedding layer, found dead in every table because `text-embedding-004` was retired, moved to a local `nomic-embed-text` through Ollama after probing revealed **Google had revoked 20 of 22 Gemini keys as leaked** — `migrations/2026-08-10-reembed-local-vectors.sql` plus `npm run reembed` handle the transition, and `embeddingRefreshCron` keeps it current. `GoalEmbeddingService` (11 tests) makes goal-vector sync a diff rather than a rebuild. Hermes' OpenAI-compatible API server enabled on loopback with its toolset pinned to `todo`, activating the `HERMES_GATEWAY` provider at tier 1. Production dumps moved out of the repo tree to `/home/gio/praxis-backups` (mode 700). Delegation to opencode's free models failed three times over ~75 minutes; the Hermes CLI on an explicit NVIDIA/GLM-5.2 route is what works.

- **2026-08-10** — **Local self-hosting, launch hardening, Axiom tiering + goal-keyed retrieval.** Full detail in the section above. Headlines: live money-path auto-approve closed (stub oracle, plus the `.env` flag that made the code fix inert); 404 stack-trace leak closed; service worker no longer intercepts cross-origin requests (this was breaking all Supabase auth); 17 anonymous unauthenticated LLM relays gated behind `AXIOM_ALLOW_ANON_PROXIES`; landing page rebuilt as 10 sections with pricing imported from `premiumTiers`; funnel instrumentation end-to-end (`POST /api/funnel`, migration applied); Axiom providers tiered by subscription (`providersForAudience`, free = local models only, audience-scoped response cache); `GoalContextService` replaces recency-ordered dumps with a budgeted per-goal evidence pack; `domainNormalize` fixes a vocabulary drift that had disabled every domain comparison in the codebase (1 of ~2800 entries matched canonically). Commits `8e04cc7`, `db2da72`, `ffa1bf3`, `aaf9ecc`, `d0579c2` on `launch/landing-and-security-fixes`.

- **2026-06-23** — **Axiom Boost tier + multi-language coach + Groups Events-pin.** (Boost, 3.2) `profiles.axiom_tier` ('free'|'boost') + `utils/axiomTier` single gate — `hasFreeAxiom` (premium/admin/boost ride PP-free) + higher `BOOST_AI_DAILY_CAP`, wired into the notebook Axiom query + `/coaching` gate (`enforceAiBudget` gained a `capOverride`). Stripe scaffolding: `createBoostCheckout` → `POST /stripe/create-boost-checkout` on `STRIPE_PRICE_ID_BOOST[_ANNUAL]`; the webhook flips `axiom_tier='boost'` when a sub's price `isBoostPrice` (granted while active/trialing, revoked on cancel/delete). `UpgradePage` has a distinct "Axiom Boost · Premium+" card → `/create-boost-checkout` (shows "Boost active" when held). Config left: create the Stripe Boost price + set the env (manual_actions). Tests: `axiomTier` (3), `boostPrice` (3). (Multi-language coach, 3.2) `utils/languageName` (code→name, e.g. it→Italian; tested 4) applied to the coaching reply + analysis prompts and the morning brief (labels kept English so the parser survives, values written in the user's language). (Groups Events-pin) `GroupEventsBanner` surfaces upcoming `group_activities` as a one-tap-RSVP strip above the room tabs — **completes Groups v2.** 425 backend + 85 frontend green.

- **2026-06-23** — **Sponsored challenges (ad-model phase 2 — the last open ad phase).** A sponsor funds a seasonal event: `createSponsoredChallenge` escrows `reward_pp × slots` via `spendPP('challenge_fund')` (reward 10–1000, slots 1–1000, validated dates) and inserts a `seasonal_event` carrying the sponsor (`sponsor_id`, `sponsor_reward_pp`, `sponsor_slots`, `sponsor_paid_count`, `sponsor_closed`; migration `2026-06-23-sponsored-challenges.sql` + `seasonal_event_participants.sponsor_paid_at`). On completion the participant is auto-paid from escrow: `settleSponsoredCompletion` is fired wherever a seasonal completion is detected (check-in streak block + `incrementSeasonalMetric`), with a per-participant `sponsor_paid_at` conditional-stamp double-pay guard and a slot cap so payouts never exceed the escrow. `closeSponsoredChallenge` (sponsor-only, conditional-close double-refund guard) returns `reward_pp × (slots − paid)`. Routes `POST /seasonal-events/sponsor` + `/:id/close` (both `blockAgents` — financial). UI: `/seasonal` page shows a "💸 Sponsored · +N PP on completion" + "N payouts left" chip; a "Sponsor" button opens `SponsorChallengeDialog` (type→metric picker, target/reward/slots, datetime window, live escrow preview) → `POST /sponsor`; sponsors get a "Close & refund" button on their own challenges → `/:id/close`. Tests: `sponsoredChallenge.test.ts` (11 — escrow/refund-on-fail/validation, settle once/raced/exhausted/unsponsored/incomplete, close refund/non-sponsor/double-refund). 415 backend + 85 frontend green. Completes all four ad-model phases (promoted offers, affiliate cards, sponsored challenges, verified-outcome bounties).

- **2026-06-23** — **Seasonal events finished end-to-end.** Found the feature ~80% pre-built (tables/seed, controller incl. leaderboard, routes, dashboard `SeasonalEventCard`, check-in streak wiring). Fixed the real gap: only `streak_challenge` auto-advanced — added `SeasonalEventProgress.incrementSeasonalMetric` (+1/action, capped, single completed_at; tested 5) wired into goal completion / check-in / bet win so `goal_crusher`/`participation`/`betting` events actually progress. Added the `/seasonal` discover page + per-event leaderboard (surfaces the previously-unused leaderboard endpoint).

- **2026-06-22/23** — **Groups v2 complete.** Governance (roles/private/approval/rules via `utils/roomMembership` + migration), `GroupSettingsDialog` + `GroupMemberDrawer` (role chips, admin role selector, mod approve/reject queue), pinned board posts (`posts.pinned_at`, mod-gated `POST /posts/:id/pin`), mute-group toggle (wired existing `/mutes`), and @mentions (`utils/mentions.extractHandles` tested 6; `sendRoomMessage` resolves to members + distinct `mention` ping; composer `@`-autocomplete). Events pin area SHIPPED 2026-06-23 (`GroupEventsBanner` — upcoming `group_activities` surfaced as a one-tap-RSVP strip above the room tabs, "See all" → Wall tab). **Groups v2 is now fully complete.**

- **2026-06-22** — **Money-path tests + god-component splits (roadmap T4 debt).** (Tests) `bountyMoneyPath.test.ts` (24): `createBounty` escrow=rewardPp×slots + refund-on-insert-fail + validation-before-PP; `claimBounty` guards (missing/closed/expired/full/own/3-cap/duplicate) + earn-path-never-moves-PP; `resolveBountyClaimOnGoalCompletion` active→verified double-payout guard, past-grace no-pay; `closeBounty` refund math + double-refund guard. `detoxCommit.test.ts` (8): single `spendPP('bet_stake')` happy path, refund on goal-tree save error AND bet-insert error, gates (no RescueTime link / over-50%-cap / range) reject before PP. supabase+pp mocked (bettingResolution/pointsSpend pattern), no source changes. Closes the controller-level money-path gap (betting was already covered). (Refactors — ProfilePage slice pattern) NotebookPage 1164→752 (−35%, four slices): `components/NotebookAnalyticsSection` (entry-type/14-day/day-of-week/hourly/moods/tags/AI-insights), `components/NotebookCalendarSection` (two-month emoji calendar), `components/NotebookEntryCard` (the ~190-line entry card — header/content/sparkline/tags/rhymes constellation/inline reply; page keeps a thin renderEntryCard wrapper so both call sites are untouched; `formatEntryTime` moved to notebookMeta). EditableTrackerForm 998→512 (−49%, three slices): `TrackerRowFields` (per-tracker-type metric inputs) + `trackerSuggestions.getTrackerSuggestions` (21-case type→search-library switch as a Promise-returning helper, −17 search-lib imports) + `useTrackerForm` hook (all logic — 15 state atoms, 3 effects, 12 handlers; the form is now pure JSX consuming the hook bag). Behaviour unchanged; tsc clean, 85/85 frontend + new backend tests green. Both god-components now roughly half their original size. (Long tail) NotesPage 1059→837: `useDiaryExport` hook (the 3 PDF export flows + `exporting` flag) + `NotesExportDialog` (three-tier picker UI) + `GoalEditDialog` (create/branch/edit node dialog). tsc clean, 85/85 green throughout. Still open (lower value): NotebookPage `renderDayDetail`; NotesPage bet/claim/achieve/suspend dialogs; ChatRoom (1053), AICoachPage (857), BettingPage (850).

- **2026-06-14** — **OAuth fix, part 3 — library-only PKCE exchange.** After part 2, login showed "no valid flow state found": the design itself was wrong — `detectSessionInUrl:false` PLUS a manual `exchangeCodeForSession` in AuthCallback meant the single-use `?code=` had two potential consumers / a broken consumer. Reverted to the canonical pattern: `detectSessionInUrl:true` (the client is the sole exchanger, bare code + stored verifier, on load) and AuthCallback no longer exchanges — it waits for `getSession()`/`onAuthStateChange(SIGNED_IN)` then routes (5s fallback → /login). Exactly one consumer ⇒ no race, no URL-vs-code bug. Residual failure mode is now purely config: the login origin's `/auth/callback` must be in Supabase's Redirect URL allowlist (see manual_actions.txt) — the verifier lives in the starting origin's localStorage.

- **2026-06-14** — **OAuth login-loop fix, part 2 (the real root cause).** The loop returned. The 2026-06-12 fix (`detectSessionInUrl:false`, AuthCallback as sole exchanger) actually _exposed_ a latent bug: `AuthCallback` called `exchangeCodeForSession(window.location.href)` — the **full URL** — but supabase-js v2 sends that argument verbatim as `auth_code` with no URL parsing, so the token endpoint rejected it. While `detectSessionInUrl` was true the library's own auto-exchange (correct bare code) had masked the broken call; turning it off left the broken URL-passing exchange as the only one → every Google login bounced to `/login`. Fix: pass the bare `params.get('code')`. (`client/src/features/auth/AuthCallback.tsx`.)

- **2026-06-12 (late)** — **OAuth login-loop fix.** Google sign-in bounced back to the login screen: Supabase auth logs showed server-side login succeeding every time, but the client never kept the session, and `/authorize` kept re-firing with referer `/auth/callback`. Root cause: supabase-js v2 defaults BOTH `flowType:'pkce'` and `detectSessionInUrl:true`, so the library auto-exchanged the single-use `?code=` on page load while `AuthCallback` also called `exchangeCodeForSession` — the losing racer got "flow state not found" → error → redirect to `/login`. Which racer won depended on bundle/module timing, so the bug surfaced only after the (unrelated) android-shell deploy reshuffled chunks. Fix: `detectSessionInUrl:false` + explicit `flowType:'pkce'` in `lib/supabase.ts` (AuthCallback is the ONLY exchange owner), and AuthCallback now checks `getSession()` before treating an exchange error as a failed login (duplicate exchange from a second tab/back-button no longer loops).

- **2026-06-12** — **Android shell + native screen-time bridge** (§3.7, option c — iOS deferred, needs Xcode/macOS). Capacitor 7 (CLI 8 needs Node 22; box runs 20) in remote-URL mode (`client/capacitor.config.ts` loads praxisweb.xyz — every Vercel deploy reaches the app, no store releases). Native: `UsageStatsPlugin.java` (UsageStatsManager → social-media foreground minutes since local midnight, 12-app social list, `PACKAGE_USAGE_STATS` special permission with settings deep-link), registered in `MainActivity`. JS: `lib/nativeUsage.ts` bridge (graceful web no-op) + `NativeScreenTimeCard` on the dashboard (renders only inside the shell — "measured on-device"). Build steps in `manual_actions.txt` (needs Android SDK). Future tier: usage→oracle attestation for mobile detox proof, native push.

- **2026-06-12** — **SpeedDial "Plan" — calendar+map waypoints with two-way Google sync.** New 📅 PLAN action: title + date/time + optional place (forward-geocoded via existing `/notebook/geocode` so the plan pins on the map). `POST /calendar/plan` creates a future-dated `notebook_entries` row (`entry_type: 'plan'`, shows on the Praxis calendar 📌 + map) and, when Google Calendar is linked (existing `GoogleCalendarService` OAuth), mirrors it as a Google event, storing `metadata.google_event_id`. Pull side: `POST /calendar/sync` imports upcoming Google events (60d horizon) as plan waypoints, deduped by `metadata.google_event_id` — fired automatically when the Plan sheet opens. NO migration (uses the entries `metadata` jsonb). Sheet shows live sync status; edits/deletions don't propagate yet (create-level sync only — noted future work).

- **2026-06-12** — **Oracle housekeeping cron + done-for-today** (roadmap T1.1/T1.3/T1.4). (a) `oracleHousekeepingCron` (daily 05:00 UTC, in `index.ts` boot): pass 1 AUTO-VERIFY — completed detox windows (driven from active bets → owner's node `detox` params) and bounty claims (rescuetime waits for window completion, github attempts daily) run `runOracleVerification` server-side, so finishers wake up paid with credentials minted, zero taps; pass 2 sweep expired PP bets — extracted `sweepExpiredPPBets()` from the webhook-only handler, WHICH NOTHING EVER CALLED in production (expired bets sat active forever); pass 3 `BountyService.expireBounties()` — closes past-deadline bounties (refunds sponsor's unpaid escrow) + fails claims past the 2-day grace, after auto-verify so honest finishers get paid first. All passes non-fatal. (b) **Done-for-today v1** (`DoneForTodayCard`): once checked in, the dashboard feed column defaults to a terminal "go live the day" card; feed behind one explicit tap (`show activity anyway`). The thesis feature: time-in-app is a cost. (c) CI: backend jobs get `NODE_OPTIONS=--max-old-space-size=4096` — the bounties commit OOMed the default-heap tsc build on runners (both workflows red; Railway deploys were unaffected — native GitHub integration, the workflows are gates).

- **2026-06-12** — **Verified-outcome bounties** (ad-model phase 4 — the moat). Sponsors pay for PROVEN outcomes, users earn the reward: inverse of an ad. New `bounties` + `bounty_claims` tables (`migrations/2026-06-12-bounties.sql`, applied to prod, RLS on), `BountyService`: create (escrows `reward_pp × slots` via `spendPP('bounty_fund')`, reward 10–1000, slots 1–100, providers `rescuetime|github` only — machine-verified), claim (≤3 active/user, not own bounty, creates root goal node with `bounty` metadata; window starts AT claim via `params.startDate` — github provider now supports startDate too, pre-claim commits excluded), resolve (hooked into `runOracleVerification` after credential mint; conditional `active→verified` update = double-payout guard; pays `bounty_reward` from escrow; 2-day grace past `ends_at`), close (sponsor-only, conditional update = double-refund guard, refunds `reward_pp × (slots − verified)`). Routes `/bounties` (fund/close = `blockAgents`; claiming earns, never spends). UI: Marketplace → new "Bounties" tab (`BountiesPanel`): open bounties w/ provider+reward+slots chips, claim, "My claims" with one-tap "Verify & collect", fund dialog w/ live escrow preview. Tests: `githubOracleStartDate.test.ts` (3) + oracle suite 15/15. All four ad-model phases now have substance except sponsored challenges (#2, blocked on events rebuild).

- **2026-06-12** — **Affiliate completion cards** (ad-model phase 3). One labeled partner offer at the goal-completion moment, matched to the completed goal's domain — never in a feed. New: `affiliate_offers` table (`migrations/2026-06-12-affiliate-offers.sql`, applied to prod, RLS on, 5 seed rows with plain URLs — swap for affiliate-tagged links when programs are approved), `affiliateController` (`pickAffiliateOffer`: exact-domain pool first, generic NULL-domain fallback, random within pool so one partner can't monopolize; match endpoint 204s and never breaks the celebration), routes `GET /affiliates/match?domain=X` + `POST /affiliates/:id/click` (click counter → revenue reporting). UI: card in the `GoalTreePage` "🏆 Topic Completed!" dialog — "Partner offer · {partner} · Praxis may earn a commission" label + CTA. Tests: `affiliatePick.test.ts` (4, green). Remaining ad phases: sponsored challenges (needs events rebuild), verified-outcome bounties (the moat).

- **2026-06-12** — **Promoted offers** (first-party goal-aligned ads, §3.5 "in-product ads" shipped). `POST /offers/:id/promote`: 100 PP for 7 days via `spendPP('promote_listing')` (refund on failure), poster-only, `blockAgents`, open offers only, no re-promote while active, **global cap 3 concurrently promoted** (scarcity — promotion can't become a feed). `GET /offers` ranks actively-promoted first, organic order untouched. UI (Offers tab): amber border + "★ Promoted" chip with paid-placement tooltip, "🎯 matches your goals" chip when offer domain matches viewer's declared goal domains (transparency: tooltip says targeting uses only declared data), owner "Promote · 100 PP" button / "Promoted until X" chip. Migration: `migrations/2026-06-12-promoted-offers.sql` (`offers.promoted_until` + partial index) — in `manual_actions.txt`; until applied, promote 400s cleanly and listing is unchanged. Drive-by fix: `createService` claimed a 30 PP posting cost but only checked balance — now charges via `spendPP('service_post')` with refund on insert failure. Ad-model rules (from targeted-ads design): first-party only, capped slots, always labeled, declared-intent matching, no third-party SDKs; next phases = affiliate completion cards → sponsored challenges → verified-outcome bounties.

- **2026-06-12** — **Detox commitment + PP stake** (screen-time oracle phase 2). `POST /oracle/detox/commit` { maxMinutesPerDay, days (3–12), stakePoints (≥10) }: creates a root goal node carrying `detox: { maxMinutesPerDay, days, metric, startDate }` + an active PP bet on it in one step (same staking rules as `/bets`: ≤ min(500, 50% balance), ≤3 active; one active detox at a time; requires linked RescueTime; `blockAgents` — PATs can't stake). Window = the `days` full days after today (fixed, not trailing — provider's new `params.startDate` rejects pre-commitment clean days and fails "still running" until the window completes); bet deadline = window end + 2-day grace, after which the expired-bet sweep forfeits the stake. `runOracleVerification` now settles bets via `resolveBetsOnGoalCompletion` (1.8× payout) on any oracle pass, and a latent bug was fixed there (`.eq('userId')` → `.eq('user_id')` — oracle passes never actually marked the node complete). UI: "Digital Detox" button + dialog on `/commitments` (stake slider, payout/forfeit preview, RescueTime-link gate) + per-bet "Verify detox" button (disabled until window ends, one tap = goal complete + bet paid + credential minted); `GoalTreePage` claim dialog prefills detox params from the node. Tests: 3 fixed-window cases added (12 total, green).

- **2026-06-12** — **Screen-time oracle (RescueTime)** — the anti-social-media proof. New `rescuetime` provider in `OracleService`: machine-verifies digital-detox goals ("social media ≤ N min/day for D days") against the user's RescueTime `daily_summary_feed`. Connection = user pastes their personal API key (validated live at connect; stored in `oracle_connections.access_token`, never exposed; `external_account` holds a masked label; key possession proves ownership → `verified=true`). Verify params `{ maxMinutesPerDay=30, days=7 (≤14, feed depth), metric='social_networking' }`. Anti-cheat: untracked days don't count as clean (max 1 missing day per 7, else "insufficient data"). Pass → goal complete + oracle credential (trust 1.0) + public `/verify/:id` proof. `connectOracle`/`validateAccount` extended to carry provider secrets + verified flag. UI: RescueTime section in `OracleConnectionsCard` (Settings) + "⏳ Auto-verify screen time" with min/day + window inputs in the `GoalTreePage` claim dialog. Tests: `src/__tests__/rescuetimeOracle.test.ts` (9, green). No migration, no env. Phase 2 (proposed): detox commitment surface with PP stake via bets, Pro-gated scheduled re-check cron, native iOS Screen Time via Capacitor shell.

- **2026-06-07** — **Bioenergetics standalone page** (`/bioenergetics`). Full bilingual (IT/EN) interactive toolkit: 5R Method practice tracker, state check-in, weekly stats, 4 content sections (5R Method, Philosophy, Diet, Way of Life). Anonymous client_hash model, standalone outside app shell. Migration: `migrations/2026-06-07-bioenergetics.sql`.
- **2026-06-08** — **Bioenergetics interactive tools + SpeedDial nav**. Added 8 tools (guided 4D breath, lifting calc, metabolic pulse/temp tracker [backend-logged], cold/heat timer, circadian tracker, posture sequence, color meditation, word association); replaced tab bar with a floating SpeedDial grouping 14 widgets into 6 themed sections. New table `bioenergetics_metabolic` + endpoints. Migration: `migrations/2026-06-08-bioenergetics-metabolic.sql`.

- **2026-06-02** — **Tests for new security primitives.** `blockAgents.test.ts` (agent actor rejected, human passes) + `credential.test.ts` (sign/verify round-trip, tamper rejected, garbage sig rejected, canonicalize key-order independent). Guards the PAT-financial-block and the tamper-evidence of verifiable credentials. 6 tests, green in-band.

- **2026-06-02** — **PAT scoping: agents can't move money** (premortem #2 #3). New `blockAgents` middleware (403 when `req.actorType === 'agent'`) on all financial mutations: place/cancel bet, real-bet checkout, create/accept duel, create/join open-bet, spend points, marketplace purchase, stream donate. A leaked Personal Access Token (pasted into agent configs → high leak surface) can now only log/read/verify, not drain the user's PP or gamble. Read + logging endpoints unchanged. Still open: per-PAT scopes/quotas beyond the financial block, and tests for the new surface.

- **2026-06-02** — **Premortem #2 fixes: location privacy + journal de-noise.** (Privacy) Location-on-logs is now a user-controllable setting (`locationPrefs`, Settings → Data Sharing) and coordinates are coarsened to ~100 m (3 dp, was 5 dp / ~1 m) — both `TrackerSection` and `NoteGoalDetail` respect the toggle + coarsening; capture skips entirely when off. (De-noise) `'tracker'` auto-pin entries are kept on the map but filtered out of the journal feed + calendar day-detail (`journalEntries`), since the calendar tracker-chips already represent that activity. Still open from premortem #2: PAT scoping (any `pk_live_*` Bearer = full account on every REST endpoint), tests for the new agent/credential/oracle surface, and regenerating the credential Ed25519 key locally (never use a key that appeared in a transcript).

- **2026-06-02** — **Log-location: click-time fix + SpeedDial coverage.** (a) Tracker logs now fetch a _fresh_ `getCurrentPosition` at log time (cached value was null on the first log), falling back to the cached fix — in both `TrackerSection` and `NoteGoalDetail`. (b) `NoteGoalDetail` (the tracker surface used by the SpeedDial `QuickLogDialog` and NotesPage) previously stored location only in `tracker_entries.data._location` (never shown on the map); it now also posts a `notebook_entries` pin, so SpeedDial logs appear on the map too.

- **2026-06-02** — **Log location → map pin.** When a tracker is logged in `TrackerSection` (standard + custom), it now captures the user's current location (`useCurrentLocation`) and posts a `notebook_entries` row (`entry_type: 'tracker'`, `location_lat/lng`, title "Logged <type>"), best-effort. Since the notebook map is driven by notebook_entries with coordinates, the log appears as a pin on the map (and in the calendar day-detail + feed) without map/schema changes. Other log entry points (SpeedDial QuickLogDialog) can be wired the same way.

- **2026-06-02** — **Notebook calendar/map fixes.** (1) Clicking a calendar day that had only tracker logs (no notebook entry) no longer shows "nothing logged" — the day-detail now reads `calendarActivityData` (tracker types + count from `/trackers/calendar`) and shows tracker chips alongside notebook entry cards. Root cause: tracker logs write only `tracker_entries`, never `notebook_entries`. (2) The dashboard map now **always renders** (removed the no-pins placeholder) and **centers on the user's current location** via geolocation → `LocationMap userLocation` (which already recenters reactively). Calendar + map were already side-by-side (Grid 7/5). Plus auth/branding steps recorded in `manual_actions.txt` (PKCE session persistence shipped; Google consent-screen branding + Supabase custom-domain are dashboard config).

- **2026-06-02** — **Dependency bump: TypeScript 5.9 → 6.0.3** (Dependabot). TS 6 makes `moduleResolution=node10` ("node") a hard error, so switched client tsconfig to `moduleResolution: "bundler"` (correct for Vite + `module:esnext`). Verified: client `tsc --noEmit` clean + `vite build` green. (Vite 8 Dependabot PR left open — build-tool major needs a separate config/plugin pass.)

- **2026-06-02** — **Per-goal Verify button in the notebook.** Added an inline ✅ "verify" action on each active goal pill in `NotesCardTree` (threaded a new `onVerify` prop through the recursive tree). Wired in `NotesPage` to the existing claim flow (`handleAction('verify')` → verifier+evidence dialog → `POST /completions`), so requesting peer verification is one click per goal instead of select-then-Management-Actions.

- **2026-06-02** — **God-component refactor (start).** Began incrementally splitting large components into reusable children (pattern: `DashboardActivitySection`). First slice: extracted `ProfilePostsSection` + `EmbedWidgetCard` out of `ProfilePage` (~1089 lines) into `features/profile/components/` — behavior unchanged, ~55 lines + dead imports removed. Remaining: more `ProfilePage` sections, `NotebookPage`, `EditableTrackerForm`. Deliberate, one section at a time, verify after each.

- **2026-06-02** — **Tracker edit-mode default values.** Edit mode now shows the per-row value fields (removed the `renderRowFields` editMode guard in `EditableTrackerForm`), so users pre-set defaults like "3×8 @ 80 kg" without logging an entry first. Added `value` to `TEMPLATE_FIELDS` so generic-amount types (meal/steps/reading) persist their default and the last logged value carries forward. Completes the tracker-template persistence work.

- **2026-06-02** — **Per-user daily AI cap** (premortem #8 — cost backstop). `ai_usage_daily` table + atomic `bump_ai_usage(user, tokens, cap)` (`migrations/2026-06-02-ai-usage-cap.sql`); `enforceAiBudget()` (`src/utils/aiBudget.ts`) wired into Axiom chat (`personalityChat`) + notebook query (`queryNotebookAxiom`), applied to Pro too (Pro is free but still incurs LLM cost). Env `AI_DAILY_CALL_CAP` (default 300; 0 disables). Fails OPEN on infra errors so the cap never takes the feature down. Daily brief stays once/day + PP-gated. Token-level metering / cost dashboard = future refinement.

- **2026-06-02** — **Atomic PP — single-column sites converted** (full ledger coverage). Moved the remaining single-column PP mutations onto `awardPP`/`spendPP` (atomic + `pp_ledger`-recorded with reasons): Stripe **PP purchase** (`pp_purchase`), Axiom chat/brief spends + brief-unlock (`aiCoachingController`/`notebookAxiomController`/`axiomUnlockController`), achievement post, challenge join, real-bet bonus, event check-in/host, referral (both sides), tracker-log award. `aiCoaching`/`notebookAxiom` `deductPP` helpers now delegate to `spendPP` (kills their read-check-deduct race) while keeping the `marketplace_transactions` record. Remaining: combined-column updates (`checkinController` PP+reliability+streak, `postController` PP+karma, `userController` PP+onboarding, `goalController`, admin grants) — single-writer/low-race, split when touched.

- **2026-06-02** — **Money-path tests + reliability verification.** Added `src/__tests__/pp.test.ts` (5 tests) covering the atomic PP chokepoint (`awardPP`/`spendPP`): success, insufficient (RPC NULL), error-not-success, and ledger writes with `reason`. Confirmed `reliability_score` is recomputed on every check-in (`checkinController`), not a stale cron — live formula is `0.50·C + 0.25·V + 0.10·S + 0.15·K` (karma term; docs cite the older `0.65·C+0.25·V+0.10·S` — drift noted, harmless). Deeper controller-flow money tests remain a follow-up (heavy supabase mocking).

- **2026-06-01** — **Agent → user → uberwiki knowledge flow.** A connected agent proposes STRUCTURED knowledge about the user's goals (status/blockers/milestones/metrics/commitments/deadlines/facts), mined from its prior chats; the user confirms/denies; on confirm it's written to the uberwiki (`axiom_wiki_pages`). MCP is client→server (Praxis can't push to the agent), so Axiom exposes the request as the MCP resource **`praxis://knowledge-requests`** (active goals + requested fields), and the agent submits via the new **`submit_goal_knowledge`** tool (`AxiomActionRouter`). New: `migrations/2026-06-01-agent-knowledge-submissions.sql` (pending submissions table), `knowledgeController` + `/knowledge` routes (`GET /pending`, `POST /:id/confirm` → renders structured data to a markdown wiki page, `POST /:id/deny`), `KnowledgeReviewCard` on the dashboard (confirm/deny, auto-hides). Human stays the gate — no silent wiki writes. Clarifies that MCP agents complement Axiom (hands/source) rather than replace it (coach/consumer).

- **2026-06-01** — **Agent integration SDK** (friction-free AI logging). Built on existing infra (PAT system in `agentController`, MCP server in `src/mcp/praxisMcpServer.ts`, `AxiomActionRouter` tool catalog). New/changed: (1) **MCP accepts PATs** — `getUserFromBearer` now resolves `pk_live_*` Personal Access Tokens (via `authenticateApiKey`) as well as Supabase JWTs, so MCP clients (Claude) authenticate with a PAT. (2) **REST accepts PAT as Bearer** — `authenticateToken` already took `x-api-key`; now also accepts `Authorization: Bearer pk_live_*` (agents commonly use Bearer). (3) **OpenAPI 3.1 spec** — `src/openapi.ts` served at public `GET /api/openapi.json` (importable by ChatGPT Actions / Gemini): logging (`/trackers/log`, `/checkins`), context (`/goals/:userId`, `/trackers/activity-feed`, `/trackers/monthly-by-goal`), token mgmt + audit (`/agent/*`), PAT security schemes, agent-friendly error responses. (4) **Dev docs** — `docs/agent-sdk/README.md`: quickstart per platform (Claude MCP config, ChatGPT Actions, Gemini, raw REST), logging recipes, the required confirm-before-write pattern, error/retry + rate limits (100/min), revocation + audit, privacy. Gaps: full OAuth2 consent (PAT + existing `/agent/connect` code-exchange cover the practical path), per-platform npm SDK wrappers (docs cover OpenAPI/MCP/REST), MCP still Pro-gated (REST+PAT works without Pro).

- **2026-06-01** — **Premortem hardening** (economy observability + integrity + ops). (1) **PP audit ledger** — `pp_ledger` table (`migrations/2026-06-01-pp-ledger.sql`); `awardPP`/`spendPP` now record every mutation with `reason` + `balance_after` (best-effort), so the economy is auditable and "where did my PP go" is answerable. Betting + completion paths labelled (`bet_stake`/`bet_win`/`claim_bond`/etc.). (2) **Sybil gate** — demo accounts (`profiles.is_demo`) now confer zero verifier trust, so seeded users can't inflate the credential system. (3) **Boot validation** — `auditEconomy()` warns loudly (error in prod) if `increment_pp`/`spend_pp`/`pp_ledger` or `CREDENTIAL_ED25519_*` are missing — catches "deployed but forgot the migration" (which would silently reject all spends). (4) **CI type gate** — added frontend `tsc --noEmit` to `ci.yml` (vite build skips type-checking, so frontend type errors had been slipping through) + fixed the pre-existing `UnivrPage` `TabKey` errors. Deferred: per-user AI token meter/cap (#8 — partially covered by once-daily briefs + PP-gated queries), god-component refactors, prove-it external recognition (go-to-market).

- **2026-06-01** — **Atomic Praxis Points** (money-path hardening). Added `increment_pp` / `spend_pp` Postgres functions (`migrations/2026-06-01-atomic-pp.sql`) + `src/utils/pp.ts` (`awardPP`/`spendPP`). Replaced non-atomic read-modify-write (`select balance → compute → update`, which loses concurrent updates / allows double-spend) across the gamble & transfer paths: betting (stake/refund/winnings/duels), open-bets (stake/refund/payout), `duelController` helpers, completion claim-bond escrow/refund/forfeit, stream donations, marketplace purchase + coach credit, honor give, and `AxiomActionExecutor` agent stakes. `pointsController` already used an atomic `spend_points` RPC. Remaining award-only / combined-column sites (achievements, events, checkin, referral, goal completion, posts, admin) are lower-risk single awards — follow-up.

- **2026-06-01** — **Dashboard activity section** (replaces the removed "Today's Timeline" widget). Per-goal monthly volume trends (sparklines over each tracker's primary structured metric — `volume_kg`, `amount_eur`, `duration_min`, etc., bound to a goal via `trackers.node_id`) + a unified reverse-chronological feed of all tracker logs and notebook entries. New backend: `GET /trackers/monthly-by-goal` + `GET /trackers/activity-feed` (`trackerController`, no migration — reuses structured tables + `tracker_entries`/`notebook_entries`). Frontend: `DashboardActivitySection` (uses `@mui/x-charts` SparkLineChart). Also this session: removed dashboard free-note composer + Today's-Timeline gantt widget, collapsed notebook search to an icon by Export, and fixed tracker template persistence (data fields now round-trip, not just label/unit).

- **2026-06-01** — **GitHub OAuth ownership proof** (prove-it gap #4 integrity fix). Username linking is spoofable (anyone can claim any handle); OAuth login proves the user actually owns the account. New: `2026-06-01-oracle-oauth-verified.sql` (`oracle_connections.verified` + `provider_user_id`), `getGithubAuthUrl`/`handleGithubOAuthCallback`/`isGithubOAuthConfigured` in `OracleService`, `GET /oracle/github/auth` + public `GET /oracle/github/callback`. Connections now carry `verified` (true = OAuth-proven). Verify uses the user's own OAuth token when present. Frontend `OracleConnectionsCard` shows "Authorize with GitHub" (preferred) + username fallback (marked unverified) + verified-owner badge. Env: `GITHUB_OAUTH_CLIENT_ID/_SECRET/_REDIRECT_URI` (register one OAuth App; leave blank to keep username-only).

- **2026-06-01** — **GitHub oracle provider** (prove-it gap #4, first real provider). Verifies coding goals against the **public** GitHub REST API by linked username — no OAuth app or secret required (optional `GITHUB_TOKEN` only raises the rate limit). `githubProvider` in `OracleService` counts public PushEvent commits in a window (default ≥1 in 30d, params-tunable `{sinceDays, min, repo}`), validates usernames at connect time. New connection CRUD: `GET /oracle/connections`, `POST /oracle/connect`, `DELETE /oracle/connect/:provider`. Frontend: `OracleConnectionsCard` in Settings (link/unlink GitHub) + "🤖 Auto-verify with GitHub" button in the goal verification dialog. Verified → goal complete + oracle credential (trust 1.0). Gap #4 now has a working real provider; Strava/etc. plug in the same way.

- **2026-06-01** — **External oracle auto-proof — scaffold** (prove-it gap #4). Provider-agnostic framework for machine-attested completions that outrank peer-verified (oracle credentials mint with `trust_weight = 1.0`). New: `migrations/2026-06-01-oracle-credentials.sql` (`credentials.source`/`oracle_provider`, `oracle_connections` table), `src/services/OracleService.ts` (`OracleProvider` interface + registry + bundled `stub` provider + `runOracleVerification` → marks node complete + mints oracle credential), `oracleController` + `oracleRoutes` (auth: `GET /oracle/providers`, `POST /oracle/verify`). Frontend shows a 🤖 machine-verified badge on `/verify/:id` + proof wall. Real providers (GitHub/Strava) plug in via `registerOracleProvider()` + OAuth later. Env: `ALLOW_STUB_ORACLE`. **All four prove-it gaps now have shipped substance** (#4 is scaffold pending a real provider).

- **2026-06-01** — Fixed `bettingController.createBet` dropping `goal_node_id` (hardcoded `null`) — PP bets now resolve/pay out on peer-verified completion.

- **2026-06-01** — **Claim bond / honor pool** (prove-it gap #2b, staked verification). Claiming completion now escrows a `min(15, balance)` PP bond (`completion_requests.claim_bond`). Approved → bond refunded alongside the +20 PP bonus; rejected → bond forfeited to the verifier (honor pool reward for catching false claims). False claims now cost something; honest verification pays. New: `migrations/2026-06-01-claim-bond-honor-pool.sql`, escrow/refund/forfeit logic in `completionController`, bond hint in the `GoalTreePage` verification dialog. (Realized as a claim-bond on the verification rather than the original bet-stake plan, because `bets.goal_node_id` is hardcoded `null` — bet↔goal linkage is broken and out of scope here.) Gap #2 now closed (#2a trust + #2b stake). Remaining: #4 external oracle.

- **2026-06-01** — **Credential trust weighting** (prove-it gap #2a, anti-collusion/Sybil). Each credential now snapshots a 0..1 verifier `trust_weight` = `0.5·reliability + 0.3·accountAge(/30d) + 0.2·priorVerifications(/10)`, flagged `low_trust` below 0.25. Folded into the signed payload (tamper-evident), surfaced on `/verify/:id` + profile proof wall. New: `migrations/2026-06-01-credential-trust-weight.sql`, `computeVerifierTrust()` in `CredentialService`. Remaining gap #2b: verifier stake / honor pool (rejected stake → verifiers). Gap #4 (external oracle) still open.

- **2026-06-01** — **Verifiable Completion Credentials** (the "prove-it" unlock). Every peer-approved goal completion now mints a portable, tamper-evident credential: a canonical JSON payload signed with **Ed25519**. Server holds the private key, publishes the public key — anyone verifies offline, no Praxis account. New: `migrations/2026-06-01-verifiable-credentials.sql` (`credentials` table, public-read RLS), `src/services/CredentialService.ts` (key load/sign/verify/mint, dev ephemeral-key fallback), `src/controllers/credentialController.ts` + `credentialRoutes.ts` (public `GET /credentials/public-key`, `/:id`, `/user/:userId`), minting wired into `completionController.respondToCompletionRequest` (non-fatal). Frontend: public `/verify/:credentialId` page (`PublicCredentialPage`, skips shell) + profile proof wall (`VerifiedCredentialsSection`). Env: `CREDENTIAL_ED25519_PRIVATE_KEY/_PUBLIC_KEY/_KEY_ID` (see `.env.example` + `manual_actions.txt`). Approval DM + notification now include the verify URL. Closes prove-it gap #3 (proof portability); gaps #1 (evidence) already shipped, #2 (staked/weighted verification) + #4 (external oracle) remain.

- **2026-05-26** — Demo-user seeder (`POST /admin/seed-demo-users`). 20 accounts with full seeding: auth, profile (username/points/streak), goal trees, checkins (streak-length), tracker entries (per-type), and diary entries. Idempotent. Removed from Proposed → Shipped in Admin.
- **2026-05-11** — Axiom multi-LLM provider management (Phase 10-11). Full provider registry in `AICoachingService.ts` with 14 OpenAI-compat, 5 keyless proxies, 5 Ollama endpoints. Admin UI toggles enable/disable, adjusts priority, runs health checks. New endpoints: `PUT /admin/axiom/providers/:name/toggle`, `PUT /admin/axiom/providers/:name/priority`, `POST /admin/axiom/check-providers`. New migration: `2026-05-11-axiom-provider-health.sql`. All 11 phases of master `.env` integration complete.

- **2026-05-11** — **Wiki integration** (Phase 12-14). Persistent semantic search via `llmwiki` (Rust) for:
  - Daily brief context (14-day window)
  - Notebook entries (50 entries max)
  - Tracker logs (100 entries max)
  - Provider fallback caching
  - Wiki schema migration SQL in `manual_actions.txt`
  - Services: `AxiomWikiWriterService`, `AxiomWikiMaterializer`, `AxiomWikiSearchService`
  - Dockerfile multi-stage build (Rust + Node.js)
  - Railway volume mount for `/wiki` directory
  - Response caching in `AICoachingService`
  - Context window reduced (30d → 14d)
  - Notebook/tracker entry limits adjusted (100→50, 200→100)

- **2026-05-07** — Notebook Resumes cost reduction for NotebookLM queries (TTL cache in `notebook_resumes` table).
- **2026-04-27** — Tracker backfill script + Stripe production hardening. New `scripts/backfill-trackers.ts` (CLI: `npm run backfill-trackers -- --user-id=all`) idempotently fills the per-category structured tables from legacy `tracker_entries` JSONB. Stripe: moved webhook raw-body parsing ahead of `express.json()` (was silently breaking signature verification), added `auditStripe()` startup validation that warns/errors on missing `STRIPE_PRICE_ID*` / `STRIPE_WEBHOOK_SECRET` / `CLIENT_URL` and flags non-`sk_live_*` keys in production. New `scripts/stripe-cli-test.sh` for local end-to-end webhook testing. `.env.example` now documents the full Stripe block; production setup steps in `manual_actions.txt`.
- **2026-04-27** — Profile embeddings for semantic text affinity matching. New `profile_embeddings` table + `match_profiles_by_text` RPC + `TextAnalysisService` for Gemini embedding generation. Composite match score: 50% goal + 35% text affinity + 10% geo + 5% reliability. Admin endpoint: `POST /api/admin/refresh-profile-embeddings`.
- **2026-04-19** — Document created. Removed events from Discover (this session). PDF notebook export shipped.

## Fails Page — Full Implementation (2026-05-29)

Anonymous community fails board. Failure is data. Trying again is the metric.

### Schema additions

- `fails.user_id` (nullable uuid) — stored for avg stats, never exposed in API
- `fails.is_manual` (bool) — user-posted vs auto-detected
- `fails.retried_at` (timestamptz) — when user tapped "Try Again"

### Auto-detection (cron)

- Missed check-in (existing — runs 10:30 UTC daily)
- Missed tracker: tracker active, no entry in 3+ days → log `missed_tracker`
- Missed deadline: goal.deadline passed, progress < 100% → log `missed_deadline`

### API

- `GET /fails` — anonymous list (no user_id in response)
- `GET /fails/stats` — total, byType, avg_per_user (total/distinct_users)
- `POST /fails` (auth) — manual post: fail_type='manual', details
- `POST /fails/:id/retry` (auth) — set retried_at = now()

### UI

- Header: avg fails/user this week, positive framing
- "Post a Fail" inline form at top
- Feed: each card shows type, details, timeago, "↩ Try Again" button
- Try Again sets local state + calls retry endpoint
- Filter by type

---

## Backlog — Improvement Queue (2026-05-29)

### 🔴 High priority

**Streak-at-risk push notification**

- Cron at 20:00 UTC: check users with streak > 0 and no check-in today
- Send push via existing VAPID infrastructure (`pushRoutes`, `web-push`)
- Message: "Your N-day streak is at risk — one tap to protect it."
- 1h build. Highest-retention lever in the app.

**Personalized Axiom check-in prompts**

- ✅ SHIPPED (found live while auditing the queue, 2026-08-24): `AxiomMorningBriefService.generate`
  pulls the last check-in note from a 3–7-day window and injects a `FOLLOWUP:` prompt line
  referencing their exact words + goal ("what happened, no fluff"); the answer rides the brief's
  `followup` field (dashboard + Discord/Slack webhook payloads render it).

**Onboarding simplification** — ✅ SHIPPED (verified 2026-06-12): `OnboardingPageV2` is the live `/onboarding` route (single screen: name + first goal + optional avatar, client-side domain inference, `onboarding_completed` set on submit), guarded in `PrivateRoute`. The plan below is historical.

**Goal dependency edges**

- Add `unlock_requires: string[]` to GoalNode model
- UI: dashed line in goal tree from blocked → unlocked
- Axiom uses unlock chain to sequence method recommendations
- No new table — JSONB field on existing goal_nodes

**Weekly cohort challenges** — ✅ SUPERSEDED (2026-06-23): shipped as seasonal events (`seasonal_events` + `/seasonal` discover page + leaderboards) and `cohorts` (`cohortRoutes`, per-cohort leaderboard); see §3.6.

### 🟡 Medium priority

**Goal dependency edges**

- Add `unlock_requires: string[]` to GoalNode model
- UI: dashed line in goal tree from blocked → unlocked
- Axiom uses unlock chain to sequence method recommendations
- No new table — JSONB field on existing goal_nodes

**Weekly cohort challenges**

- Time-bounded group (7 or 30 days) with its own leaderboard slice
- Builds on: groups table, bets, challenges. New: `challenge_cohorts` table with start/end dates
- Auto-close + award PP to top 3 at end date via cron

**PDF monthly review export** — ✅ SHIPPED (found live while auditing the queue, 2026-08-24): Pro-gated `GET /axiom/export/monthly-pdf` streams a `pdfkit`-rendered review (`AxiomPdfExportService`, fed by `AxiomMonthlySummaryService`) — route lives under Axiom rather than the wiki path first sketched here.

### 🟢 Lower priority / future

**Apple Health / Google Fit auto-sync**

- HealthKit (iOS) via React Native wrapper or web API
- Auto-log fitness trackers from health data — near-zero friction
- Requires: native app or PWA with Health API access

**Public commitment contracts**

- Goal with deadline + financial stake visible on profile
- Anyone can hold you to it. Axiom grades completion.
- Social pressure = strongest accountability mechanism known

**Discord/Slack webhook**

- ✅ SHIPPED (found live while auditing the queue, 2026-08-24): `profiles.axiom_webhook_url`
  (migration `2026-05-29-cohort-challenges.sql`), `PUT/GET /axiom/webhook` (Pro-gated, URL masked on read),
  `deliverBriefToWebhook` fired fire-and-forget after every generated brief with
  Discord `{content}` / Slack `{text}` / custom JSON payloads (`WebhookDeliveryService`).

**Annual "Praxis Wrapped"** — ✅ SHIPPED (found live while auditing the queue, 2026-08-24): `PraxisWrappedService` runs December 1 06:00 UTC through the cron lease, generates per-user year-in-review entries (`wrapped_YYYY`, idempotent — skips users who already have one).

---

## Univr Elections — Anonymous Advisory Elections (2026-06-05)

Standalone anonymous elections for `/univr` — no auth, no shell, no SPID/GIA/CIE. Identity via `client_hash`. Outcomes are advisory (client_hash is sybliable).

### Schema — New Tables

- `univr_elections` — container (title, phase, candidates JSONB, department, dates, created_by client_hash, total_voters)
- `univr_election_votes` — one row per hash per election (candidate_id, weight, UNIQUE election_id+voter_hash)

### Backend — New Public Endpoints (no auth, generalLimiter)

| Method | Path                            | Purpose                               |
| ------ | ------------------------------- | ------------------------------------- |
| POST   | `/api/univr/elections`          | Create election                       |
| GET    | `/api/univr/elections`          | List elections (?phase=voting)        |
| GET    | `/api/univr/elections/:id`      | Get election + computed results       |
| POST   | `/api/univr/elections/:id/vote` | Cast vote (server-side GVN weight)    |
| PATCH  | `/api/univr/elections/:id`      | Update election (creator client_hash) |
| DELETE | `/api/univr/elections/:id`      | Delete election (creator client_hash) |

GVN weight = `sqrt(1 + prior_univr_interactions)` across `univr_complaints` + `univr_posts` + `univr_events` matching `voter_hash`.

### Frontend — UnivrElections.tsx

New tab (🗳️) between polls and books. Views: list with phase badges, detail + vote, results bar charts, create form.

---

## Bioenergetics — Standalone Interactive Toolkit (2026-06-07)

Standalone `/bioenergetics` page (no auth, no app shell) synthesizing Monica Dimonte's 5R Method, Klages/Reich/Peat bioenergetic philosophy, and Bronze Age Mindset into a bilingual (IT/EN) interactive toolkit with practice tracker, check-in, and analytics.

### Pattern

- Standalone page outside RootLayout, lazy-loaded, LEGACY_NAV_PATHS entry — identical to `/univr`
- Anonymous `client_hash` model (same as univr — no user accounts)
- Bilingual Italian/English — Italian primary for 5R content (Monica's original), English alongside

### Schema — New Tables

- `bioenergetics_5r_log` — practice log (client_hash, exercise_name, duration_minutes, difficulty 1-5, notes)
- `bioenergetics_checkins` — state check-in (client_hash, energy/tension/mood 1-10, sleep_hours)

Both tables: anon RLS (insert/select true), `created_at` default now().

### Backend — New Public Endpoints (generalLimiter)

| Method | Path                                 | Purpose                                                                    |
| ------ | ------------------------------------ | -------------------------------------------------------------------------- |
| POST   | `/api/bioenergetics/5r/log`          | Log a 5R practice session                                                  |
| GET    | `/api/bioenergetics/5r/history`      | Recent practices (?client_hash)                                            |
| POST   | `/api/bioenergetics/checkin`         | Submit state check-in                                                      |
| GET    | `/api/bioenergetics/checkin/history` | Recent check-ins (?client_hash)                                            |
| GET    | `/api/bioenergetics/stats/weekly`    | Weekly aggregates (avg energy/mood/tension/sleep, total practices/minutes) |

### Frontend — Components

- `BioenergeticsPage.tsx` — root wrapper (dark immersive theme, lang provider, GDPR banner)
- `BioenergeticsContent.tsx` — tab bar + section switching + check-in + stats footer
- `Bioenergetics5RMethod.tsx` — 5 levels, relaxation techniques, color therapy, practice log form + history
- `BioenergeticsPhilosophy.tsx` — Klages/Reich/Peat with key concepts per thinker
- `BioenergeticsDiet.tsx` — 6 principles (rice/fish/veg/ferments/slow-cook/circadian) + hippocampal degeneration causes
- `BioenergeticsWayOfLife.tsx` — 7 principles + two kinds of life (yeast-like vs higher)
- `BioenergeticsCheckin.tsx` — energy/tension/mood sliders + sleep hours input
- `BioenergeticsStats.tsx` — weekly bar chart averages + practice summary cards
- `BioenergeticsGDPRNotice.tsx` — dismissible privacy banner (localStorage)

### Source Files

- `bioenergetics/monica_pdf.txt` — 5R Method content in Italian
- `bioenergetics/Default_Klages,_Reich,_and_Peat__Towards_a_Bioen.txt` — bioenergetic philosophy
- `bioenergetics/[SOC BRO] bronze age mindset.txt` — way of life principles

### Expansion — Interactive Tools + SpeedDial Nav (2026-06-08)

Eight interactive tools added; top tab bar replaced by a floating SpeedDial.

**Guided sessions** (`BioenergeticsGuided.tsx`) — full-screen player launched from 5R tab: **4D breath** (belly→lungs two-stage wave, slow, custom SVG), diaphragmatic/box/4-4-8/4-7-8/6bpm, color meditation, grounding, progressive relaxation, autogenic training.

**New tool components** (all bilingual, dark theme):

- `BioenergeticsLifting.tsx` — 1RM (Epley+Brzycki), warmup ramp, %1RM load table, per-lift PR tracker (localStorage)
- `BioenergeticsMetabolic.tsx` — **backend-logged** pulse + temperature (Ray Peat/Broda Barnes). Slot-aware (waking/post-meal/afternoon), zone interpretation, dual 30-day trend charts, reference ranges
- `BioenergeticsColdHeat.tsx` — hormetic cold/heat timer, 8 protocols, round countdown + audio, weekly log (localStorage)
- `BioenergeticsCircadian.tsx` — 7-habit daily checklist, score ring, aligned-day streak, 7-day history (localStorage)
- `BioenergeticsPosture.tsx` — Lowen grounding/posture guided hold sequence (5 positions, Reich-zone tags)
- `BioenergeticsSpeedDial.tsx` — framer-motion FAB navigator (6 sections: Practice/Body/Mind/Nutrition/Rhythm/Knowledge)

**New table**: `bioenergetics_metabolic` (client_hash, slot, temperature_c, pulse, anon RLS) — `migrations/2026-06-08-bioenergetics-metabolic.sql` (applied to prod).

**New endpoints**: `POST /api/bioenergetics/metabolic`, `GET /api/bioenergetics/metabolic/trend`, plus earlier `GET /streak` and `GET /stats/monthly`.

Navigation: `BioenergeticsContent.tsx` rewritten — Tabs → grouped sections + SpeedDial; check-in/stats/export remain global; section persisted in sessionStorage.

---

## PROPOSAL — Bioenergetic Praxis (2026-06-08)

Restructure the core app so its mechanics embody the same bioenergetic
principles as the `/bioenergetics` toolkit and Axiom's imprint. Energy charges
and discharges; a life is not meant to push without pause; warmth over
pressure; never reinforce helplessness; adapt to the user's charge.

### Status

- **Done** — Axiom imprint (`BIOENERGETIC_IMPRINT`, `AICoachingService.getIdentity`).
- **Done (prototype)** — `currentCharge` signal (depleted/steady/charged) synthesized
  from recent mood + check-in recency in `AxiomPersonaService`, surfaced in the
  persona block so Axiom calibrates restoration vs action. No migration (JSONB).
- **Proposed** — the product-mechanics changes below.

### Principle → mechanic

| Principle                                         | Current (often anti-bioenergetic)                | Proposed                                                                                |
| ------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------- |
| Charge → discharge → rest                         | Streak rewards unbroken output                   | Make **recovery a logged, rewarded state**; reward the cycle, not the grind             |
| Anti-helplessness (Peat/Seligman)                 | Broken streak / lost bet = shame, penalty spiral | A missed day offers **one small winnable action** to re-charge; never a penalty cascade |
| Circadian / light-heat                            | Daily loop is time-agnostic                      | Anchor brief to wake+light, recap to wind-down; surface energy/pulse-temp app-wide      |
| Reich de-armoring (flow not block)                | Stuck goals sit greyed                           | Detect **armored nodes** (no movement) and nudge the smallest unblocking action         |
| Two kinds of life (structure vs stress-expansion) | Leaderboard/rank = comparison stress             | Default to **self-referential progress + cooperative pods**; rank opt-in                |
| Adapt to charge                                   | Engagement pushes uniformly                      | App **backs off when depleted, pushes when charged** (uses `currentCharge`)             |

### Lowest-risk next slices (additive, reversible)

1. **Energy on check-in** — **DONE (2026-06-08)**. Nullable `checkins.energy_level`
   (1-5) + a 5-dot selector in `CheckInWidget`; `POST /checkins` accepts `{energy}`;
   `AxiomPersonaService.currentCharge` prefers it over the mood proxy. Additive.
2. **Recovery state** — **DONE (2026-06-08)**. `checkins.is_recovery` + a "🌙 Recovery
   day" toggle in `CheckInWidget`. A recovery check-in sustains the streak (rest
   logged counts — no penalty) and reads as depleted charge. Additive; no streak-break
   change (missed-day absorption stays the streak shield's job).
3. **Charge-aware quests** — **DONE (2026-06-08, light)**. `GET /gamification/quests`
   returns `charge` (`utils/charge.ts`); `DailyQuestsWidget` frames by it — depleted →
   "optional today, one small win" banner; charged → "reach for the stretch". Quest
   selection RNG (Postgres `get_daily_quests_for_user`) is left untouched; SQL-level
   difficulty by charge remains a future, riskier step.

### Risk

Done items are additive (no scoring/streak/quest-engine change). The deeper version
of item 3 (mutating the quest-selection SQL) should ship behind a `system_config` flag.

---

## Onboarding Simplification Plan (2026-05-29)

### Problem

Current flow: Welcome → Profile (6 fields) → Domains → Finish → Goal Selection = **5 screens, 9 fields, 2 required**.

- Step 0 (Welcome) and Step 3 (Finish) = pure dead weight, no input
- Domain selection is redundant — first goal reveals domain
- Bio/occupation/education/age/username are not needed before using the app
- Username uniqueness async check adds latency and cognitive load
- "Identity verification coming soon" erodes trust

### Target

**1 screen, 2 fields, < 30 seconds to first goal.**

### New flow

```
Screen 1 (only screen):
  - "What's your name?" (required, text)
  - "What are you working on right now?" (required, text → becomes first goal node)
  - Optional: tap avatar to upload photo
  - Submit → create profile + create first goal node + navigate to /dashboard
```

### Deferred to /profile nudge (shown after 3 days)

- Username (auto-generated from name: `giovanni.p`)
- Age range
- Bio, occupation, education
- Additional domain interests

### Implementation

1. Replace 4-step `OnboardingPage` with single-screen `OnboardingPageV2`
2. On submit: upsert profile (name, avatar) + POST /goals/:userId/node (first goal text)
3. Domain: server-side keyword classifier on goal text → auto-assign, user can change later
4. `onboarding_completed` set immediately on submit (not gated on goal-selection page)
5. Remove `GoalSelectionPage` from onboarding guard — it stays accessible but is no longer mandatory
6. Add `ProfileNudgeCard` to /dashboard: "Complete your profile" shown days 3–7 for new users with incomplete profiles

---

## External oracles: native health + LinkedIn (2026-08-13)

- **Native Health Connect oracle (`health_connect`)** — the Android shell now
  carries a plain-Java Health Connect bridge (`HealthConnectPlugin.java`,
  registered in MainActivity like the existing UsageStats plugin — no third-party
  plugin). It reads steps/calories/weight from Google Health Connect, the
  aggregate of MyFitnessPal, Yazio, Fitbit, Google Fit and Samsung Health, so
  those apps work without their own (MFP's public API is gone, Yazio's is
  partner-only). The bridge posts one sample per user-day to
  `POST /oracle/health-sample` (today/yesterday only — no backfill), and the
  `health_connect` oracle verifies goals from the `health_samples` table at
  trust 0.7 (self-device). `evaluateHealthWindow` is pure + unit-tested
  (7 tests). Migration: `migrations/2026-08-13-health-samples.sql`.
- **LinkedIn oracle (`linkedin`)** — OAuth (auth-url/callback, verified identity
  via /userinfo) for professional goals. LinkedIn has no public activity feed
  for third parties, so this proves account ownership, not activity. Env-gated
  (`LINKEDIN_CLIENT_ID/_SECRET/_REDIRECT_URI`).
- **Strava** — existing code, now ready to go live once `STRAVA_*` env is set.
- **UI** — Settings → Auto-proof providers gains a LinkedIn row and a
  "Health (steps/weight/calories)" section; the dashboard shows a
  `NativeHealthCard` (Android shell only, auto-posts today's sample).
- Manual actions in `manual_actions.txt` item 18.

## OAuth2 provider factory + diet/study oracles (2026-08-13)

- **Generic OAuth2 factory** — `src/services/oauth2OracleFactory.ts` turns a
  config object (auth/token endpoints, scope, profile fetch, verify rule) into a
  full `OracleProvider`: `isConfigured()`, `getAuthUrl()`, `handleCallback()`
  (token exchange + verified connection upsert), and `verify()` (profile check,
  401→refresh→retry). New providers are a config, not a copy-paste of the
  GitHub/Strava/LinkedIn flow. 8 unit tests.
- **Cronometer (diet)** — verifies nutrition goals (calories/protein/carbs) from
  the user's logged diary. Env-gated: `CRONOMETER_*`.
- **Quizlet (study)** — verifies study goals (sets studied in a window) from real
  study activity. Env-gated: `QUIZLET_*`.
- **`scripts/set-oracle-env.sh`** — edit the values at the top and run; writes
  the Strava/LinkedIn/Cronometer/Quizlet env vars to `.env` and restarts the
  backend, turning each provider on in one step.
- UI: Settings → Auto-proof providers now auto-lists Diet (Cronometer) and Study
  (Quizlet) rows; buttons only appear once the env is set.
- Manual actions: `manual_actions.txt` #19.

## Android release tooling (2026-08-13)

- `client/android/build-release.sh` — one command for debug APK, signed release
  APK, or the Play-Store AAB (`cap sync` + gradle). Requires an Android SDK
  machine (not this box).
- `client/android/key.properties.example` + signing config in `app/build.gradle`:
  release builds sign automatically when `key.properties` exists (gitignored),
  stay unsigned otherwise. Keystore never committed.
- Full build + Play Console + Health Connect declaration runbook:
  `manual_actions.txt` #20.

## Native Android app replaces the Capacitor shell (2026-08-24)

The WebView shell (`app.praxisweb.xyz` v0.0.2) is **gone**. `client/android/`
is now a full Jetpack Compose app — the merge of the abandoned `android-native/`
prototype (from the stale `/home/gio/dev/praxis_webapp` clone, branch
`freerouting-omniroute`) with the native-only pieces the Capacitor line had
built on top of the webview. One codebase, one identity.

- **Identity preserved**: applicationId stays `app.praxisweb.xyz`, versionCode
  3 (1.0.0), signed with the same keystore as the published build (SHA-256
  `410acc47…`, password in the local `key.properties`). Existing installs
  update in place; the internal Kotlin packages remain `com.praxis.android`.
- **What came from the native prototype** (~90 files): Room offline-first cache
  - pending-mutation write queue with idempotency keys replayed by a
    WorkManager sync worker; Retrofit API layer (190+ endpoints against
    `praxisweb.xyz/api/`); ~70 routes covering the web app's route map;
    biometric setting; DataStore settings; deep links; share target.
- **Firebase stripped** — Firestore chat and FCM were removed (no
  `google-services.json` existed anywhere; chat now polls the REST API every
  5 s while a conversation is open and reads from Room offline). Push is a
  follow-up that should go through the backend's existing push routes.
- **What was ported from the Capacitor line**: all four home-screen widgets
  (streak / today / capture / logs) with their RemoteViews layouts and the
  snapshot architecture — `WidgetRefreshWorker` fetches `/api/widget/summary`
  every 15 min, refreshes the Supabase session directly (URL + publishable key
  are BuildConfig constants; expiry parsed from the JWT `exp` claim at login),
  and `WidgetStore` keeps the credential in EncryptedSharedPreferences while
  the snapshot sits in plain prefs (widget redraws must not pay keystore
  latency); the UsageStats screen-time bridge became
  `util/ScreenTimeManager.kt` + a Settings screen ("Social media minutes &
  detox proof") that can post today's number to the detox oracle.
- **Deep links**: `praxis://checkin|notebook?capture=1|planner|trackers` from
  widgets, `https://praxisweb.xyz/*` paths, `praxis-auth://` OAuth returns
  (parsed in `SessionScreens.AuthCallbackScreen`; scheme must be allow-listed
  in GoTrue's redirect URLs), and the ACTION_SEND share sheet into the
  notebook composer.
- **Placeholder screens made real**: wiki browser/index/graph/cells, uberwiki
  hub (+ new `/api/uberwiki/*` client endpoints), ontology, aura-web, gantt /
  plan / trajectory, coherence trio, lattice, marketplace, MCP chat, words,
  admin, cohorts, seasonal events, trackers, camera (system camera via
  FileProvider → notebook entry), CV and credential verify pages, Physis.
- **Release size**: minified release APK ≈ 2.9 MB (was 15 MB shell, 93 MB
  before that). Build: `cd client/android && ./gradlew assembleDebug` or
  `bundleRelease`. Signing material stays gitignored; the correct keystore
  password lives only in local `key.properties`.

## Device power features + pro media/AI (2026-08-24, in flight)

Extends the native app with the device surfaces a journal should have, and
sells the same media/AI powers as Pro features on the web. Layers:

### Backend

- `POST /api/notebook/ai-scan` (`requirePro`) — free-note analysis over the
  existing AICoachingService fast tier: summary, themes, sentiment, extracted
  action items, suggested tags. Structured JSON so both clients render it.
- `public/.well-known/assetlinks.json` — app-link verification for
  praxisweb.xyz (native app opens https links without the disambiguation
  chooser).
- Tracker logs already accept free-form `data`, so **media tracker logging**
  rides in `data.attachments` — no migration. Notebook entries already carry an
  `attachments` jsonb column fed by Supabase Storage (`notebook-files` bucket,
  path `notebook/{userId}/{ts}.{ext}`) — no migration either.

### Native Android

- Permissions: RECORD_AUDIO, fine/coarse location, READ_CONTACTS.
- Media capture screen: photo (system camera), audio (MediaRecorder, m4a),
  video (CaptureVideo, capped) → upload straight to Supabase Storage with the
  user's JWT → attach to a notebook entry or log against a tracker.
- Offline uploads: new `pending_uploads` Room table; SyncWorker drains it
  before replaying pending mutations, so captures taken offline land later.
- Location: framework LocationManager one-shot; optional auto-tag on composer
  and media saves (entries already take lat/lng).
- Contacts: READ_CONTACTS list + per-contact invite share (SMS or share sheet).
- Widgets: QuickLogWidget (four one-tap tracker buttons that enqueue through
  the offline queue — they work with no network), VoiceNoteWidget (one tap
  straight into audio capture via `praxis://capture-audio`).
- AI scan: "Analyze" on the notebook composer calls ai-scan and renders the
  structured result.

### Webapp Pro gating

- premiumTiers FEATURES gains `media_tracker_logging` and `ai_note_scan` rows.
- NotebookPage: AI scan button (Pro) with result panel.
- QuickLogDialog: photo/audio/video attachment row (Pro).

## Android client consolidation (2026-08-24)

One canonical native app lives at `client/android/` on `main`. Every other
copy is gone or archived, with its history preserved on GitHub:

- **`archive/native-app-prototype`** — the full 20-commit history of the
  Compose prototype from the stale clone (`/home/gio/dev/praxis_webapp`,
  formerly symlinked as `/home/gio/praxis_webapp`, branch
  `freerouting-omniroute`). That branch on origin was an older point of the
  same line and was deleted; the archive supersedes it. The stale clone,
  the GitHub tarball snapshot (`praxis_webapp-main`) and the standalone
  `~/Praxis/praxis-android` repo moved to cold storage at
  `/home/gio/dev/.archive/praxis-clients/` (README inside explains each).
- **`android-extract`** — fast-forwarded to include the standalone repo's
  final docs commit (`3684cf3`), so the subtree-split history + its README now
  live only here.
- `~/dev/praxis-backend` still embeds an old Capacitor `client/android`
  (state of upstream `5095342`, zero unique work) — safe to delete in that
  checkout whenever convenient; left alone because it is a separate working
  tree.

### Backend repo consolidation (2026-08-24, same sweep)

`github.com/ilPez00/praxis-backend` was already archived/read-only on GitHub;
its pushed history is a strict ancestor of webapp `main`. The local full-stack
checkout (`~/dev/praxis-backend`) moved to
`~/dev/.archive/praxis-clients/backend-fullstack-checkout`. Its single
unpushed commit — the original Duel-a-Friend SpeedDial variant, later
reimplemented on `main` — is preserved as branch
**`archive/backend-pre-consolidation`**. Uncommitted relicensing work found in
that checkout (MIT → BUSL-1.1 + package.json rename) was **not** applied to
the webapp; it is saved as
`.archive/praxis-clients/praxis-backend-uncommitted.patch` awaiting a decision.

## Health Connect restored + Calendar screen + notebook share-to (2026-08-24)

- **Health Connect bridge back (Kotlin, native)** — commit `ebd8cb15` dropped the
  Capacitor-era plugin; this rebuilds it natively for the Compose app:
  `health/HealthConnectBridge.kt` reads today's steps / calories / latest weight
  (yesterday fallback for empty mornings) from Google Health Connect — the
  aggregate of MyFitnessPal, Yazio, Fitbit, Google Fit and Samsung Health.
  Permissions use the alpha07 ActivityResult contract
  (`createRequestPermissionResultContract`); minSdk stays 24 via
  `tools:overrideLibrary`, every HC touch guarded by SDK_INT >= 26.
  - UI: `HealthScreens.HealthConnectScreen` (route `health`, deep link
    `praxis://health`) — grant → see today's numbers → "Sync to oracle".
  - Auto-push: `SyncWorker` posts one sample per run (soft-fail) so the
    `health_connect` oracle keeps verifying goals without user action.
  - API: `POST /oracle/health-sample` wired through `PraxisApi.submitHealthSample`.
- **Calendar (Android)** — `CalendarScreens.CalendarScreen` (route `calendar`,
  `praxis://calendar`): next 14 days of Google Calendar events (when linked)
  merged with last-7-days tracker activity. Web side gets the missing
  **Settings → Google Calendar** connect card (`GoogleCalendarCard.tsx`),
  which SpeedDial's plan sheet already pointed at. Backend OAuth flow unchanged.
- **Share-to on notebook entries** — each entry card in the native notebook gains
  a Share button (system `ACTION_SEND` chooser: WhatsApp/Telegram/mail…).
  Notebook home also gains 📅 Calendar / ♥ Health buttons.
- **Artifacts**: versionCode 4, versionName 3.1.0 (aligned to web line).
  ⚠️ Signing caveat: local `praxis-keystore.jks` was replaced at 13:55 and now
  yields cert `989bece3…` while the published v3.0 build is `410acc47…`
  (a `wrongkey-44AF.bak` sits beside it). Play will REJECT an update signed
  with a mismatched key — resolve which jks is the real upload key before the
  next store upload. Debug builds unaffected.

## Android graphical parity with the webapp + feature hardening (2026-08-26)

The WebView shell (default UI mode) already mirrors praxisweb.xyz pixel-for-pixel;
this cycle closes the visual gap for the **native Compose mode** and fixes a
latent GPS bug. versionCode 9, versionName 3.2.4.

- **Theme slots pinned to the web ramp** — `ui/theme/Color.kt` now fills every
  M3 slot the Compose baseline used to leave as off-brand purple-grays:
  surfaceContainer* → #080808/#0f0f0f/#111111/#141414/#1a1a1a (web bg/input/
  surface/raised), outline #222222, outlineVariant #2a2a2a, primaryContainer
  amber-tinted #3d2f18, secondaryContainer violet-tinted #2c2544. Cards, chips,
  dividers, text-field outlines and chat bubbles now render web colors with no
  per-screen edits. `PraxisColors` exposes the shared constants (success/info,
  amber ramp, avatar gradient). `PraxisTheme` is now explicitly always-dark.
- **Typography parity** — `ui/theme/Type.kt` ships Inter (body), Plus Jakarta
  Sans (display/headings) and JetBrains Mono (chrome micro-labels) as font
  resources; PJS/JB Mono are variable TTFs driven through `FontVariation` on
  API 26+. Type scale mirrors index.tsx (weights, line-heights, negative
  tracking on display sizes). `LabelMonoSmall` reproduces the web's uppercase
  bold mono micro-label signature.
- **Design-system primitives** — new `ui/components/design/PraxisUi.kt`:
  `PraxisCard` (surface fill + 1px #222 border r12 / raised variant),
  `PraxisButton` (135° amber→dark-amber gradient + glow shadow, violet variant,
  amber outline; r10, 44dp min, near-black label), `PraxisChip`
  (translucent accent tint .12/.25), `PraxisSectionLabel`, `PraxisEmptyState`,
  `PraxisTopBar` (48dp header + back chevron + 1px bottom border).
- **Bottom nav rebuilt** — `PraxisBottomNav` now matches `layout/BottomNav.tsx`:
  56dp row on surface/95 behind a 1px top border, outline icons, mono uppercase
  labels, spring-animated amber top-edge indicator, web-identical tab families
  (NOTEBOOK/SOCIAL/ME prefix matching) + saveState/restoreState tab switching.
- **Screens migrated** — ~76 Button sites → PraxisButton and ~40 Card sites →
  PraxisCard across all screens; destructive actions keep raw red buttons.
  `DataUi.ScreenScaffold` renders the real PraxisTopBar instead of a stacked
  Back button, so every JSON-driven screen gets the web chrome at once.
- **Chat bubble fix** — own messages rendered on the wrong side: ChatScreen
  compared senderId against the partner id; now uses the exposed
  `ChatViewModel.meId`. Own bubbles sit right on amber-tinted container.
- **GPS bug fix (functional)** — nothing ever _requested_
  ACCESS_FINE_LOCATION/COARSE at runtime, so "Tag with current location"
  silently produced null coordinates for everyone. CaptureScreens now asks at
  switch-flip time (matching the app's request-at-use pattern), degrades to an
  explanatory status when denied, and warns when no provider is enabled
  (`LocationHelper.isAnyProviderEnabled`).
- **Status bar / shell backdrop** — hardcoded #0F1115 → #080808 (web --color-bg)
  in MainActivity and the WebView background.
- **Feature audit result**: camera (TakePicture/CaptureVideo + FileProvider +
  Supabase upload + offline queue), GPS (LocationHelper one-shot, now with
  runtime grant), steps (HealthConnectBridge ≥26 guard + SyncWorker
  attestation), app connections (share-to target, praxis:// deep links,
  https App Links, praxis-auth:// OAuth return, READ_CONTACTS picker,
  FileProvider share-out) and offline (Room caches, PendingUpload/Mutation
  queues drained by SyncWorker with idempotency keys, connectivity interceptor
  attestation), app connections (share-to target, praxis:// deep links,
  https App Links, praxis-auth:// OAuth return, READ_CONTACTS picker,
  FileProvider share-out) and offline (Room caches, PendingUpload/Mutation
  queues drained by SyncWorker with idempotency keys, connectivity interceptor
  fast-fail, WebView offline panel) all verified wired end-to-end.

## Settings completeness pass (2026-08-28)

Active Roadmap A–H was complete; this pass fills the dead/partial Settings
controls instead of starting new surface area.

- **Real "Export My Data"** — `GET /settings/export` (auth-guarded, behind
  `authenticateToken`). `src/controllers/userExportController.ts` aggregates the
  caller's own rows across `profiles`, `goal_trees`, `notebook_entries`,
  `checkins`, `trackers`, `tracker_entries`, `posts`, `user_achievements`,
  `bets`, `matches` (filtered by `user_id`, or `id` for profiles — never
  another user's data) and streams a pretty-printed JSON blob with a
  `praxis-data-<id>.json` `Content-Disposition`. Frontend `SettingsPage`
  fires `api.get('/settings/export', { responseType: 'blob' })` and triggers an
  anchor download with a `CircularProgress` + error toast. Test:
  `src/__tests__/userExport.test.ts` (self-export 200, unauth 401-via-next).
- **Wired "Interface Language"** — selector now calls `i18n.changeLanguage`,
  persists to new `profiles.language` (default `'en'`), and the load effect
  reconciles `<html lang>` + state from the profile on mount. Sub-label notes
  coverage is English-first.
- **Persisted visibility toggles** — `match_visibility` → `profiles.match_visibility`,
  `profilePublic` → `profiles.is_public` (both already existed, were unwired),
  and `shareReflections` → new `profiles.share_reflections`. All write through
  the existing `supabase.from('profiles').update(...)` pattern with rollback +
  toast. Load effect hydrates the three from the profile.
- **Schema (additive, default-safe)** — migration `2026-08-28-profile-language.sql`
  adds `profiles.language text NOT NULL DEFAULT 'en'` and
  `profiles.share_reflections boolean NOT NULL DEFAULT false`.
- **Out of scope (noted follow-up):** the uberwiki/search backend does not yet
  _read_ `share_reflections`/`match_visibility`/`is_public` to gate results;
  the toggles now persist correctly but the gating logic is a separate task.
  fast-fail, WebView offline panel) all verified wired end-to-end.

## Premortem + validation pass (2026-08-28)

Full premortem: `docs/PREMORTEM.md`. This section records what was validated and
what changed in the plan as a result.

### Validation run this cycle

- **Backend regression (targeted):** 42/42 jest pass across `matchVisibility`,
  `MatchingEngineService`, `publicPlans`, `roomMembership` (the surfaces touched
  by the settings/matching work). The full 86-file suite is not run in-harness
  (30s command cap; process-group wait blocks detached runs) — the subset covers
  the changed surface.
- **Frontend:** `npm test` (vitest run) → **35 files / 184 tests passed** (9.4s).
- **UI capture:** Playwright over the prebuilt `client/dist` via `vite preview`,
  using the system Chromium (`/usr/bin/chromium`) — no browser download. 10
  screenshots (desktop + mobile × root/login/terms/privacy/widget) + a textual
  `ui-report.json` written to `/tmp/shots` and preserved under
  `docs/screenshots/2026-08-28/`. Script: `scripts/ui-screenshot.cjs`.
  - Landing `/` and `/login` render correctly (real H1, value prop, FAQ accordion,
    "bring your own AI" CTA, email/password + Google + language selector).
  - `/terms` + `/privacy` render **substantive** content with **real operator
    details** (Giovanni Pezzin, Via Ugo Foscolo 31, 31033 Castelfranco Veneto,
    gio@praxisweb.xyz) and an honest "not yet reviewed by a lawyer" notice.

### Corrections to this plan

- **STALE note:** the earlier "Terms of Service + Privacy Policy" entry says
  three placeholders `[OPERATOR LEGAL NAME]` / `[CONTACT EMAIL]` / `[POSTAL
ADDRESS]` still need filling. They are **already filled** (verified live this
  cycle). Remaining work = lawyer review only, not copy placeholders.
- **Still open (from the settings pass):** the uberwiki/search backend does not
  yet _read_ `share_reflections` / `match_visibility` / `is_public` to gate
  results — the toggles now persist but the gating logic is a separate task.

### New defects found (UI inspection)

- **[PROD BUG] Sentry DSN broken** — `client/.env` has
  `VITE_SENTRY_DSN_REACT=https://o429798.ingest.sentry.io/5` (malformed: no
  public key, truncated project id). `Sentry.init` throws `Invalid Sentry Dsn`
  on **every page load** → client error monitoring is dead in prod and the
  console is polluted on every route. `manual_actions.txt` documents Sentry as
  "off by design," so the fix is to **clear** the DSN (clean disable path in
  `client/src/lib/sentry.ts`) **or** supply a valid key. Either removes the
  error. Added to `manual_actions.txt` (2026-08-28).
- **[MINOR] Bare `/widget` 404s** — the embeddable streak widget is backend-
  served; the bare frontend `/widget` route shows "Page not found." Not
  user-facing in normal flow (the widget is embedded with a user param). Logged,
  not fixed.
- **[a11y MINOR] Login page** — duplicate label text (`Email Address * Email
Address *`) and no `<h1>` element. Cosmetic; fix when the auth screen is
  next touched.

### Plan implications

The premortem (`docs/PREMORTEM.md`) ranks the real kill risks as GTM
non-execution (F1), real-money legal (F2), pansa SPOF (F3) and the activation
gap (F5) — **not** code quality. Engineering is a strength here. Human-gated
actions carried into `manual_actions.txt`: Sentry DSN fix, legal review,
offsite DB backup, `NODE_ENV` flip, and the outreach-only weeks rule.

## 4. UI/UX hardening (2026-08-29)

- **Modal focus trap + restore** on the SpeedDial bottom sheets (`client/src/components/common/SpeedDial.tsx` `Sheet`): the `role="dialog" aria-modal="true"` panel now traps `Tab`/`Shift+Tab` and returns focus to the trigger on close — closing the one real modal-a11y gap in the otherwise cohesive system. 3 unit tests in `client/src/__tests__/SpeedDial.test.tsx`.
- Audit note: the premium dark theme (`client/src/index.tsx`) + `index.css` already enforce consistency (single elevation ramp, focus-visible gold ring, reduced-motion, documented AA contrast ramp). A broader _visual_ refresh would need an explicit design direction (palette/spacing/type) rather than blind churn.
- **IconButton hover affordance** (`client/src/index.tsx` MuiIconButton): consistent `rgba(255,255,255,0.06)` hover background + transition app-wide, so icon-only buttons read as interactive everywhere.
- **Icon-only accessible names**: added `aria-label` to every icon-only `IconButton` app-wide. Completed via a JSX-aware scan that confirmed **0 genuine misses across all 282 `IconButton`s**. Surfaces covered: Gantt/Plan/Notebook/NoteEdit/DiaryFeed/UnivrSearch, AccountabilityBuddy/UnivrCarpool/BookExchange/Housing, NotesCardTree/NoteGoalDetail/AxiomReply/ChatRoom, EventsPage edit/delete; trackers (EditableTrackerForm, StrengthTrends, TrackerSection, CustomTrackerBuilder, CustomTrackerForm, DomainPlanSection), notebook (AxiomQueryDialog, MediaToLogDialog, NotebookCalendarSection, NotebookEntryCard, NotebookPage, NotebookGanttSection), leaderboard (share), ontology (reset/edit), profile (AchievementsSection, ProfilePage), plan (goal complete/edit/remove + manual task complete/edit/delete), gantt (remove task + trajectory refresh), goals (GoalNodeComponent/GoalNodeDisplay toggle/edit/add/delete/suggest), notes (DayDetailView, GoalNotesPanel emoji/delete, DiaryFeed edit/sort/filter, AuthoringModal send/close, PublicNotebookPage back), bioenergetics (shadow/philosophy/color-meditation/stats/exercises/today), aura-web (camera/mic/capture/send), daily-quests refresh, camera (flip/capture/close), admin tabs (refresh/priority/delete/use-location/promote/toggle), dashboard (DesktopWidget/GoalWidgets/AxiomDailyProtocol/AxiomMorningBrief), coherence (day-nav/remove-task), groups/matches/chat attach/camera/link/reply/back, univr (complaints vote/flag, events vote/delete), uberwiki (query/edit), wiki (graph refresh/close, index copy). Decorative-only buttons use `aria-hidden`. One intentional `aria-hidden` toggle in `AxiomMorningBrief` (duplicates the header expand control).

### Templated enumerations (tracker model v2) — ✅ shipped 2026-08-29

- Schema-free (lives in `trackers.goal` JSONB as `goal.enumeration` + `goal.options`): no migration, self-hosters + Railway get it free.
- Backend: `enumerationSchema` + `EnumerationInput` in `src/schemas/trackerSchemas.ts`; `saveEnumeration` in `src/controllers/trackerController.ts` (PUT `/trackers/:type/enumeration`, merges into `goal` preserving `template_rows`; 400 on bad `enumeration`; 503 on missing table); route wired in `src/routes/trackerRoutes.ts` with `validateBody`. Test: `src/__tests__/trackerEnumeration.test.ts` (3).
- Frontend: `EditableTrackerForm` gains an "Edit template → Predefined options" panel (Switch + newline/comma option list, `Label = n` sets a value) and, in log mode, a chip picker that logs a single option via `POST /trackers/log` (`data: { option, value: 1 }`); options already logged today get a green check. `useTrackerForm` seeds enum state from `goal` and persists it alongside the row template. `parseEnumerationOptions` (pure) in `client/src/features/trackers/trackerFormModel.ts`. Tests: `client/src/__tests__/trackerEnumeration.test.tsx` (7).
- Useful for reading/meditation/study/log — close the "I did a thing" loop without free-form rows.

## 5. Code-first evaluation and work plan (2026-08-31)

Written from the source tree alone — no docs read before the assessment — then
reconciled against `BUSINESS_PLAN.md`, `PREMORTEM.md` and the commercial
framing at the top of this file. Section 5.1 is the measurement, 5.2 the
verdict, 5.3–5.8 the actual plan: numbered tasks, files, acceptance criteria,
and the calendar slot each one is allowed to occupy.

**Calendar position.** The 90-day plan numbers weeks from 2026-08-26. Today is
day 5 — inside **Weeks 1–2, "Outreach only. No code."** Everything in 5.4
onward is therefore _queued_, not started. The only work permitted before
2026-09-09 is T1 (a security take-down, not a feature) and T2/T3 (operational
defects that are already listed in `manual_actions.txt`). If a task below is
started early, the plan it belongs to has already failed in the way the
premortem predicted.

### 5.1 What the repository measures

| Dimension            | Measured 2026-08-31                                                                                                    |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Backend              | 558 TS files · 93,102 lines · 93 routers · ~90 controllers · ~130 services                                             |
| Frontend             | 515 files · 107,701 lines · 77 routes · 54 feature folders                                                             |
| Database             | 195 migrations · ~190 tables · 6 migrations using `vector(...)`                                                        |
| Scheduled work       | 21 `cron.schedule` call sites                                                                                          |
| Commits              | 1,734 since 2026-02-06, effectively one author                                                                         |
| Backend tests        | 120 suites · 1,180 tests · all passing (74s)                                                                           |
| Client tests         | 48 files · 246 tests · all passing (16s)                                                                               |
| Type looseness       | ~2,162 `: any` / `as any` occurrences across both projects                                                             |
| Client build         | 4.9 MB `client/dist`, code-split per route                                                                             |
| Routers without auth | 10 (`auth`, `bioenergetics`, `campusExam`, `funnel`, `health`, `media`, `publicWidget`, `university`, `univr`, `wiki`) |

Growth since the 2026-08-26 business-plan snapshot: +152 commits, +56 backend
files, +32 client files, in five days. Users over the same window: unchanged.

### 5.2 Verdict

Four products share one goal tree: (1) the accountability journal — goal tree,
13 tracker types with structured Postgres mirrors, check-ins, streaks,
offline-first PWA; (2) **Axiom** — ~45 services of LLM layer over that data,
with a ~40-provider fallback registry and local embeddings; (3) social and
commitment — rule-based matching, peer verification with anti-collusion,
duels, groups, cohorts, public widgets; (4) money and B2B — Stripe Pro / Boost
/ Physis, PP packs, real-money commitments, marketplace, managed access, orgs,
an agent-priced API, MCP, public OpenAPI.

**Defensible:** `ManagedAccessService` (one gate, refusals logged, `health`
scoped apart from `trackers` — this is what makes a coach or clinic pilot
sellable and almost nothing in the category has it); the offline six-language
lexicon with cross-language concept ids; the verified-outcome primitives
(anti-collusion limits, credentials, oracles, public verification pages);
self-hosted provider-agnostic inference, which makes the marginal cost of a
pilot user ≈ €0.

**Not defensible:** volume. Of the ~200k lines, the portion that serves the
named B2B wedge is the goal tree, trackers, check-ins, verification, managed
access, orgs and the brief. The rest is optionality with a permanent
maintenance bill.

The asset is engineering-rich and demand-poor, and the code confirms rather
than contradicts §1 of the business plan: 47 registered, 1 weekly active, €0.

### 5.3 Premortem delta (against `PREMORTEM.md`, 2026-08-28)

`PREMORTEM.md` F1–F8 stand. Two amendments:

- **New, ranked above F5 — death by surface area.** One maintainer cannot hold
  93 routers, ~190 tables and 21 cron jobs correct _while_ selling. It kills by
  drift, not by outage: features half-finished, migrations half-applied,
  nothing visibly broken on any single day. It is the mechanism by which F1
  (GTM non-execution) actually happens — retreating into the codebase is easier
  when the codebase always has something that needs doing. Mitigation is 5.5.
- **New, concrete — F9, unauthenticated write surface in `univr`.** Detail and
  fix in 5.4/T1. It is the only finding this pass that is exploitable today
  with `curl` alone.

F4 (Sentry dead) is still open, three days after being logged. That is itself
evidence for the surface-area finding.

### 5.4 Immediate defects — permitted during the outreach freeze

**T1 — `univr` write endpoints accept forged identity. (P0, ~30 min for the
take-down; ~3h for the real fix.)**

`src/routes/univrRoutes.ts` mounts, with no auth middleware and a comment
saying so deliberately:

```
POST   /api/univr/complaints/:id/vote      → handleVote
PATCH  /api/univr/complaints/:id/flag      → handleFlag
DELETE /api/univr/complaints/mine          → handleDeleteMine
POST   /api/univr/elections                → handleCreateElection
POST   /api/univr/elections/:id/vote       → handleCastVote
PATCH  /api/univr/elections/:id            → handleUpdateElection
DELETE /api/univr/elections/:id            → handleDeleteElection
```

Identity is `client_hash` / `voter_hash`, **read from the request body**. Three
consequences, all live:

1. `handleVote` (`univrController.ts:~30`) reads the current count and writes
   `count + 1` with no dedupe key at all. Unbounded, and racy on top.
2. `handleCastVote` dedupes on `(election_id, voter_hash)` where `voter_hash`
   is chosen by the caller — a loop with fresh random hashes votes as many
   times as it likes. The GVN weight, `sqrt(1 + prior interactions)` counted
   over `univr_complaints|posts|events` filtered by that same attacker-supplied
   hash, is farmable by first posting junk under the hash you intend to vote
   with.
3. `handleUpdateElection` / `handleDeleteElection` / `handleDeleteMine`
   authorise on `election.created_by === client_hash`. That is authorisation by
   knowing a value the client picks; anyone who learns or guesses a creator
   hash owns the election.

A public advisory election whose outcome can be manufactured from a shell loop
is worse than no election — it is a reputational liability attached to a real
university's name.

- **Now (freeze-safe):** comment out the seven write mounts in
  `univrRoutes.ts`, leave the `GET` routes serving, and note it in
  `manual_actions.txt`. No schema change, no client rebuild needed beyond the
  usual deploy.
- **Later (Weeks 3–4 at the earliest, and only if univr survives 5.5):** derive
  the identity server-side — `sha256(university_id || client_ip || daily_salt)`
  with the salt in `.env` and rotated daily — and add a
  `UNIQUE (complaint_id, voter_hash)` constraint so complaint votes dedupe in
  the database rather than in a read-modify-write.
- **Acceptance:** a script that posts 100 votes from one machine moves the
  count by at most 1; `PATCH`/`DELETE` on an election from a non-creator
  returns 403; `src/__tests__/univrAuth.test.ts` covers all three.

**T2 — Sentry DSN malformed (F4, carried from 2026-08-28). (P0, 10 min.)** ✅ RESOLVED 2026-08-31
`client/.env` holds `VITE_SENTRY_DSN_REACT=...` — empty in both dev and prod;
`client/src/lib/sentry.ts` treats empty as "error tracking disabled" without
throwing. The malformed DSN is gone. (Historic text: `https://o429798.ingest.sentry.io/5`
— no public key, truncated project id — so `Sentry.init` threw on every page
load. Decide once: clear the variable or supply a valid DSN. Acceptance: no
`Invalid Sentry Dsn` in the console on a cold load of `/`, `/login` and
`/dashboard`.)

**T3 — Both replication slots invalidated. (P0, ~1h + transfer time.)** ✅ OMO RESOLVED 2026-08-31
Root cause was not the assumed `pg_basebackup` reseed: omo's replica had
reconnected but the `praxis_replicator` password had been rotated on pansa, so
the WAL receiver failed auth every 5s and the slot sagged to `f|reserved`.
Set a new password, updated omo `~/.pgpass` → slot `t|reserved`, lag ~1s,
health exit 0. `air_standby` stays `f|lost` until air is reachable.
`scripts/replica-health.sh` also gained a primary-mode lost-slot check
(`CHECK_SLOTS=1`, fails loudly on any non-ignored `f|lost`) — wired as
`praxis-slot-check.{service,timer}` on pansa (5 min). Both acceptance criteria
met. (Historic text below.)
On 2026-08-30 `pg_replication_slots` reported `f|lost` for both omo (2.0 days
behind) and air (8.2 days), past the 4 GB `max_slot_wal_keep_size` ceiling, so
neither can ever catch up — while `manual_actions.txt` recorded migrations as
applied. Reseed each with `pg_basebackup` per `DR_RUNBOOK.md` §5g.
`scripts/replica-health.sh` does **not** currently query
`pg_replication_slots`; add a check that fails loudly on any row reading
`lost`, and wire it into the same timer that runs the tunnel gate.
**Acceptance:** both slots read `t|reserved`; the health script exits non-zero
against a deliberately dropped slot.

**T4 — Offsite database backup. (P1, ~2h, from `PREMORTEM.md` F3.)** ✅ DONE 2026-08-31 (except air rehearsal)
The nightly had silently drifted to `scripts/praxis-backup.sh` (local-only db +
vault) while a dead `PRAXIS_BACKUP_HOSTS` env sat unused. Now the nightly runs
the full `scripts/backup-praxis.sh` (cluster + portable + auth-data + storage +
config, gpg-encrypted) and replicates to `omo-ts:/home/gio/praxis-backups` — a
different physical machine, ciphertext-only. Verified 2026-08-31: both hosts
hold `praxis-20260831-171051.tar.gz.gpg` (222M), offsite copy decrypts cleanly,
timer active. **Remaining:** restore rehearsal on air (offline) from the
offsite copy, recorded in `manual_actions.txt` with row counts.
(Historic text below.)
**Acceptance:** a restore rehearsal on air from the offsite copy, recorded in
`manual_actions.txt` with the date and the row counts compared.

### 5.5 Subtraction — the roadmap item that pays for the others

Measured lines per feature (backend `src/**` + client `client/src/**`,
2026-08-31). Decision column is a proposal for the 2026-09-09 review, not yet
applied.

| Feature           | Backend | Client | Serves the B2B wedge? | Proposal                       |
| ----------------- | ------: | -----: | --------------------- | ------------------------------ |
| wiki / uberwiki   |   6,032 |  2,624 | Yes — pilot artefact  | **Keep**                       |
| oracle            |   2,698 |    451 | Yes — auto-proof      | **Keep**                       |
| physis            |   1,119 |  1,611 | Separately monetised  | **Keep, own price**            |
| nutrition         |     343 |    272 | Consumer retention    | **Keep**                       |
| bioenergetics     |     437 |  6,882 | No                    | **Quarantine**                 |
| univr             |     389 |  6,119 | No (and see T1)       | **Quarantine after T1**        |
| blog / helloworld |     453 |    903 | Marketing surface     | **Keep — it is top-of-funnel** |
| dreams            |     565 |    117 | No                    | **Quarantine**                 |
| rachmaninov       |     465 |      0 | No                    | **Delete**                     |
| spotify oracle    |     196 |      0 | No                    | **Delete**                     |
| reddit oracle     |     216 |      0 | No                    | **Delete**                     |
| youtube oracle    |     127 |      0 | No                    | **Delete**                     |
| campus exams      |     221 |      0 | No                    | **Delete**                     |
| lattice           |     318 |    404 | No                    | **Quarantine**                 |
| cyclops           |       0 |    434 | No                    | **Delete**                     |
| aura-web          |       0 |    288 | No                    | **Quarantine**                 |

Roughly 19k lines, ~14 routers and their cron jobs and tests come off the
maintained surface. That is the single highest-leverage engineering action
available, because every hour of maintenance it returns is an hour the plan
needs for outreach.

**First tranche DONE 2026-08-31.** The quarantine mechanism is live
(`src/utils/featureFlags.ts`, `PRAXIS_FEATURES` read once at boot; quarantine
is the default). Applied to the backend-only dead features (0 client lines, no
funnel traffic): **rachmaninov, spotify, reddit, youtube, campus exams** — all
404 on production now, re-enable by listing in `PRAXIS_FEATURES`. Client-heavy
features (univr, bioenergetics, dreams, lattice, aura-web) and the code-delete
pass wait for the 09-09 review / 30-day soak, as the plan specifies.

**Mechanics — quarantine before delete, always.**

1. **Quarantine** = the router is not mounted in `src/app.ts` unless
   `PRAXIS_FEATURES` (comma-separated, read once at boot) names it, and the
   client route is removed from `client/src/config/routes.tsx` so the chunk
   stops being built. Migrations and tables stay. Nothing is destroyed and the
   change is one line to revert.
2. **Soak for 30 days.** If nothing and nobody asks for it — check
   `funnel_events` and the access log before deciding — it is dead by
   observation, not by opinion.
3. **Delete** = routes, controllers, services, client feature folder, tests.
   Tables stay until a separate, dated decision; dropping them is the only
   irreversible step and it never happens in the same commit as the code
   removal.
4. One commit per feature, `chore(scope): quarantine X — no customer, no
traffic`, so any single removal reverts cleanly.

**Acceptance for the whole pass:** `npm test` and `cd client && npm test` stay
green; the entry chunk does not grow; `PRAXIS_FEATURES=univr,bioenergetics`
restores both without a code change.

**Explicitly not on this list:** the Android client, the MCP server and the
OpenAPI document. They are distribution, not features, and they cost nothing
per week.

### 5.6 Activation instrumentation — the one feature this plan asks for

`funnel_events` (2026-08-09) already records the pre-account funnel:
`landing_view`, `cta_click`, `pricing_view`, `signup_start`,
`signup_complete`, `onboarding_complete`, `first_goal_created`,
`first_checkin`, `first_duel_created`. It stops exactly where the question
starts. Nothing measures whether the daily loop — brief → check-in → recap —
holds a person past day 2, which is F5 and the reason the paywall sits behind
a cliff.

**T5 (Weeks 3–4, ~1 day).** ✅ DONE 2026-08-31. `brief_opened`, `checkin_logged`
(server-emitted from the check-in controller), `recap_opened` (from
`DoneForTodayCard`), `day2_return`, `day7_return`, `week1_complete` are all in
the allowlist + client union and emitting. `activation_cohorts` view +
`GET /api/admin/activation` ship the signup-week adherence report. See
`manual_actions.txt` for the wiring. Numbers populate as real users act.
Extend the same table rather than inventing a
second system:

- Add to `ALLOWED_EVENTS`: `brief_opened`, `checkin_logged`, `recap_opened`,
  `day2_return`, `day7_return`, `week1_complete`.
- Emit them from the surfaces that already exist (`AxiomMorningBrief`,
  the quick-log sheet, the evening recap), fire-and-forget, same as the
  acquisition events.
- One SQL view, `activation_cohorts`, keyed by signup week: signups, %
  reaching `first_checkin`, % returning day 2, % returning day 7, % completing
  week 1.
- One admin read (`GET /api/admin/activation`) rendering that view. No new UI
  beyond a table.

**Acceptance:** the day-90 review can answer "of everyone who signed up in
week N, how many were still logging on day 7" from one query, without a
manual export. **This report is also the pilot deliverable** — the same
cohort-adherence numbers an org buyer is being sold (`BUSINESS_PLAN.md` §6,
weeks 5–10). Build it once, use it for both.

### 5.7 Correctness debt — scheduled, not urgent

- **~2,162 `any` occurrences.** Do not sweep them. Tighten only the paths where
  a wrong shape moves money or leaks data, in this order: `realBetsController`,
  `stripeController` + the webhook, `ManagedAccessService` call sites,
  `VerificationLimitService`. Everything else waits for the file to be touched
  for another reason.
- **`handleVote`'s read-modify-write** is the pattern, not the instance —
  audit for other counters incremented in application code where a database
  constraint or `rpc` should own it. **✅ AUDITED 2026-08-31.** The two remaining
  app-code RMW counters (`affiliate_offers.clicks`, `campus_exams.confirmations`)
  are now atomic `rpc` increments (`increment_affiliate_clicks`,
  `increment_exam_confirmations`, migration `2026-08-31-atomic-counters.sql`).
  `goal_tree_edit_count` stays as-is: per-user, race-tolerant, secondary metric.
  `realBetsController`/`bettingController` `any`s already minimal and benign.
- **Jest leaks handles** ("a worker process has failed to exit gracefully").
  Harmless in CI today, but it hides real timer leaks in services with crons.
  One pass with `--detectOpenHandles`, `.unref()` where it belongs.
- **Bare `/widget` 404** and the login-page a11y nits (duplicate label, no
  `<h1>`) — fix when those screens are next touched, not before.

### 5.8 Order of work, mapped to the 90-day calendar

| Window                          | Allowed work                                                                                                                                                                              |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Now – 2026-09-09** (W1–2)     | T1 take-down, T2, T3, T4. **No features.** Everything else is outreach.                                                                                                                   |
| **2026-09-09 – 09-23** (W3–4)   | 5.5 quarantine pass (one commit per feature) · T5 activation view · org-admin adherence dashboard **only if a discovery call asked for it** · consumer paywall move (brief after 14 days) |
| **2026-09-23 – 11-04** (W5–10)  | Pilot-driven only. Weekly cohort report from T5. Close gaps in the order pilots hit them. 30-day soak on quarantined features expires ~2026-10-09 → delete pass.                          |
| **2026-11-04 – 11-25** (W11–13) | Conversion. Seat billing only if a customer is blocked on it. Real-money legal review before any outreach that mentions commitments.                                                      |

**Do not, in any window, without a named customer asking:** add a router, add
a tracker type, add an LLM provider, add an oracle, start a new feature folder.
The measurement in 5.1 is what happens when that rule does not exist.

**Kill criteria are unchanged** (`BUSINESS_PLAN.md` §6). Note that none of them
are engineering criteria — the code is not what is being tested at day 90, and
this section exists so that stays true.

## 6. Mobile feature roadmap (2026-08-31) — proposed, scaffolded

Full plan with per-feature rationale, API notes, effort and compliance
constraints: `docs/MOBILE_FEATURE_PLAN.md`. Android scaffolds are in this
commit and compile; none are wired into the UI yet.

The audit finding behind the plan: the native layer already has the sensors
(UsageStats, Health Connect, HealthKit, five widgets, biometric, location,
camera, mic). What it does not have is **the operating system's assistant, the
lock screen, and the other apps on the phone** — the three places a habit app
must live to be opened without willpower.

| #   | Feature                                        | Scaffold                                                        | State                                                                             |
| --- | ---------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| 1a  | Launcher shortcuts (+ assistant entry point)   | `feature/assistant/PraxisShortcuts.kt`, `res/xml/shortcuts.xml` | static set wired in manifest; dynamic per-tracker push ready, not called          |
| 1b  | iOS App Intents / Siri                         | `praxis-ios/Praxis/Intents/PraxisIntents.swift`                 | scaffold only, never compiled (no Xcode on this machine)                          |
| 2   | Duel progress on the lock screen               | `feature/duel/DuelLiveNotifier.kt`                              | ongoing notification + channel; needs push-driven updates                         |
| 3a  | Health-verified commitments                    | `feature/health/CommitmentVerifier.kt`                          | reads `ExerciseSessionRecord`; `READ_EXERCISE` added to manifest                  |
| 3b  | Detox duels on real app minutes                | `feature/detox/DetoxDuelEvaluator.kt`                           | settles against `ScreenTimeManager.perApp`; needs daily sampling in `SyncWorker`  |
| 4a  | LinkedIn "add to profile" credential           | `feature/credential/LinkedInCredential.kt`                      | URL builder complete; needs OG image on the public credential page                |
| 4b  | Instagram Stories share                        | `feature/share/StoryShare.kt`                                   | intent complete; needs the 1080×1920 card renderer                                |
| 5   | NFC tap check-in                               | `feature/nfc/NfcCheckInActivity.kt`                             | manifest-registered; needs tag-writing UI and server provenance                   |
| 6   | Biometric gate on real-money stakes            | —                                                               | uses existing `util/BiometricAuth.kt`; highest value-to-effort item on the list   |
| 7   | On-device voice transcription                  | `feature/voice/OnDeviceTranscriber.kt`                          | complete; needs wiring to `VoiceNoteWidget`                                       |
| 8   | Axiom tier routing (own GPU → device → server) | `feature/ai/AxiomRouter.kt`                                     | mesh probe + fallback; **blocked on a scoped network security config**, see below |

### Decisions recorded

- **Instagram is a screen-time signal and a share target, nothing else.** The
  Basic Display API is dead; there is no supported way to read a feed. Any
  feature premised on reading Instagram content should be cut at design time.
- **LinkedIn needs no API.** The "add to profile" certification URL takes
  prefilled parameters including a verification link. The restricted LinkedIn
  API is not on the path.
- **`NO_DATA` is a first-class verdict** in both verifiers. These settle real
  money, so "we cannot see" must never collapse into "failed" — and a detox day
  with no sample voids rather than passes, or revoking usage access becomes the
  winning strategy.
- **The Android assistant binding is deliberately unimplemented.** That API
  moved repeatedly through 2025–26; static shortcuts are stable and ship now,
  and the assistant capability is a marked TODO rather than a guess.
- **`AxiomRouter` cannot work until cleartext is scoped.** The mesh gateway
  speaks plain HTTP on a Tailscale address, which this target SDK blocks. Fix
  with a `domain-config` limited to that host — never `usesCleartextTraffic="true"`,
  which would reopen cleartext for the API too. Until then `select()` falls
  through to the server, which is the safe failure.

### iOS sequencing

App Intents first (largest single win), then Live Activities, then HealthKit
verification and the two share loops. **Apply for the `FamilyControls`
entitlement at the start**, not when the screen-time feature is reached — Apple
is slow and frequently refuses it outside parental-control apps.

### Repo topology fix (2026-08-31)

`praxis-android` had held the **iOS** project since 2026-08-29, when an iOS port
was force-pushed over its `main`. The Android app it used to hold is fully
absorbed into `client/android` here — all 105 Kotlin files present, this copy a
strict superset — so nothing was lost, but the repo↔project mapping was wrong in
a way that would eventually cost someone an afternoon.

Target: **one repo per project.** `praxis_webapp` is the single Praxis
application repo (server + web client + shipped Android app); `praxis-ios` is the
iOS client; `praxis-mcp-server` is the MCP server. The current SwiftUI tree is on
`praxis-ios` branch `swiftui-port`; promoting it to `main` and archiving
`praxis-android` needs a force-push and is written up in
`randomesh/MANUAL_ACTIONS.md` §2.5.

Also relevant here: `~/Praxis/praxis_webapp` is a second clone of this repo on
omo, and production runs from `~/praxis_webapp`. Two clones of a deploy repo on
one host is how a "why didn't my change land" afternoon starts — `auto_update.sh`
fast-forwards or refuses, silently, and only `logs/auto_update.log` says which.

Next mobile steps are enumerated in `docs/MOBILE_FEATURE_PLAN.md` §11, ordered so
each is independently shippable. The first four need no new API and no store
review; the notable one is #4 — duel verdicts must be settled server-side, since
the phone must not arbitrate a bet it is a party to.

### Axiom outbound PII anonymization (Microsoft Presidio) — 2026-09-07

User PII in every prompt that leaves the box for a third-party LLM (cloud
providers, Gemini pool, DeepSeek, physis relays, Hermes gateway) is now detected
by a Presidio analyzer sidecar and replaced with reversible placeholders before
send; the response is restored to real values server-side. Fail-closed: Presidio
down → cloud/relay providers skipped, only the first-party `colibri` tier used
(raw); text-only cloud paths (multimodal, agentic, plan refit, goal
decomposition) degrade the same way. Feature is env-gated
(`AXIOM_PRESIDIO_URL` unset = off, zero new npm deps, built-in `fetch`).
Deployed live on pansa + omo (sidecar units, GHCR tag 2.2.364).

Files: `src/services/privacy/PresidioClient.ts`,
`src/services/privacy/AxiomPrivacyService.ts`, wired into `AICoachingService`
(`runWithFallback`, `runMultimodal`, `runAgentic`) and `PlanRefitService` /
`GoalDecompositionService`; tests `src/__tests__/axiomPrivacy.test.ts`; deploy
units + README under `deploy/presidio/`. Core entity set only — DATE_TIME and
LOCATION/GPE deliberately excluded so coaching keeps working on dates and
city/place context. Known gaps: English-NER-only, image attachments not
scrubbed.

### Axiom CLI last-resort fallback (OpenCode Zen free tier) — 2026-09-07

Triggered by a live incident (Gemini key pool HTTP 400 across all models/keys).
New cascade rung in `runWithFallback` after the physis cascade: shells out to
`opencode run --agent plan -m oczen/<model>` with the live Zen free models
(`nemotron-3-ultra-free`, `deepseek-v4-flash-free`, `mimo-v2.5-free`,
`nemotron-3.5-lightning-free`). `hy3-free` and `laguna-s-2.1-free` are dead
(removed from the live endpoint); direct keyless HTTP to Zen is rejected with
`MissingSessionID` — the free tier is gated to the OpenCode client, hence the
subprocess. Single-flight lock, 90s per-model timeout, absolute-path binary
resolution (systemd user unit PATH lacks ~/.local/bin), third-party so gated by
the same `outboundAllowed` flag as the Presidio fail-closed logic (sanitized
prompts only). Disabled via `AXIOM_CLI_FALLBACK=off`. Tests:
`src/__tests__/axiomCliFallback.test.ts` (8).

#### Chain v2 (same day): NVIDIA NIM + Hermes + Nous — `AxiomLastResortService`

Extended into a four-backend chain (nvidia → opencode → hermes → nous), all
verified live 2026-09-07:

- **NVIDIA NIM** (HTTP, `NVIDIA_API_KEY`/`AI_NVIDIA_KEY` @ `EP_NVIDIA`): NIM
  does NOT serve nemotron-3.5-lightning — defaults
  `nemotron-3-super-120b-a12b`, `nemotron-3-ultra-550b-a55b`,
  `llama-3.1-nemotron-ultra-253b` (env `AXIOM_NVIDIA_MODELS`). Parses
  `content` with `reasoning_content` fallback (nemotron reasons before
  answering — small max_tokens leaves content empty).
- **OpenCode CLI** unchanged, but `nemotron-3.5-lightning-free` now first in
  the default list (it IS live on Zen; NIM is what lacks it).
- **Hermes**: gateway form (HTTP @ `EP_HERMES`) activates when
  `HERMES_API_KEY` lands (no key yet — dormant); CLI form verified:
  `hermes -z --provider openrouter -m nvidia/nemotron-3.5-lightning:free`
  answers. Hermes' own default (gemini-flash-lite) 400s with an invalid key
  and groq 413s its payload — explicit provider/model is mandatory.
- **Nous**: key exists on omo only; the guessed portal endpoints do not
  resolve — dormant until `AXIOM_NOUS_BASE_URL` + `AXIOM_NOUS_MODELS` are set.

Env fixes shipped with it: `EP_NVIDIA`/`EP_HERMES` in each box's `~/.env` held
the URL twice (two lines) → every registry request honoring those overrides
was hitting a malformed URL; deduplicated. Key placeholders added
(`HERMES_API_KEY`, `NOUS_API_KEY`). Renamed from `AxiomCliFallbackService`;
tests `src/__tests__/axiomLastResort.test.ts` (11).

## Impeccable design skill — installed + first sweep (2026-09-28)

- **Global:** skill v4.4.0 via `skills.sh` to `~/.agents/skills/impeccable`
  (symlinked into `~/.claude/skills`, `~/.hermes/skills`, +70 other agent
  dirs); manual symlinks added for OpenCode
  (`~/.config/opencode/skills`), Codex (`~/.codex/skills`), Gemini
  (`~/.gemini/skills`). Note: `npx impeccable install` is broken upstream
  (bundle 404, "report at .../issues/479") — `skills.sh` is the working path.
  Cline has no skills dir; Cursor/Windsurf absent on this box.
- **Project:** `DESIGN.md` (North Star "Midnight Dojo", extracted from
  `tokens.ts` + `index.tsx` palette + `index.css` @theme) +
  `.impeccable/design.json` sidecar + detector hook on (`.impeccable/config.json`,
  manifests for Claude Code + Codex). Project skill copies in `.claude/skills`,
  `agent/skills` (local-only, not committed).
- **Sweep:** detector 49 → 1 (only the Polaroid test-string false positive
  left standing). 23 status-accent stripes → tint wash + 1px seam (quote /
  reply-preview edges kept neutral-divider); 16 width/height/padding
  transitions → `scaleX`/`scaleY` / `translateX` / dropped dead CSS
  (`.progress` ×2, `.goal-card`, `.selected-goal-card`, `.progress-fill`).
  Ignores persisted: `bounce-easing=typing-bounce`, file-scoped `side-tab`
  for the 4 neutral quote edges.
- Verify: `cd client && npx tsc --noEmit` clean, `npx vitest run` 253/253.
- Caution: delegated sweeps needed repair (double-scaled bars, `#000` label
  on dark bg, broken CLITab gradient, deleted skill dirs) — verify delegates
  file-by-file, never let them touch dot-dirs.

## Polish pass — Landing / Today / Leaderboard / Notebook (2026-09-28)

Visible deltas after the "no meaningful change" round (user chose polish-all):

- **Landing:** Hero dropped gradient-clipped headline (solid ink + amber),
  eyebrow chip, orb trio + grid overlay (one static amber wash), CTA pulse
  (hover lift), metric-strip stats (one quiet proof line). `SectionHeading`
  lost the kicker prop; all 5 call sites updated. DailyLoop cards became a
  numbered timeline rail. FeatureGrid 12 card-soup became 2-col editorial rows.
- **Today (`/today`):** action list became a time-gutter timeline with
  priority rail dots; source chip demoted to icon; domain + metric surfaced;
  error state gained retry.
- **Leaderboard:** "Your rank" hero-metrics card compacted to identity row +
  tabular stat line; gradient IconButton-share became a real Button; stale
  `praxis-app.vercel.app` share URL → `praxisweb.xyz`; missed dynamic stripe
  on points rows removed.
- **Notebook:** header emoji → drawn journal icon; subtitle to AA tone;
  Analytics collapse toggle keyboard-operable with visible focus.
- Verify: detector 1 (known Polaroid test-string FP), tsc clean, vitest 253/253.

### E2E against an isolated Supabase stack (2026-11-20)

The Playwright suite could not be run anywhere without writing into real data.
`npm run test:e2e` defaults to `:3000`/`:3001/api`, and on the box that serves
Praxis `:3001` **is** the live backend — while the "local" Supabase tier is the
production API tier (`supabase_kong_praxis_webapp` aliased onto
`supabase-db-prod`, see `scripts/start-supabase-stack.sh`). The nightly job in
CI starts a backend with no `SUPABASE_URL`, so every DB-backed spec skipped
there: 265 tests discovered, a third of them never executed anywhere.

`bash scripts/e2e-local.sh` is the runner. Everything it starts is disposable:

| piece | address | what keeps it away from production |
| --- | --- | --- |
| Supabase | `:54421` API, `:54422` Postgres, `:54424` mailpit | own project (`tests/e2e/supabase/config.toml`, `project_id = praxis_e2e`), own containers/volume/network, ports in 544xx; the runner refuses 54321/54322 and the praxisweb hosts, and requires `supabase_db_praxis_e2e` to exist before it uses the API |
| backend | `:3101` | never `:3001`; `MASTER_ENV_PATH` points at nothing so `/home/gio/.env` cannot smuggle the production credentials in, `DISABLE_CRONS=1` keeps ~18 jobs (briefs, mails, payouts) out of a test run |
| frontend | `:3100` vite, or `--built` → `client/dist-e2e` served by the backend on `:3101` | never `:3000`; the built mode writes its own directory, never `client/dist` (that is what production serves) |

**It verifies the wiring before a test runs.** After the servers are up the
runner asserts that the module the browser will execute names the e2e stack and
the e2e API — vite's transformed `src/lib/supabase.ts` in dev mode, the built
`dist-e2e/assets/*.js` in `--built` mode. If either still names the production
origin, it stops. (That guard is what caught `PUBLIC_SUPABASE_ORIGIN`, below.)

**A schema had to exist first, and the repository had none.** `migrations/` is
deltas; the second file alphabetically fails on an empty database with
`relation "profiles" does not exist`, because `profiles`, `goal_trees` and
`user_subscriptions` were created in the Supabase dashboard in Feb 2026 and never
written down — `migrations/setup.sql` says as much in its own header, and
production's `schema_migrations` records 5 of 211 files. `npm run migrate` also
could not run at all (`pg` is not a declared dependency and `@types/pg` is
missing, so ts-node refused to compile a script that `tsc` never covered —
`scripts` is outside `tsconfig.include`).

So `tests/e2e/supabase/baseline.sql` is the snapshot: 198 tables, 843 GRANTs,
the auth.users trigger, and `schema_migrations` seeded with the 211 files the
snapshot already contains, so `npm run migrate` applies only what is newer.
`scripts/build-schema-baseline.sh` regenerates it (`pg_dump --schema-only
--no-owner`, read-only). Two traps it encodes: `--no-privileges` yields a schema
`anon`/`authenticated` get "permission denied" on, and the `auth.users` trigger
is not in a `public` dump, so a fresh stack would sign users up with no profile.
Loaded into a scrambled schema it is clean: 198 tables, 211 tracking rows,
`npm run migrate` → "nothing to do".

**Rate limits made "the whole suite at once" impossible.** One caller over
loopback means one bucket (`clientIpOf` has no `CF-Connecting-IP` to read), so
`authLimiter` at 5 attempts / 15 min fails the *first* spec file. The runner runs
two phases: the suite with raised ceilings, then `rate-limiting.spec.ts` against
a restarted backend with the real ceilings — because that spec's whole point is
to hit them. The ceilings come from `ceiling()` in
`src/middleware/rateLimiter.ts`, which **ignores its env vars whenever
`NODE_ENV === 'production'`**: 5 auth attempts per 15 minutes stays
non-negotiable on the served app (unit-tested).

**Findings this surfaced, none of them invented:**

- **The CSP blocked every browser sign-in.** `connect-src` reads
  `PUBLIC_SUPABASE_ORIGIN`, deliberately not `SUPABASE_URL` (a visitor cannot
  reach the server's loopback). The runner sets both; without it the failure
  looks like `Login failed.` with no request in the network panel.
- **`requireAdmin` answered 401 to a signed-in non-admin.** It now answers 403
  (`ForbiddenError`) — 401 means "authenticate" and sent signed-in users back to
  the login page.
- **`api-critical.spec.ts` asserted a contract that no longer exists**: signup
  answers 201 `{user}` and login 200 `{user}` — neither returns a session, so
  every authenticated assertion was really testing `Bearer undefined`. It now
  mints a token with a password grant against GoTrue, and asserts what a public
  profile *is* (200 with a column whitelist, no email) instead of the 401 the
  route never returned.
- **`core-flows.spec.ts` waited for `/dashboard`**, which has been a redirect to
  `/notebook` since the Diary→Notebook move. Six `waitForURL` calls updated.
- **The forms raced hydration.** Filling `/login` before React hydrated put the
  value in the DOM but not in the component's state, so the form posted empty
  fields; GoTrue logged `400: missing email or phone` and the UI said "Login
  failed." — reproducible by parallelism only, which is why it looked like a
  product bug. `core-flows.spec.ts` now re-submits once the app is interactive
  (`fillStable` + `submitUntil`, three attempts, each still asserting the same
  redirect). Before the fix the run was 31/11 with two login failures in flight;
  after it, 32/10/10 twice in a row.
- **There is no way to sign out.** `logout redirects to login` cannot pass
  because nothing in the client renders a sign-out control: `nav.logout` exists
  in `i18n.ts` with no consumer, and `supabase.auth.signOut()` is reached only
  from the 401 handler in `lib/api.ts`. That is a product gap the suite caught,
  not a spec to rewrite.
- **Signup no longer means onboarding.** `SignupForm` navigates to `/` with a
  "check your email" message; the spec's `signup → onboarding redirect` encodes
  the old flow.
- **Still red, and honest about why** (built mode, `--built`): 32 passed, 10
  failed, 10 skipped. Besides the two above: `/ai` and `/physis/workbench` are
  `privateRoutes` while `public-surfaces.spec.ts` asserts an anonymous visitor
  sees them; `/upgrade` no longer says "Praxis Pro";
  `[data-testid="mobile-menu"]` and the dashboard/greeting selectors have moved;
  the onboarding and "create first goal" flows have not been re-baselined since
  the redesign. That is spec-vs-product drift — each one needs a decision about
  which side is wrong before a test is touched.
- The nightly workflow still runs the old `:3000` vite path, so it keeps skipping
  every DB-backed spec and can never satisfy the two metadata assertions (vite's
  `index.html` title is not the server-rendered one). Moving that job onto this
  script needs the Supabase CLI in the runner; unverified here, so not done.

### Hosting: no cloud host survives in code (2026-11-20)

Production has been one box for months, but the code still carried the retired
Railway/Vercel deployment as its own defaults, so any unset env var pointed a
user at a host that no longer resolves:

- **OAuth callbacks** (`strava`/`spotify`/`reddit` routes) defaulted to
  `web-production-646a4.up.railway.app` — a Strava/Spotify/Reddit connect would
  hand the provider a dead `redirect_uri`.
- **Outbound mail** defaulted its links to `praxis-webapp.vercel.app`, and three
  client share strings told people to "join me" at `praxis-app.vercel.app`.
- The **OpenAPI spec**, the **MCP server's** documented endpoint and the agent-SDK
examples all published the Railway base URL.

All of them now name `https://praxisweb.xyz`. Overrides (`API_BASE_URL`,
`CLIENT_URL`, `PUBLIC_API_URL`, `PRAXIS_API_URL`, `PRAXIS_MCP_URL`) are untouched,
so a differently-hosted node still wins.

The policy surface lost the cloud with them: the Vercel origin is out of
`allowedOrigins`, the `RAILWAY_STATIC_URL` block is gone, and `connect-src` no
longer carries `*.supabase.co` / `*.vercel.app` / `*.up.railway.app` — the
self-hosted `publicSupabaseOrigin()` is the only remote origin the browser may
reach. `cors.test.ts` (13) and `csp.test.ts` pass on the narrowed policy.
`railway.toml` and `Procfile` (deploy configs for a platform nothing runs on)
were deleted; systemd starts `dist/index.js` directly.

Historical references were left alone on purpose: `docs/archive/`, `docs/plans/`
and `DOCS_MASTER.md` record what was true then, and rewriting them would forge
the record. `tests/e2e/supabase/baseline.sql` keeps its inherited column
comment ("Railway fallback") because it is a byte-faithful production dump.
