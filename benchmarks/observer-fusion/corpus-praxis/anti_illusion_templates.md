# Anti-Illusion Notebook — Open-Core Protocol Distribution

## Package Overview

The complete Anti-Illusion Notebook protocol is distributed as open-source markdown templates, reference schemas, and a minimal self-hosted Rust/Svelte web app skeleton. All data remains under user control; no corporate cloud is required.

## 1. Markdown Template Collection

### 1.1 `daily_log.md` — Daily Biometric Log Template

```markdown
# Anti-Illusion Notebook — Daily Log

**Date:** YYYY-MM-DD

## Vital Signs (mandatory, upon waking, fasted)

- Weight: ___ kg
- RHR: ___ bpm (3-min supine count)
- HRV: ___ ms (RMSSD, 3-min recording)

## Biometrics (mandatory, fasting)

- Fasting glucose: ___ mmol/L (after ≥8h fast)
- Blood pressure: _**/**_ mmHg

## Mood & Energy (mandatory, within 1h of waking)

- Valence: ___/5 (1=extremely negative, 5=extremely positive)
- Energy: ___/5 (1=exhausted, 5=energized)
- Anxiety: ___/10 (1=none, 10=panic)

## Nutrition (previous day, estimated)

- kcal intake: ___ kcal
- Protein: ___ g
- Carbs: ___ g
- Fat: ___ g
- P:C:F ratio: _**:**_:___

## Training (previous day, if applicable)

- Exercises: [list]
- Volume: ___ kg × reps × sets
- RPE: ___/10 (average)

## Medication / Supplements (today)

- Substance: ___
- Dose: ___ mg
- Administered: Yes/No

## Delusion-Audit (check R1–R5)

- R1: Weight +3σ from 30d MA? ___ YES/NO
- R2: Fasting glucose >6.1 for 5d? ___ YES/NO
- R3: HRV <40 ms for 7d? ___ YES/NO
- R4: Sleep efficiency <80% for 7d? ___ YES/NO
- R5: Mood valence <2 for 3d? ___ YES/NO

**Action if any R1–R5 triggered:** ___
```

### 1.2 `weekly_review.md` — Weekly Trend Review Template

```markdown
# Anti-Illusion Notebook — Weekly Review

**Period:** YYYY-MM-DD to YYYY-MM-DD

## 30-Day Rolling Averages

| Metric                   | Current | 30d MA | Δ vs Prior 30d | Status   |
| ------------------------ | ------- | ------ | -------------- | -------- |
| Weight (kg)              | ___     | ___    | ___ kg         | ⬆️ ⬇️ ➡️ |
| Fasting glucose (mmol/L) | ___     | ___    | ___ mmol/L     | ⬆️ ⬇️ ➡️ |
| HRV (ms)                 | ___     | ___    | ___ ms         | ⬆️ ⬇️ ➡️ |
| Sleep efficiency (%)     | ___     | ___    | ___ %p         | ⬆️ ⬇️ ➡️ |
| Mood valence (/5)        | ___     | ___    | ___            | ⬆️ ⬇️ ➡️ |
| Energy (/5)              | ___     | ___    | ___            | ⬆️ ⬇️ ➡️ |

## Trigger Summary

- R1 triggered this week: ___ times (action: ___)
- R2 triggered this week: ___ times (action: ___)
- R3 triggered this week: ___ times (action: ___)
- R4 triggered this week: ___ times (action: ___)
- R5 triggered this week: ___ times (action: ___)

## Correlations Noted

- Intervention X ↔ Metric Y: ___ (direction, strength observation)
- Pattern observed: ___
- Adjustment for next week: ___

## Action Plan — Next 7 Days

- Calorie target: ___ kcal/day (Mifflin-St Jeor × 1.25)
- Protein target: ___ g/day (2.2 × target lean mass)
- Training: ___ sessions this week (4× ideal)
- Sleep target: in bed by 22:30, 8h minimum
- Medication/supplement review: ___

**Signature:** ________________________
```

### 1.3 `monthly_summary.md` — Monthly Data Summary Template

```markdown
# Anti-Illusion Notebook — Monthly Summary

**Month:** YYYY-MM

## Net Changes This Month

| Metric                   | Start Value | End Value | Δ Absolute | Δ %   |
| ------------------------ | ----------- | --------- | ---------- | ----- |
| Weight (kg)              | ___         | ___       | ___ kg     | ___ % |
| Fasting glucose (mmol/L) | ___         | ___       | ___ mmol/L | ___ % |
| HRV (ms)                 | ___         | ___       | ___ ms     | ___ % |
| Sleep efficiency (%)     | ___         | ___       | ___ %p     | ___ % |
| Mood valence (/5)        | ___         | ___       | ___        | ___   |
| Energy (/5)              | ___         | ___       | ___        | ___   |

## Rule Trigger Count

| Rule              | This Month | Last Month | Trend    |
| ----------------- | ---------- | ---------- | -------- |
| R1 (weight +3σ)   | ___        | ___        | ⬆️ ⬇️ ➡️ |
| R2 (glucose >6.1) | ___        | ___        | ⬆️ ⬇️ ➡️ |
| R3 (HRV <40)      | ___        | ___        | ⬆️ ⬇️ ➡️ |
| R4 (sleep <80%)   | ___        | ___        | ⬆️ ⬇️ ➡️ |
| R5 (mood <2)      | ___        | ___        | ⬆️ ⬇️ ➡️ |

## Key Correlations

- Most impactful intervention: ___ (which change moved which metric)
- Surprising finding: ___
- Protocol adjustment: ___

## Next Month Targets

- Weight target: ___ kg (±2 kg)
- Fasting glucose target: ___ mmol/L (±0.3)
- HRV target: ___ ms (≥40)
- Sleep efficiency target: ___ % (≥80)
- Mood valence target: ___ /5 (≥3)
- Energy target: ___ /5 (≥3)

**Protocol version:** 1.0
**Export date:** YYYY-MM-DD
```
