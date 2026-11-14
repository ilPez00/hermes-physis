//! Experiments module for Physis hierarchy level measurement
//!
//! Five tests defined in physis_findings.txt Section 10

//! ### Test 1: Pearson correlation between initial coherence_score and node survival count
//!
//! Measurement: across N CLI reinvocations.
//! Target: report r coefficient (expected positive: higher initial coherence -> better survival).
//!
//! Computation:
//!   - Create N nodes with varying initial coherence_score values
//!   - Track survival (persistence) across N CLI reinvocations
//!   - Compute Pearson r between initial coherence_score and survival_count
//!   - Report: r coefficient, p-value, N

//! ### Test 2: Survival rate ratio (cell_pinned / unpinned) after content edits
//!
//! Measurement: after content edits.
//! Target: ratio >= 2:1.
//!
//! Computation:
//!   - Create N/2 nodes with cell_pin = Some(("domain", "mode"))
//!   - Create N/2 nodes without cell_pin
//!   - Perform content edit operation on all nodes
//!   - Track which nodes survive across next CLI invocation
//!   - Compute: survival_pinned / survival_unpinned
//!   - Report: ratio, pinned_survival_count, unpinned_survival_count, N

//! ### Test 3: Success rate comparison across goal types
//!
//! Measurement: grade distribution across 20 trials.
//! Target: Auto-seeded > Random goal > No goal.
//!
//! Computation:
//!   - Trial type A: Seed dream with auto-chosen high-coherence goal -> grade result
//!   - Trial type B: Seed dream with random goal -> grade result
//!   - Trial type C: No goal seeding -> grade result
//!   - Repeat each type 20 times
//!   - Compute: P(grade=Success | auto-seeded), P(grade=Success | random), P(grade=Success | none)
//!   - Report: success_rates [3], chi-square test for significance, N=60 total

//! ### Test 4: Decay rate ratio (lambda_random / lambda_persistence) in survival probability P(t) = e^(-lambda*t)
//!
//! Measurement: over 50 CLI invocations.
//! Target: ratio >= 5:1.
//!
//! Computation:
//!   - Create N persistence nodes (with cell_pin + high initial coherence)
//!   - Create N random nodes (without cell_pin, random coherence)
//!   - Track survival count across sequential CLI invocations: t=0,1,2,...,50
//!   - Fit exponential decay P(t) = e^(-lambda*t) for both groups -> get lambda_persistence, lambda_random
//!   - Report: lambda_persistence, lambda_random, ratio lambda_random/lambda_persistence, survival curves

//! ### Test 5: Spearman correlation between hypothesis fitness and supporting evidence count
//!
//! Measurement: hypothesis fitness vs evidence items.
//! Target: rho > 0.5.
//!
//! Computation:
//!   - Create hypotheses with 0, 1, 2, 3, 4, 5+ supporting evidence items
//!   - Measure hypothesis.fitness for each
//!   - Compute Spearman rank correlation rho between evidence_count and fitness
//!   - Report: rho coefficient, p-value, bins [0,1], [2,3], [4,5+], N per bi