# Atlas Java audit fixtures

These repositories contain synthetic Java source for the T1 Atlas capability audit. They are inputs to static indexing only; no fixture code or build script is executed. Each fixture directory is indexed independently unless its name explicitly contains multiple repositories or modules.

The pinned audit target is Atlas `v1.5.2` (`8021318e92f290e2cac915e4e91b514bf0a187d2`). The fixtures cover symbols, overloads and inheritance, internal calls, missing imports and external calls, data structures and operations, malformed syntax, generated sources, multi-repository names, Maven modules, later repository arrival, and incremental/deterministic output.

| Fixture | Scenario |
|---|---|
| `overloads-inheritance` | Overloads, inheritance, interface implementation, and in-project calls |
| `same-method-name-two-repos` | Same simple method name in two repository roots |
| `two-wildcard-imports` | Missing types behind competing wildcard imports |
| `fully-qualified-missing` | Fully qualified call with no declaration |
| `chess-tour-missing-repos` | Repository B calls into missing repositories A and C |
| `later-added-signature-mismatch` | Index client first, then add a provider with a different signature |
| `chained-call-unknown-return` | Chained call whose receiver has no indexed type |
| `arrays-lists-maps-loops-lambdas` | Arrays, lists, maps, loops, lambdas, and method references |
| `malformed-file` | One malformed Java file beside a valid file |
| `generated-sources` | Ordinary and build-directory generated Java files |
| `multi-module-maven` | Synthetic two-module Maven workspace |

