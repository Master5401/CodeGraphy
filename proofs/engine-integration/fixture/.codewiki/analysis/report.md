# CodeGraphy scan

Provider: Atlas 1.7.0 (`0f8379ff61a6f5ea1ae888d331f21b8b44f6a5b7`), Java structural profile.

Discovered 1 Java files; re-extracted 0; reused cached analysis for 1; 0 failed. Extracted 5 symbols and 2 relationships, including 0 resolved internal calls, 1 heuristic call candidates, and 0 unresolved callsites.

## Output files

- `facts.json`: evidence-bearing normalized facts plus raw Atlas records.
- `call-graph.dot`: solid resolved edges and dashed heuristic candidates.
- `files/`: one source-shaped facts page per analyzed Java file.

The call graph draws resolved edges as solid lines and heuristic candidates as dashed lines. Unresolved references remain in `facts.json` with evidence and a reason; a missing graph edge does not mean the source contains no call.
