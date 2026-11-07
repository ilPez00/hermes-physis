//! `physis` — the single front door to whichever Physis edition is installed.
//!
//! Physis ships as two executables with forty-odd subcommands between them, and
//! nothing told a newcomer which one to type. This binary is the answer to
//! `physis -h`: one help screen covering both halves, which says plainly what is
//! installed, what is not, and how to get the rest.
//!
//! It owns no subcommands of its own beyond `upgrade`. Everything else is
//! forwarded, unparsed, to `physis-core` or `physis-pro` — deliberately, so this
//! file never has to be updated when either of them grows a command, and so
//! their `--help`, exit codes and stdio behave exactly as if invoked directly.

use std::process::Command;

use physis_core::edition::{Edition, CORE_CLI, PRO_CLI, PRO_SUMMARY, PRO_WEB, UPGRADE_URL};

/// Subcommands served by `physis-core`.
///
/// Only used to decide who to forward to. An unknown command is passed to Pro
/// when Pro is installed (it has the larger surface and its own error message
/// is better than one invented here) and otherwise reported against Core.
const CORE_COMMANDS: &[(&str, &str)] = &[
    ("classify", "Score text against the semiotic grid"),
    ("ontology", "Show ontology stats"),
    ("facet", "Query ontology entries by orthogonal facets"),
    (
        "scan",
        "Register a directory's text files as coherence nodes",
    ),
    ("search", "Search recalled nodes by similarity"),
    (
        "node-search",
        "Fixed-token-budget search over recalled nodes",
    ),
    (
        "node-edit",
        "Swap a node's content, keeping id, verdict and edges",
    ),
    ("node-delete", "Delete a node and its index entry"),
    ("vault", "Import an Obsidian/markdown vault or git log"),
    (
        "history",
        "Import bookmarks, browser history, OPML or chat JSONL",
    ),
    ("praxis", "Backfill verdicts from a Praxis life-log export"),
    ("snapshot", "Coherence snapshot of the persisted graph"),
    (
        "assert",
        "Record a verdict on a node: success | inert | failure",
    ),
    ("dream", "Replay low-coherence and failed nodes"),
    ("quality", "Quality feedback loop"),
    (
        "hypothesis",
        "Competing hypotheses with evidence and fitness",
    ),
    ("contradiction", "Contradictions and truth maintenance"),
    ("audit", "Chronological epistemic audit log"),
    ("replay", "Reconstruct a belief state at a past timestamp"),
    ("discover", "Propose new domains from an unmapped corpus"),
    ("studio", "Open the Core studio in a browser"),
];

/// Subcommands served by `physis-pro`. Abridged on purpose: the full list is
/// `physis-pro --help`, and repeating forty entries here would rot.
const PRO_COMMANDS: &[(&str, &str)] = &[
    ("doctor", "Diagnose config, data dir, embedder and licence"),
    (
        "timeline",
        "Incident timeline from normalized operational events",
    ),
    ("report", "Render the shift report"),
    ("process", "Industrial process health"),
    ("inventory", "Inventory status"),
    ("order", "Human-in-the-loop order suggestion"),
    (
        "connect",
        "Run a live machine connector (MQTT, Modbus, serial, OPC-UA)",
    ),
    ("watch", "Auto-ingest a directory as it changes"),
    ("pack", "Pack a directory into a hard token budget"),
    ("rag", "Token-fixed retrieval over a directory"),
    ("benchmark", "Run the SECOM industrial benchmark"),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let edition = Edition::detect();

    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") | Some("help") => {
            print_help(&edition);
        }
        Some("-V") | Some("--version") => {
            println!("physis {} ({})", env!("CARGO_PKG_VERSION"), edition.name());
        }
        Some("upgrade") => print_upgrade(&edition),
        Some("web") | Some("console") => {
            // The Pro dashboards live in their own executable, so give them a
            // name at this level rather than making people find it.
            match &edition.pro_web {
                Some(path) => forward(path, &args[1..]),
                None => {
                    eprintln!("The Pro dashboards need `{PRO_WEB}`, which is not installed.\n");
                    print_upgrade(&edition);
                    std::process::exit(127);
                }
            }
        }
        Some(cmd) => dispatch(&edition, cmd, &args),
    }
}

/// Forward a subcommand to whichever executable owns it. Never returns: it
/// either execs the target or exits with a diagnostic.
fn dispatch(edition: &Edition, cmd: &str, args: &[String]) -> ! {
    let is_core = CORE_COMMANDS.iter().any(|(name, _)| *name == cmd);
    let is_pro = PRO_COMMANDS.iter().any(|(name, _)| *name == cmd);

    // Several names exist on both sides (`classify`, `scan`, `discover`,
    // `quality`, `facet`). Core wins: it is the edition that is always present,
    // so the same command means the same thing whether or not Pro is installed.
    // Every branch diverges, which is what lets this be one expression.
    if is_core {
        match &edition.core_cli {
            Some(path) => forward(path, args),
            None => missing(CORE_CLI, cmd),
        }
    } else if is_pro || edition.has_pro() {
        match &edition.pro_cli {
            Some(path) => forward(path, args),
            None if is_pro => {
                eprintln!("`physis {cmd}` is a Pro command, and Pro is not installed.\n");
                print_upgrade(edition);
                std::process::exit(127);
            }
            None => unknown(edition, cmd),
        }
    } else {
        unknown(edition, cmd)
    }
}

/// Replace this process with the target, so signals, exit codes, and an
/// attached terminal all behave as if it had been invoked directly.
fn forward(exe: &std::path::Path, args: &[String]) -> ! {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // `exec` only returns on failure.
        let err = Command::new(exe).args(args).exec();
        eprintln!("cannot run {}: {err}", exe.display());
        std::process::exit(126);
    }
    #[cfg(not(unix))]
    {
        match Command::new(exe).args(args).status() {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(err) => {
                eprintln!("cannot run {}: {err}", exe.display());
                std::process::exit(126);
            }
        }
    }
}

fn missing(exe: &str, cmd: &str) -> ! {
    eprintln!(
        "`physis {cmd}` needs `{exe}`, which is not on PATH.\n\
         Install it with:  cargo install physis-core"
    );
    std::process::exit(127);
}

fn unknown(edition: &Edition, cmd: &str) -> ! {
    eprintln!("Unknown command `{cmd}`.\n");
    print_help(edition);
    std::process::exit(2);
}

fn print_help(edition: &Edition) {
    println!("physis {} — {}", env!("CARGO_PKG_VERSION"), edition.name());
    println!();
    println!("  An engine that holds competing interpretations of what happened,");
    println!("  scores them against evidence, and keeps the ones that go on working.");
    println!();
    println!("USAGE");
    println!("  physis <command> [options]      every command's own --help still works");
    println!();

    println!(
        "CORE — the open engine{}",
        installed_note(edition.core_cli.is_some(), CORE_CLI)
    );
    for (name, about) in CORE_COMMANDS {
        println!("  {name:<14} {about}");
    }
    println!();

    println!(
        "PRO — industrial operations{}",
        installed_note(edition.has_pro(), PRO_CLI)
    );
    for (name, about) in PRO_COMMANDS {
        println!("  {name:<14} {about}");
    }
    if edition.has_pro() {
        println!(
            "  {:<14} Serve the Operations Console, Demo and Study dashboards",
            "web"
        );
        println!("  {:<14} full list: physis-pro --help", "…");
    } else {
        println!("                 Not installed — run `physis upgrade` to see what it adds.");
    }
    println!();

    println!("OTHER");
    println!("  {:<14} What Pro adds, and how to install it", "upgrade");
    println!("  {:<14} This screen", "-h, --help");
    println!("  {:<14} Version and installed edition", "-V, --version");
    println!();
    println!("STATE");
    println!("  {}", physis_core::store::data_dir().display());
    println!("  Override with PHYSIS_CORE_DIR. The CLI and the studio share it.");
}

/// `(installed)` / `(not installed)`, so the two sections are never ambiguous.
fn installed_note(present: bool, exe: &str) -> String {
    if present {
        String::new()
    } else {
        format!("   [{exe} not installed]")
    }
}

fn print_upgrade(edition: &Edition) {
    if edition.has_pro() {
        println!("Pro is already installed.");
        if let Some(p) = &edition.pro_cli {
            println!("  {PRO_CLI:<16} {}", p.display());
        }
        if let Some(p) = &edition.pro_web {
            println!("  {PRO_WEB:<16} {}", p.display());
        }
        println!();
        println!("  physis web            serve the Operations Console");
        println!("  physis doctor         check config, embedder and licence");
        return;
    }

    println!("Physis Pro — industrial operations on top of the Core engine");
    println!();
    for (title, detail) in PRO_SUMMARY {
        println!("  {title:<22} {detail}");
    }
    println!();
    println!("Pro is a separate, licensed product; Core stays open and keeps working");
    println!("without it. Nothing you build on Core needs migrating.");
    println!();
    println!("  {UPGRADE_URL}");
}
