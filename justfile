# Commands listed in CLAUDE.md. Stubs fail loudly until their task lands.

set shell := ["bash", "-euo", "pipefail", "-c"]

uv := "uv --directory data"
manifest := "--manifest-path core/Cargo.toml"

_default:
    @just --list --unsorted

# Download pinned sources, verify checksums
data-fetch:
    {{uv}} run python -m pipeline.fetch fetch

# Record hashes for sources not yet pinned (never changes an existing hash)
data-pin:
    {{uv}} run python -m pipeline.fetch pin

# Dated private backup of raw sources + manifest (outside git), default ~/mushaflab-archive
data-archive *DEST:
    {{uv}} run python -m pipeline.archive {{DEST}}

# Run the full ETL: check codepoint table, rebuild spine + layout files and the font-encoding
# table (T3.1), compile build/mushaf.db
data-build:
    {{uv}} run python -m pipeline.codepoints check
    {{uv}} run python -m pipeline.spine build
    {{uv}} run python -m pipeline.ingest_layout build
    {{uv}} run python -m pipeline.dk_encoding build
    {{uv}} run python -m pipeline.line_widths build
    {{uv}} run python -m pipeline.surah_names build
    {{uv}} run python -m pipeline.compile_db

# Integrity tests on spine, layout, ayah maps (needs `just data-build` first)
data-verify:
    {{uv}} run python -m pipeline.verify

# Python pipeline unit tests
data-test:
    {{uv}} run pytest

# Compile the synthetic fixture root into build/fixtures/synthetic.db (for the T2.2 parity test)
core-fixtures:
    {{uv}} run python -m pipeline.compile_db --root "{{justfile_directory()}}/tests/fixtures/synthetic" "{{justfile_directory()}}/build/fixtures/synthetic.db"

# cargo test --workspace
core-test: core-fixtures
    cargo test {{manifest}} --workspace

# Check the core builds for every platform target (web now, iOS/Android later).
# mushaf-db (bundled SQLite) and the CLI are host-only (D12, G-032).
core-targets:
    for t in wasm32-unknown-unknown aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android x86_64-linux-android; do \
        cargo check {{manifest}} --workspace --exclude mushaf-render-cli --exclude mushaf-db --target $t || exit 1; \
    done

# Patch round-trip vs KFGQPC for all riwayat in scope
roundtrip: (_todo "T4.6")

# SVG + PNG via reference renderer, e.g. just render PAGE=1 RIWAYAH=hafs
render *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    PAGE=1; RIWAYAH=hafs
    for arg in {{ARGS}}; do
        case "$arg" in
            PAGE=*) PAGE="${arg#PAGE=}" ;;
            RIWAYAH=*) RIWAYAH="${arg#RIWAYAH=}" ;;
            *) echo "render: unknown argument '$arg' (expected PAGE=<n> RIWAYAH=<id>)" >&2; exit 2 ;;
        esac
    done
    cargo run --release --quiet {{manifest}} -p mushaf-render-cli -- --page "$PAGE" --riwayah "$RIWAYAH"

# Golden image + line-fit + shaping-integrity tests (release; all 604 pages)
golden:
    cargo test --release {{manifest}} -p mushaf-render-cli --test golden -- --ignored

# Rewrite tests/golden/pages after a reviewed, intended change (then commit the diff)
golden-update:
    MUSHAF_UPDATE_GOLDEN=1 cargo test --release {{manifest}} -p mushaf-render-cli --test golden -- --ignored

# Page resolve+shape+justify timing (release, this machine; T3.6)
bench:
    cargo run --release --quiet {{manifest}} -p mushaf-render-cli --bin mushaf-bench

# rustfmt/clippy, ruff, license check
lint:
    cargo fmt {{manifest}} --all --check
    cargo clippy {{manifest}} --workspace --all-targets -- -D warnings
    {{uv}} run ruff check .
    {{uv}} run ruff format --check .
    {{uv}} run python -m pipeline.license_check "{{justfile_directory()}}"

_todo task:
    @echo "not implemented yet ({{task}})" >&2
    @exit 1
