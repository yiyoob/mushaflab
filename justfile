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

# Run the full ETL: check codepoint table, rebuild spine + layout files, compile build/mushaf.db
data-build:
    {{uv}} run python -m pipeline.codepoints check
    {{uv}} run python -m pipeline.spine build
    {{uv}} run python -m pipeline.ingest_layout build
    {{uv}} run python -m pipeline.ingest_riwayah write
    {{uv}} run python -m pipeline.align write
    {{uv}} run python -m pipeline.emit_patches write
    {{uv}} run python -m pipeline.ayah_maps write
    {{uv}} run python -m pipeline.phase4_report write
    {{uv}} run python -m pipeline.drift write
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
# mushaf-db (bundled SQLite), the CLI and the round-trip tool are host-only (D12, D22, G-032).
core-targets:
    for t in wasm32-unknown-unknown aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android x86_64-linux-android; do \
        cargo check {{manifest}} --workspace --exclude mushaf-render-cli --exclude mushaf-db --exclude mushaf-roundtrip --target $t || exit 1; \
    done

# Patch round-trip vs KFGQPC for all riwayat in scope (D22): Rust resolver in data mode,
# then a codepoint-for-codepoint comparison with the pinned texts
roundtrip *RIWAYAT:
    #!/usr/bin/env bash
    set -euo pipefail
    riwayat="{{RIWAYAT}}"
    riwayat="${riwayat:-shuba qalun warsh bazzi qunbul susi duri_abuamr}"
    out="{{justfile_directory()}}/build/roundtrip"
    cargo run --release -q {{manifest}} -p mushaf-roundtrip -- "{{justfile_directory()}}/data" "$out" $riwayat
    {{uv}} run python -m pipeline.roundtrip "$out" $riwayat

# HTML review sheets for the owner (T4.8): build/review/phase4/<riwayah>.html, not tracked
review-sheets *RIWAYAT:
    {{uv}} run python -m pipeline.review_sheets {{RIWAYAT}}

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
    echo "render PAGE=$PAGE RIWAYAH=$RIWAYAH: not implemented yet (T3.4)" >&2
    exit 1

# Golden image + line-fit + shaping-integrity tests
golden: (_todo "T3.5")

# Page resolve+shape+justify timing
bench: (_todo "T3.6")

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
