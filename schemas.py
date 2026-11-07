"""Tool schemas exposed to the LLM."""

ENGINE_STATUS = {
    "name": "physis_engine_status",
    "description": "Report available Physis engines and their supported commands.",
    "parameters": {"type": "object", "properties": {}, "required": []},
}

OBSERVE = {
    "name": "physis_observe",
    "description": "Scan a directory into the Physis observation log (incremental).",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Directory to observe"},
            "max_files": {"type": "integer", "description": "Maximum files to scan"},
            "doc_comments": {"type": "boolean", "description": "Derive document primitives from doc comments"},
        },
        "required": [],
    },
}

CONTEXT = {
    "name": "physis_context",
    "description": "Compile a bounded, receipt-carrying context packet for a task or path.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Repository root"},
            "task": {"type": "string", "description": "Natural-language task/question"},
            "budget": {"type": "integer", "description": "Token budget (100-4000)"},
        },
        "required": [],
    },
}

QUERY = {
    "name": "physis_query",
    "description": "Ask a structural question about the observed codebase.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Repository root"},
            "question": {"type": "string", "description": "Structural question"},
            "limit": {"type": "integer", "description": "Maximum results"},
        },
        "required": ["question"],
    },
}

WHY = {
    "name": "physis_why",
    "description": "Show the causal chain for an object from the observation log.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Repository root"},
            "object": {"type": "string", "description": "Object id or name"},
        },
        "required": ["object"],
    },
}

DIFF = {
    "name": "physis_diff",
    "description": "Structural diff between two git refs in an observed repository.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Repository root"},
            "from_ref": {"type": "string", "description": "Start ref"},
            "to_ref": {"type": "string", "description": "End ref"},
        },
        "required": [],
    },
}

COMMAND = {
    "name": "physis_command",
    "description": "Run any command advertised by an available Physis engine (physis-core or physis-replant).",
    "parameters": {
        "type": "object",
        "properties": {
            "command": {"type": "string", "description": "Engine command to run"},
            "args": {"type": "array", "items": {"type": "string"}, "description": "Command arguments"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["command"],
    },
}

REPLANT_RUN = {
    "name": "replant_run",
    "description": "Run replant: replay a verified recipe or return miss. Requires a tools registry (--tools) and store dir.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Input to replay or execute"},
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON (required for run/record)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": ["input", "tools"],
    },
}

REPLANT_RECORD = {
    "name": "replant_record",
    "description": "Record a verified replant recipe: execute actions + checks, then persist for replay. Requires --tools and --store.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Input / prompt for the request"},
            "recipe": {"type": "string", "description": "JSON recipe: {steps: [{tool, input}], checks: [{tool, input}]}"},
            "trace": {"type": "string", "description": "JSON trace metadata (optional)"},
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON (required)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": ["input", "recipe", "tools"],
    },
}

REPLANT_STATS = {
    "name": "replant_stats",
    "description": "Report replant memo counts, effectiveness and speedup.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": [],
    },
}

REPLANT_DEMO = {
    "name": "replant_demo",
    "description": "Run the replant record→replay loop via the binary and return its report.",
    "parameters": {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON (required)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": ["tools"],
    },
}

REPLANT_RECORD = {
    "name": "replant_record",
    "description": "Record a verified replant recipe: execute actions + checks, then persist for replay.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Input / prompt for the request"},
            "recipe": {"type": "string", "description": "JSON recipe: {steps: [{tool, input}], checks: [{tool, input}]}"},
            "trace": {"type": "string", "description": "JSON trace metadata (optional)"},
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "tools": {"type": "string", "description": "Path to replant tools registry JSON (required)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": ["input", "recipe", "tools"],
    },
}

REPLANT_SHOW = {
    "name": "replant_show",
    "description": "Show a learned replant record for an input.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Input to look up"},
            "path": {"type": "string", "description": "Working directory (store lives here under .physis/replant)"},
            "store": {"type": "string", "description": "Replant state directory (default: <path>/.physis/replant)"},
        },
        "required": ["input"],
    },
}

ALL_SCHEMAS = [
    ENGINE_STATUS,
    OBSERVE,
    CONTEXT,
    QUERY,
    WHY,
    DIFF,
    COMMAND,
    REPLANT_RUN,
    REPLANT_RECORD,
    REPLANT_STATS,
    REPLANT_DEMO,
    REPLANT_SHOW,
]
ALL_NAMES = [s["name"] for s in ALL_SCHEMAS]
