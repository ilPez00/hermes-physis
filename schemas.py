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
    "description": "Run the replant memo path for an input and report tier, result and saved time.",
    "parameters": {
        "type": "object",
        "properties": {
            "input": {"type": "string", "description": "Input to replay or execute"},
            "path": {"type": "string", "description": "Working directory"},
        },
        "required": ["input"],
    },
}

REPLANT_STATS = {
    "name": "replant_stats",
    "description": "Report replant memo counts, effectiveness and speedup.",
    "parameters": {
        "type": "object",
        "properties": {"path": {"type": "string", "description": "Working directory"}},
        "required": [],
    },
}

REPLANT_DEMO = {
    "name": "replant_demo",
    "description": "Run the replant demo loop and return its benchmark report.",
    "parameters": {
        "type": "object",
        "properties": {"path": {"type": "string", "description": "Working directory"}},
        "required": [],
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
    REPLANT_STATS,
    REPLANT_DEMO,
]
ALL_NAMES = [s["name"] for s in ALL_SCHEMAS]
