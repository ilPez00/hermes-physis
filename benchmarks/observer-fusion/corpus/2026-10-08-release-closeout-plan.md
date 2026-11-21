# Release closeout plan — AUD-04 exact bytes, AUD-14 publication

Date: 2026-10-08. Status: execution plan, authorized by the operator
in-session (publication of the immutable `v1.2.0` tag explicitly
authorized). Predecessor: `docs/plans/2026-10-07-physis-audit-execution.md`.

## Goal

Build `physis` + `physis-context` 1.2.0 from a pinned commit in a clean
detached worktree using the production verification pubkey, certify the
exact shipped bytes with supplied-mode acceptance plus value-check,
publish the immutable `v1.2.0` tag and release, verify the downloaded
bytes through the real customer path, and record the AUD-14 handoff
triple (URL/version/SHA256) that AUD-05's sales seam needs.

## Decisions already made (operator, 2026-10-08)

1. Publication scope: full `--go` authorized.
2. Pinned commit: wait for the concurrent session's in-flight
   `crates/physis-mcp` work (`compact.rs`, `lib.rs`, `protocol.rs`)
   to land, then pin that commit. The release ships committed state
   only; a release from the dirty main checkout is impossible anyway
   (`publish-release.py:249` refuses a dirty tree, and
   `commercial-release.sh:205` would record `source_state=dirty`).
3. Acceptance license: reuse an already-delivered gift license from
   `dist/physis-context-gift-GIFT-005..008.zip`, with holder consent
   recorded. The private seed is never read.

## Key facts established during investigation

- `just unified-release` (`justfile:100-102`) runs `commercial-release.sh`
  then `open-release.sh`. Order is load-bearing: `commercial-release.sh:214`
  writes a one-line `dist/SHA256SUMS.txt`, and `open-release.sh:60`
  rewrites it covering both archives — exactly what
  `publish-release.py:148-149` demands.
- `PHYSIS_SIGNING_PUBKEY_HEX` is unset in the environment, but the
  production public key exists at `/home/gio/secrets/physis-context-signing/pubkey.hex`
  and reads `1ad4ec09dcd20db99d391a93f8af1fd21eb11ac2c1d0a260532c7d86b0bf3a30`.
  `RELEASE.md:30` confirms no private key is needed to build.
  `seed.hex` (mode 600) is never read by this plan.
- `physis-license-admin` has no keygen (`crates/physis-license/src/bin/admin.rs:28-40`):
  `issue` derives the verification pubkey from the seed. A license that
  validates against a production-pubkey binary can therefore only come
  from someone holding the seed — hence the delivered-gift-license path.
- CI (`.github/workflows/commercial.yml:41-50`) builds and uploads an
  artifact but never publishes a release; publication stays local.
- Remote tags: only `v0.1.0-alpha.1` → `1159a4fc`. No `v1.2.0` exists.
- Hazards: `/tmp` is a 100%-full tmpfs (always `TMPDIR=/home/gio/.tmp-audit`);
  the main checkout carries another session's uncommitted work; two fresh
  release target trees need root-fs space (`df -h` preflight).

## Execution steps

1. **Commit this plan** — stage only this file (the checkout has other
   sessions' uncommitted edits to shared docs; never `git add -A`).
2. **Wait for the concurrent session to land** (blocking gate) — poll
   `git status --porcelain crates/physis-mcp` and
   `git ls-files crates/physis-mcp/src/compact.rs` until their edits are
   committed on `main`; bounded wait, then report "still waiting" rather
   than guessing a pin.
3. **Re-gate the pinned commit in a clean worktree** —
   `git worktree add --detach /home/gio/.tmp-audit/release-wt <pinned>`;
   run `just versions`, `python3 scripts/test-check-versions.py`,
   `python3 scripts/test-publish-release.py`, `just ci`,
   `bash scripts/commercial-acceptance.sh`,
   `bash scripts/check-commercial-boundary.sh`, `bash scripts/research-loop.sh`.
   Any red or UNVERIFIABLE result stops the release.
4. **Assert the pubkey against the gift license** — `unzip -p` the chosen
   `GIFT-00X` license and assert `verify_public_keys[0]` equals
   `1ad4ec09…`; a mismatch aborts (a wrong pubkey silently invalidates
   every delivered license). Record holder + GIFT id for the consent record.
5. **Build both archives** — in the worktree:
   `export TMPDIR=/home/gio/.tmp-audit`;
   `PHYSIS_SIGNING_PUBKEY_HEX=1ad4ec09… just unified-release`.
   Verify: `dist/SHA256SUMS.txt` covers exactly the two 1.2.0 archives;
   `sha256sum -c` passes; each `BUILD.txt` has `source_head=<pinned>`,
   `source_state=clean`, `verification_pubkey_hex=1ad4ec09…`; the open
   archive carries ELF `physis` + `physis-hud`, `corpus/`, the AUD-02
   `physis-computer/README.md` and the fail-closed `install.sh`; the
   commercial archive carries no `physis-license-admin`. Record both
   archive SHA256s.
6. **Certify the exact built bytes (closes AUD-04's exact-byte row)** —
   extract the commercial archive;
   `bash scripts/commercial-acceptance.sh --binary <extracted>/physis-context --license <gift license>`;
   then `python3 scripts/commercial-value-check.py --binary … --license … --work …`.
   Expect `scope=supplied`, 15 PASS / 0 FAIL, unchanged executable SHA,
   value-check 10 PASS.
7. **Publisher dry-run** — `bash scripts/publish-release.sh 1.2.0 --sha <pinned>`;
   expect `DRY RUN: local provenance validated` plus the `gh release create`
   preview.
8. **Publish** — re-check `git status --porcelain` empty in the worktree,
   then `bash scripts/publish-release.sh --go 1.2.0 --sha <pinned>`.
   Verify: `git ls-remote --tags origin v1.2.0` resolves to `<pinned>`;
   exactly three assets.
9. **Verify the customer path on downloaded bytes** —
   `gh release download v1.2.0 --repo ilPez00/physis-next`; `sha256sum -c`;
   extract; `./install.sh <dest>`; `physis --version` and
   `physis-hud --version` print 1.2.0; `physis-context license install
   <gift license>` and `license status` prints exactly `License: valid`;
   re-run supplied-mode acceptance and value-check against the downloaded
   binary. Downloaded digests must equal the built ones.
10. **AUD-14 receipt** — `tasks/handoffs/AUD-14/RECEIPT.md`: tag, commit,
    both archive SHA256s, URL form
    `https://github.com/ilPez00/physis-next/releases/download/v1.2.0/<asset>`,
    version 1.2.0, gift-license consent record. Update `tasks/todo.md`
    (AUD-04, AUD-14) only when the concurrent session's edits there have
    landed, to avoid clobbering.
11. **AUD-12 local part (optional)** — read `scripts/commercial-value-check.py`
    first to confirm its matched-task protocol; measure three real Rust
    tasks from this repo on the production binary against a ripgrep/read
    baseline, with `cargo test` as the deterministic outcome check.
    Paid outcome and repeat use remain operator rows — never presented
    as passed.
12. **AUD-05 seam, prepared not applied** — write the exact proposed edits
    for `PhysisLandingPage.tsx`, `PhysisDeliveredPage.tsx`, their tests
    and `src/__tests__/physisContextSales.test.ts` into the AUD-05
    handoff as a diff carrying the step-10 triple; touch nothing in
    `/home/gio/praxis_webapp`.
13. **Close out** — `physis remember` with outcome and actor; final report
    distinguishing certified facts from operator-gated rows.

## Risks

- **Irreversible public tag.** `v1.2.0` is immutable; a mistake needs
  `1.2.1`. Mitigation: dry-run first, publish once, verify after.
- **Wrong verification pubkey ships** and silently invalidates delivered
  GIFT licenses. Mitigation: step 4's equality assertion aborts on mismatch.
- **The concurrent session's code becomes released code.** Mitigation:
  step 3's full re-gate at their commit; any red/UNVERIFIABLE stops the
  release.
- **Disk exhaustion** from two fresh release target trees. Mitigation:
  `df -h` preflight; optional `CARGO_TARGET_DIR` reuse of existing
  caches.
- **Stray file between build and publish** fails the publisher's clean-tree
  re-check. Mitigation: publish immediately after the build.
- **The gift license is a real customer's entitlement.** Mitigation:
  holder consent as an explicit precondition, read-only use in place,
  never copied into the repo, never uploaded.
- **`gh` token lacks `contents:write`.** Mitigation: `gh auth status`
  preflight; failure after the local build leaves artifacts intact.

## Routing

Sequential, single-lane, coordinator-owned. No worker subagents: release
operations are irreversible and involve the production key. Optional
`code-review-and-quality` pass over the AUD-14 receipt before its commit.
