//! Minimal, runnable example of the observation log — one of the capabilities
//! in `.physis-capabilities` (write = `watch_fs`/`append`, read = `by_source`).
//!
//! `cargo run --example observation_log`
//!
//! It appends two typed observations to a throwaway log and reads the tail
//! back. The log is append-only JSONL: a partial write costs one record, never
//! the file.

use physis_core::observe::{self, Observation};

fn main() -> anyhow::Result<()> {
    let path = std::env::temp_dir().join("physis-example-observations.jsonl");

    let mut batch = vec![
        Observation::new("fs", "src/lib.rs")
            .with_body("read as text")
            .by("watch_fs"),
        Observation::new("terminal", "cargo test")
            .with_duration(1200)
            .with_body("exit 0")
            .by("watch_fs"),
    ];

    let (first_seq, count) = observe::append(&path, &mut batch)?;
    println!("appended {count} observation(s), first seq {first_seq}");

    for obs in observe::read_tail(&path, 10)? {
        println!(
            "seq={:<3} source={:<8} subject={:<12} duration={:?} body={:?}",
            obs.seq, obs.source, obs.subject, obs.duration_ms, obs.body
        );
    }

    // A read path: select the observations from one source.
    let all = observe::read(&path)?;
    let fs = observe::by_source(&all, "fs");
    println!("{} observation(s) from source `fs`", fs.len());

    Ok(())
}
