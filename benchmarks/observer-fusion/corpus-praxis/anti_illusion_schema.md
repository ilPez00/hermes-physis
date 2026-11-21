# Anti-Illusion Notebook — Data Schema Documentation

**Version:** 1.0 (2026-08-31)
**Purpose:** Core biometric tracking schema for the Anti-Illusion Notebook protocol. All fields are mandatory where indicated; no optional fields exist in the minimal viable schema.

## 1. Database Structure

All data resides in a single Postgres database with snake_case columns. No ORM, no abstractions. Direct SQL access.

### 1.1 Table: `weight_log`

| Column       | Type         | Constraints               | Description                            |
| ------------ | ------------ | ------------------------- | -------------------------------------- |
| `id`         | SERIAL       | PRIMARY KEY               | Auto-incrementing row ID               |
| `user_id`    | UUID         | NOT NULL                  | FK to profiles table                   |
| `date`       | DATE         | NOT NULL, UNIQUE(user_id) | Measurement date, fasting, upon waking |
| `weight_kg`  | NUMERIC(5,2) | NOT NULL                  | Body weight in kilograms               |
| `created_at` | TIMESTAMPTZ  | DEFAULT now()             | Row creation timestamp                 |
| `updated_at` | TIMESTAMPTZ  | DEFAULT now()             | Row last update                        |

**Index:** `CREATE UNIQUE INDEX idx_weight_user_date ON weight_log(user_id, date);`

**Insert example:**

```sql
INSERT INTO weight_log (user_id, date, weight_kg)
VALUES ('550e8400-e29b-41d4-a716-446655440000', '2024-01-15', 88.3);
```

### 1.2 Table: `vital_signs`

| Column       | Type         | Constraints               | Description                          |
| ------------ | ------------ | ------------------------- | ------------------------------------ |
| `id`         | SERIAL       | PRIMARY KEY               | Auto-incrementing row ID             |
| `user_id`    | UUID         | NOT NULL                  | FK to profiles table                 |
| `date`       | DATE         | NOT NULL, UNIQUE(user_id) | Measurement date, morning supine     |
| `rhr_bpm`    | NUMERIC(4,1) | NOT NULL                  | Resting heart rate, beats per minute |
| `rmssd_ms`   | NUMERIC(4,1) | NOT NULL                  | Heart rate variability, milliseconds |
| `sbp_mmhg`   | NUMERIC(4,1) | NOT NULL                  | Systolic blood pressure              |
| `dbp_mmhg`   | NUMERIC(4,1) | NOT NULL                  | Diastolic blood pressure             |
| `created_at` | TIMESTAMPTZ  | DEFAULT now()             | Row creation timestamp               |
| `updated_at` | TIMESTAMPTZ  | DEFAULT now()             | Row last update                      |

**Index:** `CREATE UNIQUE INDEX idx_vitals_user_date ON vital_signs(user_id, date);`

### 1.3 Table: `sleep_log`

| Column           | Type         | Constraints               | Description                 |
| ---------------- | ------------ | ------------------------- | --------------------------- |
| `id`             | SERIAL       | PRIMARY KEY               | Auto-incrementing row ID    |
| `user_id`        | UUID         | NOT NULL                  | FK to profiles table        |
| `date`           | DATE         | NOT NULL, UNIQUE(user_id) | Sleep date                  |
| `deep_min`       | INTEGER      | NOT NULL                  | Minutes of deep sleep       |
| `rem_min`        | INTEGER      | NOT NULL                  | Minutes of REM sleep        |
| `awake_min`      | INTEGER      | NOT NULL                  | Minutes awake during night  |
| `total_min`      | INTEGER      | NOT NULL                  | Total sleep minutes         |
| `efficiency_pct` | NUMERIC(5,2) | NOT NULL                  | Sleep efficiency percentage |
| `created_at`     | TIMESTAMPTZ  | DEFAULT now()             | Row creation timestamp      |
| `updated_at`     | TIMESTAMPTZ  | DEFAULT now()             | Row last update             |

### 1.4 Table: `glucose_log`

| Column                   | Type         | Constraints   | Description                                 |
| ------------------------ | ------------ | ------------- | ------------------------------------------- |
| `id`                     | SERIAL       | PRIMARY KEY   | Auto-incrementing row ID                    |
| `user_id`                | UUID         | NOT NULL      | FK to profiles table                        |
| `date`                   | DATE         | NOT NULL      | Measurement date                            |
| `fasting_mmol`           | NUMERIC(4,2) | NOT NULL      | Fasting glucose, mmol/L                     |
| `postprandial_mmol`      | NUMERIC(4,2) | NOT NULL      | Postprandial glucose, 2h after meal, mmol/L |
| `fasting_timestamp`      | TIME         | NOT NULL      | Time of fasting draw                        |
| `postprandial_timestamp` | TIME         | NOT NULL      | Time of postprandial draw                   |
| `created_at`             | TIMESTAMPTZ  | DEFAULT now() | Row creation timestamp                      |
| `updated_at`             | TIMESTAMPTZ  | DEFAULT now() | Row last update                             |

### 1.5 Table: `mood_log`

| Column         | Type        | Constraints   | Description                                                |
| -------------- | ----------- | ------------- | ---------------------------------------------------------- |
| `id`           | SERIAL      | PRIMARY KEY   | Auto-incrementing row ID                                   |
| `user_id`      | UUID        | NOT NULL      | FK to profiles table                                       |
| `date`         | DATE        | NOT NULL      | Check-in date                                              |
| `valence_1_5`  | SMALLINT    | NOT NULL      | Mood valence, 1=extremely negative to 5=extremely positive |
| `energy_1_5`   | SMALLINT    | NOT NULL      | Energy level, 1=exhausted to 5=energized                   |
| `anxiety_1_10` | SMALLINT    | NOT NULL      | Anxiety level, 1=none to 10=panic                          |
| `created_at`   | TIMESTAMPTZ | DEFAULT now() | Row creation timestamp                                     |
| `updated_at`   | TIMESTAMPTZ | DEFAULT now() | Row last update                                            |

### 1.6 Table: `meal_log`

| Column        | Type         | Constraints   | Description                                  |
| ------------- | ------------ | ------------- | -------------------------------------------- |
| `id`          | SERIAL       | PRIMARY KEY   | Auto-incrementing row ID                     |
| `user_id`     | UUID         | NOT NULL      | FK to profiles table                         |
| `date`        | DATE         | NOT NULL      | Meal date                                    |
| `time`        | TIME         | NOT NULL      | Meal timestamp                               |
| `food_id`     | INTEGER      | NOT NULL      | FK to food_references table                  |
| `qty_g`       | NUMERIC(6,1) | NOT NULL      | Quantity consumed, grams                     |
| `kcal_100g`   | NUMERIC(5,1) | NOT NULL      | Food calories per 100g                       |
| `kcal_total`  | NUMERIC(6,1) | NOT NULL      | Total calories consumed                      |
| `protein_g`   | NUMERIC(4,1) | NOT NULL      | Protein grams                                |
| `carbs_g`     | NUMERIC(4,1) | NOT NULL      | Carbs grams                                  |
| `fat_g`       | NUMERIC(4,1) | NOT NULL      | Fat grams                                    |
| `p_c_f_ratio` | NUMERIC(5,2) | NOT NULL      | Protein:Carbs:Fat ratio                      |
| `notes`       | TEXT         | —             | Free-form notes (e.g., "post-training meal") |
| `created_at`  | TIMESTAMPTZ  | DEFAULT now() | Row creation timestamp                       |
| `updated_at`  | TIMESTAMPTZ  | DEFAULT now() | Row last update                              |

### 1.7 Table: `session_log`

| Column           | Type          | Constraints   | Description                                     |
| ---------------- | ------------- | ------------- | ----------------------------------------------- |
| `id`             | SERIAL        | PRIMARY KEY   | Auto-incrementing row ID                        |
| `user_id`        | UUID          | NOT NULL      | FK to profiles table                            |
| `date`           | DATE          | NOT NULL      | Training date                                   |
| `exercises_json` | JSONB         | NOT NULL      | Array of {exercise, sets, reps, weight_kg, rpe} |
| `volume_kg`      | NUMERIC(10,2) | NOT NULL      | Total volume = Σ(weight_kg × sets × reps)       |
| `duration_min`   | INTEGER       | NOT NULL      | Total session duration                          |
| `rpe_average`    | NUMERIC(3,1)  | NOT NULL      | Average RPE across all sets                     |
| `created_at`     | TIMESTAMPTZ   | DEFAULT now() | Row creation timestamp                          |
| `updated_at`     | TIMESTAMPTZ   | DEFAULT now() | Row last update                                 |

### 1.8 Table: `med_log`

| Column         | Type         | Constraints   | Description               |
| -------------- | ------------ | ------------- | ------------------------- |
| `id`           | SERIAL       | PRIMARY KEY   | Auto-incrementing row ID  |
| `user_id`      | UUID         | NOT NULL      | FK to profiles table      |
| `date`         | DATE         | NOT NULL      | Administration date       |
| `substance`    | VARCHAR(64)  | NOT NULL      | Drug/substance name       |
| `dose_mg`      | NUMERIC(5,2) | NOT NULL      | Dose in milligrams        |
| `administered` | BOOLEAN      | NOT NULL      | Yes/No                    |
| `reason`       | VARCHAR(128) | —             | Reason for administration |
| `created_at`   | TIMESTAMPTZ  | DEFAULT now() | Row creation timestamp    |
| `updated_at`   | TIMESTAMPTZ  | DEFAULT now() | Row last update           |

## 2. Rule Enforcement Schema

The Anti-Illusion Notebook embeds delusion-removal rules directly in the database layer via PostgreSQL functions and triggers.

### 2.1 Function: `check_weight_rule()`

```sql
CREATE OR REPLACE FUNCTION check_weight_rule()
RETURNS TRIGGER AS $$
BEGIN
  -- R1: Weight +3σ from 30-day moving average triggers fast day
  IF NEW.weight_kg > (
    SELECT AVG(weight_kg) + 3 * STDDEV(weight_kg)
    FROM weight_log
    WHERE date >= CURRENT_DATE - INTERVAL '30 days'
      AND user_id = NEW.user_id
  ) THEN
    -- Log delusion audit entry
    INSERT INTO delusion_audit (user_id, rule, triggered_at)
    VALUES (NEW.user_id, 'R1', CURRENT_DATE);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_weight_check
AFTER INSERT ON weight_log
FOR EACH ROW EXECUTE FUNCTION check_weight_rule();
```

### 2.2 Function: `check_glucose_rule()`

```sql
CREATE OR REPLACE FUNCTION check_glucose_rule()
RETURNS TRIGGER AS $$
BEGIN
  -- R2: Fasting glucose >6.1 for 5 consecutive days
  IF NEW.fasting_mmol > 6.1 THEN
    DECLARE consecutive_count INTEGER;
    SELECT COUNT(*) INTO consecutive_count
    FROM glucose_log
    WHERE user_id = NEW.user_id
      AND date >= CURRENT_DATE - INTERVAL '4 days'
      AND fasting_mmol > 6.1;

    IF consecutive_count >= 4 THEN
      INSERT INTO delusion_audit (user_id, rule, triggered_at)
      VALUES (NEW.user_id, 'R2', CURRENT_DATE);
    END IF;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_glucose_check
AFTER INSERT ON glucose_log
FOR EACH ROW EXECUTE FUNCTION check_glucose_rule();
```

### 2.3 Function: `check_hrv_rule()`

```sql
CREATE OR REPLACE FUNCTION check_hrv_rule()
RETURNS TRIGGER AS $$
BEGIN
  -- R3: HRV <40 ms for 7 consecutive days
  IF NEW.rmssd_ms < 40 THEN
    DECLARE consecutive_count INTEGER;
    SELECT COUNT(*) INTO consecutive_count
    FROM vital_signs
    WHERE user_id = NEW.user_id
      AND rmssd_ms < 40
      AND date >= CURRENT_DATE - INTERVAL '6 days';

    IF consecutive_count >= 6 THEN
      INSERT INTO delusion_audit (user_id, rule, triggered_at)
      VALUES (NEW.user_id, 'R3', CURRENT_DATE);
    END IF;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_hrv_check
AFTER INSERT ON vital_signs
FOR EACH ROW EXECUTE FUNCTION check_hrv_rule();
```

### 2.4 Function: `check_sleep_rule()`

```sql
CREATE OR REPLACE FUNCTION check_sleep_rule()
RETURNS TRIGGER AS $$
BEGIN
  -- R4: Sleep efficiency <80% for 7 consecutive days
  IF NEW.efficiency_pct < 80 THEN
    DECLARE consecutive_count INTEGER;
    SELECT COUNT(*) INTO consecutive_count
    FROM sleep_log
    WHERE user_id = NEW.user_id
      AND efficiency_pct < 80
      AND date >= CURRENT_DATE - INTERVAL '6 days';

    IF consecutive_count >= 6 THEN
      INSERT INTO delusion_audit (user_id, rule, triggered_at)
      VALUES (NEW.user_id, 'R4', CURRENT_DATE);
    END IF;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_sleep_check
AFTER INSERT ON sleep_log
FOR EACH ROW EXECUTE FUNCTION check_sleep_rule();
```

### 2.5 Function: `check_mood_rule()`

```sql
CREATE OR REPLACE FUNCTION check_mood_rule()
RETURNS TRIGGER AS $$
BEGIN
  -- R5: Mood valence <2 for 3 consecutive days
  IF NEW.valence_1_5 < 2 THEN
    DECLARE consecutive_count INTEGER;
    SELECT COUNT(*) INTO consecutive_count
    FROM mood_log
    WHERE user_id = NEW.user_id
      AND valence_1_5 < 2
      AND date >= CURRENT_DATE - INTERVAL '2 days';

    IF consecutive_count >= 2 THEN
      INSERT INTO delusion_audit (user_id, rule, triggered_at)
      VALUES (NEW.user_id, 'R5', CURRENT_DATE);
    END IF;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_mood_check
AFTER INSERT ON mood_log
FOR EACH ROW EXECUTE FUNCTION check_mood_rule();
```

### 2.5 Table: `delusion_audit`

| Column         | Type        | Constraints   | Description                        |
| -------------- | ----------- | ------------- | ---------------------------------- |
| `id`           | SERIAL      | PRIMARY KEY   | Auto-incrementing row ID           |
| `user_id`      | UUID        | NOT NULL      | FK to profiles table               |
| `rule`         | VARCHAR(2)  | NOT NULL      | Rule identifier (R1–R5)            |
| `triggered_at` | DATE        | NOT NULL      | Date rule was triggered            |
| `resolved`     | BOOLEAN     | DEFAULT false | Whether user addressed the trigger |
| `resolved_at`  | DATE        | —             | Date rule was resolved             |
| `created_at`   | TIMESTAMPTZ | DEFAULT now() | Row creation timestamp             |

**Index:** `CREATE INDEX idx_audit_user_resolved ON delusion_audit(user_id, resolved);`

## 3. Reference Tables

### 3.1 `food_references`

| Column           | Type         | Constraints      | Description                               |
| ---------------- | ------------ | ---------------- | ----------------------------------------- |
| `food_id`        | SERIAL       | PRIMARY KEY      | Auto-incrementing                         |
| `name`           | VARCHAR(128) | NOT NULL, UNIQUE | Food name                                 |
| `kcal_100g`      | NUMERIC(6,1) | NOT NULL         | Calories per 100g                         |
| `protein_g`      | NUMERIC(4,1) | NOT NULL         | Protein per 100g                          |
| `carbs_g`        | NUMERIC(4,1) | NOT NULL         | Carbs per 100g                            |
| `fat_g`          | NUMERIC(4,1) | NOT NULL         | Fat per 100g                              |
| `serving_size_g` | NUMERIC(6,1) | —                | Typical serving size                      |
| `source`         | VARCHAR(32)  | NOT NULL         | "Open Food Facts", "USDA", "user-entered" |

### 3.2 `profile_baselines`

| Column                | Type         | Constraints   | Description                            |
| --------------------- | ------------ | ------------- | -------------------------------------- |
| `user_id`             | UUID         | PRIMARY KEY   | FK to profiles table                   |
| `height_cm`           | NUMERIC(5,1) | NOT NULL      | User height                            |
| `age`                 | INTEGER      | NOT NULL      | User age                               |
| `sex`                 | VARCHAR(16)  | NOT NULL      | "male", "female", "other"              |
| `bmr_mifflin_kkcal`   | NUMERIC(6,1) | NOT NULL      | Mifflin-St Jeor BMR                    |
| `tdee_estimate_kkcal` | NUMERIC(6,1) | NOT NULL      | Estimated TDEE at 1.25 activity factor |
| `created_at`          | TIMESTAMPTZ  | DEFAULT now() | Row creation timestamp                 |

## 4. Data Export Schema

All user data can be exported via `POST /export/data`. The export contains CSV files matching the above table schemas, plus:

- `schema.sql` — complete CREATE TABLE definitions
- `rules.md` — current delusion-removal rules (R1–R5)
- `summary_report.md` — 90-day trend analysis with Pearson correlations between interventions and outcomes
- `baseline_data.json` — initial assessment values at T0

All exports are **user-owned**. No corporate cloud stores the data after export. Self-host operators retain full control.

## 5. Migration Notes

When upgrading the schema:

1. Add new columns with `ADD COLUMN` — never `ALTER TABLE ... DROP COLUMN`
2. New triggers are opt-in — existing data is unaffected
3. Export existing data before any schema change
4. Run `./scripts/validate_schema.sh` post-migration to verify all triggers fire correctly

## 6. Security Invariants

- **No row-Level Security (RLS)** on tracking tables — all data belongs to the owning user
- **No public endpoints** that read tracking data without `user_id` JWT verification
- **No external API** that writes tracking data without scope `write` on PAT
- **All biometric data** is considered PHI-like — handle with care, anonymize before any cross-user analysis

## 7. Quick-Start SQL

```sql
-- Create the database
CREATE DATABASE anti_illusion;

\c anti_illusion

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS jsonb;

-- Create tables
CREATE TABLE weight_log (...);  -- per schema above
CREATE TABLE vital_signs (...);
CREATE TABLE sleep_log (...);
CREATE TABLE glucose_log (...);
CREATE TABLE mood_log (...);
CREATE TABLE meal_log (...);
CREATE TABLE session_log (...);
CREATE TABLE med_log (...);
CREATE TABLE delusion_audit (...);
CREATE TABLE food_references (...);
CREATE TABLE profile_baselines (...);

-- Insert baseline reference ranges
INSERT INTO food_references (name, kcal_100g, protein_g, carbs_g, fat_g, source)
VALUES ('chicken_breast', 165, 31, 0, 3.6, 'USDA'),
       ('salmon', 206, 25, 0, 12, 'USDA'),
       ('egg', 155, 13, 1, 11, 'USDA');

-- Insert baseline profile
INSERT INTO profile_baselines (user_id, height_cm, age, sex, bmr_mifflin_kkcal, tdee_estimate_kkcal)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 180, 28, 'male', 1850, 2313);
```

## 8. Validation Checklist

Before any data is considered valid for the Anti-Illusion Notebook, all of the following must be true:

- [ ] Weight logged within 30 min of waking, fasted
- [ ] RHR and HRV measured supine, 3 min minimum
- [ ] Sleep log reflects actual sleep duration (not bedtime)
- [ ] Fasting glucose drawn after ≥8h overnight fast
- [ ] Postprandial glucose drawn exactly 2h after meal start
- [ ] Mood valence/energy logged within 1h of waking
- [ ] Meal log qty_g matches actual portion size (verified by food scale)
- [ ] Training session volume_kg calculated from actual weights used
- [ ] Med log administered timestamp matches actual ingestion time
- [ ] No missing fields in any single log entry

Any entry violating these invariants is auto-flagged in `delusion_audit` with rule `R0` (data integrity violation).
