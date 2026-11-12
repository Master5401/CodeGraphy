# CodeGraphy

CodeGraphy is a local-first codebase catalog with an early Java analysis slice. It currently produces evidence-backed scan reports from Atlas; it does **not** yet produce full English method biographies or cross-repository documentation.

## What works today

- Running `codegraphy` with no arguments scans the current directory. `codegraphy init <workspace>` scans a selected directory. Both catalog supported files, analyze Java source through pinned Atlas, persist results locally, and write a Markdown summary, JSON facts, a Graphviz DOT call graph, and source-path-shaped Java fact pages.
- The catalog discovers files using Git ignore rules, identifies file types using metadata and selected signatures, indexes eligible UTF-8 text up to 2 MiB per file, and supports source browsing/search in the local UI.
- Atlas call matches tagged `HEURISTIC` stay heuristic. Facts include source evidence and provider provenance. Target source and build scripts are never executed.
- Repeat scans reuse Atlas's local cache for unchanged files.

## Current capability: honest coverage

| Area | Status |
|---|---|
| File discovery and catalog | Implemented for eligible files; this is not a semantic parser for every format. |
| Text search and source preview | Implemented for bounded readable UTF-8 files. Binary files are identified, not decoded semantically. |
| Java symbols and structural call candidates | Implemented through Atlas Java. The current CodeGraphy profile does not import Atlas CFG/dataflow. |
| Other programming languages | Cataloged where possible; semantic extraction is not enabled yet. |
| English explanations / operation atoms | Not implemented. Current Markdown states extracted facts and evidence; it does not invent behavior from names. |
| Joern | Not integrated into the product command. |
| Enola | Not integrated into the product command. |
| Multi-repository binding | Not implemented. Each `init` currently analyzes one workspace root. |
| Docs mirroring | Current reports mirror Java source paths under `.codewiki/analysis/files/`; the full repo documentation system is not implemented. |

The `.codewiki/` directory is the existing on-disk state name retained for compatibility. It contains the catalog database, Atlas cache, and analysis reports. It is safe to remove if you want a clean re-index; source files are not modified.

## Quick start from a clone

Clone the **CodeGraphy application repository** once. A target repository that is already on disk does not need to be cloned again; pass its local path. If the target exists only on a Git remote, clone that target locally first, then analyze it.

```sh
git clone <CodeGraphy-Git-URL> CodeGraphy
cd CodeGraphy
```

Use the full application checkout for development and browser UI assets; the repository currently has no configured remote URL to put in that command. Once the release CLI is built and on `PATH`, you can run it from target repositories without returning to the checkout.

Prerequisites:

- Rust toolchain specified in `rust-toolchain.toml` (including Cargo, rustfmt, and Clippy)
- Node.js 22.12+ and npm, needed for the browser UI
- Internet for the first dependency fetch (Cargo fetches the pinned Atlas engine; npm installs the pinned UI dependencies). Scanning and searching are local afterward.

### Windows PowerShell

From the CodeGraphy checkout:

```powershell
# Generate the current scan reports for a repository
.\codegraphy.ps1 init "C:\Projects\my-app"

# Or analyze the included synthetic fixture
.\codegraphy.ps1 init .\fixtures\polyglot-workspace

# Open the local browser app (select a workspace in the app)
.\codegraphy.ps1
```

The direct Rust command avoids the web build and is useful for terminal-only scans:

```powershell
cargo run -p codewiki-cli -- init "C:\Projects\my-app"
```

A built executable named `codegraphy`, run with no arguments from a repository folder, scans that current directory. Open the browser with `codegraphy ui`; the no-argument `codegraphy.ps1` checkout launcher also opens the UI.

### macOS / Linux

From the CodeGraphy checkout:

```sh
sh ./codegraphy init /path/to/my-app
sh ./codegraphy                 # build/open the local browser app
```

### Build a `codegraphy` executable

From the checkout, build the standalone CLI executable:

```sh
cargo build --release -p codewiki-cli
```

The executable is `target/release/codegraphy.exe` on Windows or `target/release/codegraphy` on macOS/Linux. Add that directory to your `PATH`, open a new terminal, then run:

```text
codegraphy init <path-to-repository>

# Or change into the repository and run with no arguments
codegraphy
```

For a temporary Windows PowerShell session, prepend the build folder with `$env:Path = "$PWD\target\release;$env:Path"`. On macOS/Linux use `export PATH="$(pwd)/target/release:$PATH"`.

This installed CLI supports terminal cataloging/analysis. To use the browser UI from a source checkout, use `codegraphy.ps1` or `sh ./codegraphy`, which also builds the web assets. A packaged GUI installer is not available yet.

## Outputs

By default, `init` writes to `<workspace>/.codewiki/analysis/`:

- `report.md` — provider, coverage counts, and output guide
- `facts.json` — normalized facts plus raw Atlas evidence
- `call-graph.dot` — solid resolved edges and dashed heuristic candidates
- `files/<source-relative-path>.java.md` — symbols and evidence associated with that Java file

For a different output directory:

```text
codegraphy init <workspace> --output <directory>
```

To render the DOT file as SVG, install Graphviz separately and run `dot -Tsvg .codewiki/analysis/call-graph.dot -o call-graph.svg` from the workspace. Graphviz is optional; the DOT file is plain text.

## How the engines are used

- **Atlas:** pinned as a Rust dependency and invoked by the `init` scan for Java structural extraction. It supplies symbols, references and call relationship candidates used by the current reports. Its heuristic resolution is preserved as such.
- **Joern:** not currently invoked by CodeGraphy. The repository contains a synthetic engine proof; that is separate from a product integration.
- **Enola:** not currently invoked by CodeGraphy. Architecture graphs and cross-repository links are future work.

Do not clone Atlas, Joern, or Enola to run the current terminal scan. Cargo fetches Atlas on first build. Their repository checkouts are only needed for engine development/evaluation.

## Multi-repository work

Multi-repository identity and reciprocal documentation updates are not available yet. For now, run `codegraphy init` separately on each local repository; their indexes remain separate and CodeGraphy will not claim that same-named symbols are the same entity. The planned linker needs explicit repository identities, evidence-backed cross-repo edges, and regeneration of both documentation trees before safe reciprocal links can be offered.

## Development checks

From the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check licenses
```

From `web/`:

```powershell
npm ci
npm run lint
npm test
npm run build
```

See `AGENTS.md` for product/engineering rules, `docs/DECISIONS.md` for implementation decisions, and `docs/WORK_LOG.md` plus `docs/ATLAS_GAP_MATRIX.md` for development evidence and provider limitations.
