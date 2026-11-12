# Third-party notices

This repository includes direct and transitive Rust and web dependencies. Exact versions are pinned in `Cargo.lock` and `web/package-lock.json`.

## Rust

Direct workspace dependencies include `anyhow`, `axum`, `blake3`, `clap`, `getrandom`, `ignore`, `rusqlite` (bundled SQLite), `serde`, `serde_json`, `tokio`, `tower-http`, and `webbrowser`. `deny.toml` records the approved SPDX allowlist and excludes unpublished internal workspace crates whose license has not been selected. Atlas `atlas-engine` v1.7.0 is pinned to commit `0f8379ff61a6f5ea1ae888d331f21b8b44f6a5b7` and distributed under its upstream MIT license. CodeGraphy uses `rusqlite` 0.39 to share Atlas's `libsqlite3-sys` version; the prior 0.42 selection could not link with Atlas's native SQLite dependency. `cargo deny check licenses` and the Atlas proof harness license check passed on 2026-11-12 with cargo-deny 0.20.2. Review license changes when updating either lockfile.

## Web

Direct runtime dependencies are React and React DOM. Development dependencies include TypeScript and Vite. `web/package-lock.json` records the resolved transitive graph. Preserve upstream license texts in dependency packages when redistributing a packaged release; do not copy dependency source into this repository without its notices.

## Engine sources

Atlas is linked through its public `atlas-engine` crate at the pinned commit. Preserve upstream license and notice files when redistributing builds. Enola and Joern remain evaluation-only, not product dependencies; preserve their upstream notices if maintainers later add them.
