# Atlas + Joern integration proof

This is the authorized proof gate, not the CodeWiki product implementation. It uses one synthetic Java fixture. Atlas and Joern parse the Java source statically; for the bytecode check, `javac` compiles this synthetic source to `.class`, and Joern parses that output. Neither the source nor the generated class is executed, and no fixture build scripts are run. No engine checkout or `vendor/` content was edited.

## Result

The proof ran real provider APIs and kept engine evidence in the outputs:

| Provider | Integration surface | Observed result |
| --- | --- | --- |
| Atlas `atlas-engine` 1.7.0 (`0f8379f`) | Public Rust facade, `Engine::extract_file_with_mode(..., Language::Java, ExtractionMode::Full)` | 5 symbols, 1 reference, 1 callsite, 8 CFG nodes / 6 CFG edges, 18 data nodes / 21 dataflow edges, 0 diagnostics |
| Joern `4.0.681+5-427508c9` (`427508c`) | `joern-parse --language JAVASRC`, then `joern-export --repr all --format neo4jcsv` | 108 exported nodes, 368 edges; includes CALL, CFG, DOMINATE, and REACHING_DEF facts |
| Joern JVM bytecode | JDK 21 `javac` on the synthetic fixture, then the staged `jimple2cpg` frontend and Neo4j CSV export | 97 nodes, 419 edges; contains `total`, `addFee`, the call, CFG, control-dependence and reaching-definition facts. Details and bytecode limitations are in [`results/bytecode/analysis.md`](results/bytecode/analysis.md). |

The versioned proof IR in [`results/normalized-facts.json`](results/normalized-facts.json) maps provider facts to entities, operations, and relationships. It retains provider labels and IDs, source paths/lines, resolution state, and provider version. Atlas ranges are zero-based in its raw payload; the normalized display line fields convert them to one-based lines while preserving the raw range. Joern's implicit `this` receiver is omitted from the explicit-argument list, while the original call `CODE` and provider IDs remain available.

The fixture's `total` method multiplies `quantity` by `unitPrice`, assigns the result to `subtotal`, calls `addFee(subtotal)`, and returns that call result. Joern's CPG CALL edge reaches the exact `addFee` METHOD node. Atlas's single-file facade emits the `addFee` reference but leaves repository-level resolution unavailable; that evidence remains `UNRESOLVED` in the normalized output. `addFee`'s operations are reported separately from `total`'s direct behavior. Flow edges retain `PROVIDER_REPORTED` state; they are not presented as independently verified claims.

Read the deterministic rendered example in [`results/method-biography.md`](results/method-biography.md), the full Atlas payload in [`results/atlas-file-facts.json`](results/atlas-file-facts.json), and the selected Joern Neo4j CSV outputs under [`results/joern-export/`](results/joern-export/).

## Reproduce

Run from the repository root in PowerShell. The Atlas checkout must be at `./atlas` at the recorded commit. The staged Joern CLI must be built at `./joern/joern-cli/target/universal/stage/bin`.

```powershell
cargo run --manifest-path proofs/engine-integration/atlas-harness/Cargo.toml -- `
  proofs/engine-integration/fixture/src/main/java/proof/PriceFlow.java `
  proofs/engine-integration/results/atlas-file-facts.json

joern/joern-cli/target/universal/stage/bin/joern-parse.bat `
  --language JAVASRC `
  --output proofs/engine-integration/results/joern-cpg.bin `
  proofs/engine-integration/fixture

joern/joern-cli/target/universal/stage/bin/joern-export.bat `
  --repr all --format neo4jcsv `
  --out proofs/engine-integration/results/joern-export `
  proofs/engine-integration/results/joern-cpg.bin

$javac = Join-Path $env:JAVA_HOME 'bin/javac.exe'
& $javac -g -d proofs/engine-integration/results/bytecode/classes `
  proofs/engine-integration/fixture/src/main/java/proof/PriceFlow.java
joern/joern-cli/target/universal/stage/frontends/jimple2cpg/bin/jimple2cpg.bat `
  --enable-file-content `
  --output proofs/engine-integration/results/bytecode/joern-bytecode-cpg.bin `
  proofs/engine-integration/results/bytecode/classes
joern/joern-cli/target/universal/stage/bin/joern-export.bat `
  --repr all --format neo4jcsv `
  --out proofs/engine-integration/results/bytecode/joern-export `
  proofs/engine-integration/results/bytecode/joern-bytecode-cpg.bin

python proofs/engine-integration/normalize.py
```

The checked-in Joern metadata CSV replaces its machine-specific `ROOT` with `${FIXTURE_ROOT}`; graph rows are unmodified. The binary CPG is omitted because the CSV and normalized facts are the review artifacts. Re-running Joern regenerates it. Atlas JSON and normalized output are deterministic for the recorded engine versions and fixture.

## Build and scope notes

- Environment used: Windows x64, Java 21.0.13, sbt 2.0.11, Rust toolchain from the repository, and Python 3.13. Java and sbt archives were verified against published SHA-256 values before use.
- Joern's `joerncli/stage` build succeeded in 19m28s. Its sbt aggregation compiled/fetched test and code-generation dependencies for multiple frontends, despite this proof using only JAVASRC. That build breadth is a real setup cost and should inform packaging decisions.
- Atlas harness compiled and ran successfully. The facade proves Java file extraction for this fixture; it does not prove repository-wide linking, full multi-file resolution, `.class` handling, or every Atlas capability claim in this isolated run.
- The Joern bytecode frontend is proven to ingest this compiled fixture, with limitations documented in its report. Atlas incremental synchronization, Enola architecture facts, and multi-repository linking remain open evaluation items; no claims are made for them here.
- No handwritten parser, resolver, CFG, or dataflow engine was introduced. `normalize.py` maps provider records and does not inspect Java source to discover syntax; fixture lines are copied only as evidence text.

## Review gate

Stop here for review. Do not proceed to full CodeWiki implementation, additional language adapters, Enola integration, or productized engine packaging until the user reviews this proof.
