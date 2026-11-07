//! The observation log — what the machine saw, append-only, forever.
//!
//! ## Why this shape
//!
//! A survey of eleven systems in this space (`computer-remake-research/`) found
//! that each plays exactly one of three roles and collapses the other two into
//! it: **observers** record honestly but their events never mean anything;
//! **indexers** make content findable but hold no position on what is true and
//! overwrite themselves; **interpreters** let a model decide, and the model
//! becomes the memory. They want to be three layers.
//!
//! This is the bottom layer. It borrows ActivityWatch's decomposition —
//! *bucket / event / watcher* — because it is the only model in that survey with
//! nothing file-specific in it. A git commit, a terminal command, a downloaded
//! page, a sensor reading and a model's own output are all the same shape:
//! something, from some source, over some interval.
//!
//! ## Three rules, and they are the layer
//!
//! 1. **Append-only.** An [`Observation`] is never rewritten and never deleted.
//!    It records what was seen, not what is true. It cannot be wrong, because it
//!    does not claim anything — which is exactly what makes it safe to build
//!    belief on top of.
//! 2. **Interval, not instant.** `duration_ms` makes *"what was happening while
//!    X"* a query rather than a join. ActivityWatch's `duration` is the detail
//!    most re-implementations drop and then miss.
//! 3. **Provenance in the record.** `produced_by` names the watcher, so a claim
//!    derived from an observation can be walked back to the thing that saw it.
//!
//! ## What this is not
//!
//! Not an index — nothing here is ranked or embedded. Not a filesystem: thirty
//! four years of semantic-filesystem attempts (Gifford 1991, BeFS, WinFS,
//! Nepomuk, and LSFS in 2025) died of being in the write path, of needing
//! applications to integrate, and of metadata that does not survive a file
//! leaving the machine. This sits beside the filesystem and observes it.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One thing that was seen.
///
/// Deliberately flat and deliberately unconstrained in `body`: a schema-first
/// design (Nepomuk) cannot record what its ontology never enumerated, which is
/// why so few applications ever integrated with it. Structure is imposed later,
/// by the layer above, and may be imposed differently tomorrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Monotonic per-log sequence. Stable across reads; the address a claim
    /// cites when it says where it came from.
    pub seq: u64,
    pub at: chrono::DateTime<chrono::Utc>,
    /// How long it went on. `None` = an instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// The bucket: `fs`, `git`, `terminal`, `model`, `human`, `sensor`…
    /// Free-form on purpose — a new source is a new string, not a migration.
    pub source: String,
    /// What was seen: a path, a command, a URL, a device id.
    pub subject: String,
    /// Content or summary. May be empty; an observation that something *changed*
    /// is still an observation.
    #[serde(default)]
    pub body: String,
    /// Which watcher produced this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<String>,
}

impl Observation {
    pub fn new(source: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            seq: 0,
            at: chrono::Utc::now(),
            duration_ms: None,
            source: source.into(),
            subject: subject.into(),
            body: String::new(),
            produced_by: None,
        }
    }
    pub fn with_body(mut self, b: impl Into<String>) -> Self {
        self.body = b.into();
        self
    }
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
    pub fn by(mut self, watcher: impl Into<String>) -> Self {
        self.produced_by = Some(watcher.into());
        self
    }
}

/// Path of the append-only log. JSONL: one observation per line, so a partial
/// write costs one record rather than the file, and `tail -f` works.
pub fn log_path() -> PathBuf {
    crate::store::data_dir().join("observations.jsonl")
}

/// Append observations, assigning sequence numbers. Returns the range written.
///
/// Opens in append mode and never rewrites: a corrupted line is a lost record,
/// not a lost log. That is the trade an append-only store exists to make.
pub fn append(path: &Path, obs: &mut [Observation]) -> anyhow::Result<(u64, u64)> {
    use std::io::Write;
    let mut next = last_seq(path)? + 1;
    let first = next;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for o in obs.iter_mut() {
        o.seq = next;
        writeln!(f, "{}", serde_json::to_string(o)?)?;
        next += 1;
    }
    Ok((first, next.saturating_sub(1)))
}

/// Highest sequence in the log, or 0 when empty.
///
/// Reads only the tail: appending must not cost a full parse of history, or
/// every write becomes linear in everything ever written.
pub fn last_seq(path: &Path) -> anyhow::Result<u64> {
    Ok(read_tail(path, 1)?.last().map(|o| o.seq).unwrap_or(0))
}

/// Read the whole log, skipping unparseable lines.
///
/// Skipping rather than failing is deliberate: a truncated final line from a
/// killed process must not make the entire history unreadable.
pub fn read(path: &Path) -> anyhow::Result<Vec<Observation>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Observation>(l).ok())
        .collect())
}

/// Read only the last `n` observations, without parsing the whole log.
///
/// ## Why this exists — measured, not anticipated
///
/// [`read`] parses the entire JSONL on every call, and every watcher calls it
/// to deduplicate. That is linear in total history, in both time and memory:
///
/// | log | disk | one `watch` run | peak RSS |
/// |---|---|---|---|
/// | 10k obs | 1 MB | 0.03 s | 19 MB |
/// | 100k obs | 16 MB | 0.40 s | 90 MB |
/// | 500k obs | 85 MB | **2.00 s** | **372 MB** |
///
/// At roughly ten new observations a minute, 500k arrives in about five weeks.
/// A continuous observer that costs two seconds and a third of a gigabyte per
/// tick is one a person notices — and **being noticed is precisely what killed
/// Nepomuk and WinFS**, neither of which failed on the quality of its
/// semantics. The research file `computer-remake-research/links.md` records
/// that; this is the same cliff arriving in the same way.
///
/// Deduplication only ever needs *recent* history: a watcher asks "have I
/// already recorded this?", and anything it could plausibly re-observe is near
/// the end of the log. Reading a bounded tail makes that check O(1) in total
/// history.
///
/// Seeks from the end in 64 KiB blocks and stops once `n` newline boundaries
/// have been passed, so cost depends on `n` rather than on file size.
pub fn read_tail(path: &Path, n: usize) -> anyhow::Result<Vec<Observation>> {
    use std::io::{Read, Seek, SeekFrom};
    if !path.exists() || n == 0 {
        return Ok(Vec::new());
    }
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    const BLOCK: u64 = 64 * 1024;

    let mut end = len;
    let mut buf: Vec<u8> = Vec::new();
    let mut lines = 0usize;
    while end > 0 && lines <= n {
        let start = end.saturating_sub(BLOCK);
        let size = (end - start) as usize;
        let mut chunk = vec![0u8; size];
        f.seek(SeekFrom::Start(start))?;
        f.read_exact(&mut chunk)?;
        lines += chunk.iter().filter(|b| **b == b'\n').count();
        chunk.extend_from_slice(&buf);
        buf = chunk;
        end = start;
    }

    let text = String::from_utf8_lossy(&buf);
    let mut all: Vec<Observation> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Observation>(l).ok())
        .collect();
    // The first line of the window may be a fragment; parsing already dropped
    // it. Take the newest `n`.
    if all.len() > n {
        all.drain(..all.len() - n);
    }
    Ok(all)
}

/// Observations from one source, newest first.
pub fn by_source(all: &[Observation], source: &str) -> Vec<Observation> {
    let mut v: Vec<Observation> = all.iter().filter(|o| o.source == source).cloned().collect();
    v.sort_by_key(|o| std::cmp::Reverse(o.seq));
    v
}

/// What else was going on while `seq` was observed.
///
/// This is the query `duration_ms` exists for: overlap in time, across sources.
/// Point events are treated as zero-length intervals, so a command that ran
/// during a long file edit is found by the same call that finds two overlapping
/// edits.
pub fn concurrent(all: &[Observation], seq: u64) -> Vec<Observation> {
    let Some(target) = all.iter().find(|o| o.seq == seq) else {
        return Vec::new();
    };
    let span = |o: &Observation| {
        let start = o.at.timestamp_millis();
        (start, start + o.duration_ms.unwrap_or(0) as i64)
    };
    let (ts, te) = span(target);
    let mut v: Vec<Observation> = all
        .iter()
        .filter(|o| o.seq != seq)
        .filter(|o| {
            let (os, oe) = span(o);
            os <= te && oe >= ts
        })
        .cloned()
        .collect();
    v.sort_by_key(|o| o.seq);
    v
}

// ── Watchers ────────────────────────────────────────────────────────────────
//
// A watcher is a function that looks at one kind of thing and returns
// observations. It holds no state beyond what it reads, so watchers are
// independent, cheap to add, and impossible to get wrong in a way that damages
// the log. `recall` (Go) reaches the same conclusion from the other direction:
// read-only adapters into stores you do not own, writes only ever to your own.

/// Watch a directory tree: emit one observation per file whose content hash
/// differs from the last observation of that path.
///
/// Hash-gating before doing anything expensive is OpenRecall's instinct
/// (`is_similar` before embedding) applied at the right level: content, not
/// pixels. A file touched but unchanged produces nothing, so re-running this is
/// close to free and the log does not fill with noise.
pub fn watch_fs(root: &Path, known: &[Observation], max_files: usize) -> Vec<Observation> {
    use sha2::{Digest, Sha256};
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            if seen >= max_files {
                return out;
            }
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            // Skip the noise every tree has. Not a security boundary — a
            // signal-to-noise one.
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let Ok(meta) = p.metadata() else { continue };
            if meta.len() > 2_000_000 {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&p) else {
                continue;
            };
            seen += 1;
            let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
            let subject = p.to_string_lossy().to_string();
            let unchanged = known
                .iter()
                .rev()
                .find(|o| o.source == "fs" && o.subject == subject)
                .map(|o| o.body == hash)
                .unwrap_or(false);
            if unchanged {
                continue;
            }
            out.push(
                Observation::new("fs", subject)
                    .with_body(hash)
                    .by("watch_fs"),
            );
        }
    }
    out
}

/// Watch git history: one observation per commit, with its duration spanning
/// back to the previous commit.
///
/// The duration is the point. A commit is an instant, but the *work* it records
/// is the interval since the last one — which makes [`concurrent`] able to
/// answer "what files were being edited while this commit was being written".
pub fn watch_git(repo: &Path, limit: usize) -> Vec<Observation> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", &format!("-{limit}"), "--format=%H%x1f%aI%x1f%s"])
        .output();
    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut rows: Vec<(String, chrono::DateTime<chrono::Utc>, String)> = Vec::new();
    for line in text.lines() {
        let mut f = line.split('\u{1f}');
        let (Some(h), Some(t), Some(s)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let Ok(at) = chrono::DateTime::parse_from_rfc3339(t) else {
            continue;
        };
        rows.push((h.to_string(), at.with_timezone(&chrono::Utc), s.to_string()));
    }
    // `git log` is newest-first; the gap to the NEXT row is the previous commit.
    let mut obs = Vec::new();
    for (i, (hash, at, subject)) in rows.iter().enumerate() {
        let dur = rows
            .get(i + 1)
            .map(|(_, prev, _)| (*at - *prev).num_milliseconds().max(0) as u64);
        let mut o = Observation::new("git", hash.clone())
            .with_body(subject.clone())
            .by("watch_git");
        o.at = *at;
        o.duration_ms = dur;
        obs.push(o);
    }
    obs.reverse(); // oldest first, so sequence order matches time order
    obs
}

/// Watch shell history: one observation per command, with its real runtime.
///
/// zsh's extended history is `: <epoch>:<elapsed>;<command>`, and `<elapsed>` is
/// the seconds the command actually ran. That is a genuine `duration_ms` rather
/// than an inferred one, which makes [`concurrent`] able to answer *"what was I
/// running while that file changed"* precisely instead of approximately.
///
/// bash history has no timestamps unless `HISTTIMEFORMAT` is set, so plain
/// lines are taken as instants rather than guessed at.
///
/// Read-only, like `recall`'s adapters into other tools' stores: this never
/// writes to the history file it reads.
pub fn watch_shell(hist: &Path, known: &[Observation], limit: usize) -> Vec<Observation> {
    // Lossy, not `read_to_string`. zsh writes metafied bytes for characters
    // outside ASCII, so a real history file is frequently NOT valid UTF-8 and
    // `read_to_string` returns Err — which this watcher would have reported as
    // "nothing changed". Silent emptiness from a source that plainly has
    // content is the worst failure shape available to a watcher, and it is what
    // shipped: the unit test used clean ASCII and never hit it.
    let Ok(bytes) = std::fs::read(hist) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let seen: std::collections::HashSet<&str> = known
        .iter()
        .filter(|o| o.source == "terminal")
        .map(|o| o.subject.as_str())
        .collect();

    // zsh writes a multi-line command as physical lines joined by a trailing
    // backslash. Read line-by-line, each continuation becomes a separate
    // timestamp-less "command" that was never run. Join them first.
    let mut joined: Vec<String> = Vec::new();
    for line in text.lines() {
        match joined.last_mut() {
            Some(prev) if prev.ends_with('\\') => {
                prev.pop();
                prev.push(' ');
                prev.push_str(line);
            }
            _ => joined.push(line.to_string()),
        }
    }

    let mut out = Vec::new();
    for line in joined.iter().rev().take(limit * 4) {
        let line = line.as_str();
        let (at, elapsed, cmd) = match line.strip_prefix(": ") {
            Some(rest) => {
                let Some((meta, cmd)) = rest.split_once(';') else {
                    continue;
                };
                let Some((ts, el)) = meta.split_once(':') else {
                    continue;
                };
                let Ok(ts) = ts.trim().parse::<i64>() else {
                    continue;
                };
                (
                    chrono::DateTime::from_timestamp(ts, 0),
                    el.trim().parse::<u64>().ok(),
                    cmd,
                )
            }
            None if !line.trim().is_empty() => (None, None, line),
            _ => continue,
        };
        let cmd = cmd.trim();
        // A command line is its own identity here; re-running the watcher must
        // not re-append what is already recorded.
        if cmd.is_empty()
            || seen.contains(cmd)
            || out.iter().any(|o: &Observation| o.subject == cmd)
        {
            continue;
        }
        let mut o = Observation::new("terminal", cmd).by("watch_shell");
        if let Some(t) = at {
            o.at = t;
        }
        // Zero-second commands are instants, not zero-length intervals.
        o.duration_ms = elapsed.filter(|e| *e > 0).map(|e| e * 1000);
        out.push(o);
        if out.len() >= limit {
            break;
        }
    }
    out.reverse(); // oldest first, so sequence order matches time order
    out
}

/// Watch an agent's own session store: one observation per turn.
///
/// This is the source the brief calls *model outputs*, and it is the one that
/// makes the substrate reflexive — the machine observes what it was asked and
/// what it answered, on the same timeline as the files it changed and the
/// commands it ran.
///
/// Read-only over JSONL, `recall`'s adapter discipline: writes go only to our
/// own log. Unparseable lines are skipped rather than fatal, because a session
/// being written to concurrently will always have a partial final line.
pub fn watch_agent(dir: &Path, known: &[Observation], limit: usize) -> Vec<Observation> {
    let seen: std::collections::HashSet<&str> = known
        .iter()
        .filter(|o| o.source == "agent")
        .map(|o| o.subject.as_str())
        .collect();

    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                files.push(p);
            }
        }
    }
    // Newest sessions first: the recent ones are the ones worth observing.
    files.sort_by_key(|p| std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok()));

    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        for line in text.lines() {
            if out.len() >= limit {
                out.reverse();
                return out;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let (Some(ts), Some(sid)) = (
                v.get("timestamp").and_then(|x| x.as_str()),
                v.get("sessionId").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            let Ok(at) = chrono::DateTime::parse_from_rfc3339(ts) else {
                continue;
            };
            // Subject is session+timestamp: stable, and unique per turn.
            let subject = format!("{}@{}", &sid[..8.min(sid.len())], ts);
            if seen.contains(subject.as_str()) {
                continue;
            }
            let kind = v.get("type").and_then(|x| x.as_str()).unwrap_or("turn");
            let body: String = v
                .get("content")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .chars()
                .take(400)
                .collect();
            let mut o = Observation::new("agent", subject)
                .with_body(format!("[{kind}] {body}"))
                .by("watch_agent");
            o.at = at.with_timezone(&chrono::Utc);
            out.push(o);
        }
    }
    out.reverse();
    out
}

/// Watch running processes: one observation per long-lived process, with its
/// real age as the duration.
///
/// Reads `/proc` directly — no dependency, no shelling out. `starttime` (field
/// 22 of `/proc/<pid>/stat`, in clock ticks since boot) plus the system boot
/// time gives when the process began, and the elapsed time to now is a genuine
/// interval.
///
/// `min_age_s` exists because a substrate that records every `ls` learns
/// nothing. Short-lived processes are noise; a build that has been running for
/// four minutes is a fact about what the machine is doing.
pub fn watch_proc(known: &[Observation], min_age_s: u64, limit: usize) -> Vec<Observation> {
    let Ok(uptime) = std::fs::read_to_string("/proc/uptime") else {
        return Vec::new();
    };
    let Some(up) = uptime
        .split_whitespace()
        .next()
        .and_then(|x| x.parse::<f64>().ok())
    else {
        return Vec::new();
    };
    let now = chrono::Utc::now();
    let boot = now - chrono::Duration::milliseconds((up * 1000.0) as i64);
    // Linux reports `starttime` in clock ticks; USER_HZ is 100 on every
    // mainstream configuration, and getting it exactly is not worth a libc dep
    // for a field used only to order and age processes.
    const TICKS_PER_SEC: f64 = 100.0;

    let seen: std::collections::HashSet<&str> = known
        .iter()
        .filter(|o| o.source == "process")
        .map(|o| o.subject.as_str())
        .collect();

    let Ok(rd) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        // comm is parenthesised and may itself contain spaces, so split after
        // the closing paren rather than on whitespace from the start.
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let comm = stat[stat.find('(').map(|i| i + 1).unwrap_or(0)..close].to_string();
        let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        let Some(start_ticks) = fields.get(19).and_then(|x| x.parse::<f64>().ok()) else {
            continue;
        };
        let started =
            boot + chrono::Duration::milliseconds((start_ticks / TICKS_PER_SEC * 1000.0) as i64);
        let age_ms = (now - started).num_milliseconds().max(0) as u64;
        if age_ms < min_age_s * 1000 {
            continue;
        }
        // pid+start identifies this run: a recycled pid is a different process.
        let subject = format!("{comm}#{pid}@{}", started.timestamp());
        if seen.contains(subject.as_str()) {
            continue;
        }
        let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline"))
            .map(|c| c.replace('\0', " ").trim().to_string())
            .unwrap_or_default();
        let mut o = Observation::new("process", subject)
            .with_body(cmdline.chars().take(200).collect::<String>())
            .by("watch_proc");
        o.at = started;
        o.duration_ms = Some(age_ms);
        out.push(o);
        if out.len() >= limit {
            break;
        }
    }
    out.sort_by_key(|o| o.at);
    out
}

/// Watch browser history: one observation per visited page.
///
/// Firefox `places.sqlite` and Chromium `History` are both SQLite, and this
/// crate has no SQLite dependency by design. Rather than take one for a
/// watcher, it shells out to the `sqlite3` CLI in **read-only immutable** mode,
/// which is also the only safe way to read a database the browser has open.
///
/// Returns nothing — and says nothing — when `sqlite3` is absent. That is a
/// missing capability, not an error: the rest of the substrate is unaffected.
pub fn watch_browser(db: &Path, known: &[Observation], limit: usize) -> Vec<Observation> {
    let firefox = db.file_name().and_then(|n| n.to_str()) == Some("places.sqlite");
    // Firefox stores microseconds since epoch; Chromium stores microseconds
    // since 1601-01-01, hence the 11644473600s offset.
    let sql = if firefox {
        format!(
            "SELECT url, COALESCE(title,''), last_visit_date/1000000 FROM moz_places \
             WHERE last_visit_date IS NOT NULL ORDER BY last_visit_date DESC LIMIT {limit};"
        )
    } else {
        format!(
            "SELECT url, COALESCE(title,''), last_visit_time/1000000-11644473600 FROM urls \
             WHERE last_visit_time > 0 ORDER BY last_visit_time DESC LIMIT {limit};"
        )
    };
    let uri = format!("file:{}?immutable=1", db.display());
    let Ok(out) = std::process::Command::new("sqlite3")
        .args(["-readonly", "-separator", "\u{1f}", &uri, &sql])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }

    let seen: std::collections::HashSet<&str> = known
        .iter()
        .filter(|o| o.source == "browser")
        .map(|o| o.subject.as_str())
        .collect();

    let text = String::from_utf8_lossy(&out.stdout);
    let mut obs = Vec::new();
    for line in text.lines() {
        let mut f = line.split('\u{1f}');
        let (Some(url), Some(title), Some(ts)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if url.is_empty() || seen.contains(url) {
            continue;
        }
        let Ok(ts) = ts.trim().parse::<i64>() else {
            continue;
        };
        let Some(at) = chrono::DateTime::from_timestamp(ts, 0) else {
            continue;
        };
        let mut o = Observation::new("browser", url)
            .with_body(title)
            .by("watch_browser");
        o.at = at;
        obs.push(o);
    }
    obs.reverse(); // oldest first
    obs
}

// ── The claim path ──────────────────────────────────────────────────────────
//
// This is the join the survey found missing everywhere. Observers record; they
// never assert. Indexers rank; they hold no position. The step from "this was
// seen" to "this is so, and here is why, and it might be wrong" exists in no
// surveyed system — and it is the only step that lets a machine notice it was
// mistaken rather than merely outdated.

/// Turn an observation into a claim, citing the observation as its evidence.
///
/// The claim enters as `Candidate` with the observation attached as supporting
/// evidence, sourced to `obs:<seq>` so the citation resolves back into the log.
/// Three properties follow, and each is deliberate:
///
/// - **The observation is untouched.** It stays in the append-only log exactly
///   as recorded. A claim derived from it is a separate object with its own
///   status, and refuting the claim never edits the record it came from.
/// - **The claim can be wrong.** That is the entire point of promoting it.
/// - **Provenance is an address, not a copy.** `obs:<seq>` is stable, so the
///   evidence can always be re-read rather than trusted.
pub fn promote(
    obs: &Observation,
    statement: impl Into<String>,
    embedding: Vec<f32>,
) -> crate::hypothesis::Hypothesis {
    let mut h = crate::hypothesis::Hypothesis::new(statement, embedding);
    let claim = if obs.body.is_empty() {
        format!(
            "{} observed {} at {}",
            obs.source,
            obs.subject,
            obs.at.to_rfc3339()
        )
    } else {
        format!(
            "{} observed {} at {}: {}",
            obs.source,
            obs.subject,
            obs.at.to_rfc3339(),
            obs.body.chars().take(200).collect::<String>()
        )
    };
    h.add_supporting_evidence(crate::hypothesis::Evidence::supports(
        format!("obs:{}", obs.seq),
        claim,
    ));
    // Provenance is not corroboration, and conflating them is an epistemic bug
    // rather than a cosmetic one.
    //
    // `add_supporting_evidence` promotes Candidate -> Supported as soon as any
    // evidence lands, which is right for a measurement and wrong here: the
    // observation is what the claim is ABOUT, not a reason to believe it. That a
    // commit happened is no evidence that an assertion about that commit is
    // true. Left alone, every promoted observation would enter the shared state
    // already looking established, and `ground` would show a wall of Supported
    // claims nobody had tested.
    //
    // The citation stays — `cited_by` walks it, and the evidence can be re-read.
    // Only the standing is reset.
    h.status = crate::hypothesis::HypothesisStatus::Candidate;
    h.recompute_fitness();
    h
}

/// The observation a claim's evidence cites, if any.
///
/// Walks `obs:<seq>` sources back into the log. Returns the observations that
/// actually exist: a citation to a sequence that is not in the log is reported
/// by its absence rather than by a silent empty result, because a dangling
/// citation means the log and the claims have diverged.
pub fn cited_by(h: &crate::hypothesis::Hypothesis, all: &[Observation]) -> Vec<Observation> {
    let mut out = Vec::new();
    for e in h
        .supporting_evidence
        .iter()
        .chain(h.contradicting_evidence.iter())
    {
        let Some(rest) = e.source.strip_prefix("obs:") else {
            continue;
        };
        let Ok(seq) = rest.trim().parse::<u64>() else {
            continue;
        };
        if let Some(o) = all.iter().find(|o| o.seq == seq) {
            out.push(o.clone());
        }
    }
    out.sort_by_key(|o| o.seq);
    out.dedup_by_key(|o| o.seq);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("physis-obs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The one invariant the whole layer rests on: appending never disturbs what
    /// is already there, and sequence numbers keep rising across calls.
    #[test]
    fn appending_never_rewrites_and_sequences_keep_rising() {
        let p = tmp("append").join("o.jsonl");
        let (a1, b1) = append(&p, &mut [Observation::new("fs", "x")]).unwrap();
        assert_eq!((a1, b1), (1, 1));
        let (a2, b2) = append(
            &p,
            &mut [Observation::new("git", "y"), Observation::new("git", "z")],
        )
        .unwrap();
        assert_eq!((a2, b2), (2, 3));
        let all = read(&p).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].subject, "x", "the first record must be untouched");
        assert_eq!(all.iter().map(|o| o.seq).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    /// A killed process leaves a half-written final line. That must cost one
    /// record, not the history.
    #[test]
    fn a_truncated_line_does_not_destroy_the_log() {
        use std::io::Write;
        let p = tmp("trunc").join("o.jsonl");
        append(&p, &mut [Observation::new("fs", "good")]).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        write!(f, "{{\"seq\":2,\"at\":\"not-a-date").unwrap();
        drop(f);
        let all = read(&p).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].subject, "good");
    }

    /// `duration_ms` earns its keep here: overlap across sources, which is the
    /// query a point-event log cannot answer at all.
    #[test]
    fn concurrent_finds_overlap_across_sources() {
        let t0 = chrono::Utc::now();
        let mut long = Observation::new("fs", "editing.rs");
        long.seq = 1;
        long.at = t0;
        long.duration_ms = Some(60_000);

        let mut during = Observation::new("terminal", "cargo test");
        during.seq = 2;
        during.at = t0 + chrono::Duration::seconds(30);

        let mut after = Observation::new("terminal", "git push");
        after.seq = 3;
        after.at = t0 + chrono::Duration::seconds(120);

        let all = vec![long, during, after];
        let c = concurrent(&all, 1);
        assert_eq!(c.len(), 1, "only the overlapping event");
        assert_eq!(c[0].subject, "cargo test");
    }

    /// Re-running a watcher over an unchanged tree must produce nothing. If this
    /// regresses, continuous observation becomes unaffordable and the log fills
    /// with noise that means nothing.
    #[test]
    fn the_fs_watcher_is_silent_when_nothing_changed() {
        let d = tmp("fs");
        std::fs::write(d.join("a.txt"), "hello").unwrap();
        let first = watch_fs(&d, &[], 100);
        assert_eq!(first.len(), 1);
        let second = watch_fs(&d, &first, 100);
        assert!(
            second.is_empty(),
            "unchanged tree must produce no observations"
        );

        std::fs::write(d.join("a.txt"), "changed").unwrap();
        let third = watch_fs(&d, &first, 100);
        assert_eq!(third.len(), 1, "a content change must be seen");
    }

    /// The join, stated as a test: promoting an observation must leave the
    /// observation alone and produce something that CAN be contradicted.
    #[test]
    fn promoting_leaves_the_observation_untouched_and_yields_a_refutable_claim() {
        let mut o = Observation::new("git", "abc123").with_body("fix the null");
        o.seq = 7;
        let before = o.clone();

        let h = promote(&o, "the null was invalid", vec![0.1, 0.2]);
        assert_eq!(o, before, "promotion must not mutate the observation");
        assert_eq!(h.supporting_evidence.len(), 1);
        assert_eq!(h.supporting_evidence[0].source, "obs:7");
        // Provenance is not corroboration: citing the observation must NOT make
        // the claim look established before anyone has tested it.
        assert_eq!(
            h.status,
            crate::hypothesis::HypothesisStatus::Candidate,
            "a promoted observation is asserted, not supported"
        );

        // And it can be refuted — which no surveyed system's record can be.
        let mut h = h;
        h.add_contradicting_evidence(crate::hypothesis::Evidence::contradicts(
            "measurement",
            "delta was exactly zero",
        ));
        assert_eq!(h.status, crate::hypothesis::HypothesisStatus::Contradicted);
    }

    /// Provenance is an address: the evidence must resolve back into the log.
    #[test]
    fn a_claim_walks_back_to_the_observation_that_caused_it() {
        let mut o = Observation::new("fs", "src/main.rs").with_body("deadbeef");
        o.seq = 42;
        let h = promote(&o, "main.rs changed", vec![0.0]);
        let found = cited_by(&h, &[o.clone()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].seq, 42);
        // A citation into an empty log resolves to nothing, not to a guess.
        assert!(cited_by(&h, &[]).is_empty());
    }

    /// zsh extended history carries the REAL runtime of each command, which is
    /// what makes cross-source overlap precise rather than inferred.
    #[test]
    fn the_shell_watcher_reads_real_durations_and_dedupes() {
        let d = tmp("shell");
        let h = d.join("hist");
        std::fs::write(
            &h,
            ": 1700000000:12;cargo test\n: 1700000100:0;ls\n: 1700000200:5;git push\n",
        )
        .unwrap();
        let first = watch_shell(&h, &[], 10);
        assert_eq!(first.len(), 3);
        let ct = first.iter().find(|o| o.subject == "cargo test").unwrap();
        assert_eq!(ct.duration_ms, Some(12_000), "12 elapsed seconds");
        let ls = first.iter().find(|o| o.subject == "ls").unwrap();
        assert_eq!(ls.duration_ms, None, "a zero-second command is an instant");
        // Oldest first, so sequence order matches time order.
        assert_eq!(first[0].subject, "cargo test");
        // Re-running must not duplicate.
        assert!(watch_shell(&h, &first, 10).is_empty());
    }

    /// A session file being written to concurrently always has a partial final
    /// line. That must cost one turn, not the whole session.
    /// A multi-line command is one command. Read as physical lines, each
    /// continuation becomes a timestamp-less entry for something that was never
    /// run as written.
    #[test]
    fn the_shell_watcher_joins_continuation_lines() {
        let d = tmp("shell-cont");
        let h = d.join("hist");
        std::fs::write(
            &h,
            ": 1700000000:2;cargo build \\\n  --release \\\n  --features cli\n: 1700000100:0;ls\n",
        )
        .unwrap();
        let obs = watch_shell(&h, &[], 10);
        assert_eq!(
            obs.len(),
            2,
            "one joined command plus ls, not four fragments"
        );
        let built = obs
            .iter()
            .find(|o| o.subject.starts_with("cargo build"))
            .unwrap();
        assert!(
            built.subject.contains("--release"),
            "got {:?}",
            built.subject
        );
        assert!(built.subject.contains("--features cli"));
        assert_eq!(built.duration_ms, Some(2_000));
    }

    /// A real zsh history is frequently not valid UTF-8. The watcher must read
    /// it anyway rather than reporting "nothing changed", which is what it did
    /// until this test existed.
    #[test]
    fn the_shell_watcher_survives_invalid_utf8() {
        use std::io::Write;
        let d = tmp("shell-utf8");
        let h = d.join("hist");
        let mut f = std::fs::File::create(&h).unwrap();
        f.write_all(b": 1700000000:3;echo ").unwrap();
        f.write_all(&[0xE9, 0x83]).unwrap(); // metafied bytes, invalid UTF-8
        f.write_all(b"\n: 1700000100:0;ls\n").unwrap();
        drop(f);
        assert!(
            std::fs::read_to_string(&h).is_err(),
            "the fixture must be invalid UTF-8"
        );

        let obs = watch_shell(&h, &[], 10);
        assert_eq!(
            obs.len(),
            2,
            "both commands must be read despite the bad bytes"
        );
        assert!(obs.iter().any(|o| o.subject == "ls"));
    }

    #[test]
    fn the_agent_watcher_skips_partial_lines() {
        let d = tmp("agent");
        std::fs::write(
            d.join("s.jsonl"),
            "{\"timestamp\":\"2026-09-12T10:00:00Z\",\"sessionId\":\"abcdef123\",\"type\":\"user\",\"content\":\"hello\"}\n             {\"timestamp\":\"2026-09-12T10:01:00Z\",\"sessionId\":\"abcdef123\",\"type\":\"assistant\",\"content\":\"hi\"}\n             {\"timestamp\":\"not-fin",
        )
        .unwrap();
        let obs = watch_agent(&d, &[], 10);
        assert_eq!(obs.len(), 2, "two good turns, one partial line skipped");
        assert!(obs.iter().all(|o| o.source == "agent"));
        assert!(obs[0].body.contains("[user]") || obs[1].body.contains("[user]"));
        assert!(
            watch_agent(&d, &obs, 10).is_empty(),
            "must dedupe on re-run"
        );
    }

    /// Short-lived processes are noise; a build running for minutes is a fact.
    /// The age gate is what keeps continuous observation affordable.
    #[test]
    fn the_process_watcher_gates_on_age() {
        // This test's own process has been alive for well under an hour.
        let long = watch_proc(&[], 3600, 50);
        let short = watch_proc(&[], 0, 50);
        assert!(
            short.len() >= long.len(),
            "a lower age gate cannot return fewer processes"
        );
        for o in &short {
            assert_eq!(o.source, "process");
            assert!(o.duration_ms.is_some(), "a process age is a real interval");
            assert!(
                o.subject.contains('#'),
                "pid must be in the identity: {}",
                o.subject
            );
        }
    }

    /// A recycled pid is a different process, so identity must include when it
    /// started — otherwise re-running the watcher silently merges two runs.
    #[test]
    fn process_identity_includes_start_time() {
        let obs = watch_proc(&[], 0, 5);
        if let Some(o) = obs.first() {
            assert!(
                o.subject.contains('@'),
                "identity needs a start stamp: {}",
                o.subject
            );
            // Re-running with these known must produce nothing for them.
            let again = watch_proc(&obs, 0, 5);
            assert!(again
                .iter()
                .all(|n| !obs.iter().any(|k| k.subject == n.subject)));
        }
    }

    /// A missing `sqlite3` is a missing capability, not a failure: the watcher
    /// returns nothing and the rest of the substrate is unaffected.
    #[test]
    fn the_browser_watcher_is_quiet_when_the_db_is_absent() {
        let d = tmp("browser");
        assert!(watch_browser(&d.join("places.sqlite"), &[], 10).is_empty());
    }

    /// `read_tail` must be bounded by `n`, not by file size — that is the whole
    /// reason it exists. Before it, a `watch` run at 500k observations cost
    /// 2.00 s and 372 MB; after, 0.05 s and 23 MB, flat.
    #[test]
    fn read_tail_is_bounded_by_n_not_by_file_size() {
        let p = tmp("tail").join("o.jsonl");
        let mut batch: Vec<Observation> = (0..5_000)
            .map(|i| Observation::new("fs", format!("/f{i}.rs")))
            .collect();
        append(&p, &mut batch).unwrap();

        let tail = read_tail(&p, 10).unwrap();
        assert_eq!(tail.len(), 10, "must return exactly n");
        assert_eq!(
            tail.last().unwrap().seq,
            5_000,
            "and they must be the NEWEST"
        );
        assert_eq!(tail.first().unwrap().seq, 4_991);

        // Asking for more than exists returns everything, not an error.
        assert_eq!(read_tail(&p, 99_999).unwrap().len(), 5_000);
        assert!(read_tail(&p, 0).unwrap().is_empty());
    }

    /// `append` calls `last_seq`, so if that read the whole log every write
    /// would be linear in everything ever written.
    #[test]
    fn appending_does_not_reparse_history() {
        let p = tmp("append-cheap").join("o.jsonl");
        let mut batch: Vec<Observation> = (0..2_000)
            .map(|i| Observation::new("fs", format!("/f{i}")))
            .collect();
        append(&p, &mut batch).unwrap();
        // The next sequence must be correct without a full parse.
        assert_eq!(last_seq(&p).unwrap(), 2_000);
        let (a, b) = append(&p, &mut [Observation::new("fs", "/new")]).unwrap();
        assert_eq!((a, b), (2_001, 2_001));
    }

    #[test]
    fn by_source_filters_and_orders_newest_first() {
        let mut a = Observation::new("fs", "1");
        a.seq = 1;
        let mut b = Observation::new("git", "2");
        b.seq = 2;
        let mut c = Observation::new("fs", "3");
        c.seq = 3;
        let v = by_source(&[a, b, c], "fs");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].subject, "3");
    }
}
