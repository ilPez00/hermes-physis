# MCP value fixes — plan, contracts and evidence

User scope: prepare fixes and development loops for other agents. No product
implementation, agent launch, pricing change or release belongs to this pass.
This amendment supersedes the earlier L01 schema-only assignment. Existing
broader integration and research work remain part of Physis.

Baseline: `493099988018ee1782b6f75fd24da53de52c71c6`, checked 2026-10-04.
Workers must recapture HEAD and status; earlier review receipts are historical.
Task cards: [MCP fix cards](../../tasks/mcp-fix-cards.md). Execution protocol:
[MCP agent loops](../../tasks/mcp-agent-loops.md) over the shared
[worker protocol](../../tasks/agent-loops.md).

## What the feedback establishes

| Concern | Source evidence / current limit | Planning consequence |
|---|---|---|
| Context dominated by receipts | Product `context_tool` serializes all of `CompiledContext`, including up to 40 candidates and full stats | Remove the ledger from default delivery; preserve it on explicit stats calls |
| Packet cap differs from reply cost | Shared compiler caps assembled text; product `handle` adds revision/cost JSON afterward | Budget the final tool-visible text separately at the MCP boundary; measure full wire and host/model cost |
| Context names objects rather than supplying source | Assembled compiler rows contain object IDs/kinds/labels; `explore` is the existing source-bearing path | Describe navigation honestly and test context → source/evidence use; do not fabricate code-context savings |
| Changes floods the context window | Product `MAX_ROWS = 200`, bodies default to 2000 bytes, no per-call row limit; open server also defaults to 200 and carries bodies | Make polling small by default; preserve explicit drill-down and complete traversal |
| Self-repo benchmark headline is biased | `research/product/bench.py` samples functions without test-scope exclusion, counts edge text rather than all response text, rewrites HOME, and lacks frozen cross-repo provenance | Withdraw broad savings/precision claims; rerun a portable, stratified benchmark |
| Absolute research dependency | `crates/physis-world/Cargo.toml` names `/home/gio/physis-core`, even as an optional dependency | Default checkout must resolve without private siblings; preserve federation through an explicitly isolated adapter |
| Licensing narrative disagrees with code | Offline signed install and Lemon Squeezy activation coexist; status computes fingerprint; local cache is accepted without the signature recheck claimed by comments | Document actual routes/trust limits; no extra DRM or provider replacement |
| Long observation hang | User reports product self-observation taking over 25 minutes. Existing open CLI observed this checkout in 0.99 s with a fresh isolated store | Preserve the report; investigate the customer's binary/root/store and phases. The open result does not refute it |
| Platform availability | Release workflow covers Linux x86_64; platform-native config/fingerprint behavior also needs scrutiny | Add native build/use proof before advertising other platforms; packaging alone is insufficient |

The reported 0.19–0.40 recall, 1,318-token reply and corrected 4.88× ratio are
external-review observations, not independently reproduced measurements in this
planning pass. The checked implementation supports the underlying concerns.
Do not copy those figures into marketing as newly certified results.

## Contracts decided before assigning workers

### Default context delivery

Keep the commercial response in its existing single MCP text channel. Preserve
JSON for nonempty packets and the useful `assembled`, `assembled_tokens`, budget,
unit and revision information. Remove candidate arrays, per-candidate scores,
drop explanations, lifted receipt ledgers and aggregate reduction claims from
the default response. A small receipt reference leads to `physis.context_stats`.
Do not send the same payload in both text and structuredContent.

Core `CompiledContext`, open CLI compile output, Runtime APIs, primitives and
research traces retain their detailed accounting. Compaction belongs at the
commercial delivery boundary, not in the world/compiler representation.

### What the budget covers

After compaction, MCP `budget` constrains the exact final `content[0].text`
using Physis's labelled heuristic units. The commercial adapter reserves the
metadata cost before selecting content; JSON escaping, revision, receipt pointer
and any enabled cost envelope are included before asserting the cap. There is
no payload added after this measurement. The internal compiler receives the
remaining content allowance; stats expose requested and effective budgets.

At zero, or below the minimum metadata cost, return an empty text block rather
than violating the cap or panicking. Document this boundary behavior for clients;
stats called with the original query/focus/budget explain the empty result. Empty
delivery costs zero by this adapter contract; do not change the shared counter's
minimum-one semantics for unrelated users.

This is not a universal model-token guarantee. MCP framing/tool definitions are
outside the text cap, and hosts render tool results differently. The benchmark
must count complete serialized responses and the actual supported host/model
representation with a real, named tokenizer. Marketing cannot equate a heuristic
unit with a billed token. No new tokenizer/model dependency in the product.

### Receipt identity and freshness

Stats must explain the packet actually delivered, including delivery cuts and
effective content allowance. Recompiling the full original content budget and
calling it that packet's receipt is incorrect. Use one deterministic delivery
calculation shared by context/stats. A receipt ID binds query, focus, requested
budget, effective configuration, world revision and delivered content.

Stats with a receipt ID return the matching ledger only for the matching world;
if the session advanced, return an explicit stale-receipt result. Do not claim
historical receipt retrieval without storing historical packets. Plain stats
calls without a receipt ID remain useful for inspecting a fresh compilation.
Session reload is L03; preserve its shared ingest/liveness repairs.

### Changes polling

Commercial defaults: 20 rows, summary-only, and an 800-unit final-text allowance.
Expose `limit` (1–200), `include_bodies` (default false), `body_cap` (0–4096
bytes, default 256 when bodies requested), and a `budget` for final text. Keep
legacy offset input working; add `next_seq` and `has_more` for reliable polling.
`since_seq` remains inclusive; next_seq resumes after the last returned item.

Every cut is visible. An oversized body/subject cannot stall the cursor forever;
return a bounded flagged summary with its sequence and a drill-down instruction.
Never advance past an item with no representation. No observations are deleted
or rewritten. Explicit larger pages still obey the chosen output allowance.
After the commercial proof, mirror the small-default behavior in the open MCP
as a separate bounded patch, keeping its source filter and row/changes aliases.

### Source and impact claims

Structural context remains navigation evidence. `explore` supplies existing
source fragments and native read/grep remains the fallback where coverage is
incomplete. Test this complete interaction, not just a cheaper pointer response.
Identical outputs at different roomy budgets are not automatically a defect;
adding irrelevant rows to spend the budget would be worse. A useful task must
obtain the source it needs, and end-to-end cost must include the ensuing reads.

Distinguish mention-file coverage against grep from true caller recall and
precision. A returned file containing the name does not independently prove the
reported edge. Zero-caller targets form their own stratum; empty answers never
inflate the main savings claim.

## Execution order and shared ownership

1. MC01 compact default context; MC02 cap the final delivery and bind receipts;
   MC03 reduce changes polling. One owner at a time on commercial `server.rs`.
2. MC04 complete L03 freshness and post-observe receipt behavior. MC05 removes
   the private checkout prerequisite; MC06 reconciles licensing/product copy.
3. MC07 runs the cross-repo benchmark; MC08 verifies native platform delivery;
   MC09 investigates the observation hang with bounded probes.

MC05 and MC06 can be assigned independently if they do not overlap another
worker's files. Platform runners and benchmark fixtures are coordinator-owned
inputs, never discovered through arbitrary home-directory scraping by a worker.
No agent is launched by this plan. Publish/release remains a later operator step.

MC01–MC04 provide immediate adoption/support value. Portable checkout and truthful
licensing are evaluation prerequisites. Benchmarks decide positioning; they must
not wait for an enterprise SKU, cloud index or cognition subsystem.

## Commercial decisions after proof

Keep price unchanged during these fixes. The feedback is a hypothesis about
market viability, not evidence that €50 is impossible or €5–30k/year is justified.
An offline binary can sell convenience, dependable integration, updates and
support; it cannot promise effective anti-copying. Do not build enforcement to
rescue an unsupported revenue model.

Test the narrow local structural-impact task with actual developers first.
If consultancies value private change-impact, collect paid-pilot evidence about
coverage, correctness and support requirements before developing team features.
If buyers value agent governance, first prove complete action/context lineage,
mutation coverage and store consistency. A mutable local JSONL file is not a
tamper-proof compliance audit log. No regulated-readiness claim from this patch.

The sellable MCP remains separate from Impossible Computer/System One research.
Preserve their interfaces and negative results; do not make compiled learning
or small-model parity part of the commercial promise before a real demonstration.

## Completion and decisions

| Question | Required evidence |
|---|---|
| Did default output become cheaper? | Same fixture/query/model tokenizer: final context response before/after, with ledger removed and no lost selected evidence |
| Is the budget honest? | Final emitted text measured after all decoration; budgets 0/1/10/200/800/1600/3200, escaping/Unicode/large-candidate cases |
| Can an agent still inspect selection? | Context receipt → matching stats; after external observe, explicit stale result rather than a substituted explanation |
| Is changes usable for polling? | Hundreds of mixed-size rows traversed without omissions/duplicates/stuck cursor; summary-to-body drill-down |
| Does another checkout build? | Clean host without `/home/gio` or quarry siblings builds the default workspace and commercial product |
| Is the product useful beyond this repo? | Five pinned public supported-language repos; production/non-test stratum, failures and coverage beside full-response cost |
| Are new platforms supported? | Native artifact launches, license install, observe, context/stats and persistent MCP interaction on each advertised target |
| Is the hang resolved? | Attributed failing fixture, phase timings, bounded regression or honest UNVERIFIABLE result; no 25-minute retry loop |

Acceptance is evidence on a specific source/binary, not a worker's summary or
an aggregate score. Missing host, tokenizer, license or runner is UNVERIFIABLE.
Existing negative results survive new measurements.
