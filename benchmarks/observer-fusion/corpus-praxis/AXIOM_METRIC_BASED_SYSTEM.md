# Axiom Metric-Based Analysis System

## Overview

As of 2026-03-15, **Axiom no longer scans user message content**. All personalization is now based on **engagement metrics** — behavioral patterns derived from timestamps, counts, and state changes only.

This change:
- ✅ **Protects user privacy** — no message content is analyzed
- ✅ **Reduces API costs** — template-based generation with metrics
- ✅ **Maintains personalization** — archetype-based messaging feels just as personal
- ✅ **Improves performance** — cached metrics, no LLM latency for most users

---

## How It Works

### 1. Engagement Metrics Calculation

The `EngagementMetricService` analyzes **behavioral metadata only**:

| Category | Metrics Tracked | Data Source |
|----------|----------------|-------------|
| **Activity Patterns** | Checkin streak, consistency score, weekly activity | `checkins` table (timestamps) |
| **Goal Patterns** | Total/active goals, completion rate, update frequency | `goal_nodes` (progress, completed flag) |
| **Social Patterns** | Network size, interactions, response rate | `matches`, `honor` tables |
| **Temporal Patterns** | Most active day/hour, session duration | `checkins` timestamps |

**No content is ever read** — only counts, timestamps, and boolean states.

### 2. User Archetypes

Based on metrics, users are classified into **7 archetypes**:

| Archetype | Characteristics | Message Strategy |
|-----------|----------------|------------------|
| **Consolidator** | Few goals, high completion | "You excel at finishing" |
| **Explorer** | Many goals, low completion | "Focus your curiosity" |
| **Achiever** | High activity, high completion | "Your momentum is strong" |
| **Struggler** | Low activity, low completion | "Every expert was a beginner" |
| **Socializer** | High social, moderate goals | "Your connections fuel you" |
| **Lone Wolf** | Low social, high goals | "Trust your process" |
| **Burnout Risk** | High activity, declining | "Balance effort with recovery" |

### 3. Motivation Styles

Users are also classified by **what motivates them**:

| Style | Trigger | Messaging Focus |
|-------|---------|-----------------|
| **Streak Driven** | High consistency | "Protect your streak" |
| **Progress Focused** | High avg progress | "Stack tangible progress" |
| **Social Accountable** | High social score | "Share your commitment" |
| **Novelty Seeking** | Low consistency | "Try a new approach" |
| **Routine Based** | High consistency | "Trust your system" |

### 4. Risk Factor Detection

The system identifies **6 risk patterns**:

| Risk Factor | Detection | Intervention |
|-------------|-----------|--------------|
| `streak_about_to_break` | High streak + low recent activity | "Check in today" |
| `goal_stagnation` | Low update frequency | "Break the stagnation" |
| `social_isolation` | Low social score | "Reconnect with network" |
| `overwhelm` | Many goals, low progress | "One tiny task" |
| `declining_activity` | Decreasing sessions | "Show up for 5 min" |
| `perfectionism_trap` | Multiple goals at 90%+ | "Done > perfect" |

---

## Daily Brief Generation Flow

```
User wakes up
    │
    ↓
┌─────────────────────────────────────┐
│ 1. Fetch Engagement Metrics         │
│    - Try cache (24h TTL)            │
│    - Calculate if expired           │
│    - Store back to cache            │
└─────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────┐
│ 2. Generate Metric-Based Brief      │
│    - Pick message by archetype      │
│    - Pick routine by motivation     │
│    - Pick challenge by risk factor  │
│    - Add resources by archetype     │
└─────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────┐
│ 3. Algorithmic Picks                │
│    - Match: RPC by goal domain      │
│    - Event: City + date scoring     │
│    - Place: City + tag scoring      │
└─────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────┐
│ 4. Store Brief                      │
│    - axiom_daily_briefs table       │
│    - Available for 14 days history  │
└─────────────────────────────────────┘
```

---

## API Changes

### Before (Content Scanning)
```typescript
// Old approach: LLM reads user's goals, journals, feedback
const prompt = `User's goals: ${goals.map(g => g.name + g.description)}...
User's journals: ${journals.map(j => j.content)}...
Write a personalized message.`;
```

### After (Metric-Based)
```typescript
// New approach: Templates selected by metrics
const metrics = await engagementMetricService.calculateMetrics(userId);
const message = messages[metrics.archetype];
const routine = routines[metrics.motivationStyle];
const challenge = challenges[metrics.riskFactors[0]];
```

---

## Database Schema

### New Table: `engagement_metrics`

```sql
CREATE TABLE engagement_metrics (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  metrics JSONB NOT NULL,  -- archetype, motivationStyle, riskFactors, etc.
  calculated_at TIMESTAMPTZ,
  expires_at TIMESTAMPTZ,  -- 24h from calculation
  created_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ
);
```

**Migration:** `migrations/20260315_create_engagement_metrics_table.sql`

---

## Code Changes

### New Files
- `src/services/EngagementMetricService.ts` — Core metric calculation
- `migrations/20260315_create_engagement_metrics_table.sql` — DB schema

### Modified Files
- `src/services/AxiomScanService.ts` — Uses metrics instead of LLM
- `src/services/AICoachingService.ts` — Metric-aware templates
- `src/controllers/aiCoachingController.ts` — Fetches metrics for context

---

## Cost Impact

### Before (Per User Per Day)
| Feature | LLM Calls | Tokens/Call | Total Tokens |
|---------|-----------|-------------|--------------|
| Daily brief | 1 | ~400 | 400 |
| Chat message | 2 | ~200 | 400 |
| Weekly narrative | 0.14 | ~300 | 42 |
| **Total** | | | **~842 tokens/day** |

### After (Per User Per Day)
| Feature | LLM Calls | Tokens/Call | Total Tokens |
|---------|-----------|-------------|--------------|
| Daily brief | 0 | 0 | 0 |
| Chat message | 0 (template) | 0 | 0 |
| Weekly narrative | 0 (template) | 0 | 0 |
| **Total** | | | **0 tokens/day** |

**Savings:** ~100% reduction in AI API costs for free users.

**Pro users** (Axiom Boost) still get LLM-generated content on demand.

---

## Privacy Guarantees

The system **NEVER** analyzes:
- ❌ Goal descriptions
- ❌ Journal entries
- ❌ Chat messages
- ❌ Feedback comments
- ❌ Post content
- ❌ Bio text

The system **ONLY** analyzes:
- ✅ Timestamps (when you check in)
- ✅ Counts (how many goals, matches, etc.)
- ✅ Progress values (0-100%)
- ✅ Boolean states (completed: true/false)
- ✅ Domain names (categorical, not descriptive)

---

## Migration Guide

### For Existing Users

1. **Run the migration:**
   ```bash
   cd praxis_webapp
   npx supabase migration up
   ```

2. **Metrics backfill (optional):**
   ```typescript
   // Run once to pre-populate cache
   import { EngagementMetricService } from './services/EngagementMetricService';
   const service = new EngagementMetricService();
   
   const users = await supabase.from('profiles').select('id');
   for (const user of users.data) {
     const metrics = await service.calculateMetrics(user.id);
     await service.storeMetrics(user.id, metrics);
   }
   ```

3. **Deploy** — existing LLM-based code gracefully falls back to templates.

---

## Future Enhancements

### Planned
- [ ] A/B testing for message variants per archetype
- [ ] Seasonal adjustments (holidays, summer slump)
- [ ] Integration with wearable data (sleep, steps)
- [ ] Predictive churn modeling

### Experimental
- [ ] On-device metric calculation (client-side)
- [ ] Federated learning for archetype refinement
- [ ] Differential privacy for aggregate insights

---

## Support

For questions about the metric-based system:
- Review `src/services/EngagementMetricService.ts` for calculation logic
- Check `docs/ARCHETYPE_GUIDE.md` for archetype details (TODO)
- See `migrations/20260315_create_engagement_metrics_table.sql` for schema

**Key principle:** If it's not a timestamp, count, or state — we don't read it.
