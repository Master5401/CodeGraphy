# CodeGraphy — Project Mission & Agent Instructions

**Canonical repository instruction file.** This is the single source of truth for CodeGraphy's product intent, architecture, semantic model, user experience, implementation order and engineering rules. Keep it at the repository root as `AGENTS.md`. Do not ask the agent to reconcile separate PRDs, build specifications or architecture documents. Older CodeWiki/CodeGraphy documents are historical and do not override this file.

## 0. Agent rules — read before every task

1. Audit actual repository/Git state and nested instructions before editing; never assume handoff status is current or discard user changes.
2. Preserve working catalog, SQLite, API, CLI, launchers and UI unless tests justify a change.
3. Reuse Atlas, Joern, Enola and mature format parsers; do not recreate language parsers, compilers, symbol resolvers, CFG or dataflow engines they already provide.
4. Polyglot/file-neutral from the foundation. Catalog every eligible file; report semantic depth per capability.
5. User-facing explanations are natural English. `READ`, `BINARY_OP`, `CALL`, `ASSIGN`, provider IDs and `v1` are internal only.
6. Every factual claim/edge/pattern needs evidence. Never infer behavior from names or invent unresolved callees.
7. Atlas baseline; Joern targeted deep analysis; Enola architecture analysis. Do not run all providers over all files.
8. No fake facts, canned biographies, regex parsers or silent fallback when provider integration fails. Report exact blockers.
9. Implement one tested vertical slice at a time; keep the app runnable.
10. Cache unchanged work, reanalyze affected scope only, bound CPU/RAM/time, keep UI usable during indexing.
11. Never execute target code/build scripts during normal indexing. Enforce file, archive and symlink safety limits.
12. Report `IMPLEMENTED`, `PARTIAL`, `STUBBED`, `MOCKED`, `UNSUPPORTED` or `UNVERIFIED` with evidence.

For each task, identify the relevant sections, trace actual code paths, implement the smallest end-to-end slice, run relevant tests, then report changed files, commands, results, fixture evidence, gaps, performance and launcher status. An unfamiliar checkout starts with the read-only audit in §15.

## 1. Product mission

**CodeGraphy is a local-first, polyglot codebase-comprehension platform.** It turns a workspace and its supporting artifacts into an interactive, searchable knowledge base that explains what files define, what their contents do, how data is represented and transformed, how control and values flow, how components depend on one another, and how the complete system fits together.

A user points CodeGraphy at a workspace and gets:

- a universal file and repository catalog;
- source and structured-content browsing;
- searchable definitions for code, configuration, schemas, SQL, bytecode and documents;
- human-readable explanations of functions, methods, classes, modules and files;
- call, data-flow, control-flow and architecture maps;
- paths and relationships across files and repositories;
- explicit evidence, coverage, uncertainty and unresolved references.

Its differentiator is a **unified evidence-backed semantic and explanation layer** over existing analysis providers.

### Product principles

- Local-first/offline core; universal discovery; deeper understanding through specialist adapters.
- Natural English first; technical IR is optional.
- Static facts, inferred patterns, documented contracts and runtime observations stay distinct.
- Direct and transitive behavior stay separate; unresolved/ambiguous/dynamic/external references remain visible.
- Browse/search while background analysis continues.

## 2. Current implementation handoff

Current implementation includes the catalog/UI described above plus a real Atlas Java structural scan. The `codegraphy init <workspace>` command invokes the pinned `atlas-engine`, normalizes and persists facts with source evidence, writes JSON/Markdown/DOT reports, and exposes saved call facts in the UI. Its current Atlas profile extracts Java structural facts and heuristic call candidates; it does not yet import CFG/dataflow into CodeGraphy's model or generate semantic operation atoms/English biographies. Joern and Enola are not product integrations, and multi-repository binding is not implemented.

Follow actual code paths and tests. A proof harness alone does not mean an engine is integrated. An engine counts as integrated only when CodeGraphy invokes it on a real fixture, receives real facts, normalizes and persists them, queries them, and presents them with provenance. Update this handoff as those capabilities change.

## 3. Architecture and technology direction

### Product architecture

```text
User → React UI → local Axum API → CodeGraphy knowledge/query layer
                                      ├─ canonical semantic model
                                      ├─ identity resolver / cross-repo linker
                                      ├─ biography and pattern composer
                                      ├─ graph/path query engine
                                      └─ analysis scheduler
                                           ├─ Atlas (baseline code analysis)
                                           ├─ Joern (targeted deep analysis)
                                           ├─ Enola (architecture graph)
                                           └─ format-specific adapters
                                      ↓
                                SQLite + FTS5 + evidence/cache
```

### Recommended stack

**Stack:** Rust 2024 core/CLI/adapters; Tokio + Axum local API; Rayon with limits; `ignore` for Git-aware discovery; `notify` for change events; BLAKE3 for fingerprints; Atlas/Joern/Enola as providers; established parsers for structured/binary formats; SQLite + FTS5; React + TypeScript + Vite; TanStack Query + Zustand; CodeMirror 6; Sigma.js + Graphology. Develop in IntelliJ IDEA with the Rust plugin.

Pin toolchain versions, engine commits/releases and adapter versions. Verify exact APIs against pinned upstream source; never invent crate names, commands, schemas or capabilities.

## 4. Analysis-engine policy

**Atlas — baseline provider.** Use its public Rust `atlas-engine` facade where stable; otherwise use a documented CLI/MCP/structured-output adapter. Reuse supported multi-language extraction, symbols/scopes/imports/references/calls, local bindings, dataflow/CFG, graph queries and incremental sync. Validate exact capabilities in the pinned version. Test known gaps: Java classpath/Maven/Gradle resolution, best-effort semantics, C/C++ preprocessing, and external targets not indexed.

**Joern — targeted deep provider.** Use when CPG/AST, CFG, control/data dependence, slices or supported taint/dataflow add value. Validate especially Java and C/C++; treat JVM bytecode as a separate capability. Run isolated, narrowly scoped and resource-bounded.

**Enola — architecture provider.** Use for repository/module/service/dependency facts, routes/handlers, HTTP, messaging topics, gRPC/GraphQL, storage and architecture links. Import documented graph/snapshot/receipt records. It does not replace statement-level CPG or method-level dataflow.

**Selection:** Atlas baseline for supported source code; Joern only for requested/high-value deep analysis; Enola for architecture or multi-repo views; dedicated adapters for other formats and gaps. Preserve raw provider facts/IDs and normalized facts. Do not equate symbols by name alone. Provider failure must not erase other providers' successful facts. Pin exact commits/releases and inspect actual APIs; never invent commands, crates, schemas or capability claims.

## 5. Execution profiles, scheduling and performance

### Profiles

- **Quick:** discover/classify/catalog files, hash them, index safe text, expose tree/search/source preview. Do not require Joern or Enola.
- **Standard:** run Atlas baseline where supported plus inexpensive structured-format adapters; extract symbols, definitions and available relationships; generate only explanations supported by available facts.
- **Deep:** run targeted Joern analysis for selected scope/query and cache results.
- **Architecture:** run Enola for architecture/multi-repo views and relevant changes.
- **Full analysis:** explicit opt-in to all applicable providers for a selected scope, never the default.

### Scheduler and caching

Cache key must include content hash, provider/version, adapter version, effective configuration, dependency/classpath context and analysis profile. Reuse unchanged results. Changed files invalidate their derived facts plus the affected relationship and summary closure, not the entire workspace. Use reverse dependencies/impact closure with bounded expansion. File-watcher events are hints; reconcile hashes periodically. Debounce bursts. Jobs need queued/running/partial/complete/failed/stale states, cancellation, retry and per-provider concurrency/RAM/time limits. Joern must not monopolize the machine. Enola relinking may have a wider scope; measure it rather than assuming per-file incremental behavior.

### Performance measurement

Do not invent speed claims. Benchmark first usable UI, discovery/catalog, baseline analysis by language/format, search, definition open, bounded path query, one-file incremental update, Joern time/peak RAM, Enola multi-repo linking, and large-graph render/interaction. Report p50/p95 where possible. Use paging, node/depth limits, progressive graph expansion and virtualized source/tree views. Never load a whole large repository/graph into browser memory.

## 6. Universal file and format contract

Every eligible file must be discovered and catalogued even if no parser understands it. Extension is a hint, not proof: combine extension, content signature, project context and safe content sniffing. Unknown files remain visible with an explicit unknown/binary classification.

- **Source:** Java, C/C++, C#, Go, Rust, Python, JS/TS, Kotlin, PHP, Ruby, Swift, Scala — declarations, scopes, types, calls, operations and supported flows.
- **Configuration:** XML, YAML, JSON, TOML, INI, `.properties`, `.env` — hierarchy, keys, values, defaults, anchors/aliases and references.
- **Build/project:** Maven, Gradle, npm/pnpm, Cargo, Go modules, Python packaging, CMake, Make, MSBuild — modules, tasks, dependencies and source roots; do not execute scripts by default.
- **JVM:** `.class`, `.jar`, `.war` — class/method/field metadata, descriptors, annotations, constant pool, bytecode and archive members.
- **Data/API:** SQL, OpenAPI, GraphQL, Protobuf, Avro, CSV, TSV, JSONL — statements, tables/columns, endpoints, messages, schemas and bounded data shape.
- **Documents/assets/binaries:** Markdown, RST, AsciiDoc, TXT, HTML, PE/ELF/Mach-O/object files, images/fonts/audio/video — sections, links, headers, symbols and metadata; no OCR/transcription by default.

Track file capability explicitly:

- **F0 Catalogued:** path, size, hash, detected type, repository and diagnostics.
- **F1 Text indexed:** searchable extracted text with exact byte/line coverage.
- **F2 Structure parsed:** AST/tree/sections/keys/statements/bytecode layout.
- **F3 Entities/definitions extracted:** symbols, keys, tables, endpoints, classes, etc.
- **F4 Relationships/flows:** resolved to a declared capability/coverage level.
- **F5 Runtime observed:** future opt-in, visually and structurally separate from static analysis.

Never label a truncated preview as a complete index. For large files, stream or process ranges. Use size limits, parser timeouts, archive expansion/depth limits, ZIP-bomb protection, path traversal protection and symlink boundaries. Preserve partial-parse diagnostics. Source viewer must fetch ranges and virtualize large files.

## 7. Universal semantic model

Normalize provider facts without erasing provider IDs or language semantics. This is a shared abstraction, not a replacement compiler IR. Preserve pointers, aliases, ownership/borrowing/lifetimes, dispatch, generics, nullability, exceptions, dynamic lookup and concrete runtime representation.

**Core records:** `Workspace`, `Repository`, `Artifact`, `Symbol`, `DataEntity`, `AccessPath`, `Operation`, `ControlRegion`, typed `Relation`, `PatternInstance`, `AtomExplanation`/`Summary`, `Evidence`, `ProviderRun`, `Diagnostic`.

**Data entities (what exists):** scalar/value; record/object; sequence/array/list/vector; mapping; set; tuple/product; enum/variant; tree; graph; pointer/reference/borrow/alias; iterator/stream/generator; function value/closure/callback; option/result/choice; resource/handle; schema/row/message/API payload; document node; binary class/method/field/section/instruction. Preserve concrete type, declared vs inferred type, generics, storage location, mutability, ownership, nullability, alias identity, allocation/lifetime and evidence.

**Access paths (which part is touched):** examples `invoice.total`, `items[i]`, `map[key]`, `request.customer.address.zip`, `*ptr`, `config["billing.tax.rate"]`, `orders[*].status`. Keep wildcard/unknown indexes where exact access is unavailable.

**Atomic operations (what happens):** an atom is the smallest meaningful action the internal model can connect to inputs/outputs. It is not necessarily a line or one AST node. Providers may combine/split syntax nodes during normalization. Initial vocabulary:

- Declarations: `DECLARE`, `PARAMETER`, `FIELD_DECLARE`, `TYPE_DECLARE`, `FUNCTION_DECLARE`, `IMPORT`, `BIND_PATTERN`.
- Values: `LITERAL`, `READ`, `WRITE`, `ASSIGN`, `COPY`, `MOVE`, `PHI_MERGE`, `DEFAULT_VALUE`.
- Computation/types: `UNARY_OP`, `BINARY_OP`, `LOGICAL_OP`, `CONCAT`, `COMPUTE`, `COMPARE`, `TERNARY_SELECT`, `CAST`, `CONVERT`, `TYPE_TEST`, `PATTERN_MATCH`, `DESTRUCTURE`.
- Memory/access: `ALLOCATE`, `CONSTRUCT`, `FIELD_READ/WRITE`, `INDEX_READ/WRITE`, `DEREFERENCE`, `ADDRESS_OF`, `ALIAS`, `RELEASE`.
- Collections: `COLLECTION_CREATE`, `INSERT`, `LOOKUP`, `UPDATE`, `REMOVE`, `ITERATE`, `SORT`, `FILTER`, `MAP`, `REDUCE`, `FOLD`, `COLLECT`.
- Calls/control: `CALL`, `CONSTRUCTOR_CALL`, `VIRTUAL_DISPATCH`, `CALLBACK_INVOKE`, `BRANCH`, `SWITCH`, `LOOP_INIT/TEST/STEP`, `BREAK`, `CONTINUE`, `RETURN`, `YIELD`.
- Exceptions/concurrency: `THROW`, `CATCH`, `RETHROW`, `FINALLY`, `ERROR_RETURN`, `PANIC`, `AWAIT`, `SPAWN`, `JOIN`, `LOCK`, `UNLOCK`, `CHANNEL_SEND/RECEIVE`.
- Effects/transforms: `FILE_READ/WRITE`, `HTTP_REQUEST/RESPONSE`, `DB_QUERY/READ/WRITE`, `MESSAGE_PUBLISH/CONSUME`, `LOG`, `ENV_READ`, `PROCESS_EXEC`, `PARSE`, `VALIDATE`, `NORMALIZE`, `ENCODE/DECODE`, `SERIALIZE/DESERIALIZE`, `HASH`, `ENCRYPT/DECRYPT`.
- Unknown: `OPAQUE_OPERATION`, `UNSUPPORTED_CONSTRUCT`, `DYNAMIC_EFFECT`.

Store both canonical `op_kind` and original `language_kind`/`provider_kind`. Each operation needs stable ID, owning symbol, ordered inputs/outputs, receiver/access path, callee/resolution, types if known, order, control region, source span, provider/version, adapter version, input hash and coverage/diagnostics.

**Relations:** typed directed edges such as `DECLARES`, `CONTAINS`, `REFERENCES`, `READS`, `WRITES`, `MUTATES`, `ALIASES`, `DERIVES_FROM`, `FLOWS_TO`, `ARGUMENT_TO_PARAMETER`, `RETURN_TO_CALLER`, `CALLS`, `DISPATCHES_TO`, `IMPLEMENTS`, `OVERRIDES`, `CONTROL_DEPENDS_ON`, `PRECEDES`, `BRANCHES_TO`, `LOOPS_TO`, `THROWS`, `CAUGHT_BY`, `IMPORTS`, `DEPENDS_ON`, `RESOLVES_TO`, `ROUTES_TO`, `PUBLISHES_TO`, `CONSUMES_FROM`, `READS_TABLE`, `WRITES_TABLE`. Every edge carries direction, evidence, provider, confidence and resolution.

**Control regions:** represent `IF/ELSE`, `SWITCH_CASE`, `LOOP`, `TRY/CATCH/FINALLY`, `MATCH_ARM` with predicate/selector, parent, edges and source span. Never narrate branch-only work as unconditional.

## 8. Decomposition: source → atoms → meaning

Follow this ladder; do not jump from text directly to prose:

| Level | Purpose |
|---|---|
| D0 Artifact | identify file/archive/document/binary |
| D1 Structure | classes, methods, XML elements, YAML keys, SQL tables/statements, bytecode methods |
| D2 Syntax | declarations, statements, expressions, branches, loops, document nodes |
| D3 Data/access | values, variables, fields, indexes, references and access paths |
| D4 Atomic action | individual computation, call, mutation, branch, return, effect |
| D5 Local meaning | how actions combine within an expression/statement/block |
| D6 Cross-unit meaning | calls, data lineage and relations across methods/files/repos |
| D7 Pattern | supported higher-level algorithm/idiom |
| D8 Definition/biography | evidence-backed explanation of a code/artifact unit |
| D9 System narrative | how modules/services/repos cooperate |

### Required example

```java
int total = price * quantity + tax.calculate(price);
```

Internally record distinct reads, multiplication, call, addition and assignment in language-defined evaluation order, with values and source spans. **Show the developer this instead:**

1. Uses the current values of `price` and `quantity` to calculate their product.
2. Calls `tax.calculate` with `price` and captures the value returned by that call.
3. Adds the product to the value returned by `tax.calculate`.
4. Stores the resulting sum in `total`.

Combined: “This statement multiplies `price` by `quantity`, calls `tax.calculate` with `price`, adds the returned value to the product, and stores the resulting sum in `total`.” Do not call the return value a tax amount unless the callee/docs prove it. If unresolved, explain only the observed call and data use.

### Data-structure meaning

For `Map<String, List<Invoice>> byCustomer` followed by `computeIfAbsent(customerId, ...)` and `.add(invoice)`, explain in English that the code creates a map from customer IDs to invoice lists, obtains or creates the current customer's list, then adds the invoice. Keep exact library behavior conditional on resolved implementation or documented API contract.

**Atoms vs algorithms:** atoms are basic actions; patterns are inferred from connected operation/control/data-flow subgraphs. Initial patterns may include filtering, mapping, reduction, search, sorting, tree/graph traversal, parsing, validation, serialization, retry, caching and dynamic programming. Recognize from graph signatures, never method names/comments alone. Record rule/version, matched operations, evidence, confidence and alternatives. Use “possible/probable” for weak matches.

Preserve language specifics: Java overloads/generics/dispatch/exceptions; C pointers/aliases/macros/allocation; C++ templates/operator overloads/RAII; Python dynamic lookup/decorators/generators; JS/TS prototypes/closures/promises; Rust ownership/borrowing/lifetimes/traits; Go interfaces/goroutines/channels; C# properties/events/delegates/LINQ. Keep opaque provider-specific facts when safe normalization is not possible.

## 9. Definitions, biographies and English-first generation

A **definition** says what a unit is and where it is defined: qualified identity/signature, repository/module, types/visibility, comments/docs, source range, callers/callees/references and analysis coverage. For non-code, provide the corresponding XML/YAML/JSON key path, SQL table/column/query, API endpoint/schema, `.class` member/descriptor or document section definition.

A **biography** explains what the implementation demonstrably does: supported purpose; inputs/parameter use; data/access paths; meaningful operations; structures created/accessed/mutated; branches/loops/exits; calls and arguments/results; outputs/returns; side effects; exceptions; direct callers/callees; separately labelled transitive behavior; evidence and coverage.

**Generation sequence:** select symbol/revision/facts → order operations and load data/control/call relations → group low-level reads with the action they support → preserve evaluation order, short-circuiting, branches, loops and effects → generate complete English sentences → compose statement/block/method summaries → add graph-supported patterns → add callee summaries separately → attach evidence to every claim → state unresolved/ambiguous/external/dynamic/unsupported gaps.

The default page shows **Overview**, **Step-by-step explanation**, **Inputs/outputs**, **Data and structures**, **Calls/relationships**, and **Source evidence**. Raw IR, operation names, node IDs and graph edges belong in a collapsed **Technical details** panel.

Use natural, developer-quality sentences, not a list of “reads X” fragments. Explain how values contribute to results. Do not infer business intent from identifiers. Never expose `READ`, `BINARY_OP`, `CALL`, `ASSIGN` or temporaries such as `v1` as the default narrative. Branch conditions and loop uncertainty must stay explicit.

Start with deterministic, offline-capable templates and context-aware grouping. An LLM may later improve fluency or answer questions over retrieved facts, but it cannot discover facts, resolve symbols or invent behavior; validate claims against evidence IDs. Deterministic output remains available and authoritative.

Compose callee summaries bottom-up. Use strongly connected components for recursion; bound depth/size; distinguish `direct_behavior` from `transitive_behavior`; track callee summary versions and invalidate dependent summaries when they change.

## 10. Identity, resolution and cross-repository linking

Never identify a symbol by bare name. Use workspace/repository, language, module/package, enclosing type/namespace, kind/name, normalized signature/parameter types, overload/generic context and provider ID mapping. Keep stable CodeGraphy IDs plus provider-ID aliases.

Resolution states must distinguish: `RESOLVED`, `AMBIGUOUS`, `HEURISTIC`, `EXTERNAL`, `UNRESOLVED`, `DYNAMIC`, `RUNTIME_OBSERVED`, `NOT_APPLICABLE`. Resolve only with sufficient evidence. Preserve all plausible candidates for ambiguous references. Do not select same-name methods blindly. Represent external targets even without source. When a missing repository is added, re-resolve affected references and dependent summaries without a full workspace rebuild. Keep architecture dependencies distinct from source-level call edges. Preserve the evidence and rule used for every link.

## 11. Storage and query contract

SQLite is the local durable source of truth. Persist workspace/repositories/artifacts, hashes and file capabilities; symbols and identity aliases; data entities/access paths; operations/operands; control regions; typed edges; pattern instances; summaries and summary-to-evidence mappings; provider runs/caches/diagnostics; dependency/invalidation metadata. Use FTS5 for paths, symbols, comments, documentation and extracted text. Use transactions/batched writes. Keep source files in the user's workspace; store references and normalized facts rather than copying all source into the database without need.

Core user queries include:

- Find a class, method, key, table, endpoint, topic or file.
- Show callers/callees and implementations of a symbol.
- Show paths from `BillAPIHandler` to `InvoiceCreation`.
- Where does this value originate and where is it written?
- What modifies `invoice.total`?
- Which methods read/write a table or publish/consume a message topic?
- Which branches can reach a return or exception?
- What changes if this method or file changes?

A deterministic intent/slot grammar can support basic natural-language-like queries. LLM assistance is optional and must not be required for query correctness/evidence retrieval. Query endpoints must support paging, cancellation, bounded depth and node count.

## 12. UI and one-command product experience

The primary interface is a smooth local browser app, not a TUI. It must offer:

- repository/workspace explorer with semantic badges and filters;
- file preview and source viewer with clickable line evidence;
- definition/biography pages;
- global search across paths, symbols, comments, configuration, schema and text;
- interactive call/data/control/architecture maps;
- click node → open definition; click edge → open source evidence; expand node → load bounded neighborhood;
- search-to-flow query workflow;
- diagnostics and analysis coverage panel;
- background indexing progress without freezing navigation;
- persistent selection/navigation context, keyboard support and reduced-motion behavior;
- progressive/WebGL graph rendering with limits and clear “show more” controls.

### One-command contract

- Installed release: `codegraphy <workspace-path>`.
- Source checkout on Unix-like systems: `./codegraphy <workspace-path>`.
- Source checkout on Windows PowerShell: `./codegraphy.ps1 <workspace-path>`.
- Without a path, launch the app and let the user select a workspace.

The launcher should prepare/build CodeGraphy's own required components as appropriate, start its local service, index the chosen workspace and open the browser. Packaged builds should ship pinned/prebuilt provider dependencies where licensing and platform support permit. Do not require users to clone/build Atlas, install Joern/Enola manually or configure Docker/Podman for normal use. Show a guided diagnostic when a component cannot run. Do not execute the target repository's build scripts as an implicit bootstrap step.

## 13. Validation and acceptance criteria

Create compact fixtures for: Java overloads/inheritance/generics/calls/branches/loops/exceptions; C/C++ pointers/structs/arrays/function pointers; Python dynamic calls/comprehensions/generators; JS/TS closures/async/promises; Rust ownership/borrowing/traits; XML/YAML/JSON/TOML nesting/references; SQL joins and reads/writes; JVM `.class`/JAR; and two repos where a missing target is added later.

Golden tests must assert entities, operations, relationships, source spans, resolution state, coverage and expected English explanation fragments. Test the English that a developer sees, not merely the internal IR.

Minimum acceptance: nested expression atoms preserve evaluation order; English explanations combine them coherently; branch/loop/exception semantics remain conditional; mutations connect to correct access paths; calls bind only when resolved; overloads stay distinct; unresolved/ambiguous/dynamic targets remain visible; every biography claim links to evidence; direct/transitive behavior stays separate; recursion is bounded; repo addition relinks only affected references; incremental equals clean rebuild; provider failure preserves other facts; partial files never claim full coverage; one-command launch and UI remain usable during analysis.

Measure separately: discovery precision/recall; parse and symbol coverage; call-resolution and dataflow accuracy; control-flow accuracy; evidence and biography factuality; unresolved classification; incremental correctness; query/UI latency; first-ready time; peak RAM. Do not combine resolved, external and heuristic results into a misleading accuracy score.

## 14. Implementation roadmap

- **M0 — Audit and stabilize.** Read-only audit of actual repo; preserve current catalog/API/SQLite/UI/launchers; map real code paths and tests; identify provider seams.
- **M1 — Provider feasibility.** Independently prove Atlas integration on supported source fixtures, Joern execution/export/import on appropriate Java and C/C++ fixtures, and Enola facts/receipt import on a multi-repo fixture. Pin versions; measure coverage/runtime/RAM. No biographies yet.
- **M2 — Canonical model.** Define versioned entities, operations, relations, provenance, evidence, resolution and capability records based on real provider outputs. Keep raw provider facts and normalized facts.
- **M3 — Atomic decomposition.** Implement one tested vertical slice: nested expression, data entities/access paths, operations, control regions, dataflow and call edges.
- **M4 — English explanations.** Add deterministic sentence realization, operation grouping, block/method composition, evidence links, pattern recognition and golden explanation tests.
- **M5 — Flow/system explorer.** Add bounded call/data/control paths, maps, Enola overlays and cross-repository relinking.
- **M6 — Incremental hardening/release.** Change invalidation, caches, scheduler, diagnostics, security, responsive UX, clean-machine one-command release and benchmarks.

Do not ask one task to implement M1–M6 at once. Each milestone has an evidence gate and a runnable app.

## 15. AI-agent workflow and immediate tasks

### Workflow

1. Read this file, inspect Git state, repository structure and nested instructions; never discard user changes.
2. Audit actual code paths/tests before claiming a feature exists.
3. Work in a feature branch; keep the app runnable.
4. Inspect/pin exact upstream engine versions and APIs before integration. If blocked, report exact commands/errors; do not substitute a fake parser.
5. Implement the smallest end-to-end slice requested; do not attempt the whole roadmap in one task.
6. Add real fixtures and golden tests for semantic facts **and English explanations**.
7. Run relevant formatting, lint, type, unit/integration tests and UI build.
8. Report status, files, commands, tests, fixtures, measured performance, limitations, remaining work and launcher status.
9. Update this file only for durable user-approved decisions. Do not create competing specs or extra instruction files unless requested.

### First task — read-only audit

> Do not modify files. Audit the current CodeGraphy repository: Rust crates, frontend, database schema, API routes, file discovery/classification/indexing, CLI/launchers, tests and UI flows. Trace actual code paths and cite file/symbol/line evidence. Classify each capability as `IMPLEMENTED`, `PARTIAL`, `STUBBED`, `MOCKED`, `UNSUPPORTED` or `UNVERIFIED`. Identify Atlas/Joern/Enola integration seams, verify one-command launch and propose the smallest next slice. Do not assume the handoff status is current.

### Second task — provider proof

> After audit review, independently test Atlas on supported source fixtures, Joern export/import on suitable Java and C/C++ fixtures, and Enola fact/snapshot import on a two-repository fixture. Pin versions; record facts, gaps, time and peak RAM. Do not change UI or implement biographies. Stop and report exact blockers; do not write substitute parsers.

### Third task — canonical model

> Map actual provider outputs into versioned `Artifact`, `Symbol`, `DataEntity`, `AccessPath`, `Operation`, `ControlRegion`, `Relation`, `PatternInstance`, `Summary`, `Evidence`, `ProviderRun` and `Diagnostic` records. Preserve raw provider IDs/facts. Demonstrate deduplication, ambiguity, cache keys and invalidation. Propose schema before biography code.

### Fourth task — semantic/English vertical slice

> On a validated fixture, extract parameters, nested arithmetic/calls, assignments, collection access/mutation, branch, loop and return. Generate coherent English atom, statement, block and method explanations with source evidence. Keep raw operation labels in the optional inspector. Golden-test every claim. Do not claim untested language support.

## 16. Definition of done

A CodeGraphy capability is complete only when it:

- works on a real fixture and not only a mock;
- has a tested end-to-end path from provider/file input to persisted result to query/API to UI;
- includes evidence, coverage and honest resolution/uncertainty;
- has tests for positive, negative, ambiguous and unsupported cases;
- has acceptable measured performance/resource use for its scope;
- survives incremental update and clean rebuild comparison where relevant;
- does not regress the one-command launch or existing working catalog.

The product is not complete merely because a dependency compiles, a graph renders, text appears in a preview, or an English-looking paragraph is hard-coded.

## 17. Upstream sources

Pin and validate exact upstream versions: Atlas (https://github.com/LordCasser/atlas), Joern (https://docs.joern.io/), Enola (https://enola.tech/docs/graph), Tree-sitter (https://tree-sitter.github.io/tree-sitter/), SCIP (https://scip-code.org/) and SQLite FTS5 (https://www.sqlite.org/fts5.html).
