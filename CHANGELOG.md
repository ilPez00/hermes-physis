# Changelog

## 1.7.0 — 2026-11-10

The release the catalog entry points at. Everything below was already in the
tree; this version number exists so the pin and the manifest agree.

### Security boundary

- **Model-facing raw execution removed.** A model can no longer name an
  executable or trigger a raw command: the learned registry maps every step to
  the bundled Python running a fixed wrapper (`stepN`, `verify`).
- **Approval is an operator action.** Turning a session into a replayable
  recipe needs `/physis approve <candidate-id>`; the approve verb is deliberately
  not a registered model tool, and candidates live in the plugin profile
  directory, never in the repository.
- **State that decides what runs is never model-supplied.** Registry, store and
  candidate paths come from operator config only.
- **Fail closed on ambiguity.** A stale or changed input blocks replay instead of
  guessing: `status: blocked`, with the changed paths named.

### Learning from verified work

- `auto_learn` turns a session's shell steps and edited files into a workflow
  candidate, but only after the operator's own `verify_cmd` exits zero. A recipe
  without a recorded intent is not written (there would be nothing to match it
  against later).
- Candidates are content-addressed and re-validated on load; a tampered or
  repo-planted candidate is not approvable.

### Replay

- Replay re-executes the real actions and the independent checks; it never
  returns a cached success. `provider_calls` is `0` on a replay by construction,
  and the demo test asserts it.
- Replant fingerprint limit raised to 256 MiB so learned candidates work under
  large interpreters.
- `physis_trace` exposes replay diagnostics (ported ReasonTrace) without
  widening the execution surface.

### Honesty fixes

- Security and provenance claims in the docs were qualified to match what the
  code does: approved commands run with the configured user's filesystem and
  network access; Python entry points need an interpreter; replay freshness
  fingerprints captured files, not arbitrary dependencies.
- `physis-core` rebuilt for host glibc 2.41 compatibility.

### Bundled engines

`bin/physis-core` and `bin/replant` are built by the `build-binaries` workflow
from this commit and carry GitHub build-provenance attestations, declared in the
catalog entry under `attested_binaries`. Verify with:

```sh
gh attestation verify bin/physis-core --repo ilPez00/hermes-physis \
  --source-digest <this commit> --predicate-type https://slsa.dev/provenance/v1
```
