# Packet renumber plan — physis-next

PLAN.md assigns P14–P18 to existing/future work:

- P14 Transplant semantic-delta/reasoning research  — complete
- P15 Minimal HUD                                     — complete
- P16 JEV/Hermes/OpenCode/model adapters               — in-progress
- P17 Additional applications (MCP server)             — in-progress
- P18 Wetware/FlyWire substrate research               — complete (docs)

The six packet files written earlier (`packets/P14*.md` … `P19*.md`) reuse
those IDs. Conflict must be resolved by renumbering to P20–P25.

## Renumbers (rename files + frontmatter `id:` + `depends:`)

| old file | new file | new id | keeps |
|---|---|---|---|
| P14-kernel-provenance.md            | P20-kernel-provenance.md           | P20 | depends: []                |
| P15-rowsource-sqlite.md             | P21-rowsource-sqlite.md            | P21 | depends: [P20]             |
| P16-watermark.md                    | P22-watermark.md                   | P22 | depends: [P21]             |
| P17-cache-stamps.md                 | P23-cache-stamps.md                | P23 | depends: [P21]             |
| P18-fk-relations.md                 | P24-fk-relations.md                | P24 | depends: [P21]             |
| P19-lfm-model-stack.md              | P25-lfm-model-stack.md             | P25 | depends: [P23]             |

## PLAN.md edits
Append six rows to the packet table, preserving order:

| P20 | Durable provenance struct in kernel        | open    |
| P21 | RowSource contract + SQLite adapter        | open    |
| P22 | Watermark change detection                 | open    |
| P23 | Per-layer content cache stamps in world     | open    |
| P24 | FK joins as kernel Relations               | open    |
| P25 | Adopt LFM + bge-small model stack          | open    |

Note: the roadmap table references P17 (MCP) still in-progress — that is the
PLAN.md P17, untouched. The new P23 cache-stamps is distinct.

## Verify
- `grep -rn '^id: P1' packets/` → empty
- `grep '^id:' packets/*.md | sort` → unique
- PLAN.md packet rows P20–P25 present and consistent
