# Decisions

## 2026-11-12 — T0 baseline

- Use Rust 1.99.1, the installed edition-2024-capable toolchain, and Node.js 22.17.1 for the scaffold. Vite's current guide requires Node 20.19+ or 22.12+; the selected Node version satisfies that requirement.
- Use npm with a committed lockfile for the web scaffold because the PRD requires web lint/test scripts but does not require pnpm.
- Keep all Rust crates feature-free in T0. No analysis engine is linked yet.
- Leave `vendor/atlas` and `vendor/enola` for maintainer bootstrap; their fork URLs and credentials are not supplied, and the PRD reserves submodule setup for the human.

## 2026-11-12 — T1 Atlas audit baseline

- Audit Atlas `v1.5.2` at commit `8021318e92f290e2cac915e4e91b514bf0a187d2`, as specified by the repository PRD. Use a temporary source archive from the existing root clone and leave the clone and `vendor/` unchanged because vendor/submodule setup remains maintainer work.
- Build only the `atlas-engine` library surface with Java requested. The measured Cargo feature graph still compiles other grammars through child-crate defaults; record that as an Atlas gap instead of patching engine manifests outside the maintainer's fork.
- Keep the attached v3.1 operation taxonomy as later design input. The selected repository PRD v4 keeps this task Java-first and schedules biographies/operation IR after the Atlas audit; do not add cross-language grammar features during T1.
- Use only synthetic Java audit fixtures. Index them statically and never run their build files or source code.

## 2026-11-12 — Review of supplied v3.2 requirements

- Record the supplied polyglot PRD and build specification as reviewed design input in `docs/REQUIREMENTS_REVIEW.md`; the user also requested a factual README/work log.
- The user resolved the scope choice as “polyglot but more support for Java” and authorized combining the PRDs and making decisions.
- Adopt `docs/PRD.md` v5.0: universal catalog and bounded text search across file types first, deeper Java semantic analysis through Atlas, then flows, missing-code/multi-repo linking, Java operation biographies, incremental sync, broader adapters, and optional Enola/Joern integrations.
- Put the minimal one-command catalog/search/source-preview UI first so the initial usable slice works end-to-end. Keep Atlas T1 gaps as explicit adapter constraints. Leave `vendor/` and root-level engine clones untouched; Atlas integration requires maintainer-provided submodule setup.
- Reason: this reconciles the v3.2 language/file-format-neutral entry point with the v4 evidence-backed Java analysis strategy and the user's explicit Java priority, while keeping the first release useful and each milestone testable.

## 2026-11-12 — M1 implementation boundaries

- Use React + TypeScript + Vite as required by the merged PRD §13 and supplied build specification. TypeScript provides build-time type checking; it compiles to JavaScript and does not add a slower runtime layer. It does add frontend build tooling and a build step.
- Keep indexed file paths relative to the selected workspace and expose a lazily expanded directory tree in the explorer. Search results and generated pages retain source paths; generated documentation is not written into or used to rearrange the analyzed repository.
- For generated Markdown in later milestones, mirror the repository-relative path under `.codewiki/docs/<repository-name>/` and append `.md` to the source filename. Cross-repository linking will regenerate affected pages in both trees; source files and existing docs are not rewritten.
- M1 format detection uses extension hints, a small set of magic-byte signatures, and UTF-8/binary detection. It does not claim language grammar, AST, symbol, operation, or biography support. Java syntax atoms and English templates remain M5 work after M2 Atlas evidence normalization.
- Treat each selected workspace as one repository in M1. Cross-repository identity matching, reciprocal links, and page regeneration remain M4 work.
- Reason: this keeps the first source-checkout app honest and useful while respecting the PRD's milestone order and evidence requirements.

## 2026-11-12 — Engine proof gate (user-authorized exception)

- Before any broader semantic implementation, run a proof-only synthetic Java fixture through the public Atlas facade and Joern, normalize real provider outputs into a compact evidence-preserving IR, include dataflow relationships, and render one deterministic biography.
- This proof-only gate overrides PRD §5/§16's later optional Joern timing for this isolated evaluation. It does not authorize building the full product, editing engine checkouts or `vendor/`, or replacing unavailable engine capabilities with custom parsing.
- Enola remains an architecture-level provider and is outside this first Atlas+Joern fixture proof; evaluate it only after the proof is reviewed or the user expands this gate.
- Evaluate JVM bytecode using Joern's separate `jimple2cpg` frontend on a `.class` compiled from the same synthetic Java fixture. Keep this evidence separate from source parsing because Jimple can fold source locals and its displayed lines are not guaranteed source ranges.
- Reason: the user explicitly requires reviewing actual dual-engine evidence before authorizing complete CodeWiki implementation.

## 2026-11-12 — Atlas Java scan slice

- Pin the product's `atlas-engine` dependency to commit `0f8379ff61a6f5ea1ae888d331f21b8b44f6a5b7`, with only its Java feature enabled. Run Atlas structural analysis from the terminal `scan` command; preserve its raw facts and normalized evidence records in SQLite and source-shaped reports under `.codewiki/analysis/`.
- Keep Atlas `NameOnly`, fuzzy, and heuristic call edges labelled `HEURISTIC`; render them as dashed candidates. Do not promote them to resolved calls. Keep unresolved references with source evidence and a reason.
- Use rusqlite 0.39 in CodeGraphy so its bundled SQLite native library matches Atlas's pinned `libsqlite3-sys` dependency. Cargo could not link the previous rusqlite 0.42 selection because it required a second incompatible `sqlite3` native link version.
- Reason: the user asked for a working terminal scan, generated code details, and call graph while keeping Atlas as the analysis provider. The current slice is Java structural analysis only; other formats remain cataloged, and Joern/Enola are not invoked by this command.
