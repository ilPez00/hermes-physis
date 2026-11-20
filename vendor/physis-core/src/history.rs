//! Personal-history importers — browser bookmarks/history, RSS /
//! read-later lists, and chat/message exports reduced to [`VaultDoc`]s.
//!
//! Pure parsers only: each takes a text export and returns docs. Callers
//! register them as labeled nodes with the shared embedder — the same
//! "source → embed → register labeled node" path the vault importer uses —
//! which is what keeps a personal log recallable by content rather than by
//! filename.

use crate::vault::VaultDoc;
use std::path::Path;

/// Pick the importer for a file by extension.
pub fn importer_for(path: &Path) -> Option<fn(&str) -> Vec<VaultDoc>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "html" | "htm" => Some(parse_bookmarks_html),
        "json" => Some(parse_history_json),
        "opml" | "xml" => Some(parse_opml),
        "txt" | "jsonl" | "ndjson" => Some(parse_messages_jsonl),
        _ => None,
    }
}

/// Netscape bookmark file: `<!DOCTYPE NETSCAPE-Bookmark-file-1>` — the export
/// format shared by Firefox and Chromium.
///
/// Each `<A HREF="…">Title</A>` becomes a doc (title = link text, body = URL).
/// Folder lines give context, but the links are the retrievable units.
pub fn parse_bookmarks_html(html: &str) -> Vec<VaultDoc> {
    let mut docs = Vec::new();
    let mut depth = 0usize;
    let mut stack: Vec<String> = Vec::new();
    for line in html.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<DT><H3")
            || (trimmed.starts_with("<DT>") && trimmed.contains("<H3"))
        {
            if let Some(open) = trimmed.find('>') {
                if let Some(close) = trimmed[open..].find("</H3>") {
                    let name = trimmed[open + 1..open + close].trim();
                    stack.push(name.to_string());
                    depth += 1;
                }
            }
            continue;
        }
        if trimmed.starts_with("</DL>") || trimmed.starts_with("</DL><p>") {
            depth = depth.saturating_sub(1);
            stack.pop();
            continue;
        }
        if let Some(a_start) = trimmed.find("<A HREF=\"") {
            let rest = &trimmed[a_start + "<A HREF=\"".len()..];
            let url = rest.split('"').next().unwrap_or_default().to_string();
            if url.is_empty() {
                continue;
            }
            let after_url = rest.split_once('>').map(|x| x.1).unwrap_or_default();
            let title = after_url
                .split("</A>")
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            let title = if title.is_empty() { url.clone() } else { title };
            let mut body = url;
            if let Some(folder) = stack.last() {
                body.push_str(&format!("\n\nbookmarked in: {folder}"));
            }
            docs.push(VaultDoc {
                source: format!("bookmark {title}"),
                title,
                body,
                headings: vec![],
            });
        }
    }
    docs
}

/// Browser history as a JSON array or object with an `items` array.
pub fn parse_history_json(json: &str) -> Vec<VaultDoc> {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let items = match &v {
        serde_json::Value::Array(a) => a.as_slice(),
        serde_json::Value::Object(o) => match o.get("items").and_then(|x| x.as_array()) {
            Some(a) => a.as_slice(),
            None => return vec![],
        },
        _ => return vec![],
    };
    let mut docs = Vec::new();
    for it in items {
        let url = it
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        if url.is_empty() {
            continue;
        }
        let title = it
            .get("title")
            .and_then(|x| x.as_str())
            .filter(|t| !t.is_empty())
            .unwrap_or(&url)
            .to_string();
        docs.push(VaultDoc {
            source: format!("history {url}"),
            title: title.clone(),
            body: format!("{title}\n\n{url}"),
            headings: vec![],
        });
    }
    docs
}

/// OPML feed list.
pub fn parse_opml(xml: &str) -> Vec<VaultDoc> {
    let mut docs = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let closes = trimmed.matches("</outline").count();
        if closes > 0 {
            for _ in 0..closes {
                stack.pop();
            }
        }
        if trimmed.contains("<outline") {
            let self_closing = trimmed.ends_with("/>");
            let is_feed = trimmed.contains("xmlUrl");
            let title = attr(trimmed, "text")
                .or_else(|| attr(trimmed, "title"))
                .unwrap_or_default();

            if is_feed {
                let xml_url = attr(trimmed, "xmlUrl").unwrap_or_default();
                let html_url = attr(trimmed, "htmlUrl").unwrap_or_default();
                let mut body = xml_url;
                if !html_url.is_empty() {
                    body.push_str(&format!("\n\npage: {html_url}"));
                }
                if let Some(folder) = stack.last() {
                    body.push_str(&format!("\n\nfeed in: {folder}"));
                }
                docs.push(VaultDoc {
                    source: format!("feed {title}"),
                    title,
                    body,
                    headings: vec![],
                });
            } else if !title.is_empty() && !self_closing {
                stack.push(title);
            }
        }
    }
    docs
}

/// Chat/message export as JSON lines.
pub fn parse_messages_jsonl(input: &str) -> Vec<VaultDoc> {
    let mut docs = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let text = v
            .get("text")
            .or_else(|| v.get("message"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .trim();
        if text.is_empty() {
            continue;
        }
        let sender = v
            .get("from")
            .or_else(|| v.get("sender"))
            .and_then(|x| x.as_str());
        let title = if text.len() > 60 { &text[..60] } else { text };
        let mut body = text.to_string();
        if let Some(s) = sender {
            body.insert_str(0, &format!("{s}: "));
        }
        docs.push(VaultDoc {
            source: format!("message {title}"),
            title: title.to_string(),
            body,
            headings: vec![],
        });
    }
    docs
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    let val = &rest[..end];
    (!val.is_empty()).then(|| val.to_string())
}

/// Read a file and reduce it through the importer matching its extension.
pub fn import_file(path: &Path) -> anyhow::Result<(Vec<VaultDoc>, String)> {
    let importer = importer_for(path)
        .ok_or_else(|| anyhow::anyhow!("no personal-history importer for {}", path.display()))?;
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok((importer(&text), name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmarks_html_parses_links_and_folders() {
        let html = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
  <DT><H3 ADD_DATE="1">Work</H3>
  <DL><p>
    <DT><A HREF="https://shopfloor.example/" ADD_DATE="2">Shop floor</A>
  </DL><p>
  <DT><A HREF="https://q1pro.local/" ADD_DATE="3">Printer</A>
</DL><p>"#;
        let docs = parse_bookmarks_html(html);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].title, "Shop floor");
        assert!(docs[0].body.contains("shopfloor.example"), "url in body");
        assert!(docs[0].body.contains("Work"), "folder context");
        assert_eq!(docs[1].title, "Printer");
    }

    #[test]
    fn test_history_json_array_and_object() {
        let arr = r#"[{"title":"Autoclave Manual","url":"https://d.example/autoclave","lastVisitTime":1}]"#;
        let docs = parse_history_json(arr);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Autoclave Manual");
        assert!(docs[0].body.contains("https://d.example/autoclave"));

        let obj = r#"{"items":[{"title":"","url":"https://e.example/x"}]}"#;
        let docs = parse_history_json(obj);
        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs[0].title, "https://e.example/x",
            "empty title falls back to url"
        );

        assert!(parse_history_json("not json").is_empty());
    }

    #[test]
    fn test_opml_parses_feeds_and_folders() {
        let opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Manufacturing">
      <outline type="rss" text="IoT Blog" xmlUrl="https://iot.example/rss" htmlUrl="https://iot.example"/>
      <outline type="rss" text="OEE Weekly" xmlUrl="https://oee.example/feed"/>
    </outline>
  </body>
</opml>"#;
        let docs = parse_opml(opml);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].title, "IoT Blog");
        assert!(docs[0].body.contains("iot.example/rss"));
        assert!(docs[0].body.contains("Manufacturing"), "folder context");
        assert_eq!(docs[1].title, "OEE Weekly");
    }

    #[test]
    fn test_messages_jsonl_parses_senders() {
        let jsonl = "{\"from\":\"gio\",\"text\":\"check the spindle bearing\"}\n{\"from\":\"line2\",\"message\":\"ok\"}\nnot json\n";
        let docs = parse_messages_jsonl(jsonl);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].title, "check the spindle bearing");
        assert!(docs[0].body.starts_with("gio: "), "sender prefixed");
        assert_eq!(docs[1].body, "line2: ok");
    }

    #[test]
    fn test_importer_for_extensions() {
        assert!(importer_for(Path::new("b.html")).is_some());
        assert!(importer_for(Path::new("b.htm")).is_some());
        assert!(importer_for(Path::new("h.json")).is_some());
        assert!(importer_for(Path::new("feeds.opml")).is_some());
        assert!(importer_for(Path::new("chat.jsonl")).is_some());
        assert!(importer_for(Path::new("notes.txt")).is_some());
        assert!(importer_for(Path::new("data.csv")).is_none());
    }
}

// ── Dynamical-trace ingestion (dependency-free adapters) ─────────────────
//
// A "trace" is a time-ordered sequence of observations/states — the
// substrate-agnostic form Lenia/CQFT-style evolution takes once simulation
// stays external. These helpers normalize caller-supplied records into forms
// `becoming` (occurrences), temporal replay, and dream/history analysis can
// consume. No simulator, no field equations, no model downloads.

use serde::{Deserialize, Serialize};

/// One point in a time-ordered trace.
///
/// `t` is an ordering key (sequence index, unix millis, or any monotonic
/// scalar the caller chooses) — `becoming::Occurrence::at` is i64. Provide at
/// least one of `embedding` or `text` for semantic use; `embedding` wins when
/// both are present and an embedder is also supplied.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TracePoint {
    /// Ordering key (monotonic within a trace for order-aware analysis).
    #[serde(default)]
    pub t: i64,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

impl TracePoint {
    pub fn at(t: i64) -> Self {
        Self {
            t,
            ..Self::default()
        }
    }
}

/// A parsed, time-ordered trace (sorted by `t`, stable by original index).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DynamicalTrace {
    pub points: Vec<TracePoint>,
}

impl DynamicalTrace {
    /// Parse a JSON array of [`TracePoint`]s.
    pub fn from_json_array(json: &str) -> anyhow::Result<Self> {
        let mut points: Vec<TracePoint> = serde_json::from_str(json)?;
        stabilize_order(&mut points);
        Ok(Self { points })
    }

    /// Parse JSON Lines / NDJSON (one [`TracePoint`] per non-empty line).
    /// Malformed lines are skipped (loss-tolerant, like other history importers).
    pub fn from_jsonl(input: &str) -> Vec<TracePoint> {
        let mut out = Vec::new();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(p) = serde_json::from_str::<TracePoint>(line) {
                out.push(p);
            }
        }
        stabilize_order(&mut out);
        out
    }

    /// Convert to [`crate::becoming::Occurrence`]s, using `embedding` when
    /// present or `embedder` over `text` when supplied. Points with neither a
    /// usable embedding nor embeddable text are skipped. Order is preserved.
    pub fn to_occurrences(
        &self,
        embedder: Option<&dyn crate::embed::VectorEmbed>,
    ) -> Vec<crate::becoming::Occurrence> {
        points_to_occurrences(&self.points, embedder)
    }

    /// Convert to [`crate::vault::VaultDoc`]s for map/retrieval-style analysis.
    /// Points without textual content are rendered from subject/label/t.
    pub fn to_vault_docs(&self, prefix: &str) -> Vec<crate::vault::VaultDoc> {
        self.points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let title = p
                    .label
                    .clone()
                    .or_else(|| p.subject.clone())
                    .unwrap_or_else(|| format!("t={}", p.t));
                let mut body = String::new();
                if let Some(ref txt) = p.text {
                    body.push_str(txt);
                } else {
                    body.push_str(&format!("trace point t={} subject={:?}", p.t, p.subject));
                }
                if let Some(ref src) = p.source {
                    body.push_str(&format!("\nsource: {src}"));
                }
                crate::vault::VaultDoc {
                    source: format!("{prefix}:{i}"),
                    title,
                    body,
                    headings: vec![],
                }
            })
            .collect()
    }
}

fn stabilize_order(points: &mut [TracePoint]) {
    // Stable sort by t preserves caller order for ties (important for
    // deterministic order-aware/replay analysis).
    points.sort_by_key(|p| p.t);
}

/// Convert trace points to `becoming::Occurrence`s (skips points with no vector).
pub fn points_to_occurrences(
    points: &[TracePoint],
    embedder: Option<&dyn crate::embed::VectorEmbed>,
) -> Vec<crate::becoming::Occurrence> {
    let mut out = Vec::new();
    for p in points {
        let ctx = if let Some(ref emb) = p.embedding {
            if emb.is_empty() {
                None
            } else {
                Some(emb.clone())
            }
        } else if let (Some(ref txt), Some(e)) = (&p.text, embedder) {
            let v = e.embed(txt);
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        };
        if let Some(context) = ctx {
            out.push(crate::becoming::Occurrence { at: p.t, context });
        }
    }
    out
}

#[cfg(test)]
mod trace_tests {
    use super::*;
    use crate::embed::RandomProjectionEmbedder;

    #[test]
    fn json_array_trace_is_time_ordered_and_deterministic() {
        let json = r#"[
            {"t": 3, "text": "gamma", "subject": "s"},
            {"t": 1, "text": "alpha", "subject": "s"},
            {"t": 2, "text": "beta", "subject": "s"}
        ]"#;
        let tr = DynamicalTrace::from_json_array(json).unwrap();
        assert_eq!(tr.points.len(), 3);
        assert_eq!(tr.points[0].t, 1);
        assert_eq!(tr.points[1].t, 2);
        assert_eq!(tr.points[2].t, 3);
        let emb = RandomProjectionEmbedder::new(32);
        let occ = tr.to_occurrences(Some(&emb));
        assert_eq!(occ.len(), 3);
        assert_eq!(occ[0].at, 1);
        assert_eq!(occ[2].at, 3);
    }

    #[test]
    fn jsonl_trace_skips_malformed_and_keeps_embeddings() {
        let input = "{\"t\":2,\"embedding\":[1.0,0.0,0.0]}\nnot json\n{\"t\":1,\"embedding\":[0.0,1.0,0.0]}\n";
        let pts = DynamicalTrace::from_jsonl(input);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].t, 1, "must be sorted by t");
        let tr = DynamicalTrace { points: pts };
        let occ = tr.to_occurrences(None);
        assert_eq!(occ.len(), 2);
        assert_eq!(occ[0].context, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn trace_to_vault_docs_is_loss_tolerant() {
        let tr = DynamicalTrace {
            points: vec![
                TracePoint {
                    t: 5,
                    text: Some("hello world".into()),
                    label: Some("msg".into()),
                    ..Default::default()
                },
                TracePoint {
                    t: 6,
                    subject: Some("sensor-x".into()),
                    ..Default::default()
                },
            ],
        };
        let docs = tr.to_vault_docs("trace");
        assert_eq!(docs.len(), 2);
        assert!(docs[0].body.contains("hello world"));
        assert!(docs[1].title.contains("sensor-x") || docs[1].body.contains("sensor-x"));
    }
}
