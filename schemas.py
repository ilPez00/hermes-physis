"""Tool schemas exposed to the LLM."""

ENGINE_STATUS = {
    "name": "physis_engine_status",
    "description": (
        "Report which lightweight Physis engine binary is in use and which "
        "commands it supports. Use this before relying on other physis tools."
    ),
    "parameters": {"type": "object", "properties": {}, "required": []},
}

OBSERVE = {
    "name": "physis_observe",
    "description": (
        "Scan a directory into the Physis observation log (incremental). "
        "Use before context/why/diff queries on a repo that may have changed."
    ),
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
    "description": (
        "Compile a bounded, receipt-carrying context packet for a task or path. "
        "Use to gather relevant code/evidence without pasting whole repos."
    ),
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
    "description": "Ask a structural question about the observed codebase, e.g. dependencies or callers.",
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
    "description": "Show the causal chain for an object from the observation log, with evidence and explicit gaps.",
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

ALL_SCHEMAS = [ENGINE_STATUS, OBSERVE, CONTEXT, QUERY, WHY, DIFF]
ALL_NAMES = [s["name"] for s in ALL_SCHEMAS]
