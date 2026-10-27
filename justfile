# Commands listed in CLAUDE.md. Stubs fail loudly until their task lands.

set shell := ["bash", "-euo", "pipefail", "-c"]

uv := "uv --directory data"
manifest := "--manifest-path core/Cargo.toml"

# One Rust build cache shared by the main checkout and every worktree, so a new
# worktree does not start cold. Override by setting CARGO_TARGET_DIR yourself.
export CARGO_TARGET_DIR := env_var_or_default("CARGO_TARGET_DIR", `git rev-parse --path-format=absolute --git-common-dir | xargs dirname` + "/core/target")

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
# table (T3.1), riwayah profiles and native layouts (T5.1, T5.2), compile build/mushaf.db
data-build:
    {{uv}} run python -m pipeline.codepoints check
    {{uv}} run python -m pipeline.spine build
    {{uv}} run python -m pipeline.ingest_layout build
    {{uv}} run python -m pipeline.dk_encoding build
    {{uv}} run python -m pipeline.corrections write
    {{uv}} run python -m pipeline.style_conversion write
    {{uv}} run python -m pipeline.dk_riwayat write
    {{uv}} run python -m pipeline.line_widths build
    {{uv}} run python -m pipeline.surah_names build
    {{uv}} run python -m pipeline.ingest_riwayah write
    {{uv}} run python -m pipeline.align write
    {{uv}} run python -m pipeline.emit_patches write
    {{uv}} run python -m pipeline.rule_mining write
    {{uv}} run python -m pipeline.ayah_maps write
    {{uv}} run python -m pipeline.profiles write
    {{uv}} run python -m pipeline.ayah_lines write
    {{uv}} run python -m pipeline.phase4_report write
    {{uv}} run python -m pipeline.drift write
    cargo run --release -q {{manifest}} -p mushaf-render-cli --bin mushaf-fitlines
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
    cargo check {{manifest}} --workspace --exclude mushaf-render-cli --exclude mushaf-db --exclude mushaf-roundtrip --exclude mushaf-wasm \
        --target wasm32-unknown-unknown --target aarch64-apple-ios --target aarch64-apple-ios-sim \
        --target aarch64-linux-android --target x86_64-linux-android
    cargo check {{manifest}} -p mushaf-wasm --target wasm32-unknown-unknown

# Patch round-trip vs KFGQPC for all riwayat in scope (D22): Rust resolver in data mode,
# then a codepoint-for-codepoint comparison with the pinned texts
roundtrip *RIWAYAT:
    #!/usr/bin/env bash
    set -euo pipefail
    riwayat="{{RIWAYAT}}"
    riwayat="${riwayat:-shuba qalun warsh bazzi qunbul susi duri_abuamr}"
    out="{{justfile_directory()}}/build/roundtrip"
    cargo build --release -q {{manifest}} -p mushaf-roundtrip
    mkdir -p "$out"
    # One process per riwayah, all at once; each writes only its own <riwayah>.jsonl
    pids=(); for r in $riwayat; do
        ( "$CARGO_TARGET_DIR/release/mushaf-roundtrip" "{{justfile_directory()}}/data" "$out" "$r" \
          && {{uv}} run python -m pipeline.roundtrip "$out" "$r" ) >"$out/$r.log" 2>&1 & pids+=($!)
    done
    fail=0; i=0; for r in $riwayat; do
        if wait "${pids[$i]}"; then tail -n 3 "$out/$r.log"; else echo "FAIL $r:"; cat "$out/$r.log"; fail=1; fi; i=$((i+1))
    done
    exit $fail

# HTML review sheets for the owner (T4.8): build/review/phase4/<riwayah>.html, not tracked
review-sheets *RIWAYAT:
    {{uv}} run python -m pipeline.review_sheets {{RIWAYAT}}

# Phase 5 review sheets: build/review/phase5/ (encoding candidates, G-100 marks), not tracked
review-phase5:
    {{uv}} run python -m pipeline.dk_riwayat write
    {{uv}} run python -m pipeline.review_phase5

# GATE G3 contact sheets (T5.5): every page of every riwayah, 24 per sheet, in
# docs/review/phase5/contact/ (untracked PNG), then the tracked review index
contact-sheets *RIWAYAT:
    cargo run --release -q {{manifest}} -p mushaf-render-cli --bin mushaf-contact -- {{RIWAYAT}}
    {{uv}} run python -m pipeline.phase5_readme write

# T6.5: rule review sheets, build/review/phase6/rules-<name>.html (not tracked), and the
# tracked summary docs/review/phase6/rules.md
review-rules *NAMES:
    {{uv}} run python -m pipeline.rule_sheets {{NAMES}}

# al-Budur pilot (TB.6): rule index from the whole book, statuses and two-way comparison for
# al-Fatihah and al-Baqarah's first rub'. Writes build/budur/ (untracked: it holds book text).
# The book lives only in the main checkout: BOOKS_DIR=<main>/data/sources/raw/books
budur-verify:
    {{uv}} run python -m pipeline.budur.run

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
    cargo run --release --quiet {{manifest}} -p mushaf-render-cli --bin mushaf-render -- --page "$PAGE" --riwayah "$RIWAYAH"

# Golden image + line-fit + shaping-integrity tests (release; all 604 pages), the web bundle
# (T8.2) and the engine session (T8.1)
golden:
    cargo test --release {{manifest}} -p mushaf-render-cli --test golden --test bundle -- --ignored
    cargo test --release {{manifest}} -p mushaf-engine --test session -- --ignored

# Rewrite tests/golden/pages after a reviewed, intended change (then commit the diff)
golden-update:
    MUSHAF_UPDATE_GOLDEN=1 cargo test --release {{manifest}} -p mushaf-render-cli --test golden -- --ignored

# Page resolve+shape+justify timing (release, this machine; T3.6)
bench:
    cargo run --release --quiet {{manifest}} -p mushaf-render-cli --bin mushaf-bench

# T8.1: the core as a WebAssembly package in build/web/pkg (wasm-bindgen-cli 0.2.129, D58)
web-wasm:
    cargo build --release -q {{manifest}} -p mushaf-wasm --target wasm32-unknown-unknown
    wasm-bindgen --target web --out-dir build/web/pkg "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/mushaf_wasm.wasm"

# T8.2: the static data bundle build/web/data/mushaf.bundle + manifest.json (D57, untracked)
web-data:
    cargo run --release -q {{manifest}} -p mushaf-render-cli --bin mushaf-bundle

# T8.1 acceptance: every golden page drawn by the WASM engine in Node is byte-identical
web-parity: web-wasm web-data
    node apps/web/tests/parity.mjs

# Web app dependencies (Vite, TypeScript, Playwright with its Chromium); once per checkout
web-setup:
    npm --prefix apps/web ci --no-audit --no-fund
    cd apps/web && npx playwright install chromium

# The site as static files in apps/web/dist (app, WASM, data bundle); not deployed anywhere
web-build: web-wasm web-data
    npm --prefix apps/web run build

# Local dev server on http://127.0.0.1:5173
web-dev: web-wasm web-data
    npm --prefix apps/web run dev

# T8.3: Playwright functional tests of every screen, offline cache included
web-test: web-wasm web-data
    npm --prefix apps/web run typecheck
    cd apps/web && npx playwright test tests/app.spec.ts

# T8.4: page-turn budget (16 ms) in Chromium with the CPU throttled 4x
web-bench: web-wasm web-data
    cd apps/web && npx playwright test tests/bench.spec.ts

# T8.5: the app's drawing of every core golden page against the golden PNGs
web-golden: web-wasm web-data
    cd apps/web && npx playwright test tests/golden.spec.ts

# All web checks (part of `just gate`); the budget measurement `web-bench` is separate, like `bench`
web-gate: web-parity
    npm --prefix apps/web run typecheck
    cd apps/web && npx playwright test tests/app.spec.ts tests/golden.spec.ts

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

# Per-commit check: lint, Rust tests, Python tests
gate-fast: lint core-test data-test

# Full gate, required before a push, a merge or a GATE. The Python checks run alongside the
# Rust chain; each step's time is printed, and logs are kept in build/gate/.
gate:
    #!/usr/bin/env bash
    set -euo pipefail
    logs="{{justfile_directory()}}/build/gate"; mkdir -p "$logs"
    step() { local s=$SECONDS; if just "$1" >"$logs/$1.log" 2>&1; then echo "ok   $1 ($((SECONDS-s))s)"; else echo "FAIL $1 ($((SECONDS-s))s), see $logs/$1.log"; return 1; fi; }
    ( step data-test && step data-verify ) & py=$!
    rust=0; for s in lint core-test core-targets golden roundtrip web-gate; do step "$s" || { rust=1; break; }; done
    wait $py || rust=1
    exit $rust

