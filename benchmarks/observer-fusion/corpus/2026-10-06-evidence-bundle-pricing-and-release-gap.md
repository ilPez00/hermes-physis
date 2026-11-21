# 2026-10-06 — evidence bundle, pricing unification, release gap

Execution log for the valuation-push pass. Worktree `worktrees/p1-extraction`
(branch `research/p1-extraction`); `main` untouched while opencode works there.

## Done

1. **P1 failure-priors bundle** `research/papers/p1_failure_priors/` (P2
   pattern): manuscript + figures + redacted 75,933-action corpus + 8 result
   JSONs + README/CLAIMS/PROVENANCE/experiment_manifest/reproduce.sh/environment.lock.
   - `verify_paper1.py --strict`: every 3-dp number in the tex traces to a
     result JSON — **0 orphans**; transfer/in-corpus/cold-start diagnostics all
     `in_tex=yes`.
   - `e74_replication.py` regenerates from `data/` bit-identically (only the
     recorded `source` path field differs).
   - pdflatex builds 12 pages.
   - Producer scripts keep their layout via `PHYSIS_PRO_HOME` (default
     `/home/gio/physis_pro`); port modifications are itemised in PROVENANCE.md.
   - Also ported: `research/results_registry/{validate.py,claims.json}` +
     `research/methodology_validation.json` (the files PORTFOLIO.md listed as
     not yet ported). PORTFOLIO.md P1 row updated.
2. **Price unification (physis-next, 21 edits / 0 misses).** Canonical offer
   is now €49 founding for the first 25 licences, €79 after — matching
   `FIRST_SALE.md`, the live Creem product (4900/7900) and the website.
   FOUNDER50 / first-50 / €50 language removed from every current-state doc
   (commercial README, LICENSING ×2, EULA_TEMPLATE, docs/commercial README,
   FAST_PATH, LISTING, DISTRIBUTION, VALUE_PROPOSITION, DEVELOPMENT_VALUE_PLAN,
   MARVELOUS_MACHINERY_PLAN, GITHUB_INTEGRATION, MCP_SHIP_NOW, GOLIVE).
   Dated receipts/audits left as history.
3. **Commercial README language claim** narrowed to reality: symbols parse
   Rust/TS/JS/Python; caller/dependency edges are Rust + Cargo only (verified
   in `crates/physis-discover/src/scan.rs`: `rust_calls` is the sole edge
   builder).
4. **praxis_webapp commit `4bc80b66`.** `/physis` now sells only Physis
   Context MCP: Praxis subscription block + dead stripe checkout state removed;
   benchmark paragraph no longer leads with the forbidden 11.7× multiple
   (caveats kept); webhook default allowlist `[7900,5000]` → `[4900,7900]` with
   comment, default-path test now proves 4900 fulfils.
   Verified: landing 11/11, creemWebhook 29/29, client `tsc --noEmit` clean.
5. **Full test gate**: `cargo test --no-fail-fast` → **68 suites ok, 0 failed**
   (after `physis observe .`; a cold worktree shows 23 red tests that are
   missing-observation state, not regressions — consider making observe a
   gate prerequisite explicitly). `python3 scripts/test-check-versions.py` OK;
   `just versions`: 30 packages agree on 1.1.1.
6. **Strategy deliverable**: `docs/commercial/VALUATION_PLAN_2026-10-06.md`
   (14 sections, evidence-backed).

## Found and fixed — hosting reality

- **Hosting is entirely local.** This machine is `pansa`; `praxisweb.xyz` is a
  cloudflared tunnel (`praxis-edge`) to Express `:3001`, which serves the SPA,
  `/api` and `/downloads` off disk. No Vercel, no Railway — earlier text in
  this log and in VALUATION_PLAN wrongly assumed cloud platforms.
- **Release gap CLOSED (same day):** 1.1.1 copied into
  `praxis_webapp/client/public/downloads/`, SHA256SUMS flipped, landing /
  delivered page / sales doc moved from 1.0.0 to 1.1.1 with sha256
  `7463dce9…`, client rebuilt. Live-verified: tarball HTTP 200, SHA256SUMS
  serves the new hash, served JS references 1.1.1. praxis commit `6b71ef77`;
  hosting-truth docs commit `5c67d51f` (rewrites PHYSIS_PUBLIC_ARCHITECTURE,
  wiki/Deployment, ADR 003 superseded, DR_RUNBOOK standby section, base URLs).
- **Still open:** payment delivery lacks an end-to-end receipt (sales runbook,
  2026-10-05) — action #2 in VALUATION_PLAN §14.

## Next

- Merge `research/p1-extraction` after review; run gates on `main`.
- Operator: publish 1.1.1, prove Creem E2E, record Demo A, send first 10 leads.
