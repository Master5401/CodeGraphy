# CodeWiki work log

This is a factual handoff for the maintainer and later agents. The repository's `docs/PRD.md` v5.0 is the implementation source of truth. See [`REQUIREMENTS_REVIEW.md`](REQUIREMENTS_REVIEW.md) for the merge rationale and reviewed design input.

## 2026-11-12 — T0 scaffold

- Added a Cargo workspace with `cli`, `server`, `core`, `backends`, `linker`, `flow`, and `bio` crates, a Vite/React web scaffold, and developer recipes.
- No product features or engine adapters were present.
- T0 baseline/toolchain assumptions are recorded in [`DECISIONS.md`](DECISIONS.md).

## 2026-11-12 — T1 Atlas audit

- Audited Atlas v1.5.2, commit `8021318e92f290e2cac915e4e91b514bf0a187d2`, the baseline specified by PRD v4.
- Built a temporary release harness against the public `atlas-engine` library, asking for Java and disabling facade default features. Child-crate defaults still caused other grammar dependencies to compile. The build took 69 seconds; the harness executable was 28,988,416 bytes.
- Ran the harness only on synthetic Java fixtures. No Java/Maven/Gradle command, fixture code, target build script, or application was executed. The Atlas CLI, TUI, and MCP server were not built.
- Wrote [`ATLAS_GAP_MATRIX.md`](ATLAS_GAP_MATRIX.md), including fixture-by-fixture results and a proposed `Backend` adapter boundary. Highlights include a dropped overload and incorrect call target, an unresolved name becoming a false self-call, missing repository identity/stubs, generated-source ownership gaps, and non-byte-identical raw databases caused by time metadata.
- Did not change the root engine clones or `vendor/`. Atlas is **audited only**, not integrated into CodeWiki. Enola and Joern have not been used.
- Prior verification after T1: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint` in `web/`, and `npm test` in `web/` all passed. Rust workspace and web tests were scaffolding-level (0 Rust tests; one web scaffold test).

## Current implementation state — M1 source-checkout app

- `codewiki.ps1 <workspace>` and `./codewiki <workspace>` build the local web assets when needed, start the Rust CLI/server on loopback, open a browser, and catalog the selected folder. Omitting the folder opens a picker. This is a source-checkout developer workflow; a packaged release binary is not present.
- `crates/core/src/catalog.rs` walks eligible files with ignore rules, skips symlinks and `.codewiki/`, sorts paths, and stores metadata in `<workspace>/.codewiki/codewiki.db`. File IDs use BLAKE3 over stable workspace/path values.
- M1 format/language classification is based on filename extension, a small set of magic-byte signatures, and UTF-8/NUL detection. UTF-8 text up to 2 MiB per file is indexed with SQLite FTS5. Files up to 128 MiB can be hashed; source preview is capped at 1 MiB. These are catalog/search bounds, not parsers.
- The API provides workspace status/open, paginated file search/list, a lazily expanded repository directory tree, and safe source preview constrained to the selected root. The UI shows L0 metadata and L1 searchable text.
- **No parser or semantic engine is integrated.** There is no Tree-sitter grammar, AST, Java symbol/call graph, operation atom, English atom decomposition, biography, multi-repository linker, or generated documentation. Extension labels such as `java` are hints, not evidence that syntax was parsed.
- The app has only been exercised with synthetic fixtures. No separate target repository was supplied or indexed. The `.codewiki/` state created by the synthetic smoke run was removed afterward.
- The user selected combined v4/v3.2 scope: polyglot catalog/search and stronger Java semantic support. The current PRD v5.0 sequences semantic support and multi-repo linking after M1.
- M1 source-checkout behavior and required Rust/web checks are verified using synthetic fixtures. Next by PRD order is M2 evidence model + Java Atlas adapter; this remains dependent on maintainer-provided `vendor/atlas`.

## Tool usage truth

| Tool | Actual usage to date |
|---|---|
| Atlas | Temporary `atlas-engine` harness for the T1 Java fixture audit only. No dependency or adapter in CodeWiki. |
| Enola | Clone present; not run, evaluated, or integrated in this work. |
| Joern | Clone present; not run, evaluated, or integrated in this work. |

## 2026-11-12 — Requirements review and handoff

- Updated the README to describe the T1 state and requirements review.
- Added [`REQUIREMENTS_REVIEW.md`](REQUIREMENTS_REVIEW.md) with the v4/v3.2 conflict, current command/engine facts, a proposed-but-not-adopted atom and file-format mapping, multi-repo design notes, and primary references.
- No product code, fixtures, dependencies, or engine files changed in this documentation slice.
- Required checks after the documentation changes: `cargo fmt --all -- --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `cargo test --workspace` passed with 0 Rust tests; from `web/`, `npm run lint` passed and `npm test` passed 1 scaffold test.

## 2026-11-12 — PRD merge decision

- User chose “polyglot but more support for Java” and authorized combining both PRDs and making implementation decisions.
- Adopt v5.0: L0 file catalog and bounded L1 text search across the workspace from the first usable release; Java semantic depth first via Atlas; structured formats and other languages gain parser depth in staged adapters.
- Move the minimum runnable catalog/search/source-preview UI ahead of semantic graphs so the first product slice is useful from one command. Keep Java flows, multi-repo linking, Java operation biographies, incremental sync, broader adapters, Enola, optional Joern and packaging in ordered later milestones.
- Preserve the T1 findings, evidence/resolution requirements, offline/local behavior, target-code execution ban, deterministic outputs, vendor boundary, and maintainer ownership of engine submodule setup.

## 2026-11-12 — M1 catalog and repository tree

- Implemented the Rust catalog, SQLite FTS5 search, loopback API, source-checkout CLI, launcher scripts, and React/TypeScript UI described above.
- Added `/api/tree` and a lazy directory explorer so file browsing follows repository paths; file search remains a separate workspace-wide view.
- Added a synthetic catalog/tree fixture test and a web source check for the tree UI.
- Corrected the handoff: this is M1 catalog/search code, not a parser, Java analyzer, atom system, or biography engine. No target project was indexed.

### M1 verification — 2026-11-12

- Passed `cargo fmt --all -- --check`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo test --workspace` (4 Rust unit tests total; the remaining crates currently have no tests).
- Passed `npm run lint`, `npm test` (4 web source checks), and `npm run build` from `web/`.
- Passed `cargo deny check licenses` using cargo-deny 0.20.2 and the new `deny.toml` policy.
- Smoke-started `codewiki.ps1` with the synthetic polyglot fixture; the command built the UI, started CodeWiki on loopback, and opened the browser. Stopped the service and removed the fixture's generated `.codewiki/` database. No target repository was indexed.
- These checks validate the current source-checkout M1 implementation; they do not establish semantic parsing, Atlas integration, multi-repository linking, or packaged-release readiness.


## 2026-11-12 — CodeGraphy CLI naming and setup

- Renamed the built executable and user-facing browser/CLI brand to CodeGraphy. Added `init` as the primary command (with `scan` retained as an alias) and source-checkout launchers `codegraphy.ps1` / `codegraphy`.
- `codegraphy init <workspace>` catalogs files, runs the current Atlas Java structural profile, persists facts, and writes the JSON, Markdown, DOT, and Java source-path report outputs under `.codewiki/analysis/`.
- Documented support limits: cataloging is broader than semantic parsing; semantic/English operation definitions, Joern/Enola product integrations, and multi-repository binding remain incomplete.
- Verified release build emits `target/release/codegraphy.exe`; fixture `init` run produced 5 symbols and 2 relationships. Rust fmt/Clippy/tests and web lint/tests/build passed.
- Recommendation: clone the CodeGraphy application repository for development. A target repository already on disk can be analyzed by path; clone a target repo only when you need a local checkout of its remote contents. Atlas is fetched as a pinned Cargo dependency; Joern and Enola clones are not needed for the current command.

- Bare `codegraphy` now scans the current working directory and generates the same evidence-backed reports as `codegraphy init .`; `codegraphy ui` opens the browser app. Source-checkout wrappers with no arguments continue to open the UI.
