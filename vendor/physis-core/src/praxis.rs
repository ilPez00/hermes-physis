//! Praxis backfill — seed coherence from the real life-log.
//!
//! Takes a Praxis export (notebook entries + habit trackers) and reduces it to
//! behavioural records, each carrying a verdict (Success / Inert / Failure).
//! Callers register them as labeled core nodes with the embedder and
//! assert the verdict, so life-log activity feeds the coherence graph.

use serde::{Deserialize, Serialize};

/// Verdict derived from a behavioural record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviourStatus {
    Success,
    Inert,
    Failure,
}

impl BehaviourStatus {
    /// The asserted coherence verdict: +1 (kept), 0 (neutral), −1 (blocked).
    pub fn as_score(self) -> f32 {
        match self {
            BehaviourStatus::Success => 1.0,
            BehaviourStatus::Inert => 0.0,
            BehaviourStatus::Failure => -1.0,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            BehaviourStatus::Success => "success",
            BehaviourStatus::Inert => "inert",
            BehaviourStatus::Failure => "failure",
        }
    }
}

/// One behavioural unit from the life-log: a notebook entry or a tracker's
/// target state. `body` is the embeddable text; `title` is the browsable label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviourRecord {
    pub kind: String,
    pub title: String,
    pub body: String,
    pub status: BehaviourStatus,
}

impl BehaviourRecord {
    pub fn labels(&self) -> Vec<String> {
        vec![format!("praxis:{}:{}", self.kind, self.title)]
    }
}

// ── Notebook entries ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct NotebookEntry {
    #[serde(alias = "content", alias = "text", alias = "notes")]
    notes: String,
    #[serde(default)]
    mood: Option<i64>,
    #[serde(default)]
    tags: Vec<String>,
}

fn status_from_mood(mood: Option<i64>) -> BehaviourStatus {
    match mood {
        Some(m) if m >= 7 => BehaviourStatus::Success,
        Some(m) if m <= 3 => BehaviourStatus::Failure,
        _ => BehaviourStatus::Inert,
    }
}

/// Parse a notebook-entries export. Accepts a bare array of
/// `{content|text, mood?, tags?}` or `{ entries: [...] }`.
pub fn parse_notebook_entries(json: &str) -> Vec<BehaviourRecord> {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let items: Vec<&serde_json::Value> = match &v {
        serde_json::Value::Array(a) => a.iter().collect(),
        serde_json::Value::Object(o) => match o.get("entries").and_then(|x| x.as_array()) {
            Some(a) => a.iter().collect(),
            None => return vec![],
        },
        _ => return vec![],
    };
    let mut out = Vec::new();
    for it in items {
        let Ok(entry) = serde_json::from_value::<NotebookEntry>(it.clone()) else {
            continue;
        };
        let notes = entry.notes.trim().to_string();
        if notes.is_empty() {
            continue;
        }
        let title = if notes.len() > 80 {
            notes[..80].to_string()
        } else {
            notes.clone()
        };
        let out_body = if entry.tags.is_empty() {
            notes
        } else {
            format!("{}\n\ntags: {}", notes, entry.tags.join(", "))
        };
        out.push(BehaviourRecord {
            kind: "notebook".to_string(),
            title,
            body: out_body,
            status: status_from_mood(entry.mood),
        });
    }
    out
}

// ── Trackers ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TrackerEntry {
    name: String,
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    target: Option<f64>,
    #[serde(default)]
    progress: Option<f64>,
    #[serde(default)]
    values: Vec<DayValue>,
}

#[derive(Deserialize)]
struct DayValue {
    #[serde(default)]
    value: Option<f64>,
}

fn status_from_tracker(
    value: Option<f64>,
    target: Option<f64>,
    progress: Option<f64>,
) -> BehaviourStatus {
    // No value recorded AND no explicit progress → we cannot judge attainment;
    // that is inert (neutral), not a failure. Treating an unrecorded day as a
    // failed one would let gaps in the log masquerade as evidence.
    if value.is_none() && target.is_none() && progress.is_none() {
        return BehaviourStatus::Inert;
    }
    // Explicit progress (0..1) wins; otherwise the value/target ratio.
    let ratio = progress
        .or_else(|| {
            value
                .zip(target)
                .map(|(v, t)| if t > 0.0 { v / t } else { v })
        })
        .unwrap_or(0.0);
    if ratio >= 0.7 {
        BehaviourStatus::Success
    } else if ratio < 0.3 {
        BehaviourStatus::Failure
    } else {
        BehaviourStatus::Inert
    }
}

/// Parse a trackers export.
pub fn parse_trackers(json: &str) -> Vec<BehaviourRecord> {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let items: Vec<&serde_json::Value> = match &v {
        serde_json::Value::Array(a) => a.iter().collect(),
        serde_json::Value::Object(o) => match o.get("trackers").and_then(|x| x.as_array()) {
            Some(a) => a.iter().collect(),
            None => return vec![],
        },
        _ => return vec![],
    };
    let mut out = Vec::new();
    for it in items {
        let Ok(t) = serde_json::from_value::<TrackerEntry>(it.clone()) else {
            continue;
        };
        if t.name.trim().is_empty() {
            continue;
        }
        let latest = t.values.iter().rev().find_map(|d| d.value);
        let value = t.value.or(latest);
        let body = format!(
            "tracker {}: {}",
            t.name,
            value
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "no value recorded".to_string()),
        );
        out.push(BehaviourRecord {
            kind: "tracker".to_string(),
            title: t.name,
            body,
            status: status_from_tracker(value, t.target, t.progress),
        });
    }
    out
}

// ── Combined export ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PraxisExport {
    #[serde(default)]
    notebook: Vec<NotebookEntry>,
    #[serde(default)]
    trackers: Vec<TrackerEntry>,
}

/// Parse a combined export `{ notebook: [...], trackers: [...] }` into records.
pub fn parse_export(json: &str) -> Vec<BehaviourRecord> {
    let Ok(export) = serde_json::from_str::<PraxisExport>(json) else {
        return vec![];
    };
    let mut out = Vec::new();
    for e in export.notebook {
        let notes = e.notes.trim().to_string();
        if notes.is_empty() {
            continue;
        }
        let title = if notes.len() > 80 {
            notes[..80].to_string()
        } else {
            notes.clone()
        };
        let body = if e.tags.is_empty() {
            notes
        } else {
            format!("{}\n\ntags: {}", notes, e.tags.join(", "))
        };
        out.push(BehaviourRecord {
            kind: "notebook".to_string(),
            title,
            body,
            status: status_from_mood(e.mood),
        });
    }
    for t in export.trackers {
        if t.name.trim().is_empty() {
            continue;
        }
        let latest = t.values.iter().rev().find_map(|d| d.value);
        let value = t.value.or(latest);
        let body = format!(
            "tracker {}: {}",
            t.name,
            value
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "no value recorded".to_string()),
        );
        out.push(BehaviourRecord {
            kind: "tracker".to_string(),
            title: t.name,
            body,
            status: status_from_tracker(value, t.target, t.progress),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notebook_mood_maps_to_status() {
        assert_eq!(status_from_mood(Some(9)), BehaviourStatus::Success);
        assert_eq!(status_from_mood(Some(5)), BehaviourStatus::Inert);
        assert_eq!(status_from_mood(Some(2)), BehaviourStatus::Failure);
        assert_eq!(status_from_mood(None), BehaviourStatus::Inert);
    }

    #[test]
    fn parse_notebook_entries_array_and_object() {
        let arr = r#"[{"content":"fixed the nozzle","mood":9,"tags":["maintenance"]}]"#;
        let docs = parse_notebook_entries(arr);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].kind, "notebook");
        assert_eq!(docs[0].status, BehaviourStatus::Success);
        assert!(docs[0].body.contains("tags: maintenance"));

        let obj = r#"{"entries":[{"text":"slow day","mood":2}]}"#;
        let docs = parse_notebook_entries(obj);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].status, BehaviourStatus::Failure);

        assert!(parse_notebook_entries("nope").is_empty());
    }

    #[test]
    fn tracker_ratio_maps_to_status() {
        assert_eq!(
            status_from_tracker(Some(9.0), Some(10.0), None),
            BehaviourStatus::Success
        );
        assert_eq!(
            status_from_tracker(Some(5.0), Some(10.0), None),
            BehaviourStatus::Inert
        );
        assert_eq!(
            status_from_tracker(Some(1.0), Some(10.0), None),
            BehaviourStatus::Failure
        );
        assert_eq!(
            status_from_tracker(None, None, Some(0.9)),
            BehaviourStatus::Success
        );
        assert_eq!(
            status_from_tracker(None, None, None),
            BehaviourStatus::Inert
        );
    }

    #[test]
    fn parse_trackers_uses_latest_day_value() {
        let arr = r#"[{"name":"reading","values":[{"value":2},{"value":5}],"target":10}]"#;
        let docs = parse_trackers(arr);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].status, BehaviourStatus::Inert, "5/10 = 0.5");
        assert!(docs[0].body.contains("reading: 5.0"));
    }

    #[test]
    fn parse_export_combines_notebook_and_trackers() {
        let combined = r#"{"notebook":[{"content":"great session","mood":8}],"trackers":[{"name":"meditate","progress":0.2}]}"#;
        let docs = parse_export(combined);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].kind, "notebook");
        assert_eq!(docs[0].status, BehaviourStatus::Success);
        assert_eq!(docs[1].kind, "tracker");
        assert_eq!(docs[1].status, BehaviourStatus::Failure);
    }

    #[test]
    fn score_and_label_are_deterministic() {
        assert_eq!(BehaviourStatus::Success.as_score(), 1.0);
        assert_eq!(BehaviourStatus::Inert.as_score(), 0.0);
        assert_eq!(BehaviourStatus::Failure.as_score(), -1.0);
        assert_eq!(BehaviourStatus::Success.as_str(), "success");
        let r = BehaviourRecord {
            kind: "notebook".into(),
            title: "t".into(),
            body: "b".into(),
            status: BehaviourStatus::Inert,
        };
        assert_eq!(r.labels(), vec!["praxis:notebook:t"]);
    }
}
