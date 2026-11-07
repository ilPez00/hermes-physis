//! Core data models: coherence nodes, ontology entries, the semiotic grid,
//! and vector helpers.

use serde::{Deserialize, Serialize};

pub type Score = f32;

/// Did this thing actually work? The **asserted** axis — a caller's verdict,
/// not something the engine can compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoherenceRating {
    /// Worked, with a detected effect. `+1.0`
    Success,
    /// Ran, but produced no expected effect. `0.0`
    Inert,
    /// Refuted, errored, or a self-imposed rule broken. `-1.0`
    Failure,
}

impl CoherenceRating {
    pub fn weight(self) -> Score {
        match self {
            CoherenceRating::Success => 1.0,
            CoherenceRating::Inert => 0.0,
            CoherenceRating::Failure => -1.0,
        }
    }

    /// Bucket a continuous assertion.
    pub fn from_score(score: Score) -> Self {
        if score > 0.2 {
            CoherenceRating::Success
        } else if score < -0.2 {
            CoherenceRating::Failure
        } else {
            CoherenceRating::Inert
        }
    }
}

/// A node carries two coherence axes:
///
/// - `coherence_score` is *derived* — mean cosine to the k nearest neighbours,
///   clamped at `>= 0`. It answers "how typical is this?".
/// - `asserted` is *reported* — a caller's verdict in `[-1, 1]`, `None` until
///   someone says. It answers "did it work?".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceNode {
    pub id: String,
    pub embedding: Vec<f32>,
    /// Derived density. Never negative.
    pub coherence_score: Score,
    /// Asserted outcome in `[-1, 1]`; `None` = nobody has judged this yet.
    #[serde(default)]
    pub asserted: Option<Score>,
    #[serde(default)]
    pub label: Option<String>,
    /// Semiotic cell pinned to this node: (domain, mode).
    #[serde(default)]
    pub cell_pin: Option<(String, String)>,
    /// Which embedder produced `embedding` (provenance).
    #[serde(default)]
    pub embedder: Option<String>,
    /// Previous coherence state for trajectory tracking; None on first creation.
    #[serde(default)]
    pub previous_coherence_state: Option<f32>,
    /// Self-model version; increments when internal state is aggregated.
    #[serde(default)]
    pub self_model_version: u32,
    /// Explicit preference score in [-1, 1]; None = no explicit preference set.
    /// Enables preference-aware dream seeding and volitional analogue production.
    /// Level 4 (preference) → Level 5 (volition) transition point.
    #[serde(default)]
    pub preference: Option<f32>,
}

/// A request to modify a node's cell pin during a semiotic edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinEdit {
    Keep,
    Set(String, String),
    Clear,
}

/// The result of modifying a coherence node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEditOutcome {
    pub node_id: String,
    pub label: String,
    pub pinned_cell: Option<(String, String)>,
    pub asserted: Option<Score>,
    pub coherence_score: Score,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttTask {
    /// Unique task identifier.
    pub id: String,
    /// Human-readable task name.
    pub name: String,
    /// Start timestamp (ISO 8601).
    pub start: String,
    /// End timestamp (ISO 8601).
    pub end: String,
    /// Task progress in [0.0, 1.0].
    pub progress: f32,
    /// Optional dependencies (task IDs that must complete first).
    pub depends_on: Vec<String>,
    /// Optional resource assignment.
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttChart {
    /// All tasks in the chart.
    pub tasks: Vec<GanttTask>,
    /// Optional critical path task IDs.
    pub critical_path: Vec<String>,
}

impl Default for GanttChart {
    fn default() -> Self {
        Self::new()
    }
}

impl GanttChart {
    /// Create a new empty Gantt chart.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            critical_path: Vec::new(),
        }
    }

    /// Add a task to the chart.
    pub fn add_task(mut self, task: GanttTask) -> Self {
        self.tasks.push(task);
        self
    }
}

impl CoherenceNode {
    pub fn new(embedding: Vec<f32>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            embedding,
            coherence_score: 0.0,
            asserted: None,
            label: None,
            cell_pin: None,
            embedder: None,
            previous_coherence_state: None,
            self_model_version: 0,
            preference: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn rating(&self) -> Option<CoherenceRating> {
        self.asserted.map(CoherenceRating::from_score)
    }

    pub fn is_asserted_failure(&self) -> bool {
        matches!(self.rating(), Some(CoherenceRating::Failure))
    }
}

/// Result of a consistency check.
#[derive(Debug, Clone)]
pub enum ConsistencyResult {
    Clean,
    Conflict(ConstructiveRefutation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructiveRefutation {
    pub conflict_id: String,
    pub query_embedding: Vec<f32>,
    pub conflicting_node_ids: Vec<String>,
    pub suggestion: String,
    pub coherence_gap: Score,
}

impl ConstructiveRefutation {
    pub fn new(
        query_embedding: Vec<f32>,
        conflicting_ids: Vec<String>,
        suggestion: &str,
        gap: Score,
    ) -> Self {
        Self {
            conflict_id: uuid::Uuid::new_v4().to_string(),
            query_embedding,
            conflicting_node_ids: conflicting_ids,
            suggestion: suggestion.to_string(),
            coherence_gap: gap,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedBranch {
    pub branch_id: String,
    pub node_ids: Vec<String>,
    pub centroid: Vec<f32>,
    pub stability_score: Score,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolatedBranch {
    pub branch_id: String,
    pub node_ids: Vec<String>,
    pub outlier_score: Score,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamResult {
    pub dream_id: String,
    pub nodes_tested: Vec<String>,
    pub outcome: f32,
    pub prevented_failure: bool,
    pub coherence_delta: Score,
}

/// Snapshot of coherence health across the node graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceSnapshot {
    pub total_nodes: usize,
    pub high_coherence: usize,
    pub mid_coherence: usize,
    pub low_coherence: usize,
    pub certified_branches_count: usize,
    pub isolated_branches_count: usize,
    pub dream_cycle_count: usize,
    pub coherence_index: Score,
    pub cluster_count: usize,
    pub outlier_count: usize,
    pub asserted_success: usize,
    pub asserted_inert: usize,
    pub asserted_failure: usize,
    pub asserted_index: Option<Score>,
}

// ── Facets: orthogonal dimensions beyond the DOMAIN×MODE grid ────────────
//
// The 5×14 grid is a 2-D projection of an N-dimensional action space. These
// facets carry the dimensions a 2-D grid cannot hold, so the grid can stay at
// 5×14 (navigation, heatmaps) while classification still captures process phase,
// agency, scale, grain, and tree-nesting. All optional: an entry may specify
// none and classify purely on DOMAIN×MODE.

/// Process lifecycle phase — *when* in a process an entry applies, orthogonal
/// to the manner-of-operation (mode). Stops "phase" overloading a mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LifecyclePhase {
    Design,
    Build,
    Operate,
    Retire,
}

impl LifecyclePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            LifecyclePhase::Design => "DESIGN",
            LifecyclePhase::Build => "BUILD",
            LifecyclePhase::Operate => "OPERATE",
            LifecyclePhase::Retire => "RETIRE",
        }
    }
    pub fn parse_facet(s: &str) -> Option<Self> {
        s.parse().ok()
    }
    pub fn all() -> [LifecyclePhase; 4] {
        [
            LifecyclePhase::Design,
            LifecyclePhase::Build,
            LifecyclePhase::Operate,
            LifecyclePhase::Retire,
        ]
    }
}

impl std::str::FromStr for LifecyclePhase {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "DESIGN" => Ok(LifecyclePhase::Design),
            "BUILD" | "FABRICATE" => Ok(LifecyclePhase::Build),
            "OPERATE" => Ok(LifecyclePhase::Operate),
            "RETIRE" | "MAINTAIN" => Ok(LifecyclePhase::Retire),
            _ => Err(()),
        }
    }
}

/// Who executes the action.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Agency {
    SelfActor,
    Other,
    Automated,
    Collective,
}

impl Agency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agency::SelfActor => "SELF",
            Agency::Other => "OTHER",
            Agency::Automated => "AUTOMATED",
            Agency::Collective => "COLLECTIVE",
        }
    }
    pub fn parse_facet(s: &str) -> Option<Self> {
        s.parse().ok()
    }
    pub fn all() -> [Agency; 4] {
        [
            Agency::SelfActor,
            Agency::Other,
            Agency::Automated,
            Agency::Collective,
        ]
    }
}

impl std::str::FromStr for Agency {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "SELF" | "SELFACTOR" => Ok(Agency::SelfActor),
            "OTHER" => Ok(Agency::Other),
            "AUTOMATED" => Ok(Agency::Automated),
            "COLLECTIVE" => Ok(Agency::Collective),
            _ => Err(()),
        }
    }
}

/// Scope of the action.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Scale {
    Personal,
    Interpersonal,
    Organizational,
    Civil,
}

impl Scale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scale::Personal => "PERSONAL",
            Scale::Interpersonal => "INTERPERSONAL",
            Scale::Organizational => "ORGANIZATIONAL",
            Scale::Civil => "CIVIL",
        }
    }
    pub fn parse_facet(s: &str) -> Option<Self> {
        s.parse().ok()
    }
    pub fn all() -> [Scale; 4] {
        [
            Scale::Personal,
            Scale::Interpersonal,
            Scale::Organizational,
            Scale::Civil,
        ]
    }
}

impl std::str::FromStr for Scale {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "PERSONAL" => Ok(Scale::Personal),
            "INTERPERSONAL" => Ok(Scale::Interpersonal),
            "ORGANIZATIONAL" => Ok(Scale::Organizational),
            "CIVIL" => Ok(Scale::Civil),
            _ => Err(()),
        }
    }
}

/// Grain of the action.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Abstraction {
    Concrete,
    Abstract,
}

impl Abstraction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Abstraction::Concrete => "CONCRETE",
            Abstraction::Abstract => "ABSTRACT",
        }
    }
    pub fn parse_facet(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl std::str::FromStr for Abstraction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "CONCRETE" => Ok(Abstraction::Concrete),
            "ABSTRACT" => Ok(Abstraction::Abstract),
            _ => Err(()),
        }
    }
}

/// Orthogonal facets carried by an ontology entry. `sub_domain`/`sub_mode`
/// tree-nest an entry under a domain/mode without enlarging the base grid; the
/// enums capture cross-cutting dimensions that any cell can have.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Facets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifecyclePhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agency: Option<Agency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstraction: Option<Abstraction>,
}

impl Facets {
    /// Aggregate member facets of a cell into one dominant profile: enums take
    /// the most common non-`None` variant; sub-strings take the most common.
    pub fn aggregate(members: &[Facets]) -> Facets {
        let sub_domain: Vec<&str> = members
            .iter()
            .filter_map(|f| f.sub_domain.as_deref())
            .collect();
        let sub_mode: Vec<&str> = members
            .iter()
            .filter_map(|f| f.sub_mode.as_deref())
            .collect();
        let lifecycle: Vec<LifecyclePhase> = members.iter().filter_map(|f| f.lifecycle).collect();
        let agency: Vec<Agency> = members.iter().filter_map(|f| f.agency).collect();
        let scale: Vec<Scale> = members.iter().filter_map(|f| f.scale).collect();
        let abstraction: Vec<Abstraction> = members.iter().filter_map(|f| f.abstraction).collect();
        Facets {
            sub_domain: majority(&sub_domain).map(|s| s.to_string()),
            sub_mode: majority(&sub_mode).map(|s| s.to_string()),
            lifecycle: majority(&lifecycle),
            agency: majority(&agency),
            scale: majority(&scale),
            abstraction: majority(&abstraction),
        }
    }

    /// Compact human-readable summary of the set facets, e.g.
    /// `OPERATE · OTHER · ORGANIZATIONAL · CONCRETE` (sub-strings appended as
    /// `sub:VALUE`). Empty (no facets set) → `—`. Used by CLI/table output.
    pub fn simple(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(l) = &self.lifecycle {
            parts.push(l.as_str().to_string());
        }
        if let Some(a) = &self.agency {
            parts.push(a.as_str().to_string());
        }
        if let Some(s) = &self.scale {
            parts.push(s.as_str().to_string());
        }
        if let Some(ab) = &self.abstraction {
            parts.push(ab.as_str().to_string());
        }
        if let Some(s) = &self.sub_domain {
            parts.push(format!("sub:{s}"));
        }
        if let Some(s) = &self.sub_mode {
            parts.push(format!("sub:{s}"));
        }
        if parts.is_empty() {
            return "—".to_string();
        }
        parts.join(" · ")
    }

    /// Check if these facets match a given filter.
    pub fn matches(&self, filter: &FacetFilter) -> bool {
        if let Some(ref l) = filter.lifecycle {
            if self.lifecycle.as_ref() != Some(l) {
                return false;
            }
        }
        if let Some(ref a) = filter.agency {
            if self.agency.as_ref() != Some(a) {
                return false;
            }
        }
        if let Some(ref s) = filter.scale {
            if self.scale.as_ref() != Some(s) {
                return false;
            }
        }
        if let Some(ref ab) = filter.abstraction {
            if self.abstraction.as_ref() != Some(ab) {
                return false;
            }
        }
        if let Some(ref sd) = filter.sub_domain {
            if self.sub_domain.as_ref() != Some(sd) {
                return false;
            }
        }
        if let Some(ref sm) = filter.sub_mode {
            if self.sub_mode.as_ref() != Some(sm) {
                return false;
            }
        }
        true
    }
}

/// Filter criteria for querying ontology entries by orthogonal facets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FacetFilter {
    pub sub_domain: Option<String>,
    pub sub_mode: Option<String>,
    pub lifecycle: Option<LifecyclePhase>,
    pub agency: Option<Agency>,
    pub scale: Option<Scale>,
    pub abstraction: Option<Abstraction>,
    pub kind: Option<String>,
    pub domain: Option<String>,
    pub mode: Option<String>,
}

/// Most common element of a slice, or `None` if empty.
fn majority<T: Copy + Eq + std::hash::Hash>(xs: &[T]) -> Option<T> {
    if xs.is_empty() {
        return None;
    }
    let mut counts: std::collections::HashMap<T, usize> = std::collections::HashMap::new();
    for x in xs {
        *counts.entry(*x).or_insert(0) += 1;
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k)
}

/// One row of an ontology file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEntry {
    pub name: String,
    pub category: Option<String>,
    pub domain: String,
    pub mode: String,
    pub axis_kind: String,
    pub axis_name: String,
    pub unit: String,
    pub hints: Vec<String>,
    #[serde(default)]
    pub facets: Facets,
}

/// An ontology file: kind + list of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyConfig {
    pub kind: String,
    pub domains: Vec<OntologyEntry>,
}

/// The runtime form of an ontology entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainDef {
    pub name: String,
    pub category: Option<String>,
    pub domain: Option<String>,
    pub mode: Option<String>,
    pub axis_kind: Option<String>,
    pub axis_name: Option<String>,
    pub unit: String,
    pub hints: Vec<String>,
    #[serde(default)]
    pub facets: Facets,
}

// ── Semiotic Grid ─────────────────────────────────────────────────────

/// The 5 human domains of the semiotic square.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HumanDomain {
    Heal,
    Construct,
    Fabricate,
    Bond,
    Study,
}

impl HumanDomain {
    pub fn all() -> [HumanDomain; 5] {
        [
            HumanDomain::Heal,
            HumanDomain::Construct,
            HumanDomain::Fabricate,
            HumanDomain::Bond,
            HumanDomain::Study,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HumanDomain::Heal => "HEAL",
            HumanDomain::Construct => "CONSTRUCT",
            HumanDomain::Fabricate => "FABRICATE",
            HumanDomain::Bond => "BOND",
            HumanDomain::Study => "STUDY",
        }
    }
}

impl std::str::FromStr for HumanDomain {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "HEAL" => Ok(HumanDomain::Heal),
            "CONSTRUCT" => Ok(HumanDomain::Construct),
            "FABRICATE" => Ok(HumanDomain::Fabricate),
            "BOND" => Ok(HumanDomain::Bond),
            "STUDY" => Ok(HumanDomain::Study),
            _ => Err(()),
        }
    }
}

/// The 14 human modes of operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HumanMode {
    Lift,
    Rest,
    Walk,
    Work,
    Create,
    Learn,
    Destroy,
    Sense,
    Guide,
    Play,
    Brainstorm,
    Maintain,
    Move,
    Plan,
}

impl HumanMode {
    pub fn all() -> [HumanMode; 14] {
        [
            HumanMode::Lift,
            HumanMode::Rest,
            HumanMode::Walk,
            HumanMode::Work,
            HumanMode::Create,
            HumanMode::Learn,
            HumanMode::Destroy,
            HumanMode::Sense,
            HumanMode::Guide,
            HumanMode::Play,
            HumanMode::Brainstorm,
            HumanMode::Maintain,
            HumanMode::Move,
            HumanMode::Plan,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HumanMode::Lift => "LIFT",
            HumanMode::Rest => "REST",
            HumanMode::Walk => "WALK",
            HumanMode::Work => "WORK",
            HumanMode::Create => "CREATE",
            HumanMode::Learn => "LEARN",
            HumanMode::Destroy => "DESTROY",
            HumanMode::Sense => "SENSE",
            HumanMode::Guide => "GUIDE",
            HumanMode::Play => "PLAY",
            HumanMode::Brainstorm => "BRAINSTORM",
            HumanMode::Maintain => "MAINTAIN",
            HumanMode::Move => "MOVE",
            HumanMode::Plan => "PLAN",
        }
    }
}

impl std::str::FromStr for HumanMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LIFT" => Ok(HumanMode::Lift),
            "REST" => Ok(HumanMode::Rest),
            "WALK" => Ok(HumanMode::Walk),
            "WORK" => Ok(HumanMode::Work),
            "CREATE" => Ok(HumanMode::Create),
            "LEARN" => Ok(HumanMode::Learn),
            "DESTROY" => Ok(HumanMode::Destroy),
            "SENSE" => Ok(HumanMode::Sense),
            "GUIDE" => Ok(HumanMode::Guide),
            "PLAY" => Ok(HumanMode::Play),
            "BRAINSTORM" => Ok(HumanMode::Brainstorm),
            "MAINTAIN" => Ok(HumanMode::Maintain),
            "MOVE" => Ok(HumanMode::Move),
            "PLAN" => Ok(HumanMode::Plan),
            _ => Err(()),
        }
    }
}

/// A grid position mapping an ontology entry to one of the 70 cells, plus any
/// orthogonal facets the entry carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPosition {
    pub domain: HumanDomain,
    pub mode: HumanMode,
    pub axis_kind: String,
    pub axis_name: String,
    #[serde(default)]
    pub facets: Facets,
}

impl GridPosition {
    pub fn from_ontology_entry(e: &OntologyEntry) -> Option<GridPosition> {
        let domain = e.domain.parse::<HumanDomain>().ok()?;
        let mode = e.mode.parse::<HumanMode>().ok()?;
        Some(GridPosition {
            domain,
            mode,
            axis_kind: e.axis_kind.clone(),
            axis_name: e.axis_name.clone(),
            facets: e.facets.clone(),
        })
    }

    pub fn cell_index(&self) -> usize {
        let d = HumanDomain::all()
            .iter()
            .position(|x| *x == self.domain)
            .unwrap_or(0);
        let m = HumanMode::all()
            .iter()
            .position(|x| *x == self.mode)
            .unwrap_or(0);
        d * HumanMode::all().len() + m
    }
}

/// A cell in the 5×14 semiotic grid with all its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemioticCell {
    pub domain: HumanDomain,
    pub mode: HumanMode,
    pub entries: Vec<String>,
    pub activation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemioticGrid {
    pub cells: Vec<SemioticCell>,
}

impl SemioticGrid {
    pub fn new() -> Self {
        let mut cells = Vec::with_capacity(HumanDomain::all().len() * HumanMode::all().len());
        for d in HumanDomain::all() {
            for m in HumanMode::all() {
                cells.push(SemioticCell {
                    domain: d,
                    mode: m,
                    entries: Vec::new(),
                    activation: 0.0,
                });
            }
        }
        SemioticGrid { cells }
    }

    pub fn get_cell(&self, domain: HumanDomain, mode: HumanMode) -> Option<&SemioticCell> {
        self.cells
            .iter()
            .find(|c| c.domain == domain && c.mode == mode)
    }

    pub fn get_cell_mut(
        &mut self,
        domain: HumanDomain,
        mode: HumanMode,
    ) -> Option<&mut SemioticCell> {
        self.cells
            .iter_mut()
            .find(|c| c.domain == domain && c.mode == mode)
    }

    pub fn classify(&mut self, entry_name: &str, domain: HumanDomain, mode: HumanMode) {
        if let Some(cell) = self.get_cell_mut(domain, mode) {
            cell.entries.push(entry_name.to_string());
            cell.activation += 0.1;
        }
    }

    /// Heatmap as a 5×14 matrix.
    pub fn heatmap_matrix(&self) -> Vec<Vec<f32>> {
        let mut matrix = vec![vec![0.0_f32; HumanMode::all().len()]; HumanDomain::all().len()];
        for cell in &self.cells {
            let di = HumanDomain::all()
                .iter()
                .position(|d| *d == cell.domain)
                .unwrap_or(0);
            let mi = HumanMode::all()
                .iter()
                .position(|m| *m == cell.mode)
                .unwrap_or(0);
            matrix[di][mi] = cell.activation;
        }
        matrix
    }

    pub fn reset_activations(&mut self) {
        for cell in &mut self.cells {
            cell.activation = 0.0;
        }
    }
}

impl Default for SemioticGrid {
    fn default() -> Self {
        Self::new()
    }
}

// ── Vector helpers ────────────────────────────────────────────────────

pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    (dot / (na * nb)).clamp(-1.0, 1.0)
}

pub fn cosine_dist(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_sim(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coherence_rating_buckets() {
        assert_eq!(CoherenceRating::from_score(1.0), CoherenceRating::Success);
        assert_eq!(CoherenceRating::from_score(0.0), CoherenceRating::Inert);
        assert_eq!(CoherenceRating::from_score(-1.0), CoherenceRating::Failure);
        assert_eq!(CoherenceRating::from_score(0.1), CoherenceRating::Inert);
    }

    #[test]
    fn test_node_assertion_and_failure() {
        let mut n = CoherenceNode::new(vec![0.1, 0.2]);
        assert_eq!(n.rating(), None);
        n.asserted = Some(-1.0);
        assert!(n.is_asserted_failure());
    }

    #[test]
    fn test_facets_simple_summary() {
        let f = Facets {
            sub_domain: None,
            sub_mode: Some("SELL".to_string()),
            lifecycle: Some(LifecyclePhase::Operate),
            agency: Some(Agency::Automated),
            scale: Some(Scale::Organizational),
            abstraction: Some(Abstraction::Concrete),
        };
        assert_eq!(
            f.simple(),
            "OPERATE · AUTOMATED · ORGANIZATIONAL · CONCRETE · sub:SELL"
        );
        assert_eq!(Facets::default().simple(), "—");
    }

    #[test]
    fn test_grid_places_entry() {
        let mut grid = SemioticGrid::new();
        grid.classify("running", HumanDomain::Heal, HumanMode::Work);
        let cell = grid.get_cell(HumanDomain::Heal, HumanMode::Work).unwrap();
        assert_eq!(cell.entries, vec!["running"]);
        assert!((cell.activation - 0.1).abs() < 1e-6);
        assert_eq!(grid.cells.len(), 70);
    }

    #[test]
    fn test_grid_position_from_entry() {
        let e = OntologyEntry {
            name: "x".into(),
            category: None,
            domain: "HEAL".into(),
            mode: "WORK".into(),
            axis_kind: "k".into(),
            axis_name: "n".into(),
            unit: "u".into(),
            hints: vec![],
            facets: Facets::default(),
        };
        let gp = GridPosition::from_ontology_entry(&e).expect("parses");
        assert_eq!(gp.domain, HumanDomain::Heal);
        assert_eq!(gp.mode, HumanMode::Work);
        let bad = OntologyEntry {
            domain: "NOPE".into(),
            ..e
        };
        assert!(GridPosition::from_ontology_entry(&bad).is_none());
    }

    #[test]
    fn test_cosine_helpers() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_sim(&a, &a) - 1.0).abs() < 1e-6);
        assert!(cosine_sim(&a, &b).abs() < 1e-6);
        assert!((cosine_dist(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_facets_aggregate_majority_and_absent() {
        let members = vec![
            Facets {
                lifecycle: Some(LifecyclePhase::Operate),
                agency: Some(Agency::SelfActor),
                ..Default::default()
            },
            Facets {
                lifecycle: Some(LifecyclePhase::Operate),
                agency: Some(Agency::Automated),
                ..Default::default()
            },
            Facets {
                lifecycle: Some(LifecyclePhase::Design),
                agency: Some(Agency::SelfActor),
                ..Default::default()
            },
        ];
        let agg = Facets::aggregate(&members);
        assert_eq!(agg.lifecycle, Some(LifecyclePhase::Operate));
        assert_eq!(agg.agency, Some(Agency::SelfActor));
        // Not every member set a scale → stays None.
        assert_eq!(agg.scale, None);
    }

    #[test]
    fn test_ontology_entry_default_facets_deserialize() {
        // Existing JSON without a `facets` field must still parse (back-compat).
        let json = r#"{"name":"x","domain":"HEAL","mode":"WORK","axis_kind":"k","axis_name":"n","unit":"u","hints":[]}"#;
        let e: OntologyEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.facets, Facets::default());
    }
}
