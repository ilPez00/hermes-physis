//! # Physis Core (`physis-core`)
//!
//! **An engine that maintains competing interpretations of reality, evaluates their
//! coherence with observations, processes, evidence, and outcomes, and preferentially
//! retains interpretations that continue to work.**
//!
//! ---
//!
//! ## Core Epistemic Thesis
//!
//! Traditional knowledge systems enforce premature convergence onto single interpretations,
//! discard contradictory evidence, or treat truth as static. `physis-core` operates under
//! an alternative paradigm:
//!
//! 1. **Competing Hypotheses**: Multiple conflicting interpretations of an observation or process
//!    coexist simultaneously in the epistemic graph.
//! 2. **Multidimensional Coherence**: Interpretations are evaluated across five distinct dimensions:
//!    - **Semantic Fit**: Alignment with known linguistic and conceptual definitions.
//!    - **Ontological Grounding**: Proximity to validated domain/mode categories in the Semiotic Grid.
//!    - **Logical Consistency**: Lack of active contradictions or structural invalidities.
//!    - **Empirical Support**: Corroboration by observed evidence and outcomes.
//!    - **Process Coherence**: Temporal and causal alignment with operational cycles.
//! 3. **Truth Maintenance & Non-Destructive Contradictions**: When evidence conflicts, the system
//!    preserves both claims, records the tension as a typed `Contradiction`, and applies contextual
//!    preferencing without destroying dissenting historical data.
//! 4. **Epistemic Audit & Time Machine Replay**: Every hypothesis generation, evidence attachment,
//!    confidence drift, and contradiction resolution is recorded in an immutable, append-only
//!    `EpistemicAuditTrail`. Any belief state can be replayed and reconstructed at any historical
//!    timestamp $t$.
//! 5. **Quality Feedback Loop**: Cell-level and context-dependent penalties penalize domains that
//!    lead to failed predictions, dynamically modifying retrieval and classification scores.
//!
//! ---
//!
//! ## Epistemic Revision Track (0.1.22)
//!
//! The [`delta_engine`] carries the revision machinery of the epistemic
//! track: a **no-perturbation invariant** (a zero embedding shift moves
//! nothing), **revision selection by declared dependency**
//! ([`delta_engine::EvaluationContext::depends_on_walk`] — the DependsOn
//! closure, not the breadth wave), **named frozen fitness weights** with a
//! per-term breakdown ([`hypothesis::Hypothesis::fitness_term_breakdown`]),
//! and **adjudication routing** ([`delta_engine::route_transition`]): a
//! proposed demotion is applied, deferred to review with a recorded
//! rationale, or refused for core-protected (`Certified`) beliefs — every
//! decision recorded with its rationale. Superseded beliefs are never
//! deleted; they stay history-readable
//! ([`hypothesis::current_hypotheses`] vs
//! [`hypothesis::hypotheses_including_history`]) and
//! [`temporal::TemporalValidity`] carries the system-invalidation leg
//! `expired_at`. Each behavior ships behind a named gate test; the
//! machinery proposes, carries, and defers — it does not verify.
//!
//! ## W1 resource integration (Phase 24 wave 1, 0.1.23)
//!
//! - **G4 per-link intake ids** — `Evidence::intake_id`,
//!   `ProvenanceLink::intake_id` and `EpistemicEvent::intake_id` stamp every
//!   link and event with the ingest episode that produced it; an
//!   [`explanation::ExplanationReport::cited_intake_ids`] resolves an
//!   explanation's citations to the concrete raw intakes (`citations_resolve_to_intake_ids`).
//! - **G6 point-in-time queries** —
//!   [`temporal::TemporalValidity::is_valid_at`] now reads the `expired_at`
//!   system leg (P0 landed it fields-only), and
//!   [`epistemic::EpistemicAuditTrail::point_in_time_status_at`] gates the
//!   audited status by the validity window — the surface can say None where
//!   history still replays the state (`point_in_time_matches_audit_trail`).
//! - **T3 evidence retraction** — [`hypothesis::Hypothesis::retract_evidence`]
//!   re-derives a hypothesis without a source (status, fitness, revision
//!   history; `Certified` is protected), and
//!   [`delta_engine::retract_evidence_with_cascade`] re-evaluates DependsOn
//!   dependents in the shadow frame (`evidence_retraction_replays_to_state_without_it`).
//! - **T4 Nixon Diamond** — the ATMS `rules_nixon` seed is a gate test
//!   proving contradictory sides retain their evidence and audit footprint
//!   (`nixon_diamond_retains_both_sides`).
//! - **G5 hybrid retrieval** — [`rag::Bm25Index`] (dependency-free Okapi
//!   BM25) beside the cosine baseline, fused by [`rag::fuse_rrf`]; the
//!   shipping decision is *derived* from a measured verdict
//!   ([`rag::measure_hybrid_vs_cosine`]) and a negative measurement ships the
//!   fused path disabled (`hybrid_fusion_vs_cosine_baseline`).
//!
//! ## Architectural Overview
//!
//! ```text
//!                              ┌────────────────────────────────────────┐
//!                              │         Text / Sensor Stream           │
//!                              └───────────────────┬────────────────────┘
//!                                                  │
//!                                           [Vector Embedder]
//!                                                  │
//!                                                  ▼
//! ┌──────────────────────┐             ┌──────────────────────┐             ┌──────────────────────┐
//! │  Semiotic Grid (70)  │ ◄────────── │    Cell Classifier   │ ──────────► │ Unsupervised Discover│
//! │ 5 Domains × 14 Modes │             │ (Nearest-Centroid)   │             │ (Ontology Gap / HDBS)│
//! └──────────────────────┘             └──────────┬───────────┘             └──────────────────────┘
//!                                                 │
//!                                                 ▼
//!                                      ┌──────────────────────┐
//!                                      │      PhysisCore      │
//!                                      └──────────┬───────────┘
//!                                                 │
//!            ┌────────────────────────────────────┼────────────────────────────────────┐
//!            ▼                                    ▼                                    ▼
//! ┌──────────────────────┐             ┌──────────────────────┐             ┌──────────────────────┐
//! │ Competing Hypotheses │             │ Contradiction Engine │             │  Epistemic Audit     │
//! │ & Predictive Fitness │             │ & Truth Maintenance  │             │ & Historical Replay  │
//! └──────────────────────┘             └──────────────────────┘             └──────────────────────┘
//!            │                                    │                                    │
//!            └────────────────────────────────────┼────────────────────────────────────┘
//!                                                 │
//!                                                 ▼
//!                                      ┌──────────────────────┐
//!                                      │ Contextual Feedback  │
//!                                      │  & Quality Penalties │
//!                                      └──────────────────────┘
//! ```
//!
//! ---
//!
//! ## Semantic observers and candidate-claim contract (additive)
//!
//! External extractors/consolidators (e.g. Cognee, sidecar or Cognee-RS behind an
//! adapter) are **optional semantic observers**, never the canonical substrate and
//! never a source of verified truth. Their emissions enter as provenance-carrying
//! *candidate claims* ([`CandidateClaimPacket`], optionally batched in a versioned,
//! lossy-explicit [`CandidateClaimEnvelope`] for adapter/sidecar interchange) via
//! [`SemanticObserver`] adapters: recorded append-only as intake/audit observations,
//! then gated into proposals (hypotheses/evidence/typed-edge or equivalence/grounding/
//! lesson candidates) by [`assess_claim`]/[`assess_claims_with_intake`] or
//! [`crate::core::PhysisCore::observe_and_assess`] (or intake-only via
//! [`crate::core::PhysisCore::observe_and_ingest`]/[`crate::core::PhysisCore::ingest_observer_claims`]).
//! Gates reuse alignment/stability/recurrence/contradiction/TMS signals and may
//! accept-as-candidate, defer, reject, or abstain — but **never auto-certify,
//! never auto-merge identities, and never bypass contradiction or DependsOn revision**.
//! Intake records what was asserted; assessment returns proposals/decisions; neither
//! registers anything as verified state. Extractor confidence is advisory, not authority.
//!
//! Observer-derived retrieval hits may be fused into receipt-carrying context
//! compilation ([`CompiledContext`]) as attributed, **unverified** candidates that
//! Physis selects/drops under a hard budget — ranked strictly below
//! engine-verified evidence, so an advisory score can fill slack but never
//! displace a verified chunk; fusion is ablatable and should be
//! enabled only on measured wins over core-only baselines. No Cognee graph
//! database is used as Physis state. Deterministic in-crate eval stubs
//! ([`compare_compiles`], outcome tallies/proxies) support fixture-based
//! comparisons, not claims.
//!
//! ---
//!
//! ## Quick Start Example
//!
//! ```rust
//! use physis_core::{
//!     PhysisCore, RandomProjectionEmbedder, Hypothesis, HypothesisStatus, Evidence, VectorEmbed,
//! };
//!
//! // 1. Initialize Embedder and Engine Core
//! let embedder = RandomProjectionEmbedder::new(64);
//! let mut core = PhysisCore::new();
//!
//! // 2. Create Competing Hypotheses for a Machine Temperature Anomaly
//! let emb_a = embedder.embed("Spindle bearing lubrication breakdown causing friction");
//! let mut hyp_a = Hypothesis::new("Spindle bearing lubrication breakdown", emb_a);
//! hyp_a.assumptions.push("Coolant pump flow is nominal".to_string());
//! // 3. Attach Empirical Observations to the hypothesis before registering
//! hyp_a.supporting_evidence.push(Evidence::supports(
//!     "vibration_sensor_accelerometer",
//!     "High frequency harmonics match bearing ball-pass frequency",
//! ));
//! let id_a = core.register_hypothesis(hyp_a);
//!
//! let emb_b = embedder.embed("Thermal sensor telemetry calibration drift");
//! let hyp_b = Hypothesis::new("Thermal sensor telemetry calibration drift", emb_b);
//! let id_b = core.register_hypothesis(hyp_b);
//!
//! // 4. Evaluate Fitness and Resolve Preferred Interpretation
//! core.transition_hypothesis(
//!     &id_a,
//!     HypothesisStatus::Supported,
//!     "Vibration telemetry corroborated bearing breakdown",
//!     Some("vibration_sensor_accelerometer".to_string()),
//! );
//!
//! // 5. Epistemic Audit Trail records registration + status transitions
//! assert_eq!(core.hypotheses.len(), 2);
//! assert_eq!(core.epistemic_audit.events.len(), 3);
//! ```
//!
//! ---
//!
//! ## Module Index
//!
//! - [`classify`]: Nearest-centroid semiotic classification against 70 canonical cells + custom axes.
//! - [`coherence_dimensions`]: Multidimensional coherence scoring (`Semantic`, `Ontological`, `Logical`, `Empirical`, `Process`).
//! - [`coherence_query`]: Query builder for filtering nodes and hypotheses by coherence bounds and verdict states.
//! - [`contradiction`]: Tension tracking, polarity detection, and contextual preferencing without information loss.
//! - [`core`]: The main [`PhysisCore`] knowledge graph containing coherence nodes, hypotheses, edges, and dreaming loops.
//! - [`discovery`]: Unsupervised ontology gap analysis and proposal clustering for novel domains.
//! - [`embed`]: Vector embedding trait [`VectorEmbed`] and lightweight deterministic [`RandomProjectionEmbedder`].
//! - [`embed_onnx`]: Optional high-fidelity ONNX embedding runtime (MiniLM / BERT) via `ort`.
//! - [`epistemic`]: Append-only audit stream and time-machine historical replay.
//! - [`explanation`]: Structured explanation report generation with provenance chains and causal grounding.
//! - [`history`]: Ingestion adapters for browser bookmarks, browser history, OPML feeds, and chat logs.
//! - [`hypothesis`]: Hypotheses, evidence polarity, predictions, revisions, and composite fitness breakdowns.
//! - [`ontology`]: Multi-domain ontology loaders (Praxis, Machine Process, Agent Workflow, Office Operations).
//! - [`praxis`]: Life-log and behavioral records with asserted verdicts and feedback integration.
//! - [`process`]: Industrial process cycles, tasks, state machines, and temporal deviations.
//! - [`provenance`]: Cryptographic hash chains and provenance tracking for epistemological traceability.
//! - [`quality`]: Reinforcement feedback loops with cell penalties and contextual fitness adjustments.
//! - [`rag`]: Token-budget bounded retrieval-augmented generation with MMR diversity filtering.
//! - [`vault`]: Markdown knowledge vault and Git commit history importers.
//! - [`studio`]: Embedded lightweight web studio GUI and RESTful API endpoints.

pub mod act;
pub mod alignment;
pub mod becoming;
pub mod bench;
pub mod chain;
pub mod classify;
pub mod coherence_dimensions;
pub mod coherence_query;
pub mod compile;
pub mod config_run;
pub mod contradiction;
pub mod core;
pub mod coverage;
pub mod delta_engine;
pub mod direction;
pub mod discovery;
pub mod dream;
pub mod edition;
pub mod embed;
pub mod embed_ngram;
pub mod epistemic;
pub mod experiments;
pub mod explanation;
pub mod gate;
pub mod grid;
pub mod ground;
pub mod history;
pub mod hypothesis;
pub mod ingest;
pub mod linkage;
pub mod map;
pub mod model_provider;
pub mod models;
pub mod ngram_table;
pub mod notebook;
pub mod observe;
pub mod observer;
pub mod observer_eval;
pub mod ontology;
pub mod oracle;
pub mod praxis;
pub mod process;
pub mod propose;
pub mod provenance;
pub mod quality;
pub mod rag;
pub mod relation;
pub mod revision;
pub mod selftest;
pub mod stability;
pub mod store;
pub mod support;
/// Typed, calibrated, interchangeable decision rungs — `rule | pattern |
/// count-table | lexical | prototype | knn | ngram | Jev | LLM | human` — and
/// the loop that compiles a repeatedly-expensive judgement into the cheapest one
/// that can express it, validates it on a held-out slice, monitors it for drift,
/// and recomputes only the region a semantic delta moved.
///
/// The contract is `WorldSlice + OntologySlice + [HeadEvidence] → Decision<O>`
/// with `{ value, distribution, confidence, provenance, cost }`; see
/// `docs/SYSTEM_ONE_SEMANTIC_JIT.md` for the architecture and
/// `examples/experiment54_system_one_heads.rs` for the measurement.
pub mod system_one;
pub mod temporal;
pub mod tokenizer;
pub mod transform;
pub mod transplant;
pub mod vault;

#[cfg(feature = "studio")]
pub mod studio;
#[cfg(feature = "studio")]
pub mod studio_communities;

#[cfg(feature = "studio")]
pub mod studio_lab;

#[cfg(feature = "embed-onnx")]
pub mod embed_onnx;

pub use classify::{CellClassifier, CellScore, TopKStrategy};
pub use coherence_dimensions::{CoherenceDimension, CoherenceProfile};
pub use coherence_query::{
    EpistemicQuery, EpistemicQueryResult, FailedPredictionSummary, HypothesisSummary,
};
pub use contradiction::{Contradiction, ContradictionParty, ResolutionStatus};
pub use core::PhysisCore;
pub use delta_engine::{
    analyse_convergence, analyse_report_convergence, classify_delta_with, compare_reports,
    evaluate_mutation, evaluate_mutation_with_params, route_transition, route_transition_with,
    AblationComparison, AdjudicationDecision, AdjudicationRoute, ConvergenceSignature, DeltaParams,
    EvaluationContext, HypothesisTransition, MutationOp, NodeDelta, OntologyDeltaReport,
    OntologyMutation, PropagationStep, RevisionWalk, WalkStep, ADJUDICATION_STRATEGIC_FLOOR,
    DEGRADATION_THRESHOLD, GAMMA, MAX_PROPAGATION_DEPTH, MAX_REVISION_WALK_NODES, MIN_IMPACT,
};
pub use discovery::{discover, DiscoveryConfig, DiscoveryReport, ProposedDomain};
pub use dream::{
    analyse_stability as analyse_history_stability, dream_over_history, ProposalKind,
    RecurrenceEvidence, RetrospectiveProposal, StabilityFlag, RETIRE_AFTER_CONTRADICTIONS,
};
pub use embed::{RandomProjectionEmbedder, VectorEmbed};
pub use epistemic::{
    EpistemicAuditTrail, EpistemicEvent, EpistemicEventType, HighWaterMark, IntakeReceipt,
};
pub use explanation::{ExplanationReport, HistoricalPrecedent};
pub use grid::{GridCellView, GridView};
pub use history::{
    importer_for as history_importer_for, points_to_occurrences, DynamicalTrace, TracePoint,
};
pub use hypothesis::{
    Evidence, EvidencePolarity, FitnessBreakdown, Hypothesis, HypothesisStatus, Prediction,
    Revision, CONTRADICTION_PENALTY_CAP, CONTRADICTION_PENALTY_PER_ITEM,
    FAILED_PREDICTION_PENALTY_CAP, FAILED_PREDICTION_PENALTY_PER_ITEM,
    FITNESS_WEIGHT_EMPIRICAL_SUPPORT, FITNESS_WEIGHT_LOGICAL_CONSISTENCY,
    FITNESS_WEIGHT_ONTOLOGICAL_FIT, FITNESS_WEIGHT_PREDICTIVE_SUCCESS, FITNESS_WEIGHT_SEMANTIC_FIT,
};
pub use models::*;
pub use observe::{observation_from_trace_point, observations_from_trace};
pub use ontology::OntologyLoader;
pub use praxis::{BehaviourRecord, BehaviourStatus};
pub use process::{
    ProcessConstraint, ProcessCycle, ProcessDeviation, ProcessGoal, ProcessIntervention,
    ProcessMeasurement, ProcessOutcome, ProcessPlan, ProcessResource, ProcessState, ProcessTask,
    StateTransition, TaskState,
};
pub use provenance::{ProvenanceChain, ProvenanceLink};
pub use quality::{
    ContextualQualityTracker, FitnessContext, FitnessRecord, QualityFailure, QualityTracker,
};
pub use rag::{count_tokens, RagChunk, RagCorpus, RetrievalResult, TokenFixedRetriever};
pub use relation::{RelationType, TypedEdge};
pub use stability::{
    assess_robustness, evidence_ablation, HypothesisMargin, HypothesisSnapshot, PerturbationOp,
    PerturbationScenario, RobustnessReport, ScenarioOutcome,
};
pub use temporal::TemporalValidity;
pub use transform::{
    apply, find_homomorphisms, Constraint, ConstraintKind, PatElem, Predicate, PropKind,
    Proposition, TraceStep, Transform, Triple, TriplePattern, WorldState, ACCEPT_FLOOR,
};
pub use vault::{collect_labels as collect_vault_labels, scan_vault, VaultDoc};

#[cfg(feature = "studio")]
pub use studio::{run, run_with_model, StudioState};

#[cfg(feature = "embed-onnx")]
pub use embed_onnx::{OnnxConfig, OnnxEmbedder, PoolingStrategy};

// Semantic-observer candidate ingest (Cognee/extractors as observers, never authority)
pub use ingest::{
    hypothesis_from_claim, lesson_proposal, record_candidate_intake, record_candidate_intake_batch,
    relation_proposal, CandidateClaimEnvelope, CandidateClaimPacket, ClaimPayload, ClaimSource,
    ConsolidatedLesson, EntityMention, EquivalenceProposal, ExtractorInfo, GroundedConcept,
    ProposedRelation, RetrievalHit,
};
pub use observer::{
    MockObserver, NoopObserver, ObserverInput, RecordingObserver, SemanticObserver,
};

// Verification/promotion gates for observer claims (abstention-first, no auto-merge/truth)
pub use gate::{
    assess_claim, assess_claims_with_intake, assess_grounding, gate_claim, gate_equivalence,
    gate_lesson, gate_relation, gate_retrieval_hit, AssessedIntake, ClaimGateSummary, GateDecision,
    GateOutcome, GateReasonCode, GroundingGateResult, LessonGateResult, RelationGateResult,
    ResolutionGateResult, RetrievalHitGateResult,
};

// Receipt-carrying context compilation (deterministic, budgeted)
pub use compile::{
    candidates_from_retrieval_claims, compile_context, compile_from_retrieval,
    compile_from_retrieval_with_gated_observer_claims, compile_from_retrieval_with_observer_claims,
    compile_fused, CompileCandidate, CompileOptions, CompileStats, CompiledContext, DropReason,
    DropReceipt, EmbedderInfo as CompileEmbedderInfo, SelectionReceipt,
};

// Action/mutation preview, affected-set (DependOn/TMS) explanation, and staged commit helpers
pub use revision::{
    explain_affected_set, fingerprint_state, preview_mutation, preview_retraction_cascade,
    transact, transact_and_persist, transact_and_persist_default, AffectedSetExplanation,
    CommitReceipt, DriftError, MutationPreview, RollbackReceipt, StagedMutation, StateFingerprint,
};

// "Why supported / affected now" TMS explanation surfaces (deterministic, read-only)
pub use support::{explain_support_snapshot, SupportSnapshot};

// Deterministic observer/candidate evaluation stubs (in-crate, no IO/network)
pub use observer_eval::{
    accepted_equivalence_pair_keys, accepted_relation_keys, assessed_contradiction_count,
    assessed_outcome_counts, assessed_outcomes_by_kind, assessed_reason_counts_by_kind,
    build_observer_fixture_report, canonical_pair_key, classify_gate_reason, compare_compiles,
    reason_category_for_code,
    context_recall_proxy, gate_outcome_counts, gate_outcome_reason_counts_by_kind,
    gate_outcomes_by_kind, grounding_outcomes, lesson_outcomes,
    observer_fixture_report_from_assessed, observer_fixture_report_from_outcomes,
    observer_selected_count, observer_selection_ratio, pair_precision_recall_f1,
    precision_recall_f1, relation_outcomes, resolution_contradiction_yield, resolution_outcomes,
    stable_accepted_lessons, summarize_assessed, summarize_gate_outcomes, utilization,
    CompileComparison, GateAssessmentSummary, GateOutcomeCountsByKind,
    GateOutcomeReasonCountsByKind, ObserverFixtureReport, ReasonCategory, ResolutionOutcomeCounts,
    SimpleOutcomeCounts,
};
