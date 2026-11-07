//! Vault importers — Obsidian/markdown notes, plain text, and `git log`.
//!
//! Each source is reduced to [`VaultDoc`]s; callers register them as labeled core
//! nodes with the embedder so a knowledge vault is recallable by content.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single retrievable unit pulled out of a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDoc {
    /// Where the doc came from (file path, or `git <short-hash>` for commits).
    pub source: String,
    /// Human title: first H1, Obsidian `title:` frontmatter, or the file stem.
    pub title: String,
    /// Full body text (the recallable content).
    pub body: String,
    /// Markdown headings (`h1:Title`, `h2:Sub`, …) extracted from the body.
    pub headings: Vec<String>,
}

impl VaultDoc {
    /// Browsable content labels: the title plus one per heading (`title → h1:…`).
    pub fn labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self
            .headings
            .iter()
            .map(|h| format!("{} → {}", self.title, h))
            .collect();
        labels.push(self.title.clone());
        labels
    }
}

/// First non-empty H1, Obsidian `title:` frontmatter, or the file stem.
pub fn title_of(path: &Path, body: &str) -> String {
    // Obsidian frontmatter: a leading `---\n` block may carry `title: "X"`.
    let stripped = body.trim_start();
    if let Some(rest) = stripped.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let front = &rest[..end];
            for line in front.lines() {
                let line = line.trim();
                if let Some(t) = line.strip_prefix("title:") {
                    let t = t.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !t.is_empty() {
                        return t;
                    }
                }
            }
        }
    }
    // First H1.
    for line in body.lines() {
        let line = line.trim();
        if let Some(t) = line.strip_prefix("# ") {
            let t = t.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    // Fallback: file stem.
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string())
}

/// Extract `h1:`…`h3:` headings.
pub fn headings_of(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let hash_len = trimmed.chars().take_while(|c| *c == '#').count();
        if hash_len == 0 {
            continue;
        }
        let rest = trimmed[hash_len..].trim();
        if rest.is_empty() {
            continue;
        }
        let level = hash_len.min(3);
        out.push(format!("h{level}:{rest}"));
        if out.len() >= 20 {
            break;
        }
    }
    out
}

/// Recursively scan a directory for markdown/text notes.
pub fn scan_vault(dir: &Path) -> Vec<VaultDoc> {
    let mut docs = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];

    while let Some(current) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden dirs and common non-note dirs
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                }
                dirs.push(path);
            } else if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if matches!(
                    ext.as_str(),
                    "md" | "mdx" | "txt" | "rst" | "adoc" | "asciidoc" | "org"
                ) {
                    if let Ok(body) = std::fs::read_to_string(&path) {
                        if body.trim().len() < 8 {
                            continue;
                        }
                        let rel_path = path
                            .strip_prefix(dir)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();
                        let title = title_of(&path, &body);
                        let headings = headings_of(&body);
                        docs.push(VaultDoc {
                            source: rel_path,
                            title,
                            body,
                            headings,
                        });
                    }
                }
            }
        }
    }
    docs.sort_by(|a, b| a.source.cmp(&b.source));
    docs
}

/// Parse `git log --format=%H%x1f%s%x1f%b%x1e` output into commit docs.
pub fn parse_git_log(output: &str) -> Vec<VaultDoc> {
    let mut docs = Vec::new();
    for record in output.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        let mut fields = record.splitn(3, '\x1f');
        let hash = fields.next().unwrap_or_default().trim();
        let subject = fields.next().unwrap_or_default().trim();
        let body = fields.next().unwrap_or_default().trim();
        if hash.is_empty() && subject.is_empty() {
            continue;
        }
        let short = hash.get(..7).unwrap_or(hash);
        let mut full = subject.to_string();
        if !body.is_empty() {
            full.push_str("\n\n");
            full.push_str(body);
        }
        docs.push(VaultDoc {
            source: format!("git {short}"),
            title: if subject.is_empty() {
                short.to_string()
            } else {
                subject.to_string()
            },
            body: full,
            headings: vec![],
        });
    }
    docs
}

/// Run `git log` in `repo` and reduce the most recent `max` commits to docs.
pub fn scan_git_log(repo: &Path, max: usize) -> Vec<VaultDoc> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--format=%H%x1f%s%x1f%b%x1e", &format!("-n{max}")])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            parse_git_log(&text)
        }
        _ => vec![],
    }
}

/// Reduce a whole vault to labeled core nodes (label + embeddable text).
pub fn collect_labels(docs: &[VaultDoc]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for d in docs {
        for label in d.labels() {
            out.push((label, d.body.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(body: &str) -> VaultDoc {
        VaultDoc {
            source: "notes/a.md".to_string(),
            title: title_of(Path::new("notes/a.md"), body),
            body: body.to_string(),
            headings: headings_of(body),
        }
    }

    #[test]
    fn test_title_from_h1_and_frontmatter() {
        assert_eq!(md("# My Note\n\nbody").title, "My Note");
        assert_eq!(
            md("---\ntitle: \"Vendor Report\"\n---\n# Ignored\n\nbody").title,
            "Vendor Report"
        );
        assert_eq!(title_of(Path::new("notes/a.md"), "just some body"), "a");
    }

    #[test]
    fn test_headings_extracted() {
        let d = md("# Top\n\n## Sub A\n\n### Sub B\n\nplain text\n");
        assert_eq!(d.headings, vec!["h1:Top", "h2:Sub A", "h3:Sub B"]);
        assert_eq!(md("body only\nmore\n").headings.len(), 0);
    }

    #[test]
    fn test_labels_cover_title_and_headings() {
        let d = md("# Release Notes\n\n## Fixes\n\nstuff");
        let labels = d.labels();
        assert!(labels.contains(&"Release Notes".to_string()));
        assert!(labels.contains(&"Release Notes → h2:Fixes".to_string()));
    }

    #[test]
    fn test_parse_git_log() {
        let canned = "abc1234\x1fAdd autoclave cycle log\x1fLonger body here\x1e\
                      def5678\x1fFix bearing offset\x1f\x1e";
        let docs = parse_git_log(canned);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].source, "git abc1234");
        assert_eq!(docs[0].title, "Add autoclave cycle log");
        assert!(docs[0].body.contains("Longer body here"));
        assert_eq!(docs[1].title, "Fix bearing offset");
        let lone = parse_git_log("feed0000\x1f\x1f\x1e");
        assert_eq!(lone[0].title, "feed000");
    }

    #[test]
    fn test_scan_vault_reads_notes_and_skips_binaries() {
        let dir =
            std::env::temp_dir().join(format!("physis_core_vault_test_{}", std::process::id()));
        let notes = dir.join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("one.md"), "# First Note\n\nbody here").unwrap();
        std::fs::write(notes.join("two.txt"), "plain title\nsome content").unwrap();
        std::fs::write(notes.join("binary.bin"), [0u8, 159, 146, 150]).unwrap();

        let docs = scan_vault(&dir);
        let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();
        assert!(titles.contains(&"First Note"));
        assert!(
            titles.contains(&"two"),
            "txt file picked up with stem title"
        );
        assert_eq!(docs.len(), 2, "binary is not a note");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collect_labels_pairs_label_with_body() {
        let docs = vec![md("# Alpha\n\ncontent")];
        let pairs = collect_labels(&docs);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "Alpha → h1:Alpha");
        assert_eq!(pairs[0].1, "# Alpha\n\ncontent");
        assert_eq!(pairs[1].0, "Alpha");
    }
}
