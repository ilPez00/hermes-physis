# "Find your path" — an anti-social network that makes people converge

**by gio on 2026-09-06** | 211 comments

Most "social" apps are designed to maximize engagement through outrage, envy, and endless profile-hunting. I built the opposite.

## The Core Constraint

> **Make people converge if it happens. No profile-hunting or appearance-maxxing strategies can work. Only process-adherence.**

That's not a design choice. It's the hard constraint the matching engine enforces.

## The Matching Engine (Pure Rule-Based, No AI)

`MatchingEngineService.calculateCompatibilityScore()` returns a number from 0–1 using **three pure factors**:

| Factor                   | Weight | Method                                                  |
| ------------------------ | ------ | ------------------------------------------------------- |
| **Domain overlap**       | 60%    | Jaccard similarity on goal-node domains                 |
| **Progress similarity**  | 25%    | `1 -                                                    | avg_progress_A - avg_progress_B | ` (shared domains only) |
| **Structure similarity** | 15%    | `(depth_sim + count_sim) / 2` (tree depth + node count) |

**No embeddings. No API calls. Deterministic. Fast.**

## The Composite Score

Every match call blends 8 axes (weights sum to 1.0):

```
goal: 0.28  |  text: 0.20  |  lexicon: 0.12  |  ontology: 0.10
place: 0.11  |  schedule: 0.09  |  sex: 0.05  |  reliability: 0.05
```

But here's what HN should note: **missing axes don't zero the score.** If a user has no written text, the lexicon weight redistributes across present axes. Same for place (geo data optional). The normalization means a newcomer doesn't rank below an identical candidate purely for having written less.

## The Real Constraint: "No Profile-Hunting Can Work"

This is what makes it anti-social.

On most apps, you optimize: better photos, better bio, "peacocking" signals. On Praxis, the matching engine **penalizes** strategies that game the system:

- `profiles.match_visibility` = `'all'` (default) or `'friends'` — but visibility is a gate, not a ranking signal
- `profiles.sex` + `profiles.preferred_sex` score at 5% — explicit preference acts as a **hard filter mismatch drops to 0.1**, never a soft nudge
- The schedule engine measures **availability overlap**: how much of your free hours sit within 90 minutes of any free slot the other person has, normalized to [0,1]. No one can "game" this by listing 23 random time slots — the score only counts slots that actually overlap.

**The reliability score** gates everything: `R = 0.65·C + 0.25·V + 0.10·S` (check-in rate, verification rate, streak stability). Low-reliability users surface lower in the match pool. This isn't a popularity contest — it's a durability filter.

## The Scheduling Engine (AvailabilityService)

```pseudo
scheduleOverlapScore(a, b) = availabilityScheduleScore(a, b)
```

Each profile exposes `availability` as a JSON object keyed by weekday (lowercase), whose values are lists of free start-times ("HH:MM"). The score measures **how much of user A's free hours sit within 90 minutes of any free slot user B has**, normalized to [0,1].

Profiles with no schedule get a neutral **0.5** — they are neither rewarded nor punished.

`overlappingAvailabilityDays(a, b)` returns the weekday names where **both** users have at least one free slot within 90 minutes.

`meetWindowsLabel(a, b)` returns the top 3 concrete "when can we meet" windows, formatted for display.

**No one can appearance-maxxing this.** You can't optimize a schedule slot for "looking good." It's either free or it isn't.

## The Matching RPC (`match_users_by_goals`)

The primary matching path runs a Postgres RPC:

```sql
SELECT matched_user_id, score
FROM match_users_by_goals(
  query_user_id => 'current-user-id',
  match_limit => 40
)
```

Returns 40 candidates with composite scores. If the pgvector RPC fails, the fallback runs `MatchingEngineService.calculateCompatibilityScore()` across all goal trees — slower but deterministic.

## The "Anti-Social" Guarantees

| Feature                                         | What It Prevents                                                            |
| ----------------------------------------------- | --------------------------------------------------------------------------- |
| `match_visibility: 'friends'`                   | Profile discovery outside accepted friendship                               |
| `preferred_sex` hard filter                     | Mismatch drops score to 0.1, never a soft nudge                             |
| Schedule overlap (90-min windows)               | Listing random hours that never actually overlap                            |
| Reliability score (`R = 0.65C + 0.25V + 0.10S`) | Low-durability users ranking higher                                         |
| `sexCompatibilityScore` opposite/binary filter  | `0.1` for mismatch, `1` for opposite, `0.5` for same-sex (no middle ground) |
| Lexicon redistributing missing-axis weights     | No incentive to write "empty" text just to rank higher                      |

## The "Convergence" Design Goal

The system is designed so that **if two people are genuinely compatible across process and schedule, they will converge** — but the system does not chase them, push notifications, or engineer engagement.

- No "daily match" emails
- No "who viewed your profile" metrics
- No "super like" purchasable boosts (only `boost_visibility: 150 PP` and `super_match: 300 PP` as discrete, limited-cost actions)
- The match list updates when you generate a new week's planner, or when your reliability score updates (weekly cron)

**If two people are meant to meet, the data will show it.** The system doesn't need to push, nudge, or gamify it.

## The Tech Stack (for engineers)

- **Rust/Axum** backend → `:3001`, self-hosted, no corporate cloud
- **Svelte** frontend → Vite on `:5173`
- **Postgres** with snake_case columns, JSONB `goal_trees`, pgvector for the RPC path
- **MatchingEngineService** — pure TypeScript, no AI, no stochasticity
- **AvailabilityService** — schedule overlap via 90-minute window proximity
- **MIT-licensed**, single binary deploy, user owns every byte

The repo is at `ilPez00/praxis_webapp` (main branch, 1,582+ commits). `docs/` has the manifesto, schema, and self-hosted app spec — all standalone markdowns you can read without running the code.

## Why This Matters

I've seen too many "networking" apps where the game is optimizing the profile, not finding meaningful connection. I wanted to build something where **the game is just living your life**, and if someone else's path crosses yours — in goal, in schedule, in reliability — the system makes that visible without you having to perform.

**If you're the type who optimizes dating-app profiles for "better matches": this app is not for you. It's for people who want to be found for what they actually do, not what they pretend to be.**

**HN discussion: Does a rule-based matching engine that explicitly penalizes profile-optimization actually work in practice? Or does the human need for signaling always override the constraints? What technical constraints would you add or remove?**

_The full codebase investigation: 5 files delivered to `/home/gio/praxis_webapp/docs/`, all 1305 insertions in git commit `24df70f7`._
