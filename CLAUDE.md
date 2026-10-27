# CLAUDE.md — Multi-Qirāʾāt Muṣḥaf

Read this first. Then read `PROJECT_PLAN.md`, but only the phase currently in progress.

## What this project is
A muṣḥaf (website first, then native iOS/Android) with fixed-line pages, runtime-changeable qirāʾāt, and calligraphic justification.
Everything is changeable at the cluster level: any letter or mark can be added, removed, replaced or resized.
Stage 1 covers the 8 riwāyāt for which KFGQPC publishes full text, in the **15-line Madinah ʿUthmānī script only**. Indo-Pak (15- and 13-line) and Stage 2 (the other 12 riwāyāt) are out of scope until told otherwise. Do not build Indo-Pak features, but do not hardcode "one script, one layout" either: `script_id` and `layout_id` are explicit everywhere (see PROJECT_PLAN §0).

## Non-negotiable rules
1. **Never write, edit, "fix" or generate Qurʾānic text or qirāʾāt data from memory.**
   Text comes only from pinned source files, or from patches that carry provenance. If data is missing or a diff looks wrong, stop and report it. Do not guess.
   This includes **test expectations**: an expected value about the Qurʾān (a reading, an ayah boundary, a numbering shift, a count) must cite a source (file + location, or book + page). Without a citation it is marked `hypothesis` and may not be used as a passing criterion until the owner confirms it.
2. **Never modify files under `data/sources/raw/`.** They are checksummed in `data/sources/MANIFEST.json`. Verify checksums before every pipeline run.
3. **Source of truth is text in git** (spine, patches, rules, option groups as JSON/YAML). `mushaf.db` is a build artifact. Never hand-edit it.
4. **Every patch and rule has a `source` and a `status`**: `raw-derived`, `rule-derived`, `cross-checked`, `muqri-verified`. Only the owner (Yusuf) may set `cross-checked` or `muqri-verified`. These two statuses are never written into patch/rule files: they live only in `data/approvals/`, which Claude never edits. Changes there are made only on the owner's explicit instruction (the signing guard was removed by the owner on 2026-10-26).
   Data that only one independent source supports is additionally tagged `single-source`.
5. **One edition per stack.** Spine text, line layout and font for a given `layout_id` must come from the same printed edition. Never pair a text from one printing with a layout or font from another without an ADR and a word-level diff in `known-gaps.md`.
6. **No silent rendering failure.** Any `.notdef` glyph, inserted dotted circle (U+25CC), or mark without an anchor attachment is a hard test failure, never a warning.
7. **One resolver.** All resolution, shaping and justification logic lives in `core/`. Platform apps only draw what the core returns and never reimplement logic.
8. **Deterministic and pure.** The same inputs (spine, patch stack, options, font) always give byte-identical output on every platform. Layout and justification arithmetic in `core/` uses fixed-point integers (font units / 26.6-style), not floats. No clocks, no randomness, no network at runtime.
9. **No network at runtime in the apps.** Network is allowed only in `data/` scripts, and only to domains listed in `data/sources/MANIFEST.json`.
10. **Respect licenses.** Do not copy fonts, text or layout data into any tracked location until its row in `data/sources/LICENSES.md` says redistribution is allowed. Third-party reference repos are cloned to `/tmp`, never vendored.
11. **Free mode is a feature, not a bug.** The engine must never refuse an edit because it is non-canonical. A separate lint layer flags it. Any page in a non-canonical state carries a visible "edited" marker in every render and export; it cannot be switched off.

## Working method
- Enter plan mode at the start of each phase. State the task IDs you will do and the acceptance command for each.
- Do one task at a time. Run its acceptance command. Commit with message `T<id>: <summary>`.
- Before each commit run `just gate-fast` (lint, Rust tests, Python tests). Run the full gate (`just gate`: all platform targets, goldens, round-trip) only once per phase, right before the phase's final push. Do not re-run it for later doc-only or status changes.
- Test first for anything in `core/` and the data pipeline.
- Phases run in the order given in PROJECT_PLAN §5 (Phase 4 before Phase 3), not in numeric order.
- Stop at every `GATE` in the plan and summarize results. Do not start the next phase without the owner's go-ahead.
- When a decision (D-number) is needed, write an ADR in `docs/decisions.md` with options, evidence and choice. Never decide silently.
- Keep a running `docs/known-gaps.md`. Anything unmatched, unaligned or unverified is listed there. Never hide a gap by omission.

## Commands (created in T0.1; keep them working)
```
just data-fetch      # download pinned sources, verify checksums
just data-pin        # record hashes for new, unpinned sources (never changes a pinned hash)
just data-archive    # dated private backup of raw sources + manifest (outside git)
just data-build      # run the full ETL, produce build/mushaf.db and exports
just data-verify     # integrity tests on spine, layout, ayah maps
just data-test       # pytest for the Python pipeline
just core-test       # cargo test --workspace
just core-targets    # cargo check core for wasm32, iOS and Android targets
just roundtrip       # patch round-trip vs KFGQPC for all riwāyāt in scope
just render PAGE=1 RIWAYAH=hafs   # SVG + PNG via reference renderer
just golden          # golden image + line-fit tests
just bench           # page resolve+shape+justify timing
just lint            # rustfmt/clippy, ruff, license check
just web-setup       # once: npm ci for apps/web + Playwright's Chromium
just web-wasm        # core -> WebAssembly package in build/web/pkg (wasm-bindgen-cli 0.2.129)
just web-data        # static data bundle build/web/data/mushaf.bundle (D57)
just web-parity      # WASM draws every golden page byte-identically (Node)
just web-build       # static site in apps/web/dist (deployable files; do not deploy unasked)
just web-dev         # local dev server http://127.0.0.1:5173
just web-test        # Playwright: every screen, offline cache
just web-bench       # page-turn budget, Chromium CPU /4
just web-golden      # app drawing vs core golden PNGs
just web-gate        # all web checks (run by `just gate`)
just gate-fast       # per-commit check: lint, core-test, data-test
just gate            # full gate, once before a phase's final push; prints step times
```

## Conventions
- Rust for `core/` (workspace crates). Python 3.12 + `uv` for `data/`. TypeScript for `apps/web` (over the core compiled to WebAssembly, D5); Swift/SwiftUI and Kotlin/Compose for the later mobile apps. The web app (`apps/web`, Vite + TypeScript, no UI framework, D58) only draws the core's SVG and JSON from `mushaf-wasm`; the page engine shared by the CLI and WASM is `core/mushaf-engine` (D56). Everything in `core/` must build for the web and mobile targets (`just core-targets`); no platform-specific code in `core/` outside `mushaf-wasm` and `mushaf-ffi`.
- Cluster ID format: `surah:ayah:word:cluster`, 1-based, e.g. `3:133:1:1`. IDs refer to the **Ḥafṣ spine** of a given spine version (`spine_version` in `data/spine/`). IDs are stable within a spine version; a new spine version ships a migration map from old IDs to new.
- Items that do not exist in the spine (inserted words, letters, ayah-end markers) get IDs of the form `<anchor-id>+ins.<n>` where `n` is an explicit integer sequence, e.g. `9:100:12:1+ins.1`. Never rely on string sorting of IDs for order.
- Clusterization uses an explicit, tested codepoint table (`data/pipeline/codepoint_classes.*`), not Unicode general categories alone.
- Riwāyah IDs: `hafs`, `shuba`, `warsh`, `qalun`, `bazzi`, `qunbul`, `duri_abuamr`, `susi`. Note that al-Dūrī of Abū ʿAmr and al-Dūrī of al-Kisāʾī are different riwāyāt.
- Unicode normalization: none. Store codepoints exactly as in the source. Normalization rules, where needed, live in one documented function.
- Arabic in code comments and docs is fine. In code identifiers use transliteration.
- Large generated files (DB, PNGs) are not committed unless under `tests/golden/`.

## Definition of done for any task
Acceptance command passes, tests added, known gaps updated, ADR written if a decision was made, commit made.
