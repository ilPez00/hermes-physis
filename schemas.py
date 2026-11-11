"""Tool schemas exposed to the LLM.

Descriptions are written as operating instructions: what the tool answers and
what to do when it cannot. A model that reads only the schema should be able
to use the plugin without the README.
"""

ENGINE_STATUS = {
    "name": "physis_engine_status",
    "description": (
        "Check the bundled Physis engines: versions, advertised commands, "
        "indexed node count and where the index is stored. Run this first if "
        "any other physis tool reports a missing engine."
    ),
    "parameters": {"type": "object", "properties": {}, "required": []},
}

OBSERVE = {
    "name": "physis_observe",
    "description": (
        "Index a repository into the local coherence graph. Run this before "
        "physis_query/physis_context/physis_why on a repo that may have "
        "changed. The store is one per repository, keyed by the repository "
        "root (nearest .git/.hg/.svn), so scanning a subdirectory and scanning "
        "the root share one graph; the path argument selects what gets scanned. "
        "The reply carries repo_root and store."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Repository or directory to scan (default: current directory)",
            },
        },
        "required": [],
    },
}

CONTEXT = {
    "name": "physis_context",
    "description": (
        "Compile a fixed-token context packet for a task directly from a "
        "corpus directory: the engine reports baseline tokens, packed tokens, "
        "compression ratio and the clusters it used, and returns the packed "
        "context itself. Works on code and docs; no prior scan required."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "task": {
                "type": "string",
                "description": "The task or question the packet must support",
            },
            "path": {"type": "string", "description": "Corpus directory (default: current directory)"},
            "budget": {
                "type": "integer",
                "description": "Token budget for the packet (100-4000, default 800)",
            },
            "mode": {
                "type": "string",
                "enum": ["code", "docs"],
                "description": (
                    "code (default) uses the scanned coherence graph and sees "
                    "source files; docs uses the markdown/text context compiler"
                ),
            },
        },
        "required": ["task"],
    },
}

QUERY = {
    "name": "physis_query",
    "description": (
        "Search indexed nodes by similarity and return matching files/snippets "
        "with scores. Use for 'where is X handled' questions. Requires "
        "physis_observe first; a 'No recalled nodes' message means the index "
        "is empty for this store."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "question": {"type": "string", "description": "What to search for"},
            "path": {"type": "string", "description": "Working directory"},
            "limit": {
                "type": "integer",
                "description": "Maximum matches (1-50, default 10)",
            },
        },
        "required": ["question"],
    },
}

WHY = {
    "name": "physis_why",
    "description": (
        "Return the indexed evidence around a named object (file, symbol, "
        "label): the nodes the engine associates with it, with similarity and "
        "coherence. This is retrieval, not a fabricated causal story: if the "
        "object is absent, say the evidence is missing."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "object": {"type": "string", "description": "Label, symbol or path to trace"},
            "path": {"type": "string", "description": "Working directory"},
            "budget": {
                "type": "integer",
                "description": "Token budget for evidence (100-4000, default 600)",
            },
        },
        "required": ["object"],
    },
}

DIFF = {
    "name": "physis_diff",
    "description": (
        "Ask whether a transformation went the way it was asked to: reads a "
        "before and an after file, compares the change against the null of "
        "doing nothing, and returns moved/tokens/retention. Use `want` to say "
        "what the change was supposed to achieve."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "before": {"type": "string", "description": "Path to the before file"},
            "after": {"type": "string", "description": "Path to the after file"},
            "want": {
                "type": "string",
                "enum": ["shorter", "longer", "rephrased"],
                "description": "What the change was supposed to do (default: shorter)",
            },
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["before", "after"],
    },
}

CLASSIFY = {
    "name": "physis_classify",
    "description": (
        "Classify a text or file against the 5x14 semiotic grid and return the "
        "top cells, the margin over the runner-up, and whether the result is "
        "effectively a tie."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to classify"},
            "file": {"type": "string", "description": "File to classify (alternative to text)"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": [],
    },
}

SNAPSHOT = {
    "name": "physis_snapshot",
    "description": (
        "Summarise the coherence graph: node count, coherence index, "
        "high/mid/low distribution and asserted verdicts. Cheap health check "
        "before or after indexing."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": [],
    },
}

CHAIN = {
    "name": "physis_chain",
    "description": (
        "One-pass analysis of a corpus: structure, bounded context, coverage "
        "gaps with a shortlist per gap, drift detection, and the label-permuted "
        "control the results are scored against. Heavier than physis_context; "
        "use it for an overview or a gap audit, not for every question."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Corpus directory (default: current directory)"},
            "task": {"type": "string", "description": "Optional query to focus the pass"},
            "budget": {"type": "integer", "description": "Token budget (100-4000, default 1200)"},
            "threshold": {
                "type": "number",
                "description": "Cosine below which a record counts as unplaced (default 0.75)",
            },
        },
        "required": [],
    },
}

GRID = {
    "name": "physis_grid",
    "description": (
        "Show the adaptive semiotic grid: every domain and mode the loaded "
        "ontologies define, the shape, how many cells are populated, and which "
        "embedder is active. The grid is not fixed at 5x14; extra ontologies "
        "(built-in or PHYSIS_ONTOLOGY_DIR) add axes."
    ),
    "parameters": {"type": "object", "properties": {}, "required": []},
}

INGEST = {
    "name": "physis_ingest",
    "description": (
        "Append observations to the engine log from a real source: fs (file "
        "changes), git (commits), terminal (shell history), agent (another "
        "coding agent's session transcripts under ~/.claude/projects), process "
        "(every process's /proc/<pid>/cmdline), browser (history database) or "
        "all. terminal/agent/process/browser are sensitive and refused unless "
        "the operator enables them in "
        "plugins.entries.physis.settings.ingest_sources (default: fs, git). "
        "Append-only and idempotent per source; dry_run shows without writing."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "source": {
                "type": "string",
                "enum": ["fs", "git", "terminal", "agent", "process", "browser", "all"],
                "description": (
                    "Observation source (default: fs). terminal/agent/process/"
                    "browser are opt-in sensitive sources."
                ),
            },
            "path": {"type": "string", "description": "Path for fs/git sources"},
            "max": {"type": "integer", "description": "Maximum observations (default 500)"},
            "dry_run": {"type": "boolean", "description": "Show what would be appended"},
        },
        "required": [],
    },
}

OBSERVED = {
    "name": "physis_observed",
    "description": (
        "Read the observation log: what was seen, by which source, and when. "
        "Use it after physis_ingest to verify what actually entered the world. "
        "A disabled sensitive source is refused by name and filtered out of an "
        "unfiltered read."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "source": {
                "type": "string",
                "description": (
                    "Only this source bucket. terminal/agent/process/browser "
                    "require the operator to enable them in "
                    "plugins.entries.physis.settings.ingest_sources."
                ),
            },
            "limit": {"type": "integer", "description": "Maximum observations (default 20)"},
            "concurrent": {
                "type": "string",
                "description": "What overlapped this observation in time",
            },
        },
        "required": [],
    },
}

REPLANT_RUN = {
    "name": "replant_run",
    "description": (
        "Replay a verified tool recipe for an exact request, or return miss. "
        "The recipe's real actions and checks run again - no cached success is "
        "returned. Refuses with blocked=true when the task inputs fingerprinted "
        "at record time have changed. The tools registry is operator-owned "
        "(replant_tools setting) or the plugin-generated learned registry from "
        "/physis approve; you cannot name one, and it is never taken from a "
        "repository path."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Request prompt to match"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["input"],
    },
}

REPLANT_RECORD = {
    "name": "replant_record",
    "description": (
        "Record a verified recipe once against the operator's replant_tools "
        "registry: execute its actions and independent checks, then persist it "
        "for replay. You may compose steps out of the tools the operator "
        "declared, but you cannot name a registry, a state directory or a "
        "candidate. Pass artifacts= (files the work depends on) to fingerprint "
        "the inputs; a later replay whose inputs changed is refused. Approving a "
        "learned workflow is exclusively the operator's /physis approve."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Request prompt to remember"},
            "recipe": {
                "type": "string",
                "description": "JSON recipe: {\"steps\":[{\"tool\",\"input\"}],\"checks\":[...]}",
            },
            "trace": {"type": "string", "description": "JSON trace metadata (optional)"},
            "artifacts": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Input files this recipe depends on (hashed at record time)",
            },
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["input", "recipe"],
    },
}

REPLANT_STATS = {
    "name": "replant_stats",
    "description": "Report replant receipt counts: records, replays, misses, blocks and replay share.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": [],
    },
}

REPLANT_DEMO = {
    "name": "replant_demo",
    "description": (
        "Zero-config demonstration of the record->replay loop using the "
        "bundled counter fixture: records once, replays twice, returns the "
        "operations and stats. Use it to prove the engine works before wiring "
        "a real registry."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": [],
    },
}

REPLANT_SHOW = {
    "name": "replant_show",
    "description": "Show the learned replant record (if any) for an exact request.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Request prompt to look up"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["input"],
    },
}

RECIPE_LOOKUP = {
    "name": "physis_recipe_lookup",
    "description": (
        "Read-only reuse check: is there an approved recipe for this request, "
        "what does it depend on, and are those dependencies still present and "
        "unchanged? Use it to decide whether the work is a verified procedure "
        "you should not re-derive. It never executes anything and cannot "
        "approve one: execution stays with the operator's '/physis replay "
        "<prompt>' and approval with '/physis approve <id>'. When replayable is "
        "false, the reason is in tracked/artifacts/approval."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Request prompt to look up"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["input"],
    },
}

CANDIDATES = {
    "name": "physis_candidates",
    "description": (
        "List workflow candidates Physis learned from sessions that passed their "
        "verification command (auto_learn). Each candidate carries the request it "
        "answered, the exact shell steps, the edited files and the check. "
        "Candidates are stored in the plugin's own profile directory "
        "(~/.physis/replant/candidates), never in a repository. Nothing replays "
        "until the operator records it with /physis approve <id> - approval is "
        "not possible through this or any other registered tool."
    ),
    "parameters": {"type": "object", "properties": {}, "required": []},
}


TRACE = {
    "name": "physis_trace",
    "description": (
        "Port of ReasonTrace diagnostics: compute evidence coverage, answer "
        "grounding, unsupported claims, unexplained actions, repeated calls "
        "and loops for a reasoning trace JSON (ReasonTrace format)."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "trace": {"type": "string", "description": "ReasonTrace JSON (string or object)"},
            "path": {"type": "string", "description": "Path to a ReasonTrace JSON file"},
        },
        "required": [],
    },
}

ALL_SCHEMAS = [
    TRACE,
    ENGINE_STATUS,
    OBSERVE,
    SNAPSHOT,
    CONTEXT,
    QUERY,
    WHY,
    DIFF,
    CLASSIFY,
    CHAIN,
    GRID,
    INGEST,
    OBSERVED,
    REPLANT_STATS,
    REPLANT_DEMO,
    REPLANT_SHOW,
    RECIPE_LOOKUP,
    CANDIDATES,
]
ALL_NAMES = [s["name"] for s in ALL_SCHEMAS]
