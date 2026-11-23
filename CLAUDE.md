# CLAUDE.md — Multi-Qirāʾāt Muṣḥaf

Read this first. Then read `PROJECT_PLAN.md` (the milestone in progress only) and
`docs/ARCHITECTURE.md`. Arabic terms and their code names are in `docs/GLOSSARY.md`.

## What this project is
A muṣḥaf (website first, then native iOS/Android) that displays any of the **20 riwāyāt of
al-Shāṭibiyyah and al-Durrah**, with fixed-line pages and calligraphic justification. Every wajh
the books allow is a reader option, and the related uṣūl stay within the combinations the books
allow. Layouts are independent of the riwāyah: `layout_id`, `script_id` and the writing style are
explicit everywhere.

The data model (D67, `docs/ARCHITECTURE.md`): words; farsh entries; uṣūl occurrence tables;
riwāyah profiles (wujūh and allowed combinations); writing conventions (ḍabṭ); ayah counting
systems; layouts. They live in `corpus/` as text tables, compiled to SQLite.

**Sources of truth (D68):** al-Qāḍī's *al-Budūr al-Zāhira* and *al-Futūḥ al-Bāhira*, by the ṭuruq
of al-Shāṭibiyyah and al-Durrah only (no al-Ṭayyibah). The KFGQPC muṣḥafs are a check, never an
authority: each prints one wajh per place. Ibn Mihrān's *al-Mabsūṭ* and al-Nashshār's
*al-Budūr* are excluded (other ṭuruq).

**Legacy (frozen, D67):** the patch-model app — `data/patches/`, `data/rules/`, `data/options/`,
`data/profiles/`, the 20-step `just data-build`, and the crates `mushaf-resolve`, `mushaf-rules`,
`mushaf-db`, `mushaf-roundtrip` — keeps building and rendering the 8 KFGQPC riwāyāt until the new
model reproduces each one (M3), then it is deleted. The shaping, justification and drawing crates
(`mushaf-shape`, `mushaf-justify`, `mushaf-draw`, `mushaf-layout`, `mushaf-engine`, `mushaf-wasm`)
and the web app are kept and move onto the new model.

## Non-negotiable rules
1. **Never write, edit, "fix" or generate Qurʾānic text or qirāʾāt data from memory.**
   Text comes only from pinned source files, or from patches that carry provenance. If data is missing or a diff looks wrong, stop and report it. Do not guess.
   This includes **test expectations**: an expected value about the Qurʾān (a reading, an ayah boundary, a numbering shift, a count) must cite a source (file + location, or book + page). Without a citation it is marked `hypothesis` and may not be used as a passing criterion until the owner confirms it.
2. **Never modify files under `data/sources/raw/`.** They are checksummed in `data/sources/MANIFEST.json`. Verify checksums before every pipeline run.
3. **Source of truth is text in git:** the `corpus/` tables (TSV/JSONL for large tables, YAML for small hand-written files), and, for the legacy app until M3, its spine, patches, rules and option groups. `build/corpus.db` and `mushaf.db` are build artifacts. Never hand-edit them.
4. **Every corpus row, patch and rule has a `source` and a `status`**: `raw-derived`, `rule-derived`, `cross-checked`, `muqri-verified`. Only the owner (Yusuf) may set `cross-checked` or `muqri-verified`. These two statuses are never written into data files: they live only in `data/approvals/`, which Claude never edits. Changes there are made only on the owner's explicit instruction. Sign-off is per table, with excepted rows listed (D70).
   A fact one book supports is additionally tagged `single-source`; one both books state is `cross-source`.
5. **One edition per stack.** Spine text, line layout and font for a given `layout_id` must come from the same printed edition. Never pair a text from one printing with a layout or font from another without an ADR and a word-level diff in `known-gaps.md`.
6. **No silent rendering failure.** Any `.notdef` glyph, inserted dotted circle (U+25CC), or mark without an anchor attachment is a hard test failure, never a warning.
7. **One resolver.** All resolution, shaping and justification logic lives in `core/`. Platform apps only draw what the core returns and never reimplement logic.
8. **Deterministic and pure.** The same inputs (spine, patch stack, options, font) always give byte-identical output on every platform. Layout and justification arithmetic in `core/` uses fixed-point integers (font units / 26.6-style), not floats. No clocks, no randomness, no network at runtime.
9. **No network at runtime in the apps.** Network is allowed only in `data/` scripts, and only to domains listed in `data/sources/MANIFEST.json`.
10. **Respect licenses.** Do not copy fonts, text or layout data into any tracked location until its row in `data/sources/LICENSES.md` says redistribution is allowed. Third-party reference repos are cloned to `/tmp`, never vendored.
11. **Free mode is a feature, not a bug.** The engine must never refuse an edit because it is non-canonical. A separate lint layer flags it. Any page in a non-canonical state carries a visible "edited" marker in every render and export; it cannot be switched off.
12. **Legacy is frozen.** No new features or data in the legacy paths listed above. Fixes only when the legacy app stops building or rendering. New work goes into `corpus/` and the new crates.

## Working method
- Enter plan mode at the start of each milestone. State the task IDs you will do and the acceptance command for each.
- Do one task at a time. Run its acceptance command. Commit with message `M<n>.<k>: <summary>`.
- Before each commit run `just gate-fast` (lint, Rust tests on a sample of riwāyāt, Python tests, corpus check; a few minutes with cargo-nextest). Before every merge to `main` run the full gate locally, `just gate` (all riwāyāt via `MUSHAF_FULL=1`, platform targets, goldens, round-trip, web checks): GitHub Actions CI is switched off (D71). Do not re-run it for doc-only or status changes.
- One PR per milestone.
- Test first for anything in `core/` and the data pipeline.
- Stop at every milestone gate and summarize results. Do not start the next milestone without the owner's go-ahead.
- When a decision (D-number) is needed, write an ADR as `docs/adr/D<n>.md` (and add its row to the index in `docs/adr/README.md`) with options, evidence and choice. Never decide silently: ask the owner when no decision covers the case.
- Keep a running `docs/known-gaps.md`. Anything unmatched, unaligned or unverified is listed there. Never hide a gap by omission.

## Commands (keep them working)
```
just data-fetch      # download pinned sources, verify checksums
just data-pin        # record hashes for new, unpinned sources (never changes a pinned hash)
just data-archive    # dated private backup of raw sources + manifest (outside git)
just corpus-check    # validate corpus/ (sources, statuses, references)
just corpus-build    # compile corpus/ into build/corpus.db
just data-build      # legacy ETL -> build/mushaf.db (skips when unchanged; FORCE=1 rebuilds)
just data-verify     # legacy integrity tests on spine, layout, ayah maps
just data-test       # pytest for the Python pipeline
just core-test       # Rust tests (cargo-nextest when installed)
just core-targets    # cargo check core for wasm32, iOS and Android targets
just roundtrip       # legacy patch round-trip vs KFGQPC
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
just web-bench       # page-turn budget (16 ms), Chromium CPU /4
just web-golden      # app drawing vs core golden PNGs
just web-gate        # all web checks (run by `just gate`)
just gate-fast       # per-commit check: lint, core-test (riwayah sample), data-test, corpus-check
just gate            # full gate (MUSHAF_FULL=1), before every merge to main; prints step times
```

## Conventions
- Rust for `core/` (workspace crates): the engine, `mushaf-corpus` (load, check, build) and `mushaf-text` (letter features). Python 3.12 + `uv` for `data/`, only for importers from pinned sources and books. TypeScript for `apps/web` (over the core compiled to WebAssembly, D5); Swift/SwiftUI and Kotlin/Compose for the later mobile apps. The web app (`apps/web`, Vite + TypeScript, no UI framework, D58) only paints the core's draw list (canvas in a worker, D63). Everything in `core/` must build for the web and mobile targets (`just core-targets`); host-only crates (SQLite, CLIs) are excluded there.
- Word ID format: `surah:ayah:word`, 1-based, Kūfī ayah count, e.g. `3:133:1`, as in the Ḥafṣ word table. Letters within a word are addressed `surah:ayah:word:letter`. Items that do not exist in the word table get `<anchor-id>+ins.<n>` with an explicit integer sequence. Never rely on string sorting of IDs for order.
- Clusterization uses an explicit, tested codepoint table (`data/pipeline/codepoint_classes.*`), not Unicode general categories alone.
- Riwāyah IDs (by qāriʾ): `qalun`, `warsh` (nafi); `bazzi`, `qunbul` (ibn_kathir); `duri_abuamr`, `susi` (abu_amr); `hisham`, `ibn_dhakwan` (ibn_amir); `shuba`, `hafs` (asim); `khalaf_an_hamzah`, `khallad` (hamzah); `abul_harith`, `duri_kisai` (kisai); `ibn_wardan`, `ibn_jammaz` (abu_jafar); `ruways`, `rawh` (yaqub); `ishaq`, `idris` (khalaf_ashir). Note that al-Dūrī of Abū ʿAmr and al-Dūrī of al-Kisāʾī are different riwāyāt.
- Unicode normalization: none. Store codepoints exactly as in the source. Normalization rules, where needed, live in one documented function.
- Arabic in code comments and docs is fine. In code identifiers use transliteration (`docs/GLOSSARY.md`).
- Large generated files (DB, PNGs) are not committed unless under `tests/golden/`.

## Definition of done for any task
Acceptance command passes, tests added, known gaps updated, ADR written if a decision was made, commit made.
