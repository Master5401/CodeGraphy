# CodeWiki — Product Requirements (v5.0, polyglot with Java semantic priority)

Working title: CodeWiki (check for name collisions before any public release). Audience: Codex / coding agents and the human maintainer.

## 1. What we are building

A local-first, file-format-neutral codebase comprehension tool. The user runs one command on a folder or workspace. CodeWiki catalogs every eligible file, indexes readable text, applies format/language adapters where available, and opens a browser UI where the user can:

- Search files and readable text across the workspace; search methods, classes, and fields where a semantic adapter provides them.
- Find flows between two methods: call paths from A to B, through which methods, with which source lines.
- Open a method biography: deterministic, plain-English, evidence-linked explanation (inputs, operations, data movement, calls, returns), with no AI.
- See missing code honestly: calls into absent repositories become named external/unresolved symbols. Adding the repository later links them automatically.
- Read source beside every claim, and click graph nodes or edges to jump to source lines.
- Stay current after Git changes without rescanning everything.

Polyglot at the catalog and text-search layers from the first usable release. Java receives the first and deepest semantic support through the Atlas adapter. Other languages and structured/binary formats use the same evidence model and gain structural/semantic depth in staged adapters. Every UI and query must state the capability actually achieved for each file; cataloging a file does not imply that its code or contents were understood.

## 2. Non-negotiable principles

- Evidence before prose. Every fact and generated sentence carries a source location (file, line range, repo, revision) or is labelled `INFERRED`.
- Deterministic. Same source, engine versions, and config produce byte-identical facts and docs.
- Uncertainty is first-class. Never present guesses as facts or silently choose among candidates.
- No AI in the shipped product: no LLM calls or embeddings. AI tools may write the code; the product does not use them.
- Local and private. No telemetry or network calls during indexing/querying. Loopback-only server.
- Static analysis only on target code. Never execute target Maven/Gradle scripts, hooks, or code.
- Partial results beat failure. One bad file or engine failure must not fail the workspace; show diagnostics.

## 3. Users and primary workflow

Primary user: a developer investigating a ticket in an unfamiliar, undocumented codebase.

Core loop: `codewiki /path/to/workspace` → browse and search all discovered files → for Java, search `BillAPIHandler`, open its evidence-backed page, ask “flows from BillAPIHandler to InvoiceCreation,” inspect candidate paths, and click an edge to read the call site. Other file types appear with their supported text, structure, symbols, and relationships and explicit coverage states.

## 4. Scope and slices

| Slice | Goal | Done when |
|---|---|---|
| M0 | Scaffold and engine audit | Workspace builds; Atlas gap matrix on synthetic Java fixtures is written. **Complete.** |
| M1 | Polyglot file catalog + one-command local UI | `codewiki <workspace>` discovers eligible files, stores universal FileRecords in SQLite, indexes bounded readable text in FTS5, starts a loopback service and opens an explorer/search/source-preview UI. |
| M2 | Evidence model + Java Atlas adapter | Atlas Java output is normalized into CodeWiki-owned storage with evidence, status, diagnostics and capability metadata; Java declarations/references/calls are searchable. |
| M3 | Java call graph + bounded flows | A → B returns ranked, source-linked candidate paths and a bounded interactive call map. |
| M4 | Missing-code stubs + multi-repository linking | Adding a repository re-evaluates unresolved Java references; unique evidence-backed targets resolve, ambiguity and contract mismatches remain explicit, and affected repository views refresh from a shared workspace graph. |
| M5 | Java operation IR, data movement + biographies | Tested Java syntax maps to canonical operations; deterministic English templates produce evidence-linked biographies and data-flow views. |
| M6 | Incremental sync | Git/content-hash refresh handles add/edit/delete/rename and matches clean rebuild results on fixtures. |
| M7 | More language and file-format adapters | Prioritized adapters add syntax, structure or semantic facts without changing the canonical model; every capability is tested and surfaced. |
| M8 | Enola service/route adapter | Supported HTTP, messaging and service seams are linked with engine provenance when measured coverage needs justify the adapter. |
| M9 | Optional Joern adapter + release hardening | Joern is integrated only if measured gaps justify its runtime/build cost; packaged binaries, export and optional read-only MCP meet release checks. |

M1 catalogs and text-searches the full supported workspace without claiming semantic understanding. Java receives semantic priority in M2–M6. Other languages and formats are not excluded from the model; deeper support is staged in M7. Runtime tracing, LLM features, PDF export, desktop shell, cloud service, IDE plugin, and automatic code changes remain out of MVP scope.

**User-authorized proof gate (2026-11-12):** before starting the broader semantic implementation, run one synthetic Java fixture through the public Atlas facade and Joern. Capture real engine outputs, evidence-preserving normalized facts, operation IR, dataflow relationships, and one deterministic method biography. Keep the proof isolated under `proofs/engine-integration/`. Do not continue into the full implementation until the user reviews this proof. This isolated evaluation temporarily overrides Joern's later optional M9 timing; it does not change the engine source, add an unmeasured product dependency, or permit a replacement parser. Enola is not part of the first fixture proof.

## 5. Engine strategy (reuse, do not rebuild)

| Engine | Role | Integration | License (verify exact commit) |
|---|---|---|---|
| Atlas (`LordCasser/atlas`, Rust, tree-sitter + SQLite) | Primary Java semantic backend: symbols, references, call graph, dataflow/CFG | Rust library dependency on `atlas-engine` from maintainer-provided `vendor/atlas` (pinned submodule); disable TUI and MCP features; M2 onward | MIT |
| Enola (`enola-labs/enola`, Go) | Optional repository/service/route/message graph and cross-repo signals | Sidecar only after measured need; M8; exact artifact pinned/checksummed and user-consented if downloaded | Apache-2.0 |
| Joern (`joernio/joern`, Scala/JVM) | Optional deep CPG/dataflow for gaps Atlas cannot meet | M9 only if a measured fixture gap warrants its cost; never bundled by default; needs a JDK | Apache-2.0 |

Atlas is the LordCasser project, not another project named Atlas. It is best-effort rather than compiler-grade; external symbols not indexed may have no call edges. T1's measured limitations are recorded in the gap matrix. Forks live under the maintainer's GitHub account and are consumed as submodules pinned to a commit/tag (start at Atlas v1.5.2). Never edit `vendor/` in place. Patch on a branch inside the fork, upstream where possible, then deliberately move the submodule pointer. Long-lived divergence needs written justification from the T1 gap matrix.

Forking, submodule setup, and credentials are manual maintainer tasks. Codex works inside the CodeWiki repo. Initial `vendor/` contains only Atlas and Enola; do not add/clone Joern before M9 and unless the gap evidence justifies it. A sibling clone must never be referenced by relative path from product code. All engine access goes through `Backend`; no other code touches engine internals. Atlas T1 limitations are recorded in `docs/ATLAS_GAP_MATRIX.md`; never map its heuristic edges to `RESOLVED` based only on a confidence number.

## 6. One-command experience

Canonical commands: installed release `codewiki /path/to/workspace` or `codewiki` (project picker); source checkout command `./codewiki /path/to/workspace` or `.\codewiki.ps1 /path/to/workspace` (`just run /path/to/workspace` is an optional convenience). The source-checkout wrapper builds CodeWiki-owned components from pinned lockfiles and bootstraps web dependencies if missing; it reports prerequisites and network needs. Developer prerequisites are in README; end users of a packaged release need no language analysis toolchains.

`codewiki <path>` is idempotent. Verify OS, free disk, writable state dir, and print one-line status. Start server on `127.0.0.1` with a per-session random token on every API request; validate Host and Origin. Open browser immediately, without waiting for indexing. Discover repositories/projects/files automatically (section 9), then index in background with stages discovery, classification, text, structure, Java symbols, relationships, linking, ready. Allow browsing/search of indexed results while indexing continues. Re-running updates without duplicate/corrupt index. Capabilities and diagnostics are shown per file; M1 can report catalog/text capability before semantic indexing exists.

On failure, state failed stage, keep logs/partial results, offer retry, leave no orphan process or half-written index. Publish snapshots atomically and keep last good one.

Release target is a single binary with embedded web UI; Atlas linked in, no MVP sidecar. No Docker, JVM, Node, or separately installed database for a packaged release. Optional sidecars are acquired only after explicit consent and checksum verification; UI says what is fetched. UI works offline after install. State: per-workspace index at `<workspace>/.codewiki/` (suggest `.gitignore`, never auto-edit user files); app config in OS config dir.

## 7. Architecture

One Rust binary:

```text
codewiki
├── cli       clap launcher: run, index, sync, search, flows, status, doctor
├── server    axum HTTP API, embedded web assets (rust-embed), progress SSE
├── core      NEIR types, discovery, file capabilities, SQLite store, snapshots, resolution states, diagnostics
├── backends  Backend trait + AtlasBackend (library); Enola/Joern later
├── linker    workspace manifest, stubs, candidate matching, contract check
├── flow      graph build + path search
├── bio       operation IR → summaries → biography pages
├── adapters  universal classification; text, structured, binary and language adapters by capability
└── web/      Vite + React + TypeScript UI
```

Adapters ingest parser/engine output into CodeWiki's own SQLite `codewiki.db`. UI/query layers read only that database. Every eligible file gets a `FileRecord`, even when no parser supports it. Initial fields: stable file/workspace/repository IDs, relative path, display name, extension, detected format, optional media type/encoding, byte size, content hash, optional line count, binary flag, classification method, parser IDs, capability level, parse status, ignored reason/diagnostics, and source revision. An absent fact is left absent or `UNRESOLVED` with a reason; it is not guessed.

Capability levels: **L0** identified (metadata/hash); **L1** readable text indexed; **L2** syntax/structure parsed; **L3** entities/references extracted; **L4** semantic relationships/flows resolved; **L5** runtime-observed (optional and isolated from static facts). M1 targets L0 for every eligible file and bounded L1 for readable text. Later adapters add deeper levels.

```rust
trait Backend {
    fn capabilities(&self) -> Capabilities;
    fn index(&self, ws: &Workspace, jobs: &Jobs) -> Result<RunReport>;
    fn sync(&self, ws: &Workspace, changes: &ChangeSet) -> Result<RunReport>;
    fn export_facts(&self, snapshot: SnapshotId, sink: &mut dyn FactSink) -> Result<()>;
}
```

Every fact/edge keeps engine name/version, source revision, file/range, resolution status, confidence, and diagnostics.

Minimum core tables for M1–M4:

```sql
repositories(id, name, root_path, revision, available)
files(id, repo_id, path, language, content_hash, parse_status)
symbols(id, repo_id, file_id, kind, qualified_name, name, signature, params, return_type,
        range_start, range_end, is_external, source_fingerprint)
symbol_ext(symbol_id, level, candidates_json, inferred_params_json, inferred_return_json,
           repo_available)
callsites(id, caller_id, file_id, range, receiver_text, name, arity, arg_types_json)
call_edges(id, callsite_id, caller_id, callee_id, status, confidence, dispatch_kind,
           engine, provenance_json)
link_candidates(callsite_id, candidate_symbol_id, evidence_json, rank)
diagnostics(id, scope, kind, message, file_id?, range?)
analysis_runs(id, engine, engine_version, config_hash, revision, started, finished)
snapshots(id, revision, dirty, created, status)
-- FTS5: symbols_fts(name, qualified_name, signature), source_fts(text)
-- later: operations, data_nodes, dataflow_edges, cfg_*, biographies, summary_deps, flow_cache
```

Symbol IDs are deterministic hashes of repo, language, package, enclosing type, kind, name, and parameter types; never line numbers. Source ranges are separate.

## 8. Missing code, stubs and linking

Resolution states (use exactly these):

| State | Meaning | Rule |
|---|---|---|
| `RESOLVED` | One target, strong evidence | May appear in confirmed paths |
| `AMBIGUOUS` | Several plausible targets | List all; never pick |
| `HEURISTIC` | Weak name/package match | Hidden from confirmed paths by default |
| `EXTERNAL` | Known outside symbol, not indexed | Show package, signature, version if known |
| `UNRESOLVED` | Reference seen, target unknown | Keep call site and reason |
| `DYNAMIC` | Runtime receiver/config/reflection-dependent | Candidate set, labelled |
| `RUNTIME_OBSERVED` | Imported trace (later) | Separate provenance |
| `CONTRACT_MISMATCH` | Added provider conflicts with inferred call-site contract | Preserve both signatures and evidence; do not link as resolved |
| `DUPLICATE_PROVIDER` | Multiple repositories provide the same qualified identity | Preserve candidates; manifest precedence must be explicit |

Completeness is a separate scale: L0 receiver/class unknown; L1 several candidate classes; L2 one qualified name without declaration; L3 signature known without body; L4 body analysed.

For every reference whose target is not indexed, create a stub from call-site text. Name/arity are facts from the site. Parameter types come from argument static types (literal, declared local/parameter, `new T()`, known return); otherwise unknown. Return type comes from assignment target, typed return, typed receiving parameter, or boolean condition; otherwise unknown. Unify call sites; conflicts become overload candidates. Every inferred element is labelled `INFERRED` with evidence line; never present as verified.

Call resolution order (stop at first that applies): fully qualified name; variable/field/this/super receiver via declared type; type-name receiver (explicit import, same package, nested, `java.lang`; wildcard imports only where workspace contains class); unqualified call (current class, superclasses and note missing source, static imports); chained call (previous return type required, otherwise L0 reason `receiver-type-unknown`); overloads (argument count then types; retain ties).

When repository is added: match exact identity, then artifact coordinates/module, then signature/arity/compatible parameter types (widening allowed). Unique evidence-backed match → `RESOLVED`; multiple → `AMBIGUOUS`; disagreeing signature → `CONTRACT_MISMATCH` with both signatures and call-site line. Same method name in two repos is not a clash because identity includes repository/package. Same FQN in two repos → `DUPLICATE_PROVIDER`; only explicit manifest precedence can break tie. Keep one canonical workspace graph and regenerate affected pages for both participating repositories from it, including reciprocal links where evidence supports them. Do not modify source documentation files as a side effect.

Optional `codewiki.yaml`:

```yaml
workspace: billing-platform
repositories:
  - { name: billing-core, path: ./billing-core }
  - { name: shared-lib, packages: [com.acme.shared], coordinates: "com.acme:shared:1.4.0" }
precedence: [billing-core, shared-lib]
ignore: ["**/build/**", "**/target/**", "**/generated/**"]
```

## 9. Discovery

Automatic, no manual file lists. M1 discovers and catalogs eligible files whether or not a language/format adapter exists. Use Git roots (including nested repositories, submodules and worktrees), ignore rules, bounded traversal, symlink policy, file-size limits, extension/MIME hints, magic bytes and manifest context. For Java ownership use Maven/Gradle files, `src/main/java`, `src/test/java`, package declarations, `module-info.java`, imports and dependency coordinates; loose Java belongs to a synthetic loose-source project with low-confidence ownership. Other project manifests become hints as adapters are added. Build files are read only as text and never run. Optional `codewiki.yaml` can name a workspace/repositories and override ignore rules.

### 9.1 File-format coverage plan

Every eligible file receives L0 metadata in M1; bounded, decodable text receives L1 indexing. The following are adapter targets, not a claim that all formats are structurally understood in M1:

| Category | Examples | Target facts when adapter exists |
|---|---|---|
| Source | Java (priority), C/C++, C#, Go, Rust, Python, JavaScript/TypeScript, Kotlin, PHP, Ruby, Swift, Scala | Syntax nodes, declarations, references, operations, source ranges; semantic resolution only when evidence/provider supports it. |
| Configuration | XML, YAML, JSON, TOML, INI, `.properties`, `.env` | Document hierarchy, key/path, values, anchors and explicit references. |
| Build/project manifests | Maven, Gradle, npm/pnpm, Cargo, Go modules, Python, CMake, Make, MSBuild | Project/module/target and declared dependencies. Read as data; never execute scripts/tasks. |
| JVM artifacts | `.class`, `.jar`, `.war` | Class/method/field descriptors, constant-pool and bytecode metadata, archive membership, with strict archive limits. |
| SQL | `.sql`, migrations | Statements, tables, columns, constraints, reads/writes and explicit schema references. |
| API/schema | OpenAPI/Swagger, GraphQL, Protobuf, Avro | Services, operations, endpoints, messages, fields, schemas and references. |
| Documentation | Markdown, RST, AsciiDoc, TXT, HTML, source comments | Text, headings, links, anchors and code fences; symbol links only from explicit evidence or labelled heuristics. |
| Tabular data | CSV, TSV, JSONL | Headers, records, bounded samples, and inferred types clearly labelled as inferred. |
| Native/unknown binaries | PE, ELF, Mach-O, object files, unknown data | Format, sections, imports/exports/symbols where available, offsets and hash; unknown files stay cataloged. |
| Assets | Images, fonts, audio/video | Format and safe metadata such as dimensions/duration; no OCR/transcription by default. |

For each adapter, publish its parser/version, supported fact kinds, size limits, diagnostics, fixture coverage, and capability level. Extensions are hints, not proof of file type.

## 10. Flow search between two methods

Inputs: two endpoints (names, partial names, or selected symbols), kind (call default; data, control, service, mixed later), limits.

Endpoint resolution: exact qualified name, exact simple name, prefix, FTS, fuzzy. Multiple matches return candidates and ask; never choose silently. Deterministic query grammar (show interpreted query): `A -> B` / “flows from A to B” means paths; “callers of X” / “who calls X” means callers; “callees of X” / “what does X call” means callees; “reachable from A” means forward reachability; “impact of X” means callers and data dependents; “where does v originate” means data origin (M5+).

M3 algorithm: build in-memory adjacency (CSR) from `call_edges` for selected snapshot; default filter `RESOLVED` plus `EXTERNAL` leaves, candidate/heuristic edges only when requested. Find shortest path with bidirectional BFS, enumerate additional simple paths (Yen-style) up to `max_paths` and `max_depth`, track visited nodes for cycles, prune using reverse reachability from target. Never enumerate unbounded paths.

Rank fewer non-resolved edges, shorter length, fewer framework hops, then stable alphabetical order. Each path edge shows call site (file/line), dispatch kind, resolution status, confidence, engine provenance. Result shows endpoints, paths, omitted count (“at least N” if exact count expensive), coverage (parsed files, unresolved/ambiguous edges, unsupported constructs), and separates “no indexed path” from “not analyzed/unsupported”.

Defaults: `max_depth=12`, `max_paths=50`, `include_external=true`, `include_tests=false`. Static paths are possible paths, never guaranteed runtime behaviour. Performance targets (declared benchmark only, warm, p95): symbol search ≤300 ms, open definition ≤200 ms, benchmark flow query ≤1 s. Publish measured values; do not claim universal numbers.

## 11. Java operation IR and method biographies (M5)

Pipeline: Java syntax/body → canonical operation IR → local summary → link callees → compose caller summary with one-level callee gist → render. Collapse recursion groups. Stubs say “source not available”. Later language adapters map their syntax into the same IR and declare unsupported constructs. Preserve language-specific distinctions such as Rust move/borrow, Java virtual dispatch, Go channels, dynamic-language attribute calls, and null/optional behavior; do not erase semantics to force a common prose claim.

Operation IR kinds: parameter/local/field/property, read/write, literal, allocation/construction, assignment/alias, arithmetic, comparison/logic, cast/conversion, collection operation (put/get/add/remove/contains/iterate/sort/filter/map/reduce), object operation, call/dispatch, branch, loop, break/continue, return, throw/catch/finally, await/yield, synchronization, and boundary (HTTP, database, file, serialization, message). Common data shapes include scalar, sequence, mapping, set, tuple/record, object/struct, pointer/reference/alias, optional/union/result, stream/channel, and external resource; preserve each source-language type and semantics alongside any normalized shape. Data-flow edges include `READS`, `WRITES`, `FLOWS_TO`, `ARG_TO_PARAM`, `RETURN_TO_CALL`, `ALIAS_OF`, `MUTATES`, `CONTROL_DEPENDS_ON`, `THROWS_TO`, and `CAUGHT_BY`.

Each language adapter must version a syntax-to-atom mapping and define deterministic English templates, source evidence mapping, unsupported syntax, resolution behavior, parser/grammar version and golden fixtures. The template vocabulary is shared; language-specific facts (for example Rust moves/borrows, Java virtual dispatch, Go channels, Python dynamic attributes) remain distinct. Java mappings and templates are implemented/tested first. No adapter may emit a sentence without source evidence and an honest resolution/derivation state.

Fixed sentence templates, using words only from operators, types, control flow, and identifiers as written:

| IR pattern | Template | Label |
|---|---|---|
| parameters | Takes n values: types and names. | FACT |
| `T v = f(args)` | Calls f with args and stores the result in v. | FACT |
| `v = a * b` | Multiplies a by b and stores it in v. | FACT |
| map built in loop | Builds a map from the collection: each element becomes key K and value V. | FACT/DERIVED |
| `if (c) A else B` | If c: A. Otherwise: B. | FACT |
| `for (x : coll)` | For each x in coll: inner steps. | FACT |
| analysed callee | Calls g, which gist-of-g. | DERIVED |
| stub call | Calls Class.m (source not available). Expected contract: signature. | INFERRED |

Rules: gist is parameters + at most two dominant actions + return, one sentence; about 8 sentences max per method; expressions truncate at 40 chars; never claim business intent unless operations show it; separate direct facts from callee-derived facts; every sentence expands to source lines; changing callee gist hash marks callers dirty, unchanged hash stops propagation.

Page sections: identity, existing Javadoc (`DECLARED`), summary, ordered steps, data movement, calls with resolution state, used methods (observed signatures), mini call tree, callers, fields, exceptions, unknowns, evidence, coverage.

## 12. Incremental sync (M6)

Detect changes by Git diff plus content hashes (dirty tree and non-Git folders; renames and deletes). Track build-file changes separately because they may broadly affect resolution. Invalidate: reparse changed file, replace facts, re-resolve its references and references to changed/deleted symbols, rebuild affected edges, dirty dependent biographies and flow cache, regenerate affected pages. Publish snapshots atomically; keep last good one. Test incremental output equals a clean full rebuild on every fixture.

## 13. UI requirements (MVP screens only)

Layout: left rail (workspace switcher, repo/package/file tree, global search), center (overview/source/results/graph), collapsible inspector (symbol/edge details, evidence, status, coverage). The explorer tree uses the exact paths relative to each repository, with deterministic directory/file ordering and lazy expansion for large trees.

Generated Markdown must mirror each repository's relative source layout under `.codewiki/docs/<repository-name>/`. A source file such as `src/main/java/org/example/OrderService.java` maps to `.codewiki/docs/<repository-name>/src/main/java/org/example/OrderService.java.md`. Each page keeps source identity and evidence links. Multi-repository pages include reciprocal repository/symbol links when resolution is unique; linking updates generated pages only and never rewrites analyzed source files or existing documentation.

Screens: project picker + progress; search with disambiguation; file/source viewer; method page (source + biography + callers/callees); flow results + call map; diagnostics.

Stack: React + TypeScript + Vite; react-router; TanStack Query; Cytoscape.js (dagre or elk); CodeMirror 6 read-only syntax highlighting for registered languages and line anchors; typed API client generated from OpenAPI (`utoipa` → `openapi-typescript`). M1 needs only the simplest responsive explorer/search/source preview and may defer advanced graph dependencies.

Click node → definition; click edge → call site and resolution explanation. Graph uses bounded neighborhoods with expand/collapse, depth/path limits, edge-status/repo filters, path highlight, truncation notices, and equivalent text alternative. Edge styles distinguish all resolution states. Every screen has loading, explanatory empty, partial, stale, error, retry states. Deep links for repo, file, symbol, line range, graph query; back/forward; global-search shortcut; visible focus; non-colour status cues; reduced-motion support. Virtualize large lists; never freeze main thread on graph expansion.

## 14. Security and privacy

Bind only to `127.0.0.1`; per-session token; check Host/Origin; no wildcard CORS. Source API accepts only files inside indexed roots and rejects traversal/symlink escapes. Escape source text in HTML. No telemetry/network in index/query code (test with networking disabled where feasible). Exports warn source may be proprietary. Never log source contents.

## 15. Hard rules for Codex / coding agents

Read this PRD and `AGENTS.md` before any task. If requirements conflict, stop and ask; do not improvise. Work in small vertical slices, one slice per PR/branch; do not start later milestones. Never edit `vendor/`; engine changes only in maintainer's fork on a branch, by explicit instruction. No LLM/embedding calls in product code. No network calls in indexing/query paths. Never execute target-project build scripts or code. No proprietary/employer code, names, screenshots, or data in repo, fixtures, prompts, or test data; use synthetic/public code only.

Every task: add/update tests; run `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and for `web/` the lint and test scripts; all pass. Every fact/edge keeps evidence and resolution state. If unknown, record `UNRESOLVED` with reason. Determinism: no timestamps, random IDs, or unordered iteration in generated output; sort explicitly. Document assumptions in `docs/DECISIONS.md` with date/reason. Track licenses in `THIRD_PARTY_NOTICES.md` (use cargo-deny/cargo-about), preserve upstream notices.

## 16. Task sequence for Codex (give one at a time)

The product follows a polyglot catalog foundation and Java-first semantic depth. M0/T0/T1 are complete. Complete one milestone before starting the next.

- **M1 — Polyglot catalog and one-command UI.** Implement deterministic bounded discovery and classification; write L0 FileRecords for every eligible file and L1 FTS5 documents for bounded readable text; persist in workspace SQLite; provide a local explorer whose directory tree mirrors repository paths, search and safe source preview; start the loopback server and browser from `./codewiki <workspace>` / `.\codewiki.ps1 <workspace>` (developer checkout) and `codewiki <workspace>` (release); show progress and file capability/diagnostics. Use synthetic mixed-language/mixed-format fixtures. Do not claim semantic understanding, symbol search, graphs or biographies in M1.
- **M2 — Evidence model and Java Atlas adapter.** Add stable NEIR IDs, schema migrations/snapshots, `Backend` contract, and normalize Atlas Java files/symbols/callsites/edges/imports/diagnostics into CodeWiki storage. Respect each measured Atlas gap: preserve unresolved sites, never promote heuristic edges to resolved, and diagnose dropped/ambiguous facts. Adapter fixture tests must cite source evidence. Maintainer must provide `vendor/atlas` before the engine integration can be completed; sibling clones are not product dependencies.
- **M3 — Java call graph and bounded flows.** Implement Section 10's endpoint disambiguation, path limits/ranking, API, path table and graph with evidence-to-source navigation. Include coverage and unresolved diagnostics.
- **M4 — Missing-code registry and multi-repository linker.** Implement Section 8 and workspace manifest. Adding a repository updates the shared index, links only unique evidence-backed matches, keeps ambiguity and `CONTRACT_MISMATCH`, and regenerates affected repo/symbol views with cross-repo links/backlinks. Do not modify analyzed source files.
- **M5 — Java operations, data movement and biographies.** Implement the canonical operation/data-shape vocabulary, Java syntax-to-atom mapping, fixed English templates, evidence expansion, callee gist folding and golden tests. Mark unsupported constructs and all inferred contract elements.
- **M6 — Incremental refresh.** Implement Git/content-hash refresh for add/edit/delete/rename, dependency invalidation and atomic snapshots. Prove normalized incremental output equals a clean rebuild on fixtures.
- **M7 — Language and file-format adapter expansion.** Prioritize adapters by user value and measured coverage. For each, publish grammar/parser version, native syntax-to-canonical-atom mapping, structures/operations/templates, unsupported cases, provenance, diagnostics and fixtures. Initial targets are the source-language and structured/build/document/schema/data/binary categories in Section 9.1. Polyglot catalog remains available for all unsupported formats.
- **M8 — Enola adapter.** Only if M4/M7 measurements justify it, add evidence-backed HTTP/RPC/GraphQL/message/service seams with exact engine provenance. External artifact acquisition requires pinned checksums and consent.
- **M9 — Optional Joern and release hardening.** Integrate Joern only for measured Java/other-language gaps that justify its Scala/JVM dependency. Finish packaged binaries, offline behavior, license audit, static export and optional read-only MCP as separately bounded tasks.

Never work around missing maintainer engine setup by referencing root-level clones from product code or editing `vendor/`.

## 17. Fixtures and acceptance tests

Fixtures live in `fixtures/` and are synthetic. Required sets: overloads/inheritance; same method name in two repos; two wildcard imports with missing class; fully qualified call with no source; chesstour repo B calling missing repos A/C; later-added repo with differing signature; chained call on unknown stub; arrays/lists/maps/loops/lambdas; malformed file; generated sources; small multi-module Maven and multi-repo workspaces; HTTP route pair (M6).

| ID | Scenario | Pass condition |
|---|---|---|
| AC-00 | Mixed polyglot/file-format fixture | Every eligible source, document, config, manifest, data, binary and unknown file has L0 metadata; bounded readable text reaches L1 and is searchable; no target code/build runs |
| AC-01 | `codewiki <fixture>` on a supported checkout/release | Loopback UI opens, indexing progress is visible, file search/source preview work, no container or analysis toolchain is required for M1 |
| AC-02 | Re-run | Idempotent, no duplicate/corrupt index |
| AC-03 | Same-named overloads/packages | Separate candidates and disambiguation |
| AC-04 | Known direct call | Source-linked directed edge with correct evidence |
| AC-05 | A → B | Candidate paths, per-edge evidence, caps, labelled truncation |
| AC-06 | Ambiguous endpoint | Candidates shown; none silently chosen |
| S14 | Explicit import of missing class | L2 stub with inferred contract and evidence lines |
| S15 | Two wildcard imports, missing class | L1, both candidates, none chosen |
| S16 | Fully qualified call, no source | L2, exact name |
| S17 | Same method name in two repos, explicit import | Correct bind; other untouched |
| S18 | Add missing repo | Stub links; only dependents regenerate |
| S19 | Added repo has different signature | `CONTRACT_MISMATCH` |
| S20 | Chained call on unknown return | L0, reason `receiver-type-unknown` |
| AC-07 | Biography golden tests | Loop+map, multiplication, branch, nested gist, recursion, stub match exactly |
| AC-08 | Same input twice | Byte-identical output |
| AC-09 | Edit then sync | Only affected facts/pages change; equals clean rebuild |
| AC-10 | Broken syntax in one file | Rest indexed; diagnostic shown |
| AC-11 | Security | Loopback, token required, traversal rejected |
| AC-12 | Offline | Index/search/flows/biographies work without network |
| AC-13 | Capability honesty | Unsupported formats remain visible with L0/L1 status; UI never implies symbols/semantics for catalog-only files |

Publish per-fixture/benchmark metrics: parse coverage, symbol recall, call-edge precision/recall (verified by hand, never tool output), unresolved rate, path precision, incremental correctness, query latency p50/p95, time to first useful result, setup success by OS.

## 18. Known risks and handling

| Risk | Handling |
|---|---|
| Atlas weak on missing imports/wildcards/external symbols | M0 gap matrix, stub/linker, diagnostics |
| Atlas API changes | Pin submodule; wrap behind Backend; no engine types beyond backends/ |
| Static paths mistaken for runtime behaviour | Label possible paths; show coverage and unresolved counts |
| Biography implies business intent | Fixed templates, optional name hints off by default |
| Incremental bugs | Equivalence tests against clean rebuild on every fixture |
| Fork maintenance | Submodule pins; patch only measured gaps |
| License obligations | Notices; cargo-deny/cargo-about; check bundled/transitive licenses before release |
| Confidential code leakage | Synthetic fixtures only |

## 19. Open decisions (defaults)

- Atlas primary for Java semantic work through M6: yes; apply the T1 gap matrix and revisit only with fixture evidence.
- Copy facts into own `codewiki.db`, rather than reading Atlas DB directly: yes.
- Joern only if gap matrix proves need: yes.
- Maven and Gradle both read as text; neither is executed.
- Name remains working title CodeWiki; check collisions before public release.

## Appendix A — AGENTS.md

The root `AGENTS.md` is required. It points agents to this PRD first, enforces milestone order, prohibits vendor edits and product AI/network calls, restricts fixtures to synthetic/public code, requires the quality commands above, deterministic output, decisions documentation, evidence/status on facts and edges, and stopping when requirements conflict.

## Appendix B — Repository layout

```text
codewiki/
├── AGENTS.md
├── README.md
├── justfile
├── rust-toolchain.toml
├── Cargo.toml
├── crates/{cli,server,core,backends,linker,flow,bio}/
├── web/
├── fixtures/java/...
├── docs/{PRD.md,DECISIONS.md,ATLAS_GAP_MATRIX.md}
├── vendor/{atlas,enola}/
└── THIRD_PARTY_NOTICES.md
```
