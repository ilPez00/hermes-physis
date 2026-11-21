# Bioenergetics × LifeOS — redesign

Make the bioenergetic philosophy _operate_ instead of sitting as content, by
fusing **charge** (Reich/Peat) with the physis **LifeOS** goal/schedule engine.

## Problem with the current bioenergetics feature

- Philosophy (Klages / Reich / Peat) is read-only cards — the opposite of Reich's
  point that charge must _move_.
- ~30 tools across 6 sections = a wellness encyclopedia, no through-line.
- Two disconnected energy models: `bioenergetics_checkins` (anonymous
  `client_hash`, GDPR-walled) vs the main `checkins` (`user_id`, `energy_level`,
  `is_recovery` → `deriveCharge`). The hub's data never reaches the coach or goals.
- `currentCharge` is measured but governs almost nothing.

## Core reframe

Reich's **charge** = Peat's **metabolic energy = order** = physis **availability**.
So `currentCharge` becomes the availability input to the personal schedule:
**energy gates the work the system asks of you.** Low charge → protect (descope,
restore, don't push the armor). Charge flowing → surface real / critical-path work.
The philosophy is then _experienced through behavior_, explainable on tap.

## Slice 1 — charge → LifeOS (SHIPPED, in physis `src/life_os/tracker.rs`)

- `Charge` enum `depleted < steady < charged`; `availability()` (0.3/0.6/1.0),
  `can_sustain(cost)`.
- `LifeOSGoal.charge_cost` — min charge a goal demands (deep work = charged,
  admin = steady, recovery = depleted-ok).
- `LifeOSTracker.charge` + `set_charge`; persisted with goals (sled).
- `today()` → splits incomplete goals into **doable** (charge sustains, lowest
  progress first) vs **deferred** (needs more charge) + a bioenergetic
  **countermeasure** when depleted or drifting.
- Endpoints (public via proxy `/api/physis/v1/lifeos/*`):
  - `POST /lifeos/charge` `{ "charge": "depleted|steady|charged" }`
  - `GET  /lifeos/today` → `{ charge, doable[], deferred[], countermeasure?, drift }`
  - (plus existing `/lifeos/{goal,progress,status}`)

## Remaining slices

2. **Unify check-ins** — drive charge from the main `user_id` check-in
   (`deriveCharge`); on check-in, `POST /lifeos/charge`. Deprecate the
   `client_hash` `bioenergetics_checkins` track. Tag goals with `charge_cost`.
3. **New UI — charge "today" surface** replaces the hub front door: a Charge hero
   (state + 14-day trend) + charge-matched actions (doable goals + one proposed
   practice). The old 30-tool hub becomes a secondary **Library**.
4. **Dream-proposed practices** — map countermeasures to concrete practices:
   Reich 7-zone release (jaw/chest/diaphragm) and Peat-metabolic (cold/heat,
   light, nutrition). The 5R/breath/cold-heat tools surface contextually, not as a
   30-item grid.

## Praxis integration sketch (slice 2)

On check-in completion: `deriveCharge(userId)` → if non-null, `proxyEngine('POST',
'/api/v1/lifeos/charge', { charge }, …, userId)`. Dashboard "today" reads
`GET /api/physis/v1/lifeos/today` (tenant = userId) and renders the charge hero +
matched actions. Goals sync via `POST /lifeos/goal` with a `charge_cost`.
