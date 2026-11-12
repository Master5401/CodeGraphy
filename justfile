run workspace=".":
    npm --prefix web run build
    cargo run -p codewiki-cli -- "{{workspace}}"

dev:
    npm --prefix web run dev

test:
    cargo test --workspace
    npm --prefix web test

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    npm --prefix web run lint

build-release:
    cargo build --workspace --release
    npm --prefix web run build

init workspace=".":
    cargo run -p codewiki-cli -- init "{{workspace}}"
