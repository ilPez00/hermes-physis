# Anti-Illusion Notebook — Minimal Self-Hosted Rust/Svelte App

## Overview

A minimalist web application for the Anti-Illusion Notebook protocol. No corporate cloud, no external APIs, no fluff. Self-hosted Rust backend + Svelte frontend. All data stored in local Postgres.

## 1. Rust Backend (Axum + Sqlx)

### 1.1 `Cargo.toml`

```toml
[package]
name = "anti_illusion"
version = "1.0.0"
edition = "2021"

[dependencies]
axum = "0.7"
tower-cookies = "0.3"
sled = "0.34"  -- optional local cache
sqlx = { version = "0.7", features = ["postgres", "chrono"] }
tokio = { version = "1.36", features = ["full"] }
dotenv = "0.15"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 1.2 `src/main.rs`

```rust
use axum::{
    routing::post,
    Router,
};
use sqlx::postgres::Postgres;
use std::net::Addr;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = sqlx::postgres::PostgresPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    // Build router
    let app = Router::new()
        .route("/api/v1/weight", post(weight_handler))
        .route("/api/v1/vital_signs", post(vital_signs_handler))
        .route("/api/v1/sleep", post(sleep_handler))
        .route("/api/v1/glucose", post(glucose_handler))
        .route("/api/mood", post(mood_handler))
        .route("/api/meal", post(meal_handler))
        .route("/api/session", post(session_handler))
        .route("/api/med", post(med_handler))
        .with_state(pool);

    // Listen
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind listener");
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server error");
}
```

### 1.3 API Endpoints (POST only, no GET — data sovereignty)

| Endpoint                   | Payload                                                                                                                                                                     | Description                           |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `POST /api/v1/weight`      | `{user_id: UUID, date: DATE, weight_kg: NUMERIC}`                                                                                                                           | Log body weight, fasting, upon waking |
| `POST /api/v1/vital_signs` | `{user_id: UUID, date: DATE, rhr_bpm: NUMERIC, rmssd_ms: NUMERIC, sbp_mmhg: NUMERIC, dbp_mmhg: NUMERIC}`                                                                    | Log resting HR, HRV, BP               |
| `POST /api/v1/sleep`       | `{user_id: UUID, date: DATE, deep_min: INTEGER, rem_min: INTEGER, awake_min: INTEGER, total_min: INTEGER, efficiency_pct: NUMERIC}`                                         | Log sleep architecture                |
| `POST /api/v1/glucose`     | `{user_id: UUID, date: DATE, fasting_mmol: NUMERIC, postprandial_mmol: NUMERIC, fasting_timestamp: TIME, postprandial_timestamp: TIME}`                                     | Log glucose measurements              |
| `POST /api/mood`           | `{user_id: UUID, date: DATE, valence_1_5: SMALLINT, energy_1_5: SMALLINT, anxiety_1_10: SMALLINT}`                                                                          | Log mood check-in                     |
| `POST /api/meal`           | `{user_id: UUID, date: DATE, time: TIME, food_id: INTEGER, qty_g: NUMERIC, kcal_100g: NUMERIC, protein_g: NUMERIC, carbs_g: NUMERIC, fat_g: NUMERIC, p_c_f_ratio: NUMERIC}` | Log meal intake                       |
| `POST /api/session`        | `{user_id: UUID, date: DATE, exercises_json: JSONB, volume_kg: NUMERIC, duration_min: INTEGER, rpe_average: NUMERIC}`                                                       | Log training session                  |
| `POST /api/med`            | `{user_id: UUID, date: DATE, substance: VARCHAR, dose_mg: NUMERIC, administered: BOOLEAN, reason: VARCHAR}`                                                                 | Log medication/supplement             |

All endpoints return `{status: "ok"}` or `{error: "reason"}`.

## 2. Svelte Frontend (Minimalist)

### 2.1 `package.json`

```json
{
  "name": "anti-illusion-client",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "svelte": "^4.0.0",
    "d3": "^0.7.0",  -- for sparklines only
    "uuid": "^9.0.0"  -- for user ID generation
  },
  "devDependencies": {
    "vite": "^5.0.0",
    "svelte-preprocess": "^5.0.0",
    "ts-node": "^10.0.0"
  }
}
```

### 2.2 `src/App.svelte`

```svelte
<script>
  import { v4 as uuidv4 } from 'uuid';
  import { onMount } from 'svelte';

  let user_id = '';
  let weight = '';
  let rhr = '';
  let hrv = '';
  let glucose = '';
  let valence = '';
  let energy = '';
  let anxiety = '';
  let meals = '';

  // Generate user ID once (localStorage persistence)
  onMount(() => {
    if (!localStorage.getItem('anti_illusion_user')) {
      user_id = uuidv4();
      localStorage.setItem('anti_illusion_user', user_id);
    } else {
      user_id = localStorage.getItem('anti_illusion_user');
    }
  });

  // Handle weight log submission
  async function logWeight() {
    const response = await fetch('/api/v1/weight', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user_id,
        date: new Date().toISOString().split('T')[0],
        weight_kg: parseFloat(weight)
      })
    });
    const result = await response.json();
    if (result.status === 'ok') {
      alert('Weight logged');
      weight = '';
    }
  }

  // Handle mood log submission
  async function logMood() {
    const response = await fetch('/api/mood', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user_id,
        date: new Date().toISOString().split('T')[0],
        valence_1_5: parseInt(valence),
        energy_1_5: parseInt(energy),
        anxiety_1_10: parseInt(anxiety)
      })
    });
    const result = await response.json();
    if (result.status === 'ok') {
      alert('Mood logged');
      valence = '';
      energy = '';
      anxiety = '';
    }
  }
</script>

<div class="anti-illusion-app">

  <!-- Vital Signs -->
  <h2>Vital Signs</h2>
  <label>Weight (kg): <input type="number" bind:value:bind="weight" step="0.1" required></label>
  <label>RHR (bpm): <input type="number" step="1" required></label>
  <label>HRV (ms): <input type="number" step="1" required></label>

  <!-- Glucose -->
  <h2>Biometrics</h2>
  <label>Fasting glucose (mmol/L): <input type="number" step="0.1" required></label>

  <!-- Mood -->
  <h2>Mood & Energy</h2>
  <label>Valence (1-5): <input type="number" min="1" max="5" required></label>
  <label>Energy (1-5): <input type="number" min="1" max="5" required></label>
  <label>Anxiety (1-10): <input type="number" min="1" max="10" required></label>

  <!-- Action Buttons -->
  <div class="actions">
    <button on:click={logWeight}>Log Weight</button>
    <button on:click={logMood}>Log Mood</button>
  </div>

</div>

<style>
  .anti-illusion-app {
    font-family: system-ui, sans-serif;
    max-width: 400px;
    margin: 2rem auto;
    padding: 1rem;
    line-height: 1.5;
  }
  h2 { margin-top: 1.5rem; }
  label { display: block; margin-top: 0.5rem; }
  .actions { margin-top: 2rem; display: flex; gap: 0.5rem; }
  button {
    flex: 1;
    padding: 0.5rem 1rem;
    font-size: 1rem;
    cursor: pointer;
  }
</style>
</div>
```

## 3. Deployment — Single-Command Setup

```bash
# 1. Clone
git clone https://github.com/giovanni/anti-illusion-notebook.git
cd anti-illusion-notebook

# 2. Initialize database
createdb anti_illusion
psql -d anti_illusion -f sql/schema.sql

# 3. Set environment variables
cp .env.example .env
# Edit .env: DATABASE_URL=postgres://localhost/anti_illusion

# 4. Start backend
cd backend
cargo run --release

# 5. Start frontend (separate terminal)
cd ../client
npm install
npm run dev

# 6. Access at http://localhost:5173
# All data stored in local Postgres, no external calls
```

## 4. Data Export (User-Owned)

```bash
# Export all data as tar.gz
psql -d anti_illusion -c "\copy (SELECT * FROM weight_log) TO '/tmp/weights.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM vital_signs) TO '/tmp/vitals.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM sleep_log) TO '/tmp/sleep.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM glucose_log) TO '/tmp/glucose.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM mood_log) TO '/tmp/mood.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM meal_log) TO '/tmp/meals.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM session_log) TO '/tmp/sessions.csv' CSV HEADER"
psql -d anti_illusion -c "\copy (SELECT * FROM med_log) TO '/tmp/meds.csv' CSV HEADER"

# Also export schema
pg_dump -d anti_illusion -s > schema.sql

# Tar.gz contains all CSVs + schema.sql + rules.md
tar czf anti_illusion_export_$(date +%Y%m%d).tar.gz /tmp/weights.csv /tmp/vitals.csv /tmp/sleep.csv /tmp/glucose.csv /tmp/mood.csv /tmp/meals.csv /tmp/sessions.csv /tmp/meds.csv schema.sql docs/rules.md
```

All exported data is user-owned. No corporate cloud dependency.

## 5. Philosophy

- **No JavaScript frameworks beyond Svelte** — minimal bundle size
- **No external APIs** — all computation local
- **No user accounts system** — user_id is locally generated, optional cloud sync
- **No analytics, tracking, telemetry** — what you log is what you have
- **No dark patterns** — one page, one form, submit, done
- **Data sovereignty** — you own every byte, export anytime
