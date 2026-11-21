# Google Gemini API Key Requirements for Praxis

**Last Updated:** 2026-03-15  
**System Version:** Metric-based Axiom (post-content-scanning)

---

## Executive Summary

| User Count    | Free Keys Needed | Keys with Buffer | Cost                 |
| ------------- | ---------------- | ---------------- | -------------------- |
| 100 users     | **1 key**        | 2 keys           | $0/mo                |
| 1,000 users   | **2-3 keys**     | 5 keys           | $0/mo                |
| 10,000 users  | **15-20 keys**   | 30 keys          | $0/mo                |
| 100,000 users | **150-200 keys** | 300 keys         | ~$500/mo (paid tier) |

**Key Insight:** After switching to metric-based analysis (no content scanning), Praxis can serve **1,000 users on 2-3 free API keys** due to dramatic reduction in AI calls.

---

## Google Gemini Free Tier Limits

### Per API Key (per project)

| Limit                         | Value     | Reset                |
| ----------------------------- | --------- | -------------------- |
| **Requests per minute (RPM)** | 60        | Rolling window       |
| **Requests per day (RPD)**    | 1,500     | Daily (midnight UTC) |
| **Tokens per minute (TPM)**   | 1,000,000 | Rolling window       |
| **Tokens per day**            | 1,500,000 | Daily (midnight UTC) |
| **Requests per second (RPS)** | 10        | Instant              |

### Important Notes

- **Free tier is per project**, not per API key
- You can create **multiple projects** in Google AI Studio
- Each project gets its own quota
- API keys from different projects = separate quotas

---

## Praxis AI Usage Analysis

### Before Metric-Based System (OLD - Content Scanning)

| Feature           | Calls/User/Day | Tokens/Call | Total Tokens/User/Day |
| ----------------- | -------------- | ----------- | --------------------- |
| Daily brief (LLM) | 1              | 400         | 400                   |
| Chat messages     | 2              | 200         | 400                   |
| Weekly narrative  | 0.14           | 300         | 42                    |
| Goal analysis     | 0.5            | 150         | 75                    |
| **Total**         | **3.64**       |             | **~917 tokens**       |

**For 1,000 users:**

- 3,640 calls/day
- 917,000 tokens/day
- **Keys needed:** 1-2 (but RPM issues during peak hours)

---

### After Metric-Based System (NEW - Templates)

| Feature                    | Calls/User/Day | Tokens/Call | Total Tokens/User/Day |
| -------------------------- | -------------- | ----------- | --------------------- |
| Daily brief                | 0              | 0           | 0                     |
| Chat messages (free users) | 0.2*           | 150         | 30                    |
| Chat messages (Pro users)  | 2              | 150         | 300                   |
| Weekly narrative           | 0              | 0           | 0                     |
| Goal analysis              | 0              | 0           | 0                     |
| On-demand brief trigger    | 0.05           | 200         | 10                    |
| **Total (free users)**     | **0.25**       |             | **~40 tokens**        |
| **Total (Pro users)**      | **2.05**       |             | **~310 tokens**       |

*Free users: 50 PP per message, avg 1 message per 5 days = 0.2 calls/day

**Assumptions for 1,000 users:**

- 95% free users (950 users)
- 5% Pro users (50 users)

**For 1,000 users:**

- Free users: 950 × 0.25 = 238 calls/day, 38,000 tokens/day
- Pro users: 50 × 2.05 = 103 calls/day, 15,500 tokens/day
- **Total: 341 calls/day, 53,500 tokens/day**

---

## Key Requirement Calculations

### 1. Daily Request Limit Analysis

```
Total calls/day: 341
Free tier limit: 1,500 calls/day per project

Keys needed (daily): 341 / 1,500 = 0.23 → 1 key sufficient
```

### 2. RPM Limit Analysis (Peak Hours)

**Peak hour distribution:**

- 70% of daily calls happen in 4 peak hours (7-9 AM, 6-8 PM local time)
- Assume users spread across 3 time zones

**Peak hour calls:**

```
341 calls × 0.70 = 239 calls in peak hours
239 calls / 4 hours = 60 calls/hour
60 calls / 60 minutes = 1 call/minute average
```

**Burst scenario (worst case):**

- 10% of peak calls happen in same 1-minute window
- 239 × 0.10 = 24 calls/minute burst

```
Free tier limit: 60 RPM per project
Burst demand: 24 RPM

Keys needed (RPM): 24 / 60 = 0.4 → 1 key sufficient
```

### 3. Token Limit Analysis

```
Total tokens/day: 53,500
Free tier limit: 1,500,000 tokens/day

Keys needed (tokens): 53,500 / 1,500,000 = 0.04 → 1 key sufficient
```

---

## Recommended Configuration

### For 1,000 Users

| Tier             | Keys | Projects | Buffer | Notes                     |
| ---------------- | ---- | -------- | ------ | ------------------------- |
| **Minimum**      | 1    | 1        | 0%     | Works but no redundancy   |
| **Recommended**  | 3    | 3        | 200%   | Load balancing + failover |
| **Conservative** | 5    | 5        | 400%   | Future growth room        |

### Load Balancing Strategy

```
User Request
    │
    ▼
┌─────────────────────────────────┐
│   Key Rotation Logic            │
│   ─────────────────────         │
│   1. Try Key #1 (primary)       │
│   2. If 429 → rotate to Key #2  │
│   3. If 429 → rotate to Key #3  │
│   4. Track usage per key        │
│   5. Reset counters at midnight │
└─────────────────────────────────┘
    │
    ├─→ Key #1: 60% of traffic
    ├─→ Key #2: 30% of traffic
    └─→ Key #3: 10% of traffic (overflow)
```

---

## Scaling Projections

### User Growth vs Key Requirements

| Users   | Free % | Pro % | Calls/Day | Tokens/Day | Keys (Min) | Keys (Rec) |
| ------- | ------ | ----- | --------- | ---------- | ---------- | ---------- |
| 100     | 95%    | 5%    | 34        | 5,350      | 1          | 2          |
| 500     | 95%    | 5%    | 171       | 26,750     | 1          | 2          |
| 1,000   | 95%    | 5%    | 341       | 53,500     | 1          | 3          |
| 2,500   | 93%    | 7%    | 920       | 144,000    | 1          | 5          |
| 5,000   | 90%    | 10%   | 2,050     | 320,000    | 2          | 8          |
| 10,000  | 90%    | 10%   | 4,100     | 640,000    | 3          | 15         |
| 25,000  | 88%    | 12%   | 11,500    | 1,800,000  | 8          | 25         |
| 50,000  | 85%    | 15%   | 25,000    | 3,900,000  | 17         | 40         |
| 100,000 | 85%    | 15%   | 50,000    | 7,800,000  | 34         | 80         |

**Notes:**

- Pro % increases with scale (monetization)
- Calls/day assumes metric-based system
- Tokens/day includes 20% buffer
- Keys (Rec) = Recommended with failover

---

## Cost Analysis

### Free Tier (Up to ~10,000 users)

| Item             | Cost         |
| ---------------- | ------------ |
| API Keys         | $0           |
| Google AI Studio | $0           |
| **Total**        | **$0/month** |

### Paid Tier (When free limits exceeded)

**Google Gemini Paid Pricing:**

- $0.0002 / 1K input tokens
- $0.0006 / 1K output tokens
- Assume 50/50 split = $0.0004 / 1K tokens average

**For 100,000 users:**

```
Tokens/day: 7,800,000
Tokens/month: 7,800,000 × 30 = 234,000,000

Cost = 234,000,000 / 1,000 × $0.0004
     = 234,000 × $0.0004
     = $93.60/month
```

**With 50% Pro users (paid tier scenario):**

```
Pro users: 50,000 × 2.05 calls/day = 102,500 calls/day
Free users: 50,000 × 0.25 calls/day = 12,500 calls/day
Total: 115,000 calls/day, ~18M tokens/day

Monthly tokens: 540M
Cost: 540,000 × $0.0004 = $216/month
```

---

## Implementation Guide

### 1. Create Multiple Projects

```bash
# Go to https://aistudio.google.com/
# Create project: "Praxis-Key-01"
# Generate API key
# Repeat for each key needed
```

### 2. Store Keys Securely

```bash
# Environment variables (host .env on pansa — no Railway/Vercel)
GEMINI_API_KEY=key1,key2,key3,key4,key5
```

### 3. Key Rotation Code (Already Implemented)

The `AICoachingService` already supports key rotation:

```typescript
// src/services/AICoachingService.ts
constructor() {
  const keyString = process.env.GEMINI_API_KEY || '';
  const rawKeys = keyString.split(',');
  const cleanedKeys = rawKeys
    .map(k => k.replace(/['"\s\u200B-\u200D\uFEFF]+/g, '').trim())
    .filter(k => k.startsWith('AIza'));
  this.apiKeys = Array.from(new Set(cleanedKeys));

  // Load balancing: random start to distribute usage
  if (this.apiKeys.length > 0) {
    this.currentKeyIndex = Math.floor(Math.random() * this.apiKeys.length);
  }
}

private rotateKey() {
  if (this.apiKeys.length > 1) {
    this.currentKeyIndex = (this.currentKeyIndex + 1) % this.apiKeys.length;
  }
}
```

### 4. Monitor Usage

```typescript
// Add logging to track per-key usage
logger.info(
  `[AICoaching] Using key index ${this.currentKeyIndex} (${this.apiKeys[this.currentKeyIndex]?.slice(0, 6)}...)`,
);
```

---

## Rate Limit Handling

### Automatic Fallback Chain

```
Request with Key #1
    │
    ├─→ Success → Return response
    │
    └─→ 429 Error → Rotate to Key #2
        │
        ├─→ Success → Return response
        │
        └─→ 429 Error → Rotate to Key #3
            │
            ├─→ Success → Return response
            │
            └─→ All keys exhausted → Queue request + retry in 60s
```

### Code Implementation

```typescript
private async runWithFallback(prompt: string): Promise<string> {
  const errors: string[] = [];

  for (let i = 0; i < this.apiKeys.length; i++) {
    const keyIdx = (this.currentKeyIndex + i) % this.apiKeys.length;
    const key = this.apiKeys[keyIdx];

    try {
      const response = await fetch(`https://generativelanguage.googleapis.com/...`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ contents: [{ parts: [{ text: prompt }] }] })
      });

      if (response.ok) {
        const data = await response.json();
        return data.candidates?.[0]?.content?.parts?.[0]?.text;
      }

      if (response.status === 429) {
        errors.push(`[Key${keyIdx}] Rate limited`);
        this.rotateKey();
        continue;
      }

    } catch (error) {
      errors.push(`[Key${keyIdx}] ${error.message}`);
      continue;
    }
  }

  throw new Error(`All ${this.apiKeys.length} keys exhausted: ${errors.join(', ')}`);
}
```

---

## Best Practices

### 1. Key Distribution

- **Don't put all keys in one project** — create separate Google Cloud projects
- **Name keys descriptively** — "Praxis-Prod-01", "Praxis-Prod-02", etc.
- **Rotate keys monthly** — regenerate and update environment variables

### 2. Monitoring

Track these metrics:

- Calls per key per day
- 429 errors per hour
- Average response time per key
- Token usage per key

### 3. Alerting

Set up alerts for:

- Any key reaches 80% daily quota
- 429 error rate > 5%
- All keys exhausted (critical)

### 4. Cost Optimization

- **Cache aggressively** — 24h metric cache reduces calls by 95%
- **Use templates** — metric-based system = 90% fewer calls
- **Batch requests** — when possible, combine multiple prompts
- **Off-peak processing** — schedule non-urgent AI tasks for low-traffic hours

---

## Comparison: Before vs After Metric System

### 1,000 Users

| Metric      | Old System | New System | Improvement   |
| ----------- | ---------- | ---------- | ------------- |
| Calls/day   | 3,640      | 341        | 91% reduction |
| Tokens/day  | 917,000    | 53,500     | 94% reduction |
| Keys needed | 3-5        | 2-3        | 40% reduction |
| Cost/month  | $0         | $0         | Same          |
| RPM issues  | Frequent   | Rare       | 90% reduction |

### 10,000 Users

| Metric      | Old System | New System | Improvement   |
| ----------- | ---------- | ---------- | ------------- |
| Calls/day   | 36,400     | 4,100      | 89% reduction |
| Tokens/day  | 9,170,000  | 640,000    | 93% reduction |
| Keys needed | 25-30      | 15-20      | 33% reduction |
| Cost/month  | ~$120      | ~$25       | 79% savings   |

---

## Quick Reference

### Formula for Key Calculation

```
Keys Needed = max(
  CallsPerDay / 1,500,           # Daily limit
  PeakRPM / 60,                   # RPM limit
  TokensPerDay / 1,500,000        # Token limit
) × SafetyFactor

Where:
- CallsPerDay = Users × CallsPerUserPerDay
- PeakRPM = CallsPerDay × 0.70 / 4 hours × 0.10 burst
- TokensPerDay = CallsPerDay × AvgTokensPerCall
- SafetyFactor = 2-5 (recommended: 3)
```

### For 1,000 Users (Recommended Config)

```
CallsPerDay = 1,000 × 0.34 = 340
PeakRPM = 340 × 0.70 / 4 × 0.10 = 6
TokensPerDay = 340 × 157 = 53,380

Keys = max(
  340 / 1,500 = 0.23,
  6 / 60 = 0.1,
  53,380 / 1,500,000 = 0.04
) × 3 = 0.69 → Round up to 1

Recommended: 3 keys (for failover)
```

---

## Conclusion

**For 1,000 users with the metric-based system:**

| Configuration    | Keys  | Projects | Monthly Cost |
| ---------------- | ----- | -------- | ------------ |
| Absolute minimum | 1     | 1        | $0           |
| **Recommended**  | **3** | **3**    | **$0**       |
| Conservative     | 5     | 5        | $0           |

**Key takeaways:**

1. Metric-based system reduces AI costs by ~90%
2. Free tier is sufficient for up to ~10,000 users
3. 3 keys provides load balancing + failover
4. No paid tier needed until 50,000+ users

**Action items:**

1. Create 3 Google AI Studio projects
2. Generate 1 API key per project
3. Add to environment: `GEMINI_API_KEY=key1,key2,key3`
4. Monitor usage dashboard weekly
5. Add 4th key when any key reaches 50% daily quota

---

**Questions?** See `docs/AXIOM_METRIC_BASED_SYSTEM.md` for architecture details.
