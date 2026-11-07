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
        "changed. The index is global (one node graph); the path argument "
        "selects what gets scanned."
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

COMMAND = {
    "name": "physis_command",
    "description": (
        "Escape hatch: run any command advertised by a bundled engine "
        "(see physis_engine_status for the list). Use only when no dedicated "
        "tool covers the operation."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "command": {"type": "string", "description": "Engine command to run"},
            "args": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Command arguments",
            },
            "path": {"type": "string", "description": "Working directory"},
            "stdin": {"type": "string", "description": "Optional stdin payload"},
        },
        "required": ["command"],
    },
}

REPLANT_RUN = {
    "name": "replant_run",
    "description": (
        "Replay a verified tool recipe for an exact request, or return miss. "
        "The recipe's real actions and checks run again - no cached success is "
        "returned. Requires an operator-owned tools registry (tools=...)."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Request prompt to match"},
            "path": {"type": "string", "description": "Working directory"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON"},
            "store": {
                "type": "string",
                "description": "Replant state directory (default: <path>/.replant)",
            },
        },
        "required": ["input", "tools"],
    },
}

REPLANT_RECORD = {
    "name": "replant_record",
    "description": (
        "Record a verified recipe once: execute its actions and independent "
        "checks, then persist it for replay. Requires tools= and a recipe."
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
            "path": {"type": "string", "description": "Working directory"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON"},
            "store": {
                "type": "string",
                "description": "Replant state directory (default: <path>/.replant)",
            },
        },
        "required": ["input", "recipe", "tools"],
    },
}

REPLANT_STATS = {
    "name": "replant_stats",
    "description": "Report replant receipt counts: records, replays, misses, blocks and replay share.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory"},
            "store": {
                "type": "string",
                "description": "Replant state directory (default: <path>/.replant)",
            },
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
            "store": {
                "type": "string",
                "description": "Replant state directory (default: <path>/.replant)",
            },
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
            "store": {
                "type": "string",
                "description": "Replant state directory (default: <path>/.replant)",
            },
        },
        "required": ["input"],
    },
}

ALL_SCHEMAS = [
    ENGINE_STATUS,
    OBSERVE,
    SNAPSHOT,
    CONTEXT,
    QUERY,
    WHY,
    DIFF,
    CLASSIFY,
    CHAIN,
    COMMAND,
    REPLANT_RUN,
    REPLANT_RECORD,
    REPLANT_STATS,
    REPLANT_DEMO,
    REPLANT_SHOW,
]
ALL_NAMES = [s["name"] for s in ALL_SCHEMAS]
