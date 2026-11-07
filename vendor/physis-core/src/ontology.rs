//! Ontology loading — built-in JSON files (compiled in), file paths, or raw
//! strings, resolved into domain maps.
//!
//! The classifier consumes `human_domains` (the base praxis grid) plus
//! `custom_domains` (every extra domain-specific ontology), all sharing the
//! HEAL/CONSTRUCT/FABRICATE/BOND/STUDY × LIFT/…/PLAN vocabulary.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;

use crate::models::{DomainDef, OntologyConfig, OntologyEntry};

pub const HUMAN_ONTOLOGY_NAME: &str = "praxis_ontology";

/// Every ontology JSON this repository ships, as `(canonical name, json)`.
///
/// This is a **file registry, not a load list**. Nothing here is wired into
/// [`OntologyLoader::load_all`]; `BUILTIN_ONTOLOGIES` and `EXTRA_ONTOLOGIES`
/// below still decide what Core actually classifies with, and adding a row
/// here changes no Core behaviour.
///
/// It exists so the ontology corpus has exactly one home. `physis-pro` used to
/// carry a byte-identical second copy of 33 of these files and `include_str!`
/// them from its own `config/`; three had already drifted. Pro now builds its
/// registry from this one. Pro cannot reach these files directly —
/// `include_str!("../physis-core/config/...")` would compile here but break the
/// published crate, where `physis-core` resolves from the registry and the
/// submodule directory does not exist — so the corpus has to cross the
/// boundary as data, which is what this is.
///
/// Names are the canonical `<x>_ontology` form both crates resolve by.
pub static ONTOLOGY_SOURCES: &[(&str, &str)] = &[
    (
        "praxis_ontology",
        include_str!("../config/praxis_ontology.json"),
    ),
    (
        "machine_ontology",
        include_str!("../config/machine_ontology.json"),
    ),
    (
        "semiotic_ontology",
        include_str!("../config/semiotic_ontology.json"),
    ),
    (
        "category_ontology",
        include_str!("../config/category_ontology.json"),
    ),
    (
        "agent_ontology",
        include_str!("../config/agent_ontology.json"),
    ),
    (
        "natural_ontology",
        include_str!("../config/natural_ontology.json"),
    ),
    (
        "social_ontology",
        include_str!("../config/social_ontology.json"),
    ),
    (
        "abstract_ontology",
        include_str!("../config/abstract_ontology.json"),
    ),
    (
        "engineering_ontology",
        include_str!("../config/engineering_ontology.json"),
    ),
    (
        "office_ontology",
        include_str!("../config/office_ontology.json"),
    ),
    (
        "space_ontology",
        include_str!("../config/space_ontology.json"),
    ),
    (
        "time_ontology",
        include_str!("../config/time_ontology.json"),
    ),
    (
        "software_dev",
        include_str!("../config/software_dev_ontology.json"),
    ),
    ("devops", include_str!("../config/devops_ontology.json")),
    (
        "data_science",
        include_str!("../config/data_science_ontology.json"),
    ),
    (
        "cybersecurity",
        include_str!("../config/cybersecurity_ontology.json"),
    ),
    (
        "health_fitness",
        include_str!("../config/health_fitness_ontology.json"),
    ),
    (
        "nutrition_cooking",
        include_str!("../config/nutrition_cooking_ontology.json"),
    ),
    (
        "personal_finance",
        include_str!("../config/personal_finance_ontology.json"),
    ),
    (
        "learning_education",
        include_str!("../config/learning_education_ontology.json"),
    ),
    (
        "creative_writing",
        include_str!("../config/creative_writing_ontology.json"),
    ),
    ("music", include_str!("../config/music_ontology.json")),
    (
        "visual_arts",
        include_str!("../config/visual_arts_ontology.json"),
    ),
    (
        "relationships",
        include_str!("../config/relationships_ontology.json"),
    ),
    (
        "productivity",
        include_str!("../config/productivity_ontology.json"),
    ),
    ("travel", include_str!("../config/travel_ontology.json")),
    (
        "home_maintenance",
        include_str!("../config/home_maintenance_ontology.json"),
    ),
    (
        "mindfulness_spirituality",
        include_str!("../config/mindfulness_spirituality_ontology.json"),
    ),
    (
        "science_research",
        include_str!("../config/science_research_ontology.json"),
    ),
    (
        "business_entrepreneurship",
        include_str!("../config/business_entrepreneurship_ontology.json"),
    ),
    (
        "language_learning",
        include_str!("../config/language_learning_ontology.json"),
    ),
    (
        "outdoors_sports",
        include_str!("../config/outdoors_sports_ontology.json"),
    ),
    (
        "grid_fill",
        include_str!("../config/grid_fill_ontology.json"),
    ),
    (
        "mode_anchors",
        include_str!("../config/mode_anchors_ontology.json"),
    ),
    (
        "italian_industrial",
        include_str!("../config/italian_industrial_ontology.json"),
    ),
    (
        "industrial_semiconductor",
        include_str!("../config/industrial_semiconductor_ontology.json"),
    ),
];

/// Look up one ontology's JSON by its canonical name.
pub fn ontology_source(name: &str) -> Option<&'static str> {
    ONTOLOGY_SOURCES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, json)| *json)
}

/// The named built-in ontologies. To add one: drop a JSON file in `config/`,
/// register its name constant, add a row here and a `kind` arm in `load_all`.
pub static BUILTIN_ONTOLOGIES: &[(&str, &str, &str)] = &[
    (
        "praxis",
        "praxis",
        include_str!("../config/praxis_ontology.json"),
    ),
    (
        "machine",
        "machine",
        include_str!("../config/machine_ontology.json"),
    ),
    (
        "semiotic",
        "semiotic",
        include_str!("../config/semiotic_ontology.json"),
    ),
    (
        "office",
        "office",
        include_str!("../config/office_ontology.json"),
    ),
    (
        "agent",
        "agent",
        include_str!("../config/agent_ontology.json"),
    ),
    (
        "engineering",
        "engineering",
        include_str!("../config/engineering_ontology.json"),
    ),
];

/// Extra domain-specific ontologies, always loaded into `custom_domains`.
/// To add one: drop a JSON file in `config/` using the grid vocabulary.
pub static EXTRA_ONTOLOGIES: &[(&str, &str)] = &[
    (
        "software_dev",
        include_str!("../config/software_dev_ontology.json"),
    ),
    ("devops", include_str!("../config/devops_ontology.json")),
    (
        "data_science",
        include_str!("../config/data_science_ontology.json"),
    ),
    (
        "cybersecurity",
        include_str!("../config/cybersecurity_ontology.json"),
    ),
    (
        "health_fitness",
        include_str!("../config/health_fitness_ontology.json"),
    ),
    (
        "nutrition_cooking",
        include_str!("../config/nutrition_cooking_ontology.json"),
    ),
    (
        "personal_finance",
        include_str!("../config/personal_finance_ontology.json"),
    ),
    (
        "learning_education",
        include_str!("../config/learning_education_ontology.json"),
    ),
    (
        "creative_writing",
        include_str!("../config/creative_writing_ontology.json"),
    ),
    ("music", include_str!("../config/music_ontology.json")),
    (
        "visual_arts",
        include_str!("../config/visual_arts_ontology.json"),
    ),
    (
        "relationships",
        include_str!("../config/relationships_ontology.json"),
    ),
    (
        "productivity",
        include_str!("../config/productivity_ontology.json"),
    ),
    ("travel", include_str!("../config/travel_ontology.json")),
    (
        "home_maintenance",
        include_str!("../config/home_maintenance_ontology.json"),
    ),
    (
        "science_research",
        include_str!("../config/science_research_ontology.json"),
    ),
    (
        "industrial_semiconductor",
        include_str!("../config/industrial_semiconductor_ontology.json"),
    ),
    (
        "business_entrepreneurship",
        include_str!("../config/business_entrepreneurship_ontology.json"),
    ),
    (
        "language_learning",
        include_str!("../config/language_learning_ontology.json"),
    ),
    (
        "outdoors_sports",
        include_str!("../config/outdoors_sports_ontology.json"),
    ),
    // Targeted fill for empty/thin DOMAIN×MODE cells so every grid cell is a
    // reachable classification target.
    (
        "grid_fill",
        include_str!("../config/grid_fill_ontology.json"),
    ),
    // One MODE-pure anchor per cell — sharpens the mode axis.
    (
        "mode_anchors",
        include_str!("../config/mode_anchors_ontology.json"),
    ),
    // Italian-language anchors for Italian queries on shop floors.
    (
        "italian",
        include_str!("../config/italian_industrial_ontology.json"),
    ),
];

/// Loads and resolves ontologies into domain maps.
#[derive(Debug, Clone, Default)]
pub struct OntologyLoader {
    pub human_domains: HashMap<String, DomainDef>,
    pub machine_domains: HashMap<String, DomainDef>,
    pub custom_domains: HashMap<String, DomainDef>,
}

impl OntologyLoader {
    pub fn new() -> Self {
        Self::default()
    }

    fn entries_to_map(entries: &[OntologyEntry]) -> HashMap<String, DomainDef> {
        let mut map = HashMap::new();
        for e in entries {
            map.insert(
                e.name.clone(),
                DomainDef {
                    name: e.name.clone(),
                    category: e.category.clone(),
                    domain: Some(e.domain.clone()),
                    mode: Some(e.mode.clone()),
                    axis_kind: Some(e.axis_kind.clone()),
                    axis_name: Some(e.axis_name.clone()),
                    unit: e.unit.clone(),
                    hints: e.hints.clone(),
                    facets: e.facets.clone(),
                },
            );
        }
        map
    }

    /// Parse domain definitions from a raw JSON string.
    pub fn load_from_str(json: &str) -> anyhow::Result<HashMap<String, DomainDef>> {
        let config: OntologyConfig = serde_json::from_str(json)?;
        Ok(Self::entries_to_map(&config.domains))
    }

    /// Load domain definitions from a JSON file on disk.
    pub fn load_from_path(path: &Path) -> anyhow::Result<HashMap<String, DomainDef>> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("reading ontology {}", path.display()))?;
        Self::load_from_str(&contents)
    }

    /// Load all built-in ontologies into human/machine/custom maps.
    pub fn load_all() -> Self {
        let mut loader = Self::new();

        for (kind, _name, json) in BUILTIN_ONTOLOGIES {
            let Ok(map) = Self::load_from_str(json) else {
                eprintln!("Warning: failed to parse built-in ontology '{kind}'");
                continue;
            };
            match *kind {
                "human" | "praxis" => {
                    for (k, v) in map {
                        loader.human_domains.insert(k, v);
                    }
                }
                "machine" => {
                    for (k, v) in map {
                        loader.machine_domains.insert(k, v);
                    }
                }
                _ => {
                    for (k, v) in map {
                        loader.custom_domains.insert(k, v);
                    }
                }
            }
        }

        // Extra domain-specific ontologies → custom_domains, keyed "kind:name"
        // so identical entry names across files don't collide.
        for (kind, json) in EXTRA_ONTOLOGIES {
            match Self::load_from_str(json) {
                Ok(map) => {
                    for (name, def) in map {
                        loader.custom_domains.insert(format!("{kind}:{name}"), def);
                    }
                }
                Err(e) => eprintln!("Warning: failed to parse built-in ontology '{kind}': {e}"),
            }
        }

        loader
    }

    /// Merge a custom ontology (e.g. hand-edited in the studio) into custom_domains.
    pub fn merge_custom(&mut self, kind: &str, map: HashMap<String, DomainDef>) {
        for (name, def) in map {
            self.custom_domains.insert(format!("{kind}:{name}"), def);
        }
    }

    /// All domain definitions used for classification: the human grid, the
    /// machine ontology, and every extra domain-specific ontology. All share
    /// the grid vocabulary.
    ///
    /// Phase 10c parity fix — this used to chain only `human + custom`, which
    /// left the whole `machine_domains` bucket authored and displayed but
    /// never scoreable (the studio's "730 advertised vs 623 scoreable" gap).
    /// Iteration order is **deterministic** (sorted by name, then domain, then
    /// mode). It previously chained `HashMap::values()` directly, which meant
    /// callers saw a different order on every process run — Rust seeds its
    /// hasher randomly. Aggregates over the whole set were unaffected, but
    /// anything that slices, samples, or shows "the first N" silently changed
    /// run to run; that cost real debugging time in the perspective-discovery
    /// track, where a train/test split keyed on entry index was quietly being
    /// recomposed on each run and the resulting accuracy swing was
    /// misattributed to float nondeterminism.
    /// The ordering key for [`OntologyLoader::classification_domains`].
    /// Extracted so the sort and the test proving it total cannot drift apart.
    ///
    /// It must be a TOTAL order over the real data. Sorting on
    /// (name, domain, mode) alone is not: six built-in entries collide on that
    /// triple while differing in `hints` (e.g. "Trip Planning", "Incident
    /// Response"), so for those the tie fell through to the `HashMap`
    /// iteration order this sort exists to defeat — they still swapped between
    /// runs after 0.1.15 nominally fixed the ordering. Every field that feeds
    /// a classification text is therefore part of the key.
    #[allow(clippy::type_complexity)]
    fn order_key(
        d: &DomainDef,
    ) -> (
        &String,
        &Option<String>,
        &Option<String>,
        &Option<String>,
        &Option<String>,
        &Option<String>,
        &String,
        &Vec<String>,
    ) {
        (
            &d.name,
            &d.domain,
            &d.mode,
            &d.category,
            &d.axis_kind,
            &d.axis_name,
            &d.unit,
            &d.hints,
        )
    }

    pub fn classification_domains(&self) -> impl Iterator<Item = &DomainDef> {
        let mut all: Vec<&DomainDef> = self
            .human_domains
            .values()
            .chain(self.machine_domains.values())
            .chain(self.custom_domains.values())
            .collect();
        all.sort_by(|a, b| Self::order_key(a).cmp(&Self::order_key(b)));
        all.into_iter()
    }

    /// Every domain definition across all loaded ontologies, merged into one map.
    /// On key collision the later map wins (custom overrides builtin).
    pub fn all_domains(&self) -> HashMap<String, DomainDef> {
        let mut out = HashMap::new();
        for m in [
            &self.human_domains,
            &self.machine_domains,
            &self.custom_domains,
        ] {
            for (k, v) in m {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }

    /// Distinct categories across all domains (for the studio browser).
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .all_domains()
            .values()
            .filter_map(|d| d.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Total number of classification-reachable domains (human + machine +
    /// custom) — must equal what `classification_domains()` yields.
    pub fn entry_count(&self) -> usize {
        self.human_domains.len() + self.machine_domains.len() + self.custom_domains.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_human_ontology_loads() {
        let loader = OntologyLoader::load_all();
        assert!(
            loader.human_domains.len() >= 14,
            "expected >=14 human domains, got {}",
            loader.human_domains.len()
        );
        assert!(
            loader.entry_count() > 100,
            "expected a rich grid, got {}",
            loader.entry_count()
        );
        assert!(loader.custom_domains.len() > 50, "expected extra domains");
    }

    /// Any surface reporting an "entries" total is telling the user how much
    /// vocabulary the classifier has. Phase 10c folded `machine_domains` into
    /// classification, so entry_count and all_domains now describe the same
    /// set — the old "advertised 730, scoreable 623" gap is closed, and this
    /// test keeps them from drifting apart again.
    #[test]
    fn entry_count_matches_what_the_classifier_is_built_from() {
        let loader = OntologyLoader::load_all();
        assert_eq!(
            loader.entry_count(),
            loader.classification_domains().count(),
            "entry_count must count exactly the entries the classifier receives",
        );
        assert_eq!(
            loader.all_domains().len(),
            loader.entry_count(),
            "all_domains and the classified set must stay the same size now that \
             machine domains are reachable",
        );
        // The machine ontology's flagship entries must actually be scoreable.
        assert!(
            loader
                .classification_domains()
                .any(|d| d.name.contains("OEE")),
            "machine entries (OEE) must be reachable by the classifier",
        );
    }

    #[test]
    fn load_from_str_parses_entry() {
        let json = r#"{"kind":"test","domains":[{"name":"logic","category":"reasoning","domain":"STUDY","mode":"WORK","axis_kind":"epistemic","axis_name":"formal","unit":"tokens","hints":["therefore","implies"]}]}"#;
        let map = OntologyLoader::load_from_str(json).unwrap();
        assert_eq!(map.len(), 1);
        let d = &map["logic"];
        assert_eq!(d.domain.as_deref(), Some("STUDY"));
        assert_eq!(d.hints, vec!["therefore", "implies"]);
    }

    #[test]
    fn categories_are_distinct() {
        let loader = OntologyLoader::load_all();
        let cats = loader.categories();
        let mut sorted = cats.clone();
        sorted.dedup();
        assert_eq!(cats, sorted);
        assert!(cats.iter().any(|c| c.to_lowercase().contains("reason")));
    }

    #[test]
    fn faceted_example_loads_with_facets() {
        let json = include_str!("../config/ontology_facets.example.json");
        let map = OntologyLoader::load_from_str(json).unwrap();
        let pump = &map["Pump Maintenance"];
        assert_eq!(
            pump.facets.lifecycle,
            Some(crate::models::LifecyclePhase::Operate)
        );
        assert_eq!(pump.facets.agency, Some(crate::models::Agency::Automated));
        assert_eq!(pump.facets.sub_domain.as_deref(), Some("Repair"));
        // A facet-less entry in the example still parses (back-compat default).
        assert!(map.contains_key("Contract Negotiation"));
    }

    /// The implementation's ordering key must be TOTAL over the real built-in
    /// data, not merely sorted.
    ///
    /// A same-process test cannot catch `HashMap` iteration variance — one
    /// process, one hasher seed — so "two calls agree" would have passed while
    /// the bug was live. This asserts the invariant that actually prevents it:
    /// entries that tie under `order_key` must be genuinely interchangeable.
    /// It calls `order_key` rather than restating it, so shrinking the key
    /// fails this test instead of silently passing.
    ///
    /// Regression: 0.1.15 keyed on (name, domain, mode) only. Six built-ins
    /// collide on that triple while differing in `hints`, so their order still
    /// came from `HashMap` iteration and still changed between runs. Verified
    /// by shrinking the key back and watching this fail.
    #[test]
    fn classification_domain_order_is_a_total_order_over_builtins() {
        let loader = OntologyLoader::load_all();
        let all: Vec<&DomainDef> = loader.classification_domains().collect();
        assert!(all.len() > 100, "expected the built-in ontologies to load");

        for w in all.windows(2) {
            if OntologyLoader::order_key(w[0]) == OntologyLoader::order_key(w[1]) {
                // Tied entries must be indistinguishable in everything that
                // feeds a classification text — otherwise which one comes first
                // is decided by HashMap iteration order.
                let (a, b) = (w[0], w[1]);
                assert!(
                    a.name == b.name
                        && a.domain == b.domain
                        && a.mode == b.mode
                        && a.category == b.category
                        && a.axis_kind == b.axis_kind
                        && a.axis_name == b.axis_name
                        && a.unit == b.unit
                        && a.hints == b.hints,
                    "entries named {:?} tie on the whole ordering key yet differ in \
                     content, so their relative order comes from HashMap iteration",
                    a.name
                );
            }
        }

        let keys: Vec<_> = all.iter().map(|d| OntologyLoader::order_key(d)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(
            keys, sorted,
            "classification_domains must yield sorted order"
        );
    }
}
