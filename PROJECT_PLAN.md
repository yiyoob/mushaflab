# Multi-Qirāʾāt Muṣḥaf — Project Plan, Stage 1

Status: draft v2 · Owner: Yusuf · Executor: Claude Code, one phase at a time, with a human gate after each phase.

Stage 1 delivers the **8 riwāyāt for which full source text exists**, on a fully changeable engine, rendered in the **15-line Madinah ʿUthmānī script** with fixed-line layout and calligraphic justification. The other 12 riwāyāt are Stage 2, and Indo-Pak scripts are Stage 1b (see §0); both reuse everything built here.

### Changes in v2
- Scope narrowed to the 15-line Madinah ʿUthmānī script; Indo-Pak 15/13-line deferred to Stage 1b, with `script_id`/`layout_id` kept explicit from day one (§0, D8).
- Edition consistency: spine text, layout and font must come from one printing; S1 checks this word by word (D6, T0.3).
- S2 shapes real text instead of only checking cmap coverage; permanent no-`.notdef`/no-dotted-circle/no-unattached-mark test (T0.4, T3.5).
- Independent cross-check of the Ḥafṣ spine against a second source; `single-source` tag (T1.6, T4.6).
- Test expectations about the text need a citation or are `hypothesis` (CLAUDE.md rule 1, T4.2, T4.8).
- Owner-only statuses moved to `data/approvals/`, enforced by CI (§3.5).
- Non-canonical "edited" marker on every render/export (T7.3).
- Patch model: spine versioning, inserted-item IDs with explicit sequence, mark-op composition, behaviour on deleted targets, ayah-end markers as patchable items (§3, D9).
- Fixed-point arithmetic in the core for cross-platform determinism (D10).
- Phase 4 (raw riwāyah patches) now runs before Phase 3 (rendering).
- S3 also evaluates `harfrust` and any variable-font justification from DigitalKhatt (D3).
- S4 checks KFGQPC per-riwāyah page/line data, which may change D1 (T0.6).
- Private backup archive of raw sources (T0.2).
- Website first (D5): Phase 8 is a web app on the core compiled to WebAssembly; mobile apps move to Phase 9.

---

## 0. Scope

### In scope (Stage 1)

| `riwayah_id` | Riwāyah | Qirāʾah | KFGQPC folder |
|---|---|---|---|
| `hafs` | Ḥafṣ | ʿĀṣim | `hafs` (not `hafs-smart`, which is verse-level only) |
| `shuba` | Shuʿbah | ʿĀṣim | `shouba` |
| `warsh` | Warsh | Nāfiʿ | `warsh` |
| `qalun` | Qālūn | Nāfiʿ | `qaloon` |
| `bazzi` | al-Bazzī | Ibn Kathīr | `bazzi` |
| `qunbul` | Qunbul | Ibn Kathīr | `qumbul` |
| `duri_abuamr` | al-Dūrī | Abū ʿAmr | `doori` |
| `susi` | al-Sūsī | Abū ʿAmr | `soosi` |

Cross-check sources (established in T0.2): for Ḥafṣ, the Tanzil ʿUthmānī v1.1 text, an independent digitization. For the other 7 riwāyāt no independent source exists: QUL's Warsh script is itself sourced from KFGQPC, and QUL has no Qālūn text. All 7 are tagged `single-source`. Older KFGQPC versions (2022 mirror) and QUL Warsh are used only to detect version drift.

### Script and layout
Stage 1 builds one script and one layout family: **Madinah ʿUthmānī, 15 lines per page** (`script_id: uthmani`, `layout_id` fixed by D6). Every table, file name and API still takes `script_id` and `layout_id` as explicit keys, so Stage 1b can add Indo-Pak without migrations.

**Stage 1b — Indo-Pak (15-line and 13-line).** Not a re-skin: Indo-Pak uses a different orthographic convention (marks, pause signs, some spellings), different fonts and different text sources. In the model it is a second spine (`script_id: indopak`) with its own clusters, layouts and font, not a patch on the ʿUthmānī spine. Starts after GATE G3 when the ʿUthmānī engine is proven.

### Capabilities required by end of Stage 1
- Fixed-line pages with correct first/last word per line (Ḥafṣ layout is the anchor).
- Any riwāyah selectable at runtime.
- Runtime option groups (wujūh), e.g. sakt on/off, applied instantly.
- Free-edit mode: add/remove/replace any letter or mark, and resize any word or glyph.
- Calligraphic justification that absorbs width changes from edits and riwāyah differences.
- Fully offline.
- Round-trip exactness: `apply(patches, hafs_spine)` equals each riwāyah's KFGQPC text.

### Out of scope
The other 12 riwāyāt (Stage 2), Indo-Pak scripts and layouts (Stage 1b), tajwīd colouring, audio, translation/tafsīr, search, accounts, sync.

---

## 1. Architecture

```
SOURCES (pinned, checksummed)            DATA PIPELINE (Python, offline)
 KFGQPC 8 texts ──┐                       ingest → clusterize → align → diff
 QUL layouts  ────┼──────────────►        → patches/*.yaml, rules/*.yaml,
 DigitalKhatt fonts                         options/*.yaml, ayah_maps/*.yaml
                                            → compile → build/mushaf.db
                                                       │
CORE (Rust, shared)                                    ▼
 load spine + stack ─► resolve ─► map to lines ─► shape ─► justify ─► draw commands
   (spine → qirāʾah patches → riwāyah patches → ṭarīq → option choices → user edits)
                                                       │
                       ┌───────────────────────────────┼─────────────────┐
                       ▼                               ▼                 ▼
              reference renderer CLI            iOS app (SwiftUI)   Android app (Compose)
              (SVG/PNG, golden tests)           draws commands      draws commands
```

### Principles
- **P1** Text files in git are the source of truth. The DB is a build artifact.
- **P2** Qurʾānic text is never typed or generated by hand or by an LLM. It comes from pinned sources or from patches with provenance.
- **P3** One resolver and one layout/justification engine, in the core. The apps are thin.
- **P4** Resolution is a pure, deterministic function.
- **P5** Free mode never blocks an edit. A lint layer flags non-canonical states.
- **P6** Build a reference renderer CLI before any mobile UI. It lets Claude Code verify rendering from images without a simulator.

---

## 2. Repository layout

```
mushaf-project/
├── CLAUDE.md
├── PROJECT_PLAN.md
├── justfile
├── docs/
│   ├── decisions.md            # ADRs (D1…)
│   ├── known-gaps.md
│   ├── font-coverage.md
│   └── review/                 # generated human-review sheets
├── data/
│   ├── sources/
│   │   ├── MANIFEST.json       # url, version/commit, sha256, license row
│   │   ├── LICENSES.md
│   │   └── raw/                # downloaded, read-only, gitignored
│   ├── pipeline/               # Python (uv)
│   │   ├── fetch.py  ingest_kfgqpc.py  ingest_qul.py
│   │   ├── clusterize.py  align.py  diff.py  compile_db.py  verify.py
│   │   └── tests/
│   ├── approvals/              # owner-only: cross-checked / muqri-verified, signed commits
│   ├── spine/                  # hafs spine + clusters (jsonl), versioned, tracked
│   ├── layouts/                # line anchors per layout (json), tracked
│   ├── ayah_maps/              # per-riwāyah ayah numbering (yaml)
│   ├── patches/
│   │   ├── raw/                # <riwayah>.yaml, auto-derived
│   │   └── curated/            # farsh with citations, by qirāʾah/riwāyah
│   ├── rules/                  # uṣūl rules by qirāʾah (yaml)
│   └── options/                # option groups (yaml)
├── fonts/                      # only after license check
├── core/                       # Rust workspace
│   ├── mushaf-model/           # spine, clusters, ids, patch types
│   ├── mushaf-resolve/         # patch engine, option groups, edit overlay
│   ├── mushaf-rules/           # rule DSL evaluator
│   ├── mushaf-layout/          # line mapping, overflow policy
│   ├── mushaf-shape/           # font load, shaping
│   ├── mushaf-justify/         # justification
│   ├── mushaf-draw/            # draw commands, SVG backend
│   ├── mushaf-render-cli/
│   ├── mushaf-wasm/            # wasm-bindgen, for the website
│   └── mushaf-ffi/             # UniFFI, for the mobile apps
├── apps/
│   ├── web/
│   ├── ios/
│   └── android/
└── tests/golden/               # reference images + expected json
```

---

## 3. Data model

### 3.1 Spine and clusters
The spine is the Ḥafṣ text (KFGQPC `hafs`). A **cluster** is a base letter plus its combining marks, or a standalone item (ayah-end marker, waqf sign). Every cluster has a stable ID `surah:ayah:word:cluster`.

```json
{"id":"3:133:1:1","word":"3:133:1","kind":"letter","cps":"U+0648"}
{"id":"3:133:1:2","word":"3:133:1","kind":"letter","cps":"U+0633 U+064E"}
```

Rules:
- Lossless: concatenating all clusters reproduces the source text exactly.
- Marks are individually addressable inside a cluster: `3:133:1:2#m1`.
- Each cluster carries optional attributes: `width_em_delta`, `stretch`, `annotations{}`.
- Cluster boundaries come from an explicit codepoint class table, not Unicode general category alone (some Qurʾānic marks are `Lm` or `So`, not `Mn`). Every codepoint seen in any source must have a row; an unknown codepoint fails the pipeline.
- Ayah-end markers (with their digits) and waqf/section signs are spine items with their own `kind`, so riwāyāt can move, add or remove them through ordinary patches.
- The spine has a `spine_version`. IDs are stable within a version; a new version ships `migrations/<old>-<new>.json` mapping every old ID.

### 3.2 Patch

```yaml
- id: warsh.3.133.1.1.del
  owner: {level: riwayah, id: warsh}      # qirah | riwayah | tariq
  op: delete                              # delete | replace | insert_before | insert_after | set_marks | clear_marks | set_attr
  target: "3:133:1:1"
  source: {kind: kfgqpc-diff, ref: "warsh"}
  status: raw-derived
  rule_id: null
```

Insert example (a mark inside a cluster, e.g. small sīn for sakt):

```yaml
- id: hafs.sakt.18.1
  owner: {level: riwayah, id: hafs}
  op: set_marks
  target: "18:1:?:?"          # filled from data, not typed by hand
  payload: {add: ["U+06DC"]}
  source: {kind: rule, ref: "hafs.sakt"}
  status: rule-derived
  when: {option: hafs.sakt, choice: sakt}
```

Semantics:
- Targets refer to spine IDs. Inserts anchor to a spine ID (`insert_after`) and attach to it for line mapping.
- Inserted items get IDs `<anchor>+ins.<n>` with an explicit integer `seq` field; several inserts at one anchor are ordered by `seq`, never by string comparison. Later patches may target inserted IDs.
- Within one layer, the differ emits **at most one patch per target**: all mark changes to a cluster are merged into a single `set_marks` with `add`/`remove` lists. Two patches on the same target in the same layer are a **conflict** and raise an error. Priority is `user > option > ṭarīq > riwāyah > qirāʾah`.
- Across layers, a higher layer applies to the result of the lower ones. A patch whose target was deleted by a lower layer is an **orphan**: the resolver reports it (never silently drops it); in free mode it is shown to the user, in data builds it fails.
- Application order is deterministic: by priority layer, then by target position in the spine, then by `seq`.

### 3.3 Option group (wajh)

```yaml
- id: hafs.sakt
  title_ar: السكت
  choices: [none, sakt]
  defaults: {hafs: <per ṭarīq; VERIFY from source>}
  affects: [patch ids or rule ids]
  allowed_for: [hafs]       # advisory, used only by lint
```

### 3.4 Ayah map
Per riwāyah, maps local `(surah, ayah)` to Ḥafṣ spine ayah ranges. Handles different counting systems, split/merged ayahs, and muqaṭṭaʿāt/basmalah numbering. Pure data, filled by the pipeline and checked against the KFGQPC text.

### 3.5 Provenance statuses
`raw-derived` → `rule-derived` → `cross-checked` → `muqri-verified`. Only the owner sets the last two.

The last two are not stored in patch or rule files. They live in `data/approvals/<riwayah>.yaml` as `{target_ref, status, by, date, note}`. The compiler joins them in. CI rejects any change under `data/approvals/` that is not in a commit signed with the owner's key (CODEOWNERS + signature check). Claude never writes to this directory.

An orthogonal tag `single-source: true` marks data that no second independent source confirms.

### 3.6 DB (compiled artifact)
`riwayat`, `ayah_maps`, `clusters` (spine), `layouts`/`line_anchors`, `patches`, `rules`, `option_groups`, `fonts`. The app DB contains the same information as the text files and nothing else.

---

## 4. Decisions to make (record each as an ADR)

| ID | Question | Default / leaning | Decided in |
|---|---|---|---|
| D1 | Line policy for non-Ḥafṣ riwāyāt | Reuse Ḥafṣ line anchors, absorb difference by justification, flag overflow. First check whether KFGQPC's per-riwāyah data files carry page/line fields for their own printed muṣḥafs (possibly ayah-level only), or whether QUL has native layouts. If native layouts exist, prefer them. | P0 spike, finalized P5 |
| D2 | Font per riwāyah | Spike S2 decides. Possibly DigitalKhatt for Madanī-style scripts and KFGQPC fonts or an extended DigitalKhatt font where marks or letterforms are missing. | G0 |
| D3 | Justification implementation | A: vendor DigitalKhatt's HarfBuzz fork via FFI. B: precomputed per-word font features. C: port the justifier to Rust using digitalkhatt-js (MIT) as reference. D: `harfrust` (pure-Rust HarfBuzz port) plus variable-font axes, if DigitalKhatt publishes a variable-font justification path. Verify licenses of each. | G0 |
| D4 | Draw path | Core returns positioned glyph outlines (paths), platforms draw paths. Revisit for performance. | P3 |
| D5 | First platform | **Website first** (Rust core → WebAssembly), then iOS/Android. | Decided by owner 2026-10-21 (record ADR) |
| D6 | Ḥafṣ edition: text + layout + font | QUL Madinah V1 (1405H) with DigitalKhatt Old Madina is the known prior pairing (see `quran-madina-html`). **Precondition:** the spine text must be from the same printing. Establish which printing the KFGQPC `hafs` text follows; if it differs from the layout's printing, choose one edition and source all three from it, or record every word-level difference. Confirm in S1. | G0 |
| D7 | Licensing and distribution | Per-source rows in `LICENSES.md`. AGPL tooling (visualmetafont) used only as a tool unless cleared. Explicitly check whether KFGQPC's terms permit (a) storing their text as patches against another text, and (b) a free-edit mode. | G0 |
| D8 | Script/layout scope | Stage 1 = Madinah ʿUthmānī 15-line only. Indo-Pak 15/13 = Stage 1b, as a second spine (`script_id`), not patches on the ʿUthmānī spine. | Decided v2 (record ADR in T0.1) |
| D9 | ID and patch model details | Spine versioning + migration maps; `+ins.<n>` IDs; one patch per target per layer; orphan reporting (§3). | Decided v2 (record ADR in T0.1); revisit at end of P2 |
| D10 | Numeric determinism | Fixed-point (font units, 26.6) in all of `core/` layout and justification; floats only at the platform draw edge. | Decided v2 (record ADR in T0.1) |

---

## 5. Phases

Conventions: each task has an ID and an acceptance command. A `GATE` is a hard stop for human review. Sizes are rough: S, M, L.

**Execution order (v2):** P0 → P1 → P2 → **P4 → P3** → P5 → P6 → P7 → P8 → P9 → P10. Phase 4 is pure data work that depends only on P2, and it tells us which marks and letterforms the fonts must support before we invest in rendering. Phase numbers are kept for stable task IDs. GATE G2 (after P4) now comes before GATE G1 (after P3).

---

### Phase 0 — Foundations and spikes (M)

Goal: scaffold, acquire and checksum all sources, and answer the risky unknowns before building.

- **T0.1 Scaffold.** Repo layout, `justfile`, Rust workspace, Python `uv` project, CI running `just lint core-test`. *Accept:* `just lint core-test` passes on empty crates.
- **T0.2 Source acquisition.** *(Done; see `data/sources/MANIFEST.json` for the pinned list.)*
  - KFGQPC: the official v2 zips for 7 riwāyāt via their Internet Archive captures (the official host is unreachable from the dev machine), each checked against the archive's content digest. Bazzi v2-0 was never archived, so Bazzi uses the `thetruetruth/quran-data-kfgqpc` mirror (v7), which is also pinned for all 8 for version comparison.
  - QUL: layouts V1 1405H, V2 1421H, V4 1441H and DigitalKhatt V2; KFGQPC Ḥafṣ and Warsh word-by-word scripts. QUL has no Qālūn script.
  - DigitalKhatt: `oldmadinafont`, `madinafont` and DigitalKhatt's own encoded text with line breaks (`digitalkhatt-js`).
  - `quran-qcf4@1.1.0` (npm): per-line word lists of the 1441H print.
  - Tanzil ʿUthmānī v1.1: the independent Ḥafṣ cross-check text.
  - Write `MANIFEST.json` (url, commit/version, sha256) and `LICENSES.md`.
  - If a source needs a login or is blocked by the sandbox, the owner downloads it manually into `data/sources/raw/` and the script verifies the checksum.
  - `just data-archive` writes a dated tarball of `raw/` plus `MANIFEST.json` to a private backup location the owner chooses (outside git). Sources can change or disappear; a checksum alone cannot restore them.
  - *Accept:* `just data-fetch` verifies all checksums.
- **T0.3 Spike S1 — edition, layout and font pairing.** Determine which printing the KFGQPC `hafs` text follows (from its own metadata/docs, not memory). Determine which QUL layout matches `oldmadinafont` and `madinafont`. Diff the KFGQPC `hafs` words against the candidate layout's word list for the whole Qurʾān (count, segmentation, text) and report every difference. Reproduce page 1–2 line breaks. There are three candidate editions (1405H, 1421H, 1441H), each with layouts from up to three sources (QUL, KFGQPC per-ayah `page`/`line_*` fields, `quran-qcf4` for 1441H). Compare them, and compare DigitalKhatt's own line breaks against the print, which `quran-madina-html` reports differ on 1,620 of 8,802 lines for 1441H. Also decide whether the spine is built from the official v2 text or the 2022 mirror, since switching later means a new `spine_version`. Reference: `tarekeldeeb/quran-madina-html` PR #14 (clone to `/tmp`). *Output:* ADR for D6 with the diff summary.
- **T0.4 Spike S2 — font coverage and shaping audit.** For each of the 8 texts, list all codepoints used, and build the first draft of the codepoint class table. For each candidate font (KFGQPC riwāyah font, DigitalKhatt fonts): (1) missing codepoints; (2) **shape the full text** with HarfBuzz and report every `.notdef`, every inserted dotted circle, and every mark with no mark/mkmk attachment; (3) encoding-convention mismatches (the text uses a codepoint the font maps but for a different visual role); (4) letterform differences (e.g. whether Warsh/Qālūn text needs Maghribi forms, to confirm from the source fonts). *Output:* `docs/font-coverage.md`, ADR for D2.
- **T0.5 Spike S3 — justification feasibility.** Justify one Ḥafṣ line with each D3 option (A, B, C, D) and compare to `quran-madina-html`'s `justify.py` as oracle. Include a case where one extra mark (small sīn) widens a word. Measure effort, fidelity and build complexity for iOS/Android. *Output:* ADR for D3.
- **T0.6 Spike S4 — layouts for non-Ḥafṣ.** Check whether QUL or KFGQPC provides line layouts for Warsh/Qālūn or other riwāyāt. In particular, inspect the page/line fields in each KFGQPC riwāyah data file: are they present, at what granularity (ayah or word), and are they consistent (every page has 15 lines, no gaps)? *Output:* input to D1.
- **T0.7 License review.** Fill `LICENSES.md` fully. *Output:* ADR for D7.

**GATE G0:** owner reviews ADRs D1–D3, D6–D10 and `font-coverage.md`. Nothing else proceeds until D2, D3 and D6 are decided.

---

### Phase 1 — Ḥafṣ spine, clusters, layout anchors (M)

Goal: a verified Ḥafṣ database with correct page and line boundaries.

- **T1.1 Ingest Ḥafṣ.** Parse KFGQPC `hafs` into words `(surah, ayah, position, text)`. Separate verse-end markers and any waqf signs into their own kinds. *Accept:* 114 surahs, 6236 ayahs, word count recorded in a test constant from the source.
- **T1.2 Clusterizer.** Split into clusters per §3.1. *Accept:* property test, concatenation equals source for every ayah; no orphan combining marks.
- **T1.3 Ingest layout (D6).** Parse the QUL layout (`page_number, line_number, line_type, is_centered, first_word_id, last_word_id`). Map QUL word IDs to spine words via `(surah, ayah, position)` and text comparison. *Accept:* unmatched list is empty, or every entry is in `known-gaps.md` with a reason.
- **T1.4 Compile.** Emit `spine.jsonl`, `layouts/<id>.json`, `build/mushaf.db` (schema v1).
- **T1.5 Integrity verification.**
  - 604 pages.
  - Every word in exactly one line; words contiguous 1..N across pages.
  - Line types (ayah, surah_name, basmalah) present where the layout defines them.
  - Page 1 starts at 1:1, page 2 starts at 2:1, last page ends at 114:6.
  - *Accept:* `just data-verify` green, and a golden `page_boundaries.json` committed. The owner spot-checks 15 pages against a printed muṣḥaf.
- **T1.6 Independent cross-check.** Compare the Ḥafṣ spine against the independent ʿUthmānī text from T0.2: word count per ayah, and consonantal skeleton per word (via the one documented normalization function). Every difference goes to `docs/review/hafs-crosscheck.html` and `known-gaps.md` for the owner to adjudicate. *Accept:* report generated; zero unexplained differences after owner review.

---

### Phase 2 — Core: model, patch engine, resolver (L)

Goal: the heart of "everything is changeable", tested without any rendering.

- **T2.1 Types.** Cluster, ID, patch, option group, ayah map, edit overlay (serde, JSON/YAML).
- **T2.2 Loader.** Load spine + stack from files and from `mushaf.db`. Both loaders must produce identical structures. *Accept:* equality test.
- **T2.3 Patch engine.** All ops from §3.2, conflict detection, deterministic order. *Accept:* unit and property tests: identity (no patches gives the spine), determinism, order-independence of non-conflicting patches, conflicts raise errors.
- **T2.4 Option resolution.** Option groups, defaults per riwāyah/ṭarīq, priority order.
- **T2.5 User edit overlay.** Highest priority layer, serializable as a patch file.
- **T2.6 Line mapping.** Map resolved clusters back to lines via spine anchors. An insert-before joins the line of the word it precedes and an insert-after the line of the word it follows; nested inserts follow the same rule relative to their own anchor (owner decision, Phase 2 gate 2026-10-22). Deleted anchors fall through to the next surviving cluster. *Accept:* every resolved cluster is in exactly one line; line order preserved.
- **T2.7 Golden.** Resolved Ḥafṣ text per page equals the source. *Accept:* `just core-test`.

---

### Phase 3 — Reference renderer, Ḥafṣ vertical slice (L)

Goal: prove rendering and justification end to end on desktop before any mobile work.

- **T3.1 Shaping** per D2/D3, in `mushaf-shape`.
- **T3.2 Justification** per D3, in `mushaf-justify`. Inputs: line cluster sequence, target width, per-cluster `width_em_delta`/`stretch` overrides. Output: per-glyph positions and stretch values. Manual overrides take precedence and are never silently dropped.
- **T3.3 Draw commands + SVG backend** (D4). `resvg` rasterizes to PNG.
- **T3.4 CLI.** `just render PAGE=1 RIWAYAH=hafs` writes SVG and PNG.
- **T3.5 Goldens.** Pages 1, 2, 3, a surah-header page, a mid page, and 604. Line-fit metric: justified-line width within 0.1% of target width, with documented exceptions (short and centered lines). Visual diff with tolerance.
- **T3.5a Shaping integrity (all 604 pages).** For every page: zero `.notdef`, zero inserted dotted circles, every mark attached via an anchor, and the glyph cluster map covers every input codepoint exactly once. *Accept:* `just golden` fails on any violation.
- **T3.6 Perf harness.** Resolve + shape + justify per page, with caching keyed by `(word, state hash)`. Record desktop numbers. The 16 ms/page budget is verified on a device in Phase 8.
- **T3.7 Edit demo.** Insert a mark (small sīn) and a letter, widen a word, and confirm the line still closes. *Accept:* golden images and line-fit tests.

**GATE G1:** owner reviews 10 rendered pages against the printed muṣḥaf. (D5 is already decided: website first.) **Passed 2026-10-22 (owner); see the G1 record in `docs/decisions.md`. Known deviation from the print: G-045.**

---

### Phase 4 — Derive raw patches for the 7 other riwāyāt (L)

Goal: a machine-derived, exact, reviewable difference set for each riwāyah. This is the ground truth that rules will later replace.

Runs after Phase 2 and before Phase 3 (see execution order). Order: `shuba` → `qalun` → `warsh` → `bazzi` → `qunbul` → `susi` → `duri_abuamr`. Finish and review `shuba` first, since it is closest to Ḥafṣ and exercises the machinery.

- **T4.1 Ingest** each riwāyah text into the same cluster structure, preserving its own ayah numbering.
- **T4.2 Ayah maps.** Map each riwāyah's ayah numbering to Ḥafṣ spine ranges. Record the total ayah count per riwāyah from the data. Warsh and Qālūn are expected to number Fātiḥah and muqaṭṭaʿāt-opening surahs differently. Expected shifts used as tests (e.g. 1:4 in Ḥafṣ is 1:3 in Warsh) must cite a source location; until then they are `hypothesis` and only reported, not asserted.
- **T4.3 Aligner.** Per surah, align word sequences on the consonantal skeleton (marks stripped), using ayah anchors as constraints. Output an alignment report listing every unaligned segment. Unaligned segments go to `known-gaps.md`.
- **T4.4 Differ.** Inside aligned words, diff at cluster level into ops. Across unaligned spans, emit word insert/delete ops.
- **T4.5 Emit** `data/patches/raw/<riwayah>.yaml` with `status: raw-derived`.
- **T4.6 Round-trip.** `apply(raw patches, hafs spine)` must equal the KFGQPC target codepoint-for-codepoint. *Accept:* `just roundtrip` shows 100% for the riwāyah. Any mismatch is a bug in the aligner or differ, or a documented source anomaly. All 7 riwāyāt are tagged `single-source`: no independent digitization was found (T0.2). For each, also diff the official v2 text against the 2022 mirror version, and for Warsh against QUL's `qpc-warsh` script, and report differences; these catch version drift and transcription slips, not reading errors.
- **T4.7 Stats.** Per riwāyah: counts by op type, by surah, and by "mark-only" vs "consonant change". These show how much is uṣūl and how much is farsh/rasm.
- **T4.8 Review sheets.** Generate HTML sheets for the owner to review per surah. Include known-tricky cases: 3:133 (Warsh drops the initial wāw of *sāriʿū*), 2:184 and 2:10 (Ḥafṣ vs Warsh word differences, with Warsh numbering shifts). These are `hypothesis` until the owner confirms them against a cited source; the sheet shows what the data says, it does not assert it.

**GATE G2:** owner reviews the stats and a sample of patches for each riwāyah. Raw patches are not yet claimed to be correct qirāʾāt knowledge. They are exact mirrors of KFGQPC. **Passed 2026-10-22 (owner); see the G2 record in `docs/decisions.md`.**

---

### Phase 5 — Render all 8 riwāyāt (M)

- **T5.1 Riwāyah profiles.** Each profile selects its patch stack, ayah map and surah-transition policy (basmalah/sakt/waṣl between surahs, as data, to be verified against the source). *Accept:* resolver loads all 8 profiles.
- **T5.2 Overflow policy** (D1). Order of absorption: font contraction → word-space tightening → line scale → flag. Emit an overflow report per riwāyah listing lines that needed fallback.
- **T5.3 Font per riwāyah** (D2). Add a glyph-ID/PUA escape-hatch mapping file for marks the font lacks.
- **T5.4 Goldens per riwāyah.** Include 3:133, 2:10, 2:184 and the Fātiḥah numbering for Warsh.
- **T5.5 Fidelity review.** Render all pages for each riwāyah as contact sheets in `docs/review/`.

**GATE G3:** owner reviews contact sheets and the overflow report; D1 is finalized. **Passed 2026-10-23 (owner); see the G3 record in `docs/decisions.md`. Open: the undecided encoding groups (G-102), the Warsh/Qālūn reference print (G-106).**

---

### Phase 6 — Uṣūl rules and option groups (L)

Goal: replace raw diffs with rules so toggling works. This is where "complete changeability" becomes a rule engine instead of a lookup table.

- **T6.1 Rule DSL.** Declarative rules over cluster sequences, including cross-word context, parameterized by option choices. Rules live in `data/rules/`. Every rule has a register entry: description, source book and page, status. Design the register so tests fail if a rule and its register entry disagree.
- **T6.2 First option group: Ḥafṣ sakt.** Toggle the sakt places on/off; the small sīn (U+06DC) appears or disappears and the line re-justifies. Take the places and default from the source data, not memory. *Accept:* golden image pair (on/off) plus line-fit.
- **T6.3 Annotation channel.** Rules that change pronunciation but not glyphs (madd lengths, taqlīl, tarqīq) write to `annotations`, not to clusters.
- **T6.4 Rule sets per qirāʾah**, in this order: ʿĀṣim (Shuʿbah vs Ḥafṣ) → Nāfiʿ → Ibn Kathīr → Abū ʿAmr. For each:
  - Implement a rule, run it, and subtract the patches it explains from the raw set.
  - Remaining patches become curated farsh files under `patches/curated/`, with a citation when one is found.
  - *Accept:* `just roundtrip` stays 100% using rules + curated farsh, and the raw set shrinks to zero or to a listed residual.
- **T6.5 Rule review sheets.** For each rule, an HTML sheet listing every application with context and source.
- **T6.6 Ṭarīq layer.** Model a second ṭarīq as an option group where the sources distinguish ṭuruq, starting with one riwāyah.

**Display styles: Ḥijāzī and Maghribi for every riwāyah (owner, 2026-10-23, D36).** Any riwāyah can be shown in either script style. Phase 5 keys fonts and justification by (riwāyah, style) and renders only each riwāyah's default style (Maghribi for Warsh/Qālūn, Ḥijāzī for the rest).
- **T6.7 Encoding conversion layer** (deferred here by D13). One documented, tested conversion between KFGQPC mark conventions, built from owner-confirmed rows (the `data/approvals/` pattern of D21). Rows are data-derived where parallel evidence exists, never invented. It is applied only at render time; spine and patches stay codepoint-exact. *Accept:* conversion round-trips, and every converted codepoint traces to a confirmed row.
- **T6.8 Style conventions.** For each style, its mark conventions (sukūn, waṣl sign, pause signs, dotting): which differ, the evidence for each, and the owner-confirmed conversion rows between Ḥijāzī and Maghribi. Any convention without evidence stays unconverted and is reported, never guessed.
- **T6.9 Font coverage per (riwāyah, style)**, an S2-style audit. Every combination must pass rule 6 (no `.notdef`, dotted circle, unattached or spacing mark). Fill gaps with the escape-hatch mapping, a documented derived font, or by marking the combination unavailable. Justification is calligraphic where a font supports it (Ḥijāzī: DigitalKhatt) and spacing-only otherwise (Maghribi, until a calligraphic Maghribi font exists). Line breaks always stay the riwāyah's own layout (D1); overflow follows T5.2.
- **T6.10 Style switch and goldens.** A runtime style option per muṣḥaf, plus golden pages for every riwāyah in its non-default style. *Accept:* rule-6 integrity and line fit on all pages of every available (riwāyah, style); the owner reviews samples at GATE G4.

**GATE G4:** the owner reviews rule registers and sheets, sets `cross-checked` or `muqri-verified` where warranted, and reviews the display-style samples.

---

### Phase 7 — Free-edit mode (M)

- **T7.1 Edit API.** Add/remove/replace clusters, add/remove marks, set per-word or per-glyph width. Edits go to the overlay layer, with undo/redo.
- **T7.2 Export/import** of overlays as patch files.
- **T7.3 Lint layer.** Flags non-canonical states per riwāyah and option set (advisory only, never blocks). Any non-canonical page carries a visible "edited" marker in every render, screenshot path and export; it cannot be disabled. *Accept:* golden test of an edited page shows the marker.
- **T7.4 Re-justification.** Every edit re-justifies only affected lines (cache invalidation by `(word, state hash)`). *Accept:* tests for edit → line still closes, undo restores exact prior output.

---

### Phase 8 — Web app (L)

Platform per D5: **a website first** (owner, 2026-10-21), because it is the fastest way to exercise and test the qirāʾāt engine. The Rust core is compiled to WebAssembly and runs in the browser, so the site uses the same resolver, layout and justification as the CLI (P3). It is static: the DB and fonts are served as files and cached for offline use, with no server-side logic and no network calls beyond loading those files (rule 9 applies to everything after load).

- **T8.1 WASM bindings.** `mushaf-wasm` crate (wasm-bindgen) next to `mushaf-ffi`; build script producing a JS/WASM package. *Accept:* the same golden page resolves byte-identically in the CLI and in WASM.
- **T8.2 Data.** Serve `mushaf.db` (or a derived static format) and fonts; same content as the CLI uses. Cache for offline use.
- **T8.3 UI.** RTL pager, drawing of core draw commands (SVG or Canvas), riwāyah picker, option sheet, free-edit editor, provenance/verification-status screen, and an engine-inspector view (patch stack, applied rules, per-cluster provenance) for testing qirāʾāt.
- **T8.4 Performance.** Verify the 16 ms/page budget in a mid-range phone browser; tune caching.
- **T8.5 Tests.** Browser screenshot tests (Playwright) against core goldens.

**GATE G5:** owner uses the site daily for a week, then logs issues.

---

### Phase 9 — Mobile apps (L)
iOS and Android (UniFFI via `mushaf-ffi`), reusing the core. Expected to be much smaller than before the web app, because all logic and its tests are shared.

---

### Phase 10 — Release hardening (M)
Attribution and license screen, data-provenance screen, size budget, offline test, accessibility, crash reporting policy, and store-submission checks.

---

## 6. Test matrix

| Test | Phase | What it proves |
|---|---|---|
| Source checksums | 0 | Pipeline runs on pinned data |
| Cluster losslessness | 1 | Clusters rebuild the source text exactly |
| Codepoint table completeness | 0, 1, 4 | Every codepoint in every source has an explicit class |
| Ḥafṣ independent cross-check | 1 | Spine agrees with a second source, or every difference is adjudicated |
| Edition consistency | 0, 1 | Spine words match the layout's word list |
| Shaping integrity | 3, 5 | No `.notdef`, dotted circles or unattached marks on any page |
| Approvals integrity | all | `data/approvals/` changed only in owner-signed commits |
| Edited marker | 7 | Non-canonical pages always show the marker |
| Page/line integrity | 1 | 604 pages, no orphan or duplicate words, correct boundaries |
| Patch properties | 2 | Determinism, identity, conflict detection |
| Loader parity | 2 | Files and DB give identical structures |
| Line-fit | 3, 5 | Justified lines close within tolerance |
| Golden images | 3, 5, 6 | No visual regressions |
| Round-trip per riwāyah | 4, 6 | Patches/rules reproduce KFGQPC text exactly |
| Alignment report | 4 | No silent mismatches |
| Rule register consistency | 6 | Rules and their documentation agree |
| Edit/undo | 7 | Changeability is exact and reversible |
| Device perf | 8 | 16 ms/page budget |

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| Spine text, layout and font come from different printings | S1 word-level diff; D6 precondition; CLAUDE.md rule 5 |
| Font draws wrong output without erroring (dotted circles, unattached marks, encoding mismatch) | Full-text shaping audit in S2; shaping-integrity test on every page |
| KFGQPC text has an error we faithfully reproduce | Independent Ḥafṣ cross-check against Tanzil (T1.6); version comparison official v2 vs 2022 mirror (and QUL for Warsh); `single-source` tag on all 7 non-Ḥafṣ riwāyāt |
| Edited pages circulate as authentic | Non-removable "edited" marker (T7.3) |
| Source disappears or changes upstream | Private dated archive of `raw/` (T0.2) |
| Fonts lack marks or letterforms needed by some riwāyāt (S2) | Audit in P0, extend a font with visualmetafont, glyph-ID escape hatch |
| Justification needs a custom HarfBuzz or heavy porting (S3) | Decide D3 in P0 from a working spike, not from assumptions |
| KFGQPC text conventions (mark encodings, word splitting) differ per riwāyah | Per-riwāyah ingest tests, alignment report, `known-gaps.md` |
| Alignment errors create wrong patches | Round-trip test is exact; human review sheets |
| Non-Ḥafṣ lines overflow Ḥafṣ-anchored line widths | Overflow policy and report (D1) |
| LLM-generated religious content creeping in | CLAUDE.md rule 1; provenance on every patch; owner-only verification statuses |
| Performance on older devices | Caching by state hash; device benchmark in P8 |

---

## 8. After Stage 1

Stage 1b adds Indo-Pak 15-line and 13-line as a second spine (`script_id: indopak`) with its own sources, codepoint table, layouts, font and ayah alignment to the ʿUthmānī spine (for cross-script navigation only). It reuses the resolver, layout, justification, renderer and apps unchanged. It starts after GATE G3.

Stage 2 adds the other 12 riwāyāt, starting with Kisāʾī, then Ḥamzah/Khalaf, then Khalaf al-ʿĀshir, Ibn ʿĀmir, Yaʿqūb and Abū Jaʿfar. It reuses the same engine. The Stage 1 round-trip harness doubles as the accuracy test for the farsh-extraction pipeline, because the 8 covered riwāyāt serve as held-out ground truth. The rule sets for ʿĀṣim, Nāfiʿ, Ibn Kathīr and Abū ʿAmr are already validated by then.

---

## Appendix — First prompts for Claude Code

**Prompt 1 (start of project)**
> Read CLAUDE.md and PROJECT_PLAN.md sections 0–4 and Phase 0. Enter plan mode and propose how you will do T0.1 and T0.2. Do not start any other task.

**Prompt 2 (after T0.2)**
> Do T0.3 to T0.7 as spikes. Clone third-party reference repos to /tmp, not into this repo. Produce the ADRs and `docs/font-coverage.md`, then stop at GATE G0 and summarize the options and your recommendation for each decision.

**Prompt 3 (Phase 1)**
> G0 is approved with decisions recorded in docs/decisions.md. Do Phase 1 task by task. After each task run its acceptance command and commit. Stop at the end of the phase and list anything in known-gaps.md.
