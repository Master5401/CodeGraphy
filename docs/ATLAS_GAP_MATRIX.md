# Atlas Capability Gap Matrix

## Audit scope

This T1 audit evaluates Atlas `v1.5.2` at commit `8021318e92f290e2cac915e4e91b514bf0a187d2`, the baseline named by the repository [PRD](PRD.md). The root-level Atlas clone is at `v1.7.0-8-g0f8379ff`; the pinned tag was extracted to a temporary directory so the clone and submodule layout remain untouched. This is an upstream audit, not a qualification of a maintainer fork.

The audit built a temporary release harness against the public `atlas-engine` library with `default-features = false` and `features = ["java"]`. The harness used `Store`, `IndexPipeline`, `IndexPipelineOptions`, `ExtractionMode::Full`, and a no-op progress sink. It did not compile the Atlas CLI, TUI, or MCP server, and it did not run Maven, Gradle, Java, or fixture code. Index files and database outputs were kept under the OS temporary directory.

The requested Java-only build could not be achieved without changing Atlas manifests: Cargo feature inspection and the build log show that path dependencies enable their own default features, compiling the other bundled grammars even though the facade was requested with Java only. The release build completed in 69 seconds on Windows 11 Home 64-bit with Rust 1.99.1. The harness executable was 28,988,416 bytes. No engine source or manifest was patched.

All 11 fixture directories below are synthetic. Every indexing process exited successfully with `failed=0`; the intentionally malformed file was marked `partial` while its neighboring file was marked `success`. Results were read from Atlas SQLite tables, including `files`, `symbols`, `references`, `imports`, `callsites`, `symbol_edges`, `dataflow_edges`, and `cfg_nodes`/`cfg_edges`.

Status key: **PASS** means the tested behavior matched the requirement; **PARTIAL** means Atlas retained useful evidence but missed a required distinction or guarantee; **FAIL** means the fixture exposed incorrect or missing behavior; **N/A** means that fixture does not test that dimension.

## Fixture result matrix

| Fixture | Symbols | Overloads | Call edges | External calls | Missing imports | Dataflow | CFG | Incremental | Determinism | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| `overloads-inheritance` | **FAIL** one of 13 extracted symbols was absent from storage | **FAIL** `choose(int)` was dropped; both call sites target `choose(String)` | **FAIL** integer call resolves to the wrong overload; remaining edges are often `name_only` heuristics | N/A | N/A | **PARTIAL** source-located generic edges exist | **PARTIAL** simple return CFG only | **PASS** one-file edit equals a clean rebuild | **PARTIAL** fact tables stable; timestamp metadata changes | E1, E2, E3, E11 |
| `same-method-name-two-repos` | **PARTIAL** both files/methods stored, but package and repo identity are absent | N/A | N/A | N/A | N/A | **PARTIAL** basic source-located edges | **PARTIAL** basic entry/return nodes | N/A | N/A | E4 |
| `two-wildcard-imports` | **PASS** local class and method found | N/A | **FAIL** unresolved `convert` becomes a self-call | **PARTIAL** call site remains, but no external symbol or candidate set | **FAIL** both wildcard flags are false; no ambiguity is reported | **PARTIAL** generic call/return edges only | **PARTIAL** basic return CFG | N/A | N/A | E5 |
| `fully-qualified-missing` | **PASS** local class and method found | N/A | **PARTIAL** call site remains but has no target edge | **PARTIAL** qualified segments and call site remain unresolved; no named external stub | N/A | **PARTIAL** generic call/return edges only | **PARTIAL** basic return CFG | N/A | N/A | E6 |
| `chess-tour-missing-repos` | **PASS** local class and method found | N/A | **PARTIAL** both missing callsites remain; no call edges | **PARTIAL** no external stubs or candidate targets | **FAIL** `a.*` and `c.*` imports were absent from the imports table | **PARTIAL** generic call/return edges only | **PARTIAL** basic return CFG | N/A | N/A | E7 |
| `later-added-signature-mismatch` | **PASS** client and provider declarations found | N/A | **FAIL** `convert` remains a self-call after the provider is added | **PARTIAL** no `CONTRACT_MISMATCH`; the imported type is heuristic and the call is misbound | N/A | **PARTIAL** generic call/return edges only | **PARTIAL** basic return CFG | **PASS** after adding the provider, normalized facts equal a clean rebuild | N/A | E8, E11 |
| `chained-call-unknown-return` | **PASS** local class and method found | N/A | **PARTIAL** three callsites retained; no edges or receiver-type diagnostic | **PARTIAL** `UnknownFactory.open(...).normalize()` has no stub or `receiver-type-unknown` reason | N/A | **PARTIAL** generic call/return edges only | **PARTIAL** basic return CFG | N/A | N/A | E9 |
| `arrays-lists-maps-loops-lambdas` | **PASS** class, methods, and local declarations found | N/A | **PARTIAL** local references retained; library calls have no internal targets | N/A | N/A | **PARTIAL** 49 generic dataflow edges with source ranges; no CodeWiki operation IR | **PASS** 16 nodes and 14 edges include loop, branch, true/false branch | N/A | N/A | E10 |
| `malformed-file` | **PARTIAL** declarations recovered from the malformed file | N/A | N/A | N/A | N/A | **PARTIAL** facts still emitted for malformed input | **PARTIAL** CFG emitted, but no persisted diagnostic explains the parse error | N/A | N/A | E12 |
| `generated-sources` | **PASS** ordinary and generated declarations found | N/A | N/A | N/A | N/A | **PARTIAL** generic edges only | **PARTIAL** basic entry/return nodes | N/A | N/A | E13 |
| `multi-module-maven` | **PASS** both module source files found | N/A | **PARTIAL** cross-module edge exists with heuristic `name_only` resolution | N/A | N/A | **PARTIAL** generic argument/return edges only | **PARTIAL** basic entry/return nodes | N/A | N/A | E14 |

## Evidence notes

### E1 — Version and feature surface

The audited tag resolves to commit `8021318e92f290e2cac915e4e91b514bf0a187d2`. Atlas is MIT-licensed at that commit ([license](https://github.com/LordCasser/atlas/blob/8021318e92f290e2cac915e4e91b514bf0a187d2/LICENSE)). The build used only the engine facade, without the CLI/TUI/MCP crates. However, the Java-only request still compiled JavaScript/TypeScript, Python, C/C++, Go, C#, Rust, PHP, Ruby, Kotlin, ArkTS, and Cangjie grammar dependencies. Atlas's [engine manifest](https://github.com/LordCasser/atlas/blob/8021318e92f290e2cac915e4e91b514bf0a187d2/crates/atlas-engine/Cargo.toml) forwards language features to child crates but does not disable their default features on the path dependencies. This is a build-size and scope gap; fixing it requires an approved maintainer-fork change.

### E2 — Overloads and symbol identity

[`Base.java`](../fixtures/java/atlas-audit/overloads-inheritance/src/main/java/audit/symbols/Base.java#L4-L10) defines `choose(int)` and `choose(String)`. Atlas reported 13 extracted symbols but persisted 12; only `Base.choose(String)` remained. [`Child.java`](../fixtures/java/atlas-audit/overloads-inheritance/src/main/java/audit/symbols/Child.java#L9-L14) calls both overloads, and both stored call edges targeted `Base.choose(String)`, including the integer call at line 10.

The audited [shared symbol builder](https://github.com/LordCasser/atlas/blob/8021318e92f290e2cac915e4e91b514bf0a187d2/crates/atlas-engine/crates/extraction/src/languages/shared.rs#L218-L244) generates IDs with a `None` discriminator and sets `package_name` to `None`. The Java frontend also builds qualified names from an empty package prefix ([java.rs](https://github.com/LordCasser/atlas/blob/8021318e92f290e2cac915e4e91b514bf0a187d2/crates/atlas-engine/crates/extraction/src/languages/java.rs#L44-L49)). This matches the observed overload collision and missing package identity.

### E3 — Internal calls and resolution confidence

[`Calls.java`](../fixtures/java/atlas-audit/overloads-inheritance/src/main/java/audit/symbols/Calls.java#L5-L7) produced an edge to `Child.provide`, and `Child.provide` produced an edge to `Base.adjust`. These edges carried `confidence=1.0`, `provenance=heuristic`, and `resolved_by=name_only`. Numeric confidence alone must not be treated as `RESOLVED` by CodeWiki.

### E4 — Same method name across repositories

[`Handler.java` in repo one](../fixtures/java/atlas-audit/same-method-name-two-repos/repo-one/src/main/java/audit/alpha/Handler.java#L3-L6) and [`Handler.java` in repo two](../fixtures/java/atlas-audit/same-method-name-two-repos/repo-two/src/main/java/audit/beta/Handler.java#L3-L6) both indexed. Atlas retained distinct rows because their file IDs differ, but both methods have qualified name `Handler.lookup`; package names are null and the store has no repository identity. This cannot serve as CodeWiki's multi-repository identity/linker model.

### E5 — Wildcard imports and false self-call

[`MissingImports.java`](../fixtures/java/atlas-audit/two-wildcard-imports/src/main/java/audit/missing/MissingImports.java#L3-L8) has two wildcard imports and calls `MissingType.convert`. Atlas stored two imports, but both rows had `is_wildcard=0` (`imported_name` was `alpha`/`beta`). It retained `MissingType` as an unresolved reference, then resolved the leaf `convert` by `exact_match` to the enclosing `MissingImports.convert`, producing a self-call edge at line 8. `reference_resolutions` and `symbol_edge_candidates` were empty; there was no ambiguity state or candidate set.

### E6 — Fully qualified missing call

[`QualifiedCall.java`](../fixtures/java/atlas-audit/fully-qualified-missing/src/main/java/audit/missing/QualifiedCall.java#L4-L5) produced unresolved references for the qualified segments and a callsite for `normalize` at line 5. Atlas stored no target edge and no external symbol record, so CodeWiki would need to retain the callsite and synthesize its own `UNRESOLVED`/`EXTERNAL` record.

### E7 — Missing repositories

[`Tour.java`](../fixtures/java/atlas-audit/chess-tour-missing-repos/repo-b/src/main/java/audit/tour/Tour.java#L7-L9) produced two callsites for missing repository APIs and no call edges or stub symbols. Its `a.*`/`c.*` imports were not present in Atlas's imports table. Atlas therefore preserves some site evidence but does not satisfy CodeWiki's named-stub, inferred-contract, or later-linking requirements.

### E8 — Later provider with incompatible signature

[`ApiCaller.java`](../fixtures/java/atlas-audit/later-added-signature-mismatch/client/src/main/java/audit/client/ApiCaller.java#L5-L7) was first indexed without the provider. After [`Converter.java`](../fixtures/java/atlas-audit/later-added-signature-mismatch/provider/src/main/java/audit/provider/Converter.java#L3-L5) was added, the incremental database and clean rebuild had equal normalized fact tables. The `convert` call still targeted `ApiCaller.convert(String)` itself rather than `Converter.convert(Integer)`, and Atlas emitted no contract-mismatch state.

### E9 — Unknown chained call

[`UnknownChain.java`](../fixtures/java/atlas-audit/chained-call-unknown-return/src/main/java/audit/missing/UnknownChain.java#L4-L5) retained callsites for `UnknownFactory`, `open`, and `normalize`, but produced no call edges and no reason such as `receiver-type-unknown`. The CodeWiki adapter must supply its own resolution state and reason.

### E10 — Dataflow and CFG

[`DataStructures.java`](../fixtures/java/atlas-audit/arrays-lists-maps-loops-lambdas/src/main/java/audit/collections/DataStructures.java#L8-L24) yielded 49 `dataflow_edges` (`assign`, `read`, `return_value`, `arg_to_call`) and 16 CFG nodes/14 CFG edges. The CFG included `loop`, `branch`, `true_branch`, and `false_branch` with source ranges. The dataflow is useful evidence, but does not provide CodeWiki's operation taxonomy (for example, explicit map `put`/`get`, collection iteration, or lambda operation IR) or biography sentence labels.

### E11 — Incremental and deterministic output

Editing one method body in a four-file copy indexed one file, and its normalized fact tables equaled a clean rebuild. Adding the provider likewise produced normalized tables equal to a clean index. These are positive results for the tested cases only.

On two clean indexes of the same root, normalized fact tables (excluding time metadata) were equal. The raw SQLite database hashes differed because Atlas stores `files.index_time`, `project_metadata.last_index_time`, and `updated_at` values. The stable facts can be adapted, but the database file is not byte-identical across runs.

### E12 — Malformed file and diagnostics

[`Broken.java`](../fixtures/java/atlas-audit/malformed-file/src/main/java/audit/malformed/Broken.java#L3-L5) was stored with `files.status=partial`; [`Valid.java`](../fixtures/java/atlas-audit/malformed-file/src/main/java/audit/malformed/Valid.java#L3-L6) was stored with `status=success`. Indexing completed without a failed file, but the audited schema had no diagnostics table or persisted parse-error explanation.

### E13 — Generated sources

Atlas indexed both [`Ordinary.java`](../fixtures/java/atlas-audit/generated-sources/src/main/java/audit/generated/Ordinary.java#L3-L6) and [`Generated.java`](../fixtures/java/atlas-audit/generated-sources/target/generated-sources/generated/audit/generated/Generated.java#L3-L6). The generated file was not flagged as generated; policy to include/exclude it would need to live in CodeWiki discovery/configuration.

### E14 — Maven modules

The synthetic parent POM lists `api` and `service` modules ([pom.xml](../fixtures/java/atlas-audit/multi-module-maven/pom.xml#L8-L12)). Atlas found both Java files and emitted a call edge from [`Runner.run`](../fixtures/java/atlas-audit/multi-module-maven/service/src/main/java/audit/service/Runner.java#L5-L7) to `Values.increment`. The edge was `confidence=1.0`, `provenance=heuristic`, `resolved_by=name_only`. The Atlas database did not record Maven module/artifact ownership or a module dependency graph.

## Resource observations

Each row is a fresh process and database. `wall_ms` includes process startup; `pipeline_ms` is Atlas's own phase total. Peak working set was sampled every 10 ms, so it is approximate. These tiny synthetic fixtures are capability probes, not performance benchmarks; the PRD's benchmark thresholds are not evaluated here.

| Fixture | Files discovered | Symbols reported / stored | Pipeline ms | Process wall ms | Peak working set MiB |
|---|---:|---:|---:|---:|---:|
| `arrays-lists-maps-loops-lambdas` | 1 | 5 / 5 | 68 | 129 | 11.4 |
| `chained-call-unknown-return` | 1 | 2 / 2 | 65 | 130 | 11.3 |
| `chess-tour-missing-repos` | 1 | 3 / 3 | 74 | 137 | 11.2 |
| `fully-qualified-missing` | 1 | 2 / 2 | 75 | 130 | 9.8 |
| `generated-sources` | 2 | 4 / 4 | 75 | 139 | 11.2 |
| `later-added-signature-mismatch` | 2 | 4 / 4 | 84 | 134 | 10.6 |
| `malformed-file` | 2 | 4 / 4 | 65 | 116 | 11.3 |
| `multi-module-maven` | 2 | 4 / 4 | 68 | 124 | 11.5 |
| `overloads-inheritance` | 4 | 13 / 12 | 79 | 142 | 12.3 |
| `same-method-name-two-repos` | 2 | 4 / 4 | 82 | 141 | 11.4 |
| `two-wildcard-imports` | 1 | 2 / 2 | 64 | 122 | 11.3 |

## Recommended Backend adapter surface

Use only the public `atlas-engine` facade from `crates/backends`. The audit used the public `Store` and `IndexPipeline` surface; CodeWiki should wrap it behind `Backend` and copy facts into `codewiki.db`, not expose Atlas's SQLite schema to the UI or query layers.

The adapter should:

- Attach CodeWiki's engine name/version, source revision, repository ID, source evidence, and resolution state when ingesting Atlas facts. Atlas's database stores useful paths, ranges, confidence, provenance, and resolution strategy, but it does not provide all of CodeWiki's required provenance fields or exact resolution states.
- Recompute CodeWiki symbol IDs from repository, package, enclosing type, kind, name, and parameter types. Do not use Atlas's Java symbol IDs as canonical IDs until overload identity is fixed.
- Map Atlas `name_only`/`heuristic` edges to `HEURISTIC`, never `RESOLVED` based on numeric confidence alone. Preserve unresolved references and callsites; create CodeWiki external/unresolved stubs and candidate sets in the linker layer.
- Convert Atlas file `partial` status into CodeWiki diagnostics with file/range and a clear reason. Atlas only persisted the coarse status in this malformed-file test.
- Preserve byte and line ranges on facts/edges. For Atlas rows without edge locations (for example, containment), use the referenced symbol ranges and retain the edge provenance.
- Keep generated-source and Maven/module ownership policy in CodeWiki discovery. Atlas indexes those Java files but does not supply CodeWiki's project/repository model.
- Request a maintainer-fork fix for Java overload/package identity and feature forwarding before relying on Atlas for the required Java symbol/search behavior. Do not patch the root clone or vendor; the measured fixes and their exact fork permission belong to the maintainer workflow.

