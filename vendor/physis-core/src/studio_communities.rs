//! Phase 9.4b — community rollup endpoint (PLAN 9.4 follow-up).
//!
//! `GET /api/v1/communities` — clusters the coherence graph's labeled nodes
//! via label propagation (reusing the physis-pro `communities` pattern,
//! re-implemented here against physis-core types so the studio stays
//! self-contained) and returns per-community rollups with cohesion scores.
//!
//! Read-only invariant: this handler READS `PhysisCore` and writes only its
//! own JSON response — no mutation of the shared store.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::core::PhysisCore;
use crate::studio::Shared;

/// A community rollup as returned to the studio.
#[derive(Debug, Clone, Serialize)]
pub struct CommunityJson {
    /// Community ordinal (stable within one response).
    pub id: u32,
    /// Member node labels, sorted.
    pub members: Vec<String>,
    /// Intra-community edge density in [0,1]; 1.0 for singletons.
    pub cohesion: f32,
}

/// Rollup payload for `GET /api/v1/communities`.
#[derive(Debug, Serialize)]
pub struct CommunitiesJson {
    pub communities: Vec<CommunityJson>,
    /// Nodes that carried no label and could not be clustered.
    pub unlabeled: usize,
}

/// Undirected edge list over node IDs (deduped).
fn edges_of(core: &PhysisCore) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for e in &core.edges {
        let key = if e.source_id < e.target_id {
            (e.source_id.clone(), e.target_id.clone())
        } else {
            (e.target_id.clone(), e.source_id.clone())
        };
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

/// Label propagation over ID-space; deterministic smallest-label tie-break.
fn label_propagation(ids: &[String], adj: &[Vec<usize>], max_rounds: usize) -> Vec<u32> {
    let n = ids.len();
    let mut labels: Vec<u32> = (0..n as u32).collect();
    for _ in 0..max_rounds {
        let mut changed = false;
        for v in 0..n {
            let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
            for &u in &adj[v] {
                *counts.entry(labels[u]).or_default() += 1;
            }
            if let Some((&best, _)) = counts
                .iter()
                .min_by_key(|(&l, &c)| (std::cmp::Reverse(c), l))
            {
                if best != labels[v] {
                    labels[v] = best;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // Compact to 0..k in first-seen order.
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut next = 0u32;
    labels
        .iter()
        .map(|&l| {
            *remap.entry(l).or_insert_with(|| {
                let x = next;
                next += 1;
                x
            })
        })
        .collect()
}

/// Build the rollup from core state. Split out from the handler so it is
/// unit-testable without an HTTP stack.
pub fn rollup(
    labeled: &[(String, String)], // (id, label)
    edges: &[(String, String)],   // undirected id pairs
) -> CommunitiesJson {
    let known: HashSet<&str> = labeled.iter().map(|(id, _)| id.as_str()).collect();
    let filtered: Vec<(String, String)> = edges
        .iter()
        .filter(|(a, b)| known.contains(a.as_str()) && known.contains(b.as_str()) && a != b)
        .cloned()
        .collect();

    let ids: Vec<String> = labeled.iter().map(|(id, _)| id.clone()).collect();
    let idx_of: HashMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    for (a, b) in &filtered {
        let (&ia, &ib) = (&idx_of[a.as_str()], &idx_of[b.as_str()]);
        if !adj[ia].contains(&ib) {
            adj[ia].push(ib);
        }
        if !adj[ib].contains(&ia) {
            adj[ib].push(ia);
        }
    }

    let labels = label_propagation(&ids, &adj, 16);
    let k = labels.iter().copied().max().map_or(0, |m| m + 1);

    let mut members_by: Vec<Vec<String>> = vec![Vec::new(); k as usize];
    for ((_, label), &c) in labeled.iter().zip(labels.iter()) {
        members_by[c as usize].push(label.clone());
    }
    for m in &mut members_by {
        m.sort();
    }

    // Cohesion: intra-cluster deduped edges / possible pairs.
    let mut intra = vec![0usize; k as usize];
    for (a, b) in &filtered {
        let (ia, ib) = (idx_of[a.as_str()], idx_of[b.as_str()]);
        let (ca, cb) = (labels[ia], labels[ib]);
        if ca == cb {
            intra[ca as usize] += 1;
        }
    }
    let cohesion: Vec<f32> = members_by
        .iter()
        .zip(intra.iter())
        .map(|(members, &intra_edges)| {
            let s = members.len();
            if s < 2 {
                1.0
            } else {
                let pairs = s * (s - 1) / 2;
                intra_edges as f32 / pairs as f32
            }
        })
        .collect();

    CommunitiesJson {
        communities: members_by
            .into_iter()
            .zip(cohesion)
            .enumerate()
            .filter(|(_, (m, _))| !m.is_empty())
            .map(|(i, (members, cohesion))| CommunityJson {
                id: i as u32,
                members,
                cohesion,
            })
            .collect(),
        unlabeled: 0,
    }
}

/// Axum handler: read-only community rollup over the live core.
pub async fn api_communities(State(state): State<Shared>) -> Response {
    let s = state.read().unwrap();
    let labeled: Vec<(String, String)> = s
        .core
        .nodes
        .values()
        .filter_map(|n| n.label.as_ref().map(|l| (n.id.clone(), l.clone())))
        .collect();
    let edges = edges_of(&s.core);
    drop(s);

    let body = rollup(&labeled, &edges);
    axum::Json(body).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollup_two_disconnected_pairs() {
        let labeled = vec![
            ("n1".to_string(), "alpha/one".to_string()),
            ("n2".to_string(), "alpha/two".to_string()),
            ("n3".to_string(), "beta/one".to_string()),
            ("n4".to_string(), "beta/two".to_string()),
        ];
        let edges = vec![
            ("n1".to_string(), "n2".to_string()),
            ("n3".to_string(), "n4".to_string()),
        ];
        let r = rollup(&labeled, &edges);
        assert_eq!(r.communities.len(), 2, "{r:?}");
        assert!(r
            .communities
            .iter()
            .all(|c| c.members.len() == 2 && (c.cohesion - 1.0).abs() < 1e-6));
    }

    #[test]
    fn isolated_node_is_singleton_with_full_cohesion() {
        let labeled = vec![("solo".to_string(), "x/1".to_string())];
        let r = rollup(&labeled, &[]);
        assert_eq!(r.communities.len(), 1);
        assert_eq!(r.communities[0].members, vec!["x/1"]);
        assert!((r.communities[0].cohesion - 1.0).abs() < 1e-6);
    }

    #[test]
    fn edges_to_unknown_ids_are_ignored() {
        let labeled = vec![("a".to_string(), "q/1".to_string())];
        let edges = vec![("a".to_string(), "ghost".to_string())];
        let r = rollup(&labeled, &edges);
        assert_eq!(r.communities.len(), 1);
    }

    #[test]
    fn empty_core_yields_empty_rollup_not_error() {
        let r = rollup(&[], &[]);
        assert!(r.communities.is_empty());
    }
}
