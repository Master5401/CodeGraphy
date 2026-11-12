# Review of newly supplied CodeWiki requirements

**Reviewed:** 2026-11-12  
**Inputs:** `CODEWIKI_BUILD_SPEC.md`, `CodeWiki_Technical_Architecture_and_Codex_Build_Spec_v1.docx`, and `CodeWiki_Final_PRD_v3_2_Polyglot.docx` supplied from Downloads.  
**Status:** the maintainer resolved the conflict on 2026-11-12. The controlling spec is now the combined v5.0 `docs/PRD.md`.

## What is a user request versus an instruction inside a file

The user's request is to review the attached materials, explain the current command and engine status, address atomic decomposition, file-format coverage and multi-repository linking, and leave a README/log for later maintainers.

The build spec and PRD also contain prompts addressed to a future coding agent (for example, “First task”, “Codex kickoff prompt”, and proposed commands). Those are **requirements written inside the attachments**, not independent authorization to override the repository's `AGENTS.md` or begin a conflicting milestone. They have been reviewed as design input. They have not been silently installed as repository instructions.

## Resolution: polyglot catalog, Java semantic priority

| Topic | Repository PRD v4 | Supplied v3.2 / build spec |
|---|---|---|
| Product order | Java first; other languages later. T2 is Core + Atlas adapter, after T1 Atlas audit. | Language- and file-format-neutral from the first release; initial vertical slice catalogs and searches all files. |
| First user workflow | `just run <path>` for a source checkout; staged Java workflow. | `./codewiki[.ps1] <workspace>` bootstraps/builds, indexes, starts Axum and opens the UI in one invocation. |
| Analysis engines | Atlas is the M1–M5 backend; Enola M6 and optional Joern M7. | Tree-sitter + dedicated format adapters + optional SCIP are the baseline; Atlas/Enola/Joern are specialist adapters only when justified by measured gaps. |
| Milestone sequence | T0–T8; next slice after the completed T1 audit is T2. | Phase 0 bake-off, Phase 1 polyglot file index/usable UI, Phase 2 biographies/data flow, then multi-repo/incremental. |

The maintainer directed us to combine both PRDs: keep polyglot file discovery/catalog/search from the first usable release while providing more semantic support for Java. This decision is recorded as `docs/PRD.md` v5.0 and is the repository source of truth under `AGENTS.md`. Milestones now deliver a usable file-catalog vertical slice first, then Java semantics, call flows, multi-repository linking, Java operation biographies, incremental sync, broader adapters, and optional engine integrations. This preserves v4's evidence and safety rules and v3.2's language/file-format-neutral foundation. The initial no-op `just run` scaffold has since been replaced by the M1 catalog command; `just run` builds the web app then launches the CLI, while the platform wrappers also install pinned npm dependencies when missing.

## Current command and engine facts

### Will it work just by commands?

The current source checkout has a runnable M1 developer flow: `.\codewiki.ps1 <workspace>` on PowerShell or `./codewiki <workspace>` on macOS/Linux builds the web assets, starts the Rust loopback server, opens the browser, and catalogs the selected workspace. It includes bounded text search, a repository-shaped file tree, and safe source preview. The workspace is optional; omitting it opens the folder picker. A packaged release binary is not available yet, so an installed end-user command is not available. Indexing/querying are local and do not call external services. The first source-checkout launch may download pinned npm dependencies with `npm ci`.

The current scope is a usable file catalog, not a full code comprehension product. It does not parse syntax, resolve symbols, build call/data-flow graphs, create biographies, or link multiple repositories.

### How were Atlas, Joern, and Enola used?

- **Atlas:** T1 only. A temporary release harness called the public `atlas-engine` library on synthetic Java fixtures for the audit in [`ATLAS_GAP_MATRIX.md`](ATLAS_GAP_MATRIX.md). It did not run Atlas's CLI/TUI/MCP, was not added as a CodeWiki dependency, and is not integrated into the app. The audit found specific incorrect/missing behaviors; it did not certify Atlas as correct.
- **Joern:** a root-level clone exists, but it was not run, evaluated, or integrated.
- **Enola:** a root-level clone exists, but it was not run, evaluated, or integrated.
- Engine clones and `vendor/` were left untouched. The audit facts and temporary build are documented in [`WORK_LOG.md`](WORK_LOG.md).

The attached v3.2 text proposes specialist engine reuse, but that proposal does not change what has actually been used in this checkout.

## Atomic units and English descriptions

**Implemented state:** no language or file-format grammar is integrated. Classification uses extension hints, a small set of magic-byte signatures, and UTF-8/binary checks. Bounded readable text is searchable. The operation categories and per-language rows below are design material for later adapters, not working parser behavior.


The v3.2 PRD defines a **shared operation taxonomy and data-flow vocabulary**, not a complete fixed grammar or dictionary for every programming language. Its operation families include assignments/aliasing, arithmetic, comparisons/logic, type operations, collection operations, object operations, control flow, calls, and I/O/service boundaries. Its edge vocabulary includes `READS`, `WRITES`, `FLOWS_TO`, `ARG_TO_PARAM`, `RETURN_TO_CALL`, `ALIAS_OF`, `MUTATES`, `CONTROL_DEPENDS_ON`, `THROWS_TO`, and `CAUGHT_BY`. The build spec adds universal concepts such as `File`, `Artifact`, `Symbol`, `Scope`, `Operation`, `Reference`, `Relationship`, `EvidenceFact`, and `AnalysisCapability`.

Neither attached specification supplies a complete per-language grammar with exact syntax-node mappings and a complete English sentence dictionary. The shared atom categories below are adopted in v5; the following language rows are **illustrative starting mappings** to turn into adapter-specific golden fixtures. They are not implemented behavior or a claim of complete grammar coverage:

### Common normalized atom set

1. **Entities:** repository, project/module, file, scope, type, function/method, parameter, local, field/property, constant, external symbol, config key, endpoint, schema/table, artifact, and documentation section.
2. **Values/data shapes:** scalar, literal, sequence/array, mapping/map, set, tuple/record, object/struct, pointer/reference/alias, optional/result/union, stream/channel, and external resource. Preserve source-language type/name as evidence; normalize only the broad shape where proven.
3. **Operations:** declare, read, write, allocate/construct, copy/alias, transform/arithmetic, compare, cast/convert, collection action, call/dispatch, branch, iterate, return, throw/catch, await/yield, synchronize, and boundary action (HTTP, message, database, file, serialization).
4. **Relations:** contains, defines, references, imports, calls, reads, writes, flows-to, argument-to-parameter, return-to-call, aliases, mutates, control-depends-on, sends-to, handles, depends-on, documents, and archive-contains.
5. **Evidence state:** every atom/edge carries provider and version, repository/revision, file/range or binary offset, source hash, derivation kind, and resolution state. Unknown type, target, or value stays `UNRESOLVED` with a reason; it is not filled in from a guessed business meaning.

### Candidate source-language mapping

Each syntax adapter should map language syntax into the shared atoms while preserving native constructs and analysis limits. The English layer then uses the same deterministic templates across languages; it must not pretend different language constructs have identical runtime semantics. Java is the first planned deep adapter; the M5 Java operation IR/biography work depends on the M2 Atlas evidence adapter. Other language and format adapters follow in M7.

| Language family in build spec | Native constructs to recognize | Example shared English rendering |
|---|---|---|
| C / C++ | translation unit/include, function, class/struct/union, pointer/reference, field, allocation/free, templates, macro, virtual call, exception | “Reads through pointer `p`”; label alias/target as unresolved when points-to analysis is insufficient. |
| Java | package/import, class/interface/record/enum, field/local/parameter, method/constructor, overload, array/collection, lambda/stream, exception | “Calls `f` with `x` and stores the result in `y`.” |
| C# | namespace/using, class/struct/record, property/indexer/event, delegate, async/task, LINQ, exception | “Awaits `LoadAsync` and assigns its result to `item`.” |
| Go | package/import, type/struct/interface, receiver method, slice/map/channel, goroutine/select, defer, error return, panic/recover | “Sends `value` on `ch`”; do not infer goroutine ordering. |
| Rust | crate/module/use, struct/enum/trait/impl, ownership move/borrow, reference, pattern/match, `Option`/`Result`, iterator, async/await | “Moves `item` into `items`” or “Borrows `value`”; retain ownership semantics. |
| Python | module/import, class/function, name binding, attribute/subscript, list/dict/set/tuple, generator/await, decorator, exception | “Reads attribute `name` from `user`”; dynamic dispatch can remain unresolved. |
| JavaScript | module import/export, function/class, closure, object/array/map/set, computed property, promise/await, exception, browser/network boundary | “Awaits `fetchData` and stores its result in `data`.” |
| TypeScript | JavaScript constructs plus type/interface/enum/generic/union, optional property and type guard | “Checks whether `value` is a `User`”; syntax/type facts are distinct from runtime checks. |
| Kotlin | package/import, class/object/data class, property, extension function, nullable type, coroutine, collection/sequence, exception | “Calls `load` only when `item` is not null”; preserve safe-call behavior. |
| PHP | namespace/use, class/trait/interface, function/method, array, reference, magic/dynamic call, generator, exception | “Writes `value` to key `name` in `map`”; unknown dynamic member stays unresolved. |
| Ruby | module/class/mixin, method, block/proc, dynamic dispatch/metaprogramming, array/hash, exception | “Calls block `block` for each `item`”; dynamic target remains a candidate if not statically known. |
| Swift | module/import, struct/class/enum/protocol, property/observer, optional, closure, async/await, value/reference semantics | “Unwraps `result` if present”; preserve optional branch and value/reference distinction. |
| Scala | package/import, class/object/trait/case class, pattern match, collection, extension/given, future/async, exception | “Maps `items` to `item` using `transform`”; retain lambda and collection operation evidence. |

The exact grammar-node names, feature coverage, templates, and unsupported cases must be pinned and tested per parser/grammar version. A hand-maintained “English dictionary” should be a small template vocabulary over typed atoms, not a second natural-language parser or a source of inferred intent. Tree-sitter supplies syntax structure, while a semantic index/provider such as SCIP can add definitions/references; syntax alone does not prove types or call targets.

### File-format mapping from the build spec

| File category | Candidate atomic records | Example deterministic English |
|---|---|---|
| Source code | declarations, scopes, operations, references, calls, data/control edges | “`f` reads `x`, then calls `g(x)`.” |
| XML / YAML / JSON / TOML / INI / `.properties` / `.env` | document, object/element, key/path, scalar, reference, profile/anchor | “Sets `service.timeout` to `30`.” |
| Build manifests (Maven, Gradle, npm/pnpm, Cargo, Go, Python, CMake, Make, MSBuild) | project, module, target, declared dependency, plugin/task name as text | “Project `api` declares dependency `core`.” Never execute tasks/scripts during indexing. |
| SQL and migrations | statement, table, column, constraint, read/write, join/reference | “SELECT reads columns `id` and `name` from `users`.” |
| OpenAPI / GraphQL / Protobuf / Avro | service, endpoint/operation, request/response, message, field, schema reference | “POST `/orders` accepts `OrderRequest` and returns `Order`.” |
| Markdown / RST / AsciiDoc / TXT / HTML | document, heading, link, code fence, anchor, text search hit | “Section `Setup` links to `install.md`.” Symbol cross-links require explicit evidence or a labelled heuristic. |
| CSV / TSV / JSONL | row/record, header/field, bounded sample, inferred type with confidence | “Column `amount` contains numeric-looking values in the sampled rows.” Mark sample-derived type as inferred. |
| JVM `.class` / `.jar` / `.war` | class, method, field, descriptor, constant-pool item, archive member, bytecode attribute | “Archive `app.jar` contains class `com.example.Main`.” Enforce archive limits. |
| PE / ELF / Mach-O / object files / unknown binaries | format, section, symbol/import/export where present, hash, offset, archive/member | “Binary has section `.text`”; unknown files remain cataloged without semantic claims. |
| Images / fonts / audio / video | format, size, dimensions/duration, metadata/hash, file links | “Image is 1200×800 PNG.” No OCR or transcription by default. |

Every eligible file should at least have the universal file record. “Cataloged” or “text-searchable” is a lower capability than “parsed”, “linked”, or “semantically resolved”; expose that distinction instead of inventing prose.

## Multi-repository linking and generated docs

Both PRDs describe repositories/workspaces and later linking when a missing provider arrives. Combined v5 places this in M4, after the Java call graph. v3.2 describes cross-repository identity, unresolved references, and later resolution, but does **not** specify a literal command that merges two independently generated documentation folders.

A safe implementation model for the requested behavior is one workspace manifest plus one canonical workspace index containing both repository identities. A future command such as `codewiki repo add <path>` would add/index the new repository, re-evaluate only matching unresolved references, preserve ambiguous or mismatched results, and regenerate affected repository/symbol views from that shared graph. Repo A's view would link to Repo B and Repo B's view would include a reciprocal link when evidence supports it. The combined PRD specifies this behavior in M4; it is not implemented yet.

External support research agrees with this general shape: Enola documents a cluster configuration that produces a linked cross-repository graph and records unresolved calls rather than guessing; its docs also describe shared graph output per repository. This is useful prior art, not evidence Enola has been used here. See [Enola multi-repo docs](https://enola.tech/docs/multi-repo) and [Enola graph format](https://github.com/enola-labs/enola/blob/main/docs/GRAPH.md).

## Research notes for future implementation

- [Tree-sitter's parser guide](https://tree-sitter.github.io/tree-sitter/using-parsers/) describes concrete syntax trees with node/source positions and incremental updates. It is a syntax layer; do not equate a CST with compiler-grade target resolution.
- The [SCIP reference](https://github.com/scip-code/scip/blob/main/docs/scip.md) defines semantic symbol/occurrence data and source ranges as an interchange format. Treat providers and versions as provenance; unsupported/absent data remains explicit.
- Joern's [Code Property Graph docs](https://docs.joern.io/code-property-graph/) and [CPG specification](https://cpg.joern.io/) describe a language-agnostic attributed graph combining program constructs/relationships. The foundational paper, [Modeling and Discovering Vulnerabilities with Code Property Graphs](https://fabianyamaguchi.com/files/2014-ieeesp.pdf), combines syntax, control flow, and data dependence for graph queries. This informs representation design; it does not make Joern an implemented dependency.
- Atlas source repository: [LordCasser/atlas](https://github.com/LordCasser/atlas). Use the pinned PRD version and the local gap matrix for facts about the exact audited version, not current upstream claims.

These references are architecture research, not an instruction to add dependencies before the relevant milestone. Keep core indexing/query paths offline and do not execute analyzed repository scripts.

