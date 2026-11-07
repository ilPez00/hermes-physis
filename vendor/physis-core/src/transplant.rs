//! Structural transplant — port the shape of one project onto another.
//!
//! Two projects with the same *shape* (two Rust packages, two Android apps,
//! two static sites) differ by contents. Transplant aligns their structure
//! graphs, proposes the transplant as a plan, and names the adaptations the
//! particular differences demand — **proposals only, never writes** (the
//! same propose-don't-write discipline as [`crate::delta_engine`] and
//! [`crate::dream`]).
//!
//! Correspondences are *hypotheses with evidence*, not verdicts. Four
//! matchers vote per pair — path shape, role, name tokens, sibling
//! topology — and the fused score is a documented weighted mean. Matchers
//! that disagree leave a recorded contradiction; nothing is silently
//! resolved (Physis proposes, carries and defers; it does not adjudicate).
//! The weighting is shape-heavy on purpose: a module that was renamed but
//! kept its place must still align — the shape survives the rename, the
//! name does not.
//!
//! Slice-1 limits, stated before the gates: directory nodes align on name
//! evidence only (a directory has no role signal beyond its name); the
//! sibling matcher uses parent identity, not sibling order; content
//! adaptation is token-level drift naming, not a rewriter.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Proposal floor: a fused score below this is unaligned, and the human
/// decides. Ambiguous ties above it are aligned *and* recorded.
pub const TRANSPLANT_FLOOR: f32 = 0.5;

/// Matcher weights — shape-heavy on purpose (see module docs). Frozen like
/// the fitness weights: tuning happens as separate scored configs, never
/// by editing these in place.
pub const WEIGHT_PATH_SHAPE: f32 = 0.25;
pub const WEIGHT_ROLE: f32 = 0.30;
pub const WEIGHT_NAME_TOKENS: f32 = 0.15;
pub const WEIGHT_SIBLING: f32 = 0.30;

/// Directory names that are build output / tooling noise, not shape.
const SKIPPED_DIRS: [&str; 8] = [
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    ".gradle",
    "__pycache__",
    ".idea",
];

/// Auto-generated lockfiles: identical shape across projects, zero signal.
const SKIPPED_FILES: [&str; 3] = ["Cargo.lock", "package-lock.json", "poetry.lock"];

/// The role a path plays in a project's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    /// Project manifest (Cargo.toml, package.json, AndroidManifest.xml, …).
    Manifest,
    /// Source code.
    Source,
    /// A test.
    Test,
    /// Configuration (TOML/YAML/JSON/INI/XML that is not the manifest).
    Config,
    /// Documentation (md/rst/adoc).
    Doc,
    /// Binary asset.
    Asset,
    /// A directory.
    Directory,
    /// Anything else.
    Other,
}

/// One node of a project's shape graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeNode {
    /// Path relative to the project root, `/`-separated, deterministic order.
    pub rel_path: String,
    pub kind: ShapeKind,
    /// Lowercased name tokens (separator- and camelCase-split).
    pub tokens: Vec<String>,
    /// Directory depth (root files are 0).
    pub depth: usize,
}

/// A project's shape: every non-noise path, in deterministic order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectShape {
    pub root: String,
    pub nodes: Vec<ShapeNode>,
}

/// The four matchers that vote on a correspondence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatcherTag {
    PathShape,
    RoleMatch,
    NameTokens,
    SiblingTopology,
}

impl MatcherTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatcherTag::PathShape => "path_shape",
            MatcherTag::RoleMatch => "role",
            MatcherTag::NameTokens => "name_tokens",
            MatcherTag::SiblingTopology => "sibling_topology",
        }
    }
}

/// Split a name into lowercase tokens on separators and camelCase humps.
pub fn name_tokens(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' || c == ' ' || c == '/' {
            if !word.is_empty() {
                out.push(word.to_lowercase());
                word.clear();
            }
            prev_lower = false;
        } else if c.is_uppercase() && prev_lower {
            if !word.is_empty() {
                out.push(word.to_lowercase());
                word.clear();
            }
            word.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            word.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase();
        }
    }
    if !word.is_empty() {
        out.push(word.to_lowercase());
    }
    out
}

/// Jaccard overlap of two token/segment lists.
pub fn jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let sa: HashSet<&String> = a.iter().collect();
    let sb: HashSet<&String> = b.iter().collect();
    let inter = sa.intersection(&sb).count() as f32;
    let union = sa.union(&sb).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

fn segments(rel_path: &str) -> Vec<String> {
    rel_path.split('/').map(|s| s.to_string()).collect()
}

fn parent_of(rel_path: &str) -> String {
    match rel_path.rfind('/') {
        Some(i) => rel_path[..i].to_string(),
        None => String::new(),
    }
}

impl ShapeKind {
    /// Classify a relative path. Heuristic, ordered: manifest names, test
    /// markers, then extension buckets. Deliberately simple — role is one
    /// evidence tag among four, not an oracle.
    pub fn from_rel_path(rel_path: &str) -> ShapeKind {
        const MANIFESTS: [&str; 9] = [
            "Cargo.toml",
            "package.json",
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
            "AndroidManifest.xml",
            "pyproject.toml",
            "go.mod",
            "Gemfile",
        ];
        let name = match rel_path.rfind('/') {
            Some(i) => &rel_path[i + 1..],
            None => rel_path,
        };
        if MANIFESTS.contains(&name) {
            return ShapeKind::Manifest;
        }
        let lower = rel_path.to_lowercase();
        let stem = match name.rfind('.') {
            Some(i) => &name[..i],
            None => name,
        };
        let stem_lower = stem.to_lowercase();
        let is_test = lower.contains("/test")
            || lower.starts_with("test")
            || stem_lower.ends_with("_test")
            || stem_lower.ends_with(".test");
        if is_test {
            return ShapeKind::Test;
        }
        let ext = match name.rfind('.') {
            Some(i) => name[i + 1..].to_lowercase(),
            None => String::new(),
        };
        match ext.as_str() {
            "md" | "rst" | "adoc" => ShapeKind::Doc,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => ShapeKind::Asset,
            "toml" | "yaml" | "yml" | "json" | "ini" | "conf" | "properties" | "xml" => {
                ShapeKind::Config
            }
            "rs" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt" | "py" | "go" | "rb" | "c" | "h"
            | "cpp" | "hpp" | "swift" | "php" | "cs" => ShapeKind::Source,
            _ => ShapeKind::Other,
        }
    }
}

impl ProjectShape {
    /// Scan a project root into a deterministic shape. Build outputs, VCS
    /// metadata and lockfiles are noise and are skipped — two same-shape
    /// projects must produce the same shape however far their builds ran.
    pub fn scan(root: &Path) -> std::io::Result<Self> {
        let root_str = root.to_string_lossy().to_string();
        let root_prefix = root_str.trim_end_matches('/').to_string();
        let mut collected: Vec<(String, bool)> = Vec::new();
        let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)?;
            let mut paths: Vec<std::path::PathBuf> =
                entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let full = path.to_string_lossy().to_string();
                let rel = match full.strip_prefix(&root_prefix) {
                    Some(rest) => rest.trim_start_matches('/').to_string(),
                    None => continue,
                };
                if rel.is_empty() {
                    continue;
                }
                if path.is_dir() {
                    if SKIPPED_DIRS.contains(&name.as_str()) {
                        continue;
                    }
                    collected.push((rel.clone(), true));
                    stack.push(path);
                } else {
                    if SKIPPED_FILES.contains(&name.as_str()) {
                        continue;
                    }
                    collected.push((rel, false));
                }
            }
        }
        collected.sort();
        collected.dedup();
        let mut nodes: Vec<ShapeNode> = Vec::new();
        for (rel, is_dir) in collected {
            let kind = if is_dir {
                ShapeKind::Directory
            } else {
                ShapeKind::from_rel_path(&rel)
            };
            let name = match rel.rfind('/') {
                Some(i) => &rel[i + 1..],
                None => rel.as_str(),
            };
            let depth = segments(&rel).len().saturating_sub(1);
            let tokens = name_tokens(name);
            nodes.push(ShapeNode {
                rel_path: rel,
                kind,
                tokens,
                depth,
            });
        }
        nodes.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(ProjectShape {
            root: root_str,
            nodes,
        })
    }

    fn node(&self, rel_path: &str) -> Option<&ShapeNode> {
        self.nodes.iter().find(|n| n.rel_path == rel_path)
    }
}

/// One proposed correspondence: donor path → recipient path, with the
/// matcher evidence that produced it and the contradictions the matchers
/// left unresolved. A proposal is a hypothesis for a person to accept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrespondenceProposal {
    pub donor: String,
    pub recipient: String,
    /// Fused matcher score in [0, 1]; ≥ [`TRANSPLANT_FLOOR`].
    pub score: f32,
    pub evidence: Vec<(MatcherTag, f32)>,
    /// Unresolved matcher disagreement, recorded — never silently resolved.
    pub contradictions: Vec<String>,
}

/// Matcher scores for one (donor, recipient) pair. Directories align on
/// name evidence only; files use the full four-matcher fusion.
pub fn matcher_evidence(a: &ShapeNode, b: &ShapeNode) -> (f32, Vec<(MatcherTag, f32)>) {
    let path = jaccard(&segments(&a.rel_path), &segments(&b.rel_path));
    let role = if a.kind == b.kind { 1.0 } else { 0.0 };
    let tokens = jaccard(&a.tokens, &b.tokens);
    let sibling = if a.kind == b.kind && parent_of(&a.rel_path) == parent_of(&b.rel_path) {
        1.0
    } else {
        0.0
    };
    if a.kind == ShapeKind::Directory {
        // Directories carry no role signal beyond their name: align on the
        // two name evidences only (weights renormalised over the pair).
        let fused = WEIGHT_PATH_SHAPE * path + WEIGHT_NAME_TOKENS * tokens;
        let fused = fused / (WEIGHT_PATH_SHAPE + WEIGHT_NAME_TOKENS);
        (
            fused,
            vec![
                (MatcherTag::PathShape, path),
                (MatcherTag::NameTokens, tokens),
            ],
        )
    } else {
        let fused = WEIGHT_PATH_SHAPE * path
            + WEIGHT_ROLE * role
            + WEIGHT_NAME_TOKENS * tokens
            + WEIGHT_SIBLING * sibling;
        (
            fused,
            vec![
                (MatcherTag::PathShape, path),
                (MatcherTag::RoleMatch, role),
                (MatcherTag::NameTokens, tokens),
                (MatcherTag::SiblingTopology, sibling),
            ],
        )
    }
}

/// Record the disagreement the matchers left on the table — Physis keeps
/// the tension visible instead of picking a silent winner (it does not
/// adjudicate meaning; a person resolves it).
fn matcher_contradictions(
    a: &ShapeNode,
    b: &ShapeNode,
    evidence: &[(MatcherTag, f32)],
    ambiguous: bool,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let get = |tag: MatcherTag| -> f32 {
        evidence
            .iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    };
    let path = get(MatcherTag::PathShape);
    let tokens = get(MatcherTag::NameTokens);
    if let Some((_, role)) = evidence.iter().find(|(t, _)| *t == MatcherTag::RoleMatch) {
        if *role == 0.0 {
            out.push(format!(
                "role mismatch: donor {:?} vs recipient {:?}",
                a.kind, b.kind
            ));
        }
    }
    if tokens < 0.3 && path >= 0.25 {
        out.push(format!(
            "shape beats name: '{}' and '{}' share little vocabulary but sit alike",
            a.rel_path, b.rel_path
        ));
    }
    if tokens >= 0.5 && path < 0.3 {
        out.push(format!(
            "name beats shape: '{}' and '{}' share vocabulary but sit in different places",
            a.rel_path, b.rel_path
        ));
    }
    if ambiguous {
        out.push(format!(
            "ambiguous: '{}' and '{}' score within the tie epsilon",
            a.rel_path, b.rel_path
        ));
    }
    out
}

/// Propose donor↔recipient correspondences. Greedy 1:1 in donor scan
/// order (deterministic); each recipient is used at most once. Every
/// proposal above the floor carries its matcher evidence and any recorded
/// contradictions; below the floor the pair is left unaligned for a human.
pub fn propose_correspondences(
    donor: &ProjectShape,
    recipient: &ProjectShape,
) -> Vec<CorrespondenceProposal> {
    let mut proposals: Vec<CorrespondenceProposal> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    for a in &donor.nodes {
        #[allow(clippy::type_complexity)]
        let mut ranked: Vec<(String, f32, Vec<(MatcherTag, f32)>)> = Vec::new();
        for b in &recipient.nodes {
            if used.contains(&b.rel_path) {
                continue;
            }
            let (score, evidence) = matcher_evidence(a, b);
            ranked.push((b.rel_path.clone(), score, evidence));
        }
        // Best score first; ties break by recipient path ascending.
        ranked.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        if ranked.is_empty() {
            break;
        }
        let (best_path, best_score, best_evidence) = ranked[0].clone();
        if best_score < TRANSPLANT_FLOOR {
            continue;
        }
        let ambiguous =
            ranked.len() > 1 && best_score < 1.0 && (best_score - ranked[1].1).abs() < 1e-3;
        let b_node = recipient
            .node(&best_path)
            .expect("ranked candidate comes from the recipient shape");
        let contradictions = matcher_contradictions(a, b_node, &best_evidence, ambiguous);
        used.insert(best_path.clone());
        proposals.push(CorrespondenceProposal {
            donor: a.rel_path.clone(),
            recipient: best_path,
            score: best_score,
            evidence: best_evidence,
            contradictions,
        });
    }
    proposals
}

/// A proposed transplant: descriptive operations only — building a plan
/// touches no filesystem (the same discipline as the dream engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransplantOp {
    /// Counterpart exists and matches in name: nothing to do.
    Keep { path: String },
    /// Counterpart exists but the donor renamed it: rewrite the recipient
    /// side, adapting to the token drift the pair actually shows.
    Rewrite {
        target: String,
        donor: String,
        /// Name tokens the donor name carries that the recipient does not.
        adaptations: Vec<String>,
    },
    /// Donor path with no recipient counterpart: propose creating it at the
    /// mirrored path (parent creation implied). Never a Delete — recipient
    /// paths with no donor counterpart are listed for a human, not removed.
    Create { to: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransplantPlan {
    /// The applied direction, e.g. "alpha→beta".
    pub direction: String,
    pub ops: Vec<TransplantOp>,
    pub unaligned_donor: Vec<String>,
    pub unaligned_recipient: Vec<String>,
}

/// Build the transplant plan that applies the donor's structure to the
/// recipient, adapting for the particular differences the pair shows.
/// Pure: two calls over the same inputs give the same plan, and no path
/// on disk changes.
pub fn build_transplant_plan(
    donor: &ProjectShape,
    recipient: &ProjectShape,
    correspondences: &[CorrespondenceProposal],
) -> TransplantPlan {
    let mut ops: Vec<TransplantOp> = Vec::new();

    for corr in correspondences {
        let pair = match (donor.node(&corr.donor), recipient.node(&corr.recipient)) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        if pair.0.tokens == pair.1.tokens {
            ops.push(TransplantOp::Keep {
                path: pair.1.rel_path.clone(),
            });
        } else {
            let missing: Vec<String> = pair
                .0
                .tokens
                .iter()
                .filter(|t| !pair.1.tokens.contains(t))
                .cloned()
                .collect();
            ops.push(TransplantOp::Rewrite {
                target: pair.1.rel_path.clone(),
                donor: pair.0.rel_path.clone(),
                adaptations: missing,
            });
        }
    }

    let aligned_donor: HashSet<String> = correspondences.iter().map(|p| p.donor.clone()).collect();
    let aligned_recipient: HashSet<String> = correspondences
        .iter()
        .map(|p| p.recipient.clone())
        .collect();

    // Unaligned donor paths: propose creating them at the mirrored path —
    // a child under an already-proposed parent is covered by it.
    let mut unaligned_donor: Vec<String> = Vec::new();
    let mut proposed: HashSet<String> = HashSet::new();
    for a in &donor.nodes {
        if aligned_donor.contains(&a.rel_path) {
            continue;
        }
        if proposed.contains(&parent_of(&a.rel_path)) {
            continue;
        }
        proposed.insert(a.rel_path.clone());
        unaligned_donor.push(a.rel_path.clone());
        ops.push(TransplantOp::Create {
            to: a.rel_path.clone(),
        });
    }

    // Unaligned recipient paths: listed for a human, never deleted.
    let mut unaligned_recipient: Vec<String> = Vec::new();
    for b in &recipient.nodes {
        if !aligned_recipient.contains(&b.rel_path) {
            unaligned_recipient.push(b.rel_path.clone());
        }
    }

    TransplantPlan {
        direction: format!("{}→{}", donor.root, recipient.root),
        ops,
        unaligned_donor,
        unaligned_recipient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(rel: &str, kind: ShapeKind) -> ShapeNode {
        let name = match rel.rfind('/') {
            Some(i) => &rel[i + 1..],
            None => rel,
        };
        ShapeNode {
            rel_path: rel.to_string(),
            kind,
            tokens: name_tokens(name),
            depth: rel.split('/').count() - 1,
        }
    }

    fn shape(root: &str, nodes: Vec<ShapeNode>) -> ProjectShape {
        let mut nodes = nodes;
        nodes.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        ProjectShape {
            root: root.to_string(),
            nodes,
        }
    }

    /// The core transplant claim: two same-shape projects align one to one,
    /// and identical counterparts propose Keep, not Create.
    #[test]
    fn same_shape_projects_align_one_to_one() {
        let donor = shape(
            "/tmp/alpha",
            vec![
                node("Cargo.toml", ShapeKind::Manifest),
                node("src", ShapeKind::Directory),
                node("src/main.rs", ShapeKind::Source),
                node("src/lib.rs", ShapeKind::Source),
                node("tests", ShapeKind::Directory),
                node("tests/loop.rs", ShapeKind::Test),
            ],
        );
        let recipient = shape(
            "/tmp/beta",
            vec![
                node("Cargo.toml", ShapeKind::Manifest),
                node("src", ShapeKind::Directory),
                node("src/main.rs", ShapeKind::Source),
                node("src/lib.rs", ShapeKind::Source),
                node("tests", ShapeKind::Directory),
                node("tests/loop.rs", ShapeKind::Test),
            ],
        );
        let corr = propose_correspondences(&donor, &recipient);
        assert_eq!(corr.len(), 6, "every path aligns one to one");
        for c in &corr {
            assert_eq!(c.donor, c.recipient, "identical shapes align identically");
            assert!(c.score >= TRANSPLANT_FLOOR);
            assert_eq!(
                c.contradictions.len(),
                0,
                "clean alignment records no contradiction"
            );
        }
        let plan = build_transplant_plan(&donor, &recipient, &corr);
        assert!(
            plan.ops
                .iter()
                .all(|op| matches!(op, TransplantOp::Keep { .. })),
            "identical shapes need only Keep: {:?}",
            plan.ops
        );
        assert!(plan.unaligned_donor.is_empty());
        assert!(plan.unaligned_recipient.is_empty());
    }

    /// The shape-heavy weighting claim: a module that was renamed but kept
    /// its place still aligns — and the plan names the token drift as the
    /// adaptation the rewrite must carry.
    #[test]
    fn renamed_module_still_aligns_and_names_adaptations() {
        let donor = shape(
            "/tmp/alpha",
            vec![
                node("src", ShapeKind::Directory),
                node("src/pipeline_stage.rs", ShapeKind::Source),
            ],
        );
        let recipient = shape(
            "/tmp/beta",
            vec![
                node("src", ShapeKind::Directory),
                node("src/worker.rs", ShapeKind::Source),
            ],
        );
        let corr = propose_correspondences(&donor, &recipient);
        // Two correspondences: directory alignment + renamed file alignment
        assert_eq!(corr.len(), 2, "both directory and renamed module align");
        // Check both correspondences exist
        let dir_correspondence = corr.iter().find(|c| c.recipient == "src");
        let file_correspondence = corr.iter().find(|c| c.recipient == "src/worker.rs");
        assert!(dir_correspondence.is_some(), "directory should align");
        assert!(file_correspondence.is_some(), "renamed file should align");
        // The file correspondence should have "src/worker.rs" as recipient
        let file_corr = file_correspondence.unwrap();
        assert_eq!(file_corr.recipient, "src/worker.rs");
        // The name disagreement should be recorded
        assert!(
            file_corr
                .contradictions
                .iter()
                .any(|c| c.contains("shape beats name")),
            "the name disagreement is recorded, not hidden: {:?}",
            file_corr.contradictions
        );

        let plan = build_transplant_plan(&donor, &recipient, &corr);
        // Both the directory Keep and the file Rewrite should be present
        let keeps: Vec<&TransplantOp> = plan
            .ops
            .iter()
            .filter(|op| matches!(op, TransplantOp::Keep { .. }))
            .collect();
        let rewrites: Vec<&TransplantOp> = plan
            .ops
            .iter()
            .filter(|op| matches!(op, TransplantOp::Rewrite { .. }))
            .collect();
        // Directory gets Keep, file gets Rewrite
        assert_eq!(keeps.len(), 1, "directory should propose Keep");
        assert_eq!(rewrites.len(), 1, "renamed file should propose Rewrite");
        match &plan.ops[1] {
            TransplantOp::Rewrite {
                target,
                adaptations,
                ..
            } => {
                assert_eq!(target, "src/worker.rs");
                assert!(
                    adaptations.contains(&"pipeline".to_string())
                        && adaptations.contains(&"stage".to_string()),
                    "the adaptation names the donor tokens the recipient lacks: {:?}",
                    adaptations
                );
            }
            other => panic!("expected Rewrite at index 1, got {:?}", other),
        }
        // The first operation should be Keep for the directory
        match &plan.ops[0] {
            TransplantOp::Keep { .. } => {}
            other => panic!("expected Keep at index 0, got {:?}", other),
        }
    }

    /// Invalidate-don't-delete applies to transplant too: recipient paths
    /// with no donor counterpart are listed for a human, never proposed for
    /// removal — and building a plan twice over the same shapes gives the
    /// identical plan (pure proposal, no hidden state).
    #[test]
    fn transplant_plan_never_deletes_and_is_pure() {
        let donor = shape("/tmp/alpha", vec![node("src/lib.rs", ShapeKind::Source)]);
        let recipient = shape(
            "/tmp/beta",
            vec![
                node("src/lib.rs", ShapeKind::Source),
                node("src/legacy_only_in_beta.rs", ShapeKind::Source),
                node("README.md", ShapeKind::Doc),
            ],
        );
        let corr = propose_correspondences(&donor, &recipient);
        let plan1 = build_transplant_plan(&donor, &recipient, &corr);
        let plan2 = build_transplant_plan(&donor, &recipient, &corr);
        assert_eq!(plan1, plan2, "the same shapes give the same plan");

        assert!(
            plan1
                .ops
                .iter()
                .all(|op| !matches!(op, TransplantOp::Create { .. })),
            "nothing is proposed for creation here"
        );
        assert!(
            plan1
                .unaligned_recipient
                .contains(&"src/legacy_only_in_beta.rs".to_string()),
            "the recipient-only module is surfaced to the human: {:?}",
            plan1.unaligned_recipient
        );
        assert!(
            plan1.unaligned_recipient.contains(&"README.md".to_string()),
            "the recipient-only doc is surfaced too"
        );
        assert!(
            !plan1
                .unaligned_recipient
                .iter()
                .any(|p| p.contains("lib.rs")),
            "aligned paths are not listed as unaligned"
        );
    }

    /// Scan is deterministic, skips build noise, and classifies roles: two
    /// same-shape projects must produce identical shapes whatever their
    /// build state, so the alignment cannot be fooled by lockfiles.
    #[test]
    fn scan_is_deterministic_and_skips_build_noise() {
        let dir = std::env::temp_dir().join(format!("physis_transplant_{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(&src).expect("create fixture");
        std::fs::create_dir_all(dir.join("target")).expect("create target");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("manifest");
        std::fs::write(dir.join("Cargo.lock"), "# generated").expect("lockfile");
        std::fs::write(src.join("main.rs"), "fn main() {}").expect("source");
        std::fs::write(dir.join("target").join("artifact.bin"), b"\0").expect("build output");

        let s1 = ProjectShape::scan(&dir).expect("scan");
        let s2 = ProjectShape::scan(&dir).expect("scan");
        assert_eq!(s1, s2, "two scans of the same tree are identical");

        let paths: Vec<&str> = s1.nodes.iter().map(|n| n.rel_path.as_str()).collect();
        assert!(
            !paths.iter().any(|p| p.contains("target")),
            "build output is noise, not shape: {:?}",
            paths
        );
        assert!(
            !paths.iter().any(|p| p.contains("Cargo.lock")),
            "lockfiles are generated, not shape"
        );
        assert!(paths.contains(&"Cargo.toml"), "the manifest is shape");
        assert!(paths.contains(&"src/main.rs"), "source is shape");

        let manifest = s1.node("Cargo.toml").expect("manifest node");
        assert_eq!(manifest.kind, ShapeKind::Manifest);
        let source = s1.node("src/main.rs").expect("source node");
        assert_eq!(source.kind, ShapeKind::Source);
        assert_eq!(source.depth, 1, "src/main.rs sits one level deep");
        assert_eq!(
            source.tokens,
            vec!["main", "rs"],
            "the extension splits as a token"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
