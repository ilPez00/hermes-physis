//! Symbolic token-level homomorphism engine.
//!
//! Deterministic, no embeddings. Triples are symbolic tokens; patterns match
//! by exact token equality with variable binding; multi-pattern queries join
//! by backtracking. Transforms propose candidate world states gated by a
//! [`CoherenceProfile`] scored 0/1 per dimension.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::coherence_dimensions::{CoherenceDimension, CoherenceProfile};
use crate::models::Score;

/// Symbolic relation between two tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Predicate {
    Exchanges,
    Causes,
    Requires,
    Produces,
    Supports,
    Contradicts,
    Precedes,
    Observes,
    Custom(String),
}

impl Predicate {
    pub fn as_str(&self) -> &str {
        match self {
            Predicate::Exchanges => "Exchanges",
            Predicate::Causes => "Causes",
            Predicate::Requires => "Requires",
            Predicate::Produces => "Produces",
            Predicate::Supports => "Supports",
            Predicate::Contradicts => "Contradicts",
            Predicate::Precedes => "Precedes",
            Predicate::Observes => "Observes",
            Predicate::Custom(s) => s.as_str(),
        }
    }

    pub fn is_causal(&self) -> bool {
        matches!(
            self,
            Predicate::Causes | Predicate::Produces | Predicate::Requires
        )
    }

    pub fn invert(&self) -> Self {
        match self {
            Predicate::Exchanges => Predicate::Exchanges,
            Predicate::Causes => Predicate::Causes,
            Predicate::Requires => Predicate::Produces,
            Predicate::Produces => Predicate::Requires,
            Predicate::Supports => Predicate::Supports,
            Predicate::Contradicts => Predicate::Contradicts,
            Predicate::Precedes => Predicate::Precedes,
            Predicate::Observes => Predicate::Observes,
            Predicate::Custom(s) => Predicate::Custom(s.clone()),
        }
    }
}

/// One symbolic fact: subject —predicate→ object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triple {
    pub s: String,
    pub p: Predicate,
    pub o: String,
}

impl Triple {
    pub fn new(s: impl Into<String>, p: Predicate, o: impl Into<String>) -> Self {
        Self {
            s: s.into(),
            p,
            o: o.into(),
        }
    }
}

/// One end of a pattern: a bound variable or an exact token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatElem {
    Var(String),
    Const(String),
}

/// A triple pattern; `p = None` matches any predicate without binding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriplePattern {
    pub s: PatElem,
    pub p: Option<Predicate>,
    pub o: PatElem,
}

impl TriplePattern {
    fn bind_elem(elem: &PatElem, value: &str, binding: &mut HashMap<String, String>) -> bool {
        match elem {
            PatElem::Const(c) => c == value,
            PatElem::Var(v) => match binding.get(v) {
                Some(b) => b == value,
                None => {
                    binding.insert(v.clone(), value.to_string());
                    true
                }
            },
        }
    }

    pub fn match_against(&self, fact: &Triple) -> Option<HashMap<String, String>> {
        let mut binding = HashMap::new();
        if let Some(pred) = &self.p {
            if pred != &fact.p {
                return None;
            }
        }
        if !Self::bind_elem(&self.s, &fact.s, &mut binding) {
            return None;
        }
        if !Self::bind_elem(&self.o, &fact.o, &mut binding) {
            return None;
        }
        Some(binding)
    }

    pub fn instantiate(&self, binding: &HashMap<String, String>) -> Option<Triple> {
        let pred = self.p.clone()?;
        let s = match &self.s {
            PatElem::Const(c) => c.clone(),
            PatElem::Var(v) => binding.get(v)?.clone(),
        };
        let o = match &self.o {
            PatElem::Const(c) => c.clone(),
            PatElem::Var(v) => binding.get(v)?.clone(),
        };
        Some(Triple { s, p: pred, o })
    }
}

/// Backtracking conjunctive join: one consistent binding per result row.
pub fn find_homomorphisms(
    patterns: &[TriplePattern],
    facts: &[Triple],
) -> Vec<HashMap<String, String>> {
    fn merge(
        a: &HashMap<String, String>,
        b: &HashMap<String, String>,
    ) -> Option<HashMap<String, String>> {
        let mut out = a.clone();
        for (k, v) in b {
            match out.get(k) {
                Some(e) if e != v => return None,
                Some(_) => {}
                None => {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        Some(out)
    }

    fn walk(
        pats: &[TriplePattern],
        facts: &[Triple],
        acc: HashMap<String, String>,
        out: &mut Vec<HashMap<String, String>>,
    ) {
        let Some((head, tail)) = pats.split_first() else {
            out.push(acc);
            return;
        };
        for fact in facts {
            let Some(b) = head.match_against(fact) else {
                continue;
            };
            let Some(joined) = merge(&acc, &b) else {
                continue;
            };
            walk(tail, facts, joined, out);
        }
    }

    let mut out = Vec::new();
    walk(patterns, facts, HashMap::new(), &mut out);
    out
}

/// Epistemic status of a proposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropKind {
    Observed,
    Inferred,
    Hypothesized,
    Contradicted,
}

/// A triple with provenance and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposition {
    pub claim: Triple,
    pub kind: PropKind,
    pub confidence: Score,
    pub source: String,
    pub method: String,
}

impl Proposition {
    pub fn observed(claim: Triple, source: impl Into<String>) -> Self {
        Self {
            claim,
            kind: PropKind::Observed,
            confidence: 1.0,
            source: source.into(),
            method: "observe".to_string(),
        }
    }
}

/// A hard world constraint over triple presence/absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintKind {
    Requires { triple: Triple },
    Forbids { triple: Triple },
}

/// Named constraint checked against a [`WorldState`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub desc: String,
    pub kind: ConstraintKind,
}

impl Constraint {
    pub fn check(&self, world: &WorldState) -> bool {
        let triples = world.triples();
        match &self.kind {
            ConstraintKind::Requires { triple } => triples.contains(triple),
            ConstraintKind::Forbids { triple } => !triples.contains(triple),
        }
    }
}

/// Symbolic world: scored propositions plus hard constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldState {
    pub facts: Vec<Proposition>,
    pub constraints: Vec<Constraint>,
}

impl WorldState {
    pub fn triples(&self) -> Vec<Triple> {
        self.facts.iter().map(|f| f.claim.clone()).collect()
    }

    pub fn without_relation(&self, predicate: &Predicate) -> WorldState {
        WorldState {
            facts: self
                .facts
                .iter()
                .filter(|f| &f.claim.p != predicate)
                .cloned()
                .collect(),
            constraints: self.constraints.clone(),
        }
    }

    /// Triples mentioning neither `a` nor `b`: unaffected by an a↔b swap.
    pub fn substitution_invariants(&self, a: &str, b: &str) -> Vec<Triple> {
        self.triples()
            .into_iter()
            .filter(|t| t.s != a && t.o != a && t.s != b && t.o != b)
            .collect()
    }

    /// Descriptions of currently violated constraints.
    pub fn incoherent_after(&self) -> Vec<String> {
        self.constraints
            .iter()
            .filter(|c| !c.check(self))
            .map(|c| c.desc.clone())
            .collect()
    }
}

/// Symbolic world transforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
    Generalize {
        pattern: TriplePattern,
        role: String,
    },
    Substitute {
        from: String,
        to: String,
    },
    Invert {
        predicate: Predicate,
    },
    Compose {
        first: Predicate,
        second: Predicate,
        result: Predicate,
    },
    Project {
        predicted: Triple,
    },
    Analogize {
        source: Vec<TriplePattern>,
        map: HashMap<String, String>,
    },
}

impl Transform {
    pub fn op_name(&self) -> String {
        match self {
            Transform::Generalize { .. } => "Generalize".to_string(),
            Transform::Substitute { .. } => "Substitute".to_string(),
            Transform::Invert { .. } => "Invert".to_string(),
            Transform::Compose { .. } => "Compose".to_string(),
            Transform::Project { .. } => "Project".to_string(),
            Transform::Analogize { .. } => "Analogize".to_string(),
        }
    }

    /// (op, input description, output description, gate summary).
    pub fn spec(&self) -> (String, String, String, String) {
        match self {
            Transform::Generalize { pattern, role } => (
                "Generalize".to_string(),
                format!("pattern={pattern:?}"),
                format!("abstract role={role} to Any"),
                "logical+empirical must hold".to_string(),
            ),
            Transform::Substitute { from, to } => (
                "Substitute".to_string(),
                format!("from={from}"),
                format!("to={to}"),
                "logical+empirical must hold".to_string(),
            ),
            Transform::Invert { predicate } => (
                "Invert".to_string(),
                format!("predicate={}", predicate.as_str()),
                format!("inverse={}", predicate.invert().as_str()),
                "logical+empirical must hold".to_string(),
            ),
            Transform::Compose {
                first,
                second,
                result,
            } => (
                "Compose".to_string(),
                format!("{};{}", first.as_str(), second.as_str()),
                format!("yields {}", result.as_str()),
                "logical+empirical must hold".to_string(),
            ),
            Transform::Project { predicted } => (
                "Project".to_string(),
                "hypothesis".to_string(),
                format!("{} {:?} {}", predicted.s, predicted.p, predicted.o),
                "logical+empirical must hold".to_string(),
            ),
            Transform::Analogize { source, map } => (
                "Analogize".to_string(),
                format!("{} patterns", source.len()),
                format!("{} mappings", map.len()),
                "logical+empirical must hold".to_string(),
            ),
        }
    }
}

/// One gated transform application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub op: String,
    pub before: Vec<Triple>,
    pub after: Vec<Triple>,
    pub gates: CoherenceProfile,
    pub accepted: bool,
    pub note: String,
}

pub const ACCEPT_FLOOR: f32 = 0.5;

fn contains_claim(facts: &[Proposition], t: &Triple) -> bool {
    facts.iter().any(|f| &f.claim == t)
}

fn push_new(
    candidates: &mut Vec<Proposition>,
    claim: Triple,
    kind: PropKind,
    confidence: Score,
    method: &str,
) {
    if !contains_claim(candidates, &claim) {
        candidates.push(Proposition {
            claim,
            kind,
            confidence,
            source: "transform".to_string(),
            method: method.to_string(),
        });
    }
}

/// Apply `op` to `world`, preserving all existing facts (Observed included)
/// and gating the candidate on 0/1 coherence dimensions.
pub fn apply(world: &WorldState, op: &Transform) -> (WorldState, TraceStep) {
    let before = world.triples();
    let method = op.op_name();
    let mut candidates = world.facts.clone();
    let note: String;
    let mut op_valid = true;

    match op {
        Transform::Generalize { pattern, role } => {
            let homos = find_homomorphisms(std::slice::from_ref(pattern), &before);
            let bound: Vec<&HashMap<String, String>> =
                homos.iter().filter(|h| h.contains_key(role)).collect();
            if bound.is_empty() {
                op_valid = false;
                note = format!("role {role} never bound");
            } else {
                for h in homos {
                    let Some(concrete) = pattern.instantiate(&h) else {
                        continue;
                    };
                    let Some(role_val) = h.get(role) else {
                        continue;
                    };
                    let s = if concrete.s == *role_val {
                        "Any".to_string()
                    } else {
                        concrete.s.clone()
                    };
                    let o = if concrete.o == *role_val {
                        "Any".to_string()
                    } else {
                        concrete.o.clone()
                    };
                    push_new(
                        &mut candidates,
                        Triple {
                            s,
                            p: concrete.p.clone(),
                            o,
                        },
                        PropKind::Hypothesized,
                        0.6,
                        &method,
                    );
                }
                note = format!("generalized role {role}");
            }
        }
        Transform::Substitute { from, to } => {
            for t in &before {
                if t.s == *from || t.o == *from {
                    let s = if t.s == *from {
                        to.clone()
                    } else {
                        t.s.clone()
                    };
                    let o = if t.o == *from {
                        to.clone()
                    } else {
                        t.o.clone()
                    };
                    push_new(
                        &mut candidates,
                        Triple {
                            s,
                            p: t.p.clone(),
                            o,
                        },
                        PropKind::Inferred,
                        0.8,
                        &method,
                    );
                }
            }
            note = format!("substituted {from} -> {to}");
        }
        Transform::Invert { predicate } => {
            for t in &before {
                if t.p == *predicate {
                    push_new(
                        &mut candidates,
                        Triple {
                            s: t.o.clone(),
                            p: predicate.invert(),
                            o: t.s.clone(),
                        },
                        PropKind::Inferred,
                        0.8,
                        &method,
                    );
                }
            }
            note = format!("inverted {}", predicate.as_str());
        }
        Transform::Compose {
            first,
            second,
            result,
        } => {
            for a in &before {
                for b in &before {
                    if a.p == *first && b.p == *second && a.o == b.s {
                        push_new(
                            &mut candidates,
                            Triple {
                                s: a.s.clone(),
                                p: result.clone(),
                                o: b.o.clone(),
                            },
                            PropKind::Inferred,
                            0.8,
                            &method,
                        );
                    }
                }
            }
            note = format!("composed {}+{}", first.as_str(), second.as_str());
        }
        Transform::Project { predicted } => {
            push_new(
                &mut candidates,
                predicted.clone(),
                PropKind::Hypothesized,
                0.5,
                &method,
            );
            note = "projected hypothesis".to_string();
        }
        Transform::Analogize { source, map } => {
            let homos = find_homomorphisms(source, &before);
            if homos.is_empty() {
                op_valid = false;
                note = "no source homomorphism".to_string();
            } else {
                let unbound: Vec<&String> = map
                    .keys()
                    .filter(|k| !homos.iter().any(|h| h.contains_key(*k)))
                    .collect();
                if !unbound.is_empty() {
                    op_valid = false;
                    note = format!("unbound map keys: {unbound:?}");
                } else {
                    for h in &homos {
                        for pat in source {
                            let Some(concrete) = pat.instantiate(h) else {
                                continue;
                            };
                            let mut s = concrete.s.clone();
                            let mut o = concrete.o.clone();
                            for (var, target) in map {
                                if let Some(val) = h.get(var) {
                                    if s == *val {
                                        s = target.clone();
                                    }
                                    if o == *val {
                                        o = target.clone();
                                    }
                                }
                            }
                            push_new(
                                &mut candidates,
                                Triple {
                                    s,
                                    p: concrete.p.clone(),
                                    o,
                                },
                                PropKind::Hypothesized,
                                0.6,
                                &method,
                            );
                        }
                    }
                    note = "analogized source".to_string();
                }
            }
        }
    }

    let next = WorldState {
        facts: candidates,
        constraints: world.constraints.clone(),
    };
    let after = next.triples();

    let observed_kept = world
        .facts
        .iter()
        .filter(|f| f.kind == PropKind::Observed)
        .all(|f| after.contains(&f.claim));
    let empirical: Score = if observed_kept { 1.0 } else { 0.0 };
    let logical: Score = if op_valid && next.incoherent_after().is_empty() {
        1.0
    } else {
        0.0
    };
    let temporal: Score = if after
        .iter()
        .any(|t| t.p == Predicate::Precedes && t.s == t.o)
    {
        0.0
    } else {
        1.0
    };
    let causal: Score = {
        let mut bad = false;
        for a in &after {
            for b in &after {
                if a.s == b.s && a.o == b.o && a.p.is_causal() && b.p == Predicate::Contradicts {
                    bad = true;
                }
            }
        }
        if bad {
            0.0
        } else {
            1.0
        }
    };
    let procedural: Score = if op_valid { 1.0 } else { 0.0 };

    let mut gates = CoherenceProfile::new();
    gates.logical = CoherenceDimension::new("logical", logical, 1.0);
    gates.empirical = CoherenceDimension::new("empirical", empirical, 1.0);
    gates.temporal = CoherenceDimension::new("temporal", temporal, 1.0);
    gates.causal = CoherenceDimension::new("causal", causal, 1.0);
    gates.procedural = CoherenceDimension::new("procedural", procedural, 1.0);

    let accepted = gates.composite() >= ACCEPT_FLOOR && logical == 1.0 && empirical == 1.0;

    let step = TraceStep {
        op: method,
        before,
        after,
        gates,
        accepted,
        note,
    };
    (next, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange_world() -> WorldState {
        WorldState {
            facts: vec![
                Proposition::observed(Triple::new("Alice", Predicate::Exchanges, "Bob"), "t"),
                Proposition::observed(Triple::new("Bob", Predicate::Exchanges, "Alice"), "t"),
            ],
            constraints: vec![],
        }
    }

    #[test]
    fn reciprocal_resource_exchange_abstraction_transfers() {
        let world = exchange_world();
        let op = Transform::Generalize {
            pattern: TriplePattern {
                s: PatElem::Var("x".to_string()),
                p: Some(Predicate::Exchanges),
                o: PatElem::Var("y".to_string()),
            },
            role: "x".to_string(),
        };
        let (next, step) = apply(&world, &op);
        assert!(step.accepted, "generalization rejected: {}", step.note);
        assert!(next.triples().iter().any(|t| t.s == "Any"
            && t.p == Predicate::Exchanges
            && (t.o == "Bob" || t.o == "Alice")));
        for f in world.facts.iter().filter(|f| f.kind == PropKind::Observed) {
            assert!(next.triples().contains(&f.claim), "observed lost");
        }
    }

    #[test]
    fn substitution_preserves_invariants() {
        let world = WorldState {
            facts: vec![
                Proposition::observed(Triple::new("Alice", Predicate::Requires, "Water"), "t"),
                Proposition::observed(Triple::new("Mill", Predicate::Produces, "Flour"), "t"),
            ],
            constraints: vec![],
        };
        let invariants = world.substitution_invariants("Alice", "Alicia");
        assert!(invariants.contains(&Triple::new("Mill", Predicate::Produces, "Flour")));
        let (next, step) = apply(
            &world,
            &Transform::Substitute {
                from: "Alice".to_string(),
                to: "Alicia".to_string(),
            },
        );
        assert!(step.accepted);
        for inv in &invariants {
            assert!(next.triples().contains(inv), "invariant lost: {inv:?}");
        }
        assert!(next
            .triples()
            .contains(&Triple::new("Alicia", Predicate::Requires, "Water")));
    }

    #[test]
    fn absent_relation_yields_incoherence() {
        let triple = Triple::new("Alice", Predicate::Exchanges, "Bob");
        let world = WorldState {
            facts: vec![Proposition::observed(triple.clone(), "t")],
            constraints: vec![Constraint {
                desc: "exchange required".to_string(),
                kind: ConstraintKind::Requires {
                    triple: triple.clone(),
                },
            }],
        };
        assert!(world.incoherent_after().is_empty());
        let stripped = world.without_relation(&Predicate::Exchanges);
        assert!(!stripped.triples().contains(&triple));
        let bad = stripped.incoherent_after();
        assert_eq!(bad, vec!["exchange required".to_string()]);
    }

    #[test]
    fn invalid_analogy_rejected() {
        let world = exchange_world();
        let mut map = HashMap::new();
        map.insert("zzz".to_string(), "Nope".to_string());
        let op = Transform::Analogize {
            source: vec![TriplePattern {
                s: PatElem::Var("a".to_string()),
                p: Some(Predicate::Causes),
                o: PatElem::Var("b".to_string()),
            }],
            map,
        };
        let (_next, step) = apply(&world, &op);
        assert!(!step.accepted, "invalid analogy accepted");
        assert_eq!(step.gates.logical.score, 0.0);
    }
}
