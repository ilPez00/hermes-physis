//! Adaptive semiotic grid view.
//!
//! The grid is whatever the loaded ontologies say it is: every distinct
//! `domain` and `mode` string becomes an axis, so adding an ontology with new
//! axes grows the grid without a code change. The classic 5×14 human grid is
//! one possible shape, not a cap.
//!
//! This module only reports the shape; classification itself is string-based
//! in [`crate::classify::CellClassifier`] and was never limited to the enums.

use serde::Serialize;

use crate::classify::CellClassifier;

#[derive(Debug, Clone, Serialize)]
pub struct GridCellView {
    pub domain: String,
    pub mode: String,
    pub entries: Vec<String>,
}

/// Axes, shape and populated cells for the currently loaded ontology set.
#[derive(Debug, Clone, Serialize)]
pub struct GridView {
    pub domains: Vec<String>,
    pub modes: Vec<String>,
    /// `[domains, modes]`.
    pub shape: [usize; 2],
    pub total_cells: usize,
    pub populated_cells: usize,
    pub entries: usize,
    pub cells: Vec<GridCellView>,
}

impl GridView {
    pub fn from_classifier(classifier: &CellClassifier) -> Self {
        let mut domains: Vec<String> = classifier
            .cells
            .iter()
            .map(|c| c.domain.clone())
            .collect();
        let mut modes: Vec<String> = classifier.cells.iter().map(|c| c.mode.clone()).collect();
        domains.sort();
        domains.dedup();
        modes.sort();
        modes.dedup();

        let mut cells: Vec<GridCellView> = classifier
            .cells
            .iter()
            .map(|c| GridCellView {
                domain: c.domain.clone(),
                mode: c.mode.clone(),
                entries: c.entries.clone(),
            })
            .collect();
        cells.sort_by(|a, b| {
            a.domain
                .cmp(&b.domain)
                .then_with(|| a.mode.cmp(&b.mode))
        });

        let shape = [domains.len(), modes.len()];
        Self {
            domains,
            modes,
            shape,
            total_cells: shape[0] * shape[1],
            populated_cells: cells.len(),
            entries: classifier.cells.iter().map(|c| c.entries.len()).sum(),
            cells,
        }
    }

    /// True when the grid is no longer the classic 5×14 human projection.
    pub fn is_expanded(&self) -> bool {
        self.shape != [5, 14]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{CellClassifier, EntrySeed};

    fn clf(pairs: &[(&str, &str, &str)]) -> CellClassifier {
        let seeds = pairs.iter().map(|(name, domain, mode)| EntrySeed {
            name: (*name).to_string(),
            domain: (*domain).to_string(),
            mode: (*mode).to_string(),
            text: format!("{name} {domain} {mode}"),
            facets: Default::default(),
        });
        CellClassifier::build_seeds(seeds, |_| vec![1.0, 0.0])
    }

    #[test]
    fn grid_reflects_every_distinct_axis() {
        let view = GridView::from_classifier(&clf(&[
            ("heal work", "HEAL", "WORK"),
            ("defend teach", "DEFEND", "TEACH"),
            ("defend work", "DEFEND", "WORK"),
        ]));
        assert_eq!(view.domains, vec!["DEFEND", "HEAL"]);
        assert_eq!(view.modes, vec!["TEACH", "WORK"]);
        assert_eq!(view.shape, [2, 2]);
        assert_eq!(view.total_cells, 4);
        assert_eq!(view.populated_cells, 3);
        assert_eq!(view.entries, 3);
        assert!(view.is_expanded());
    }

    #[test]
    fn classic_shape_is_not_marked_expanded() {
        let mut seeds = Vec::new();
        for d in crate::models::HumanDomain::all() {
            for m in crate::models::HumanMode::all() {
                seeds.push(EntrySeed {
                    name: format!("{} {}", d.as_str(), m.as_str()),
                    domain: d.as_str().to_string(),
                    mode: m.as_str().to_string(),
                    text: "x".into(),
                    facets: Default::default(),
                });
            }
        }
        let classifier = CellClassifier::build_seeds(seeds, |_| vec![1.0, 0.0]);
        let view = GridView::from_classifier(&classifier);
        assert_eq!(view.shape, [5, 14]);
        assert!(!view.is_expanded());
    }
}
