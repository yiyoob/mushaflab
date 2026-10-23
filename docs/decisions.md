# Architecture Decision Records

Each decision from PROJECT_PLAN §4 gets an ADR here. A decision is never made silently.
Only the owner moves an ADR from `proposed` to `accepted`.

## Template

```
## D<n> — <title>
Status: proposed | accepted | superseded by D<m>
Date: YYYY-MM-DD
Context: why the decision is needed
Options: each option with evidence (files, measurements, links)
Decision: the choice
Consequences: what follows, what it rules out
```

## Index

| ID | Title | Status |
|---|---|---|
| D1 | Line policy for non-Ḥafṣ riwāyāt | accepted: native layouts (finalized P5) |
| D2 | Font per riwāyah | accepted: hybrid (DigitalKhatt + KFGQPC) |
| D3 | Justification implementation | accepted: Rust port on harfrust |
| D4 | Draw path | accepted with the Phase 3 plan: positioned glyph outlines |
| D5 | First platform | accepted (website first) |
| D6 | Ḥafṣ edition: text + layout + font | accepted: 1441H digital layout (reconfirmed 2026-10-22) |
| D7 | Licensing and distribution | accepted (owner-declared) |
| D8 | Script/layout scope | accepted |
| D9 | ID and patch model details | accepted; P2 addendum approved at the Phase 2 gate (2026-10-22) |
| D10 | Numeric determinism | accepted |
| D11 | Spine file format and item IDs | accepted (Phase 1 plan) |
| D12 | Core model: formats, patch semantics, DB access | accepted (Phase 2 plan); P2 addendum approved (2026-10-22) |
| D13 | Phase 4 inputs: Bazzi source, single-source, encoding handling | accepted (2026-10-22) |
| D17 | Superscript alef after non-joining letters (G-045) | rejected by the owner after review; reverted (2026-10-22) |
| D16 | Manual overrides, header text, and the "edited" marker in draw output | accepted with the Phase 3 plan |
| D15 | Page geometry, short lines and overflow (Ḥafṣ, 1441H) | accepted with the Phase 3 plan; numbers for review at G1 |
| D14 | KFGQPC to DigitalKhatt Madina encoding table (Ḥafṣ) | accepted (approach with the Phase 3 plan; lossy and owner rows decided 2026-10-22) |
| D20 | Phase 4 derivation pipeline: ingest, aligner, differ, emitter | accepted (Phase 4 plan, 2026-10-22) |
| D21 | Difference-kind tags: what the data can and cannot classify | accepted: C, rows owner-signed (2026-10-22) |
| D22 | Round-trip check for raw patches | accepted (Phase 4 plan, 2026-10-22) |
| D36 | Display styles: Ḥijāzī and Maghribi for every riwāyah | accepted (scope); built in Phase 6 |

---

## D8 — Script/layout scope
Status: accepted
Date: 2026-10-21
Context: The owner wants a correct foundation before taking on more scripts. Indo-Pak
uses its own orthographic conventions, fonts and text sources, so it is not a re-skin of
the ʿUthmānī text.
Options:
- A: Build ʿUthmānī 15-line and Indo-Pak 15/13-line together.
- B: Stage 1 = Madinah ʿUthmānī 15-line only. Indo-Pak becomes Stage 1b, as a second spine.
Decision: B (owner, plan v2).
Consequences: `script_id` and `layout_id` are explicit keys in every table, file name
and API from the start, so Stage 1b needs no migration. Indo-Pak is never modelled as
patches on the ʿUthmānī spine.

## D9 — ID and patch model details
Status: accepted (revisit at end of P2)
Date: 2026-10-21
Context: The v1 plan left inserted-item IDs, ordering, mark-op composition, deleted
targets and spine corrections undefined.
Decision (owner, plan v2; details in PROJECT_PLAN §3):
- Spine carries `spine_version`; IDs are stable within a version, and new versions ship
  a migration map.
- Inserted items: `<anchor>+ins.<n>` with an explicit integer `seq`; never string-sorted.
- At most one patch per target per layer; mark changes merge into one `set_marks`.
- A patch whose target a lower layer deleted is an orphan: reported, never dropped.
- Ayah-end markers and waqf/section signs are spine items, patchable like any cluster.
Consequences: the differ (T4.4) must merge ops per target; the resolver needs orphan
reporting from T2.3.

## D10 — Numeric determinism
Status: accepted
Date: 2026-10-21
Context: CLAUDE.md requires byte-identical output. Floating-point results can differ
across CPUs, compilers and platforms (desktop x86/ARM, iOS, Android).
Decision: all layout and justification arithmetic in `core/` uses fixed-point integers
(font units, 26.6 where sub-unit precision is needed). Floats are allowed only at the
platform draw edge, after the core's output is final.
Consequences: the shaping backend chosen in D3 must expose integer positions (HarfBuzz
and its ports do). Golden outputs are compared exactly, not with tolerance, at the
draw-command level.

## D5 — First platform
Status: accepted
Date: 2026-10-21
Context: The engine (resolver, rules, layout, justification) must be tested thoroughly
before any app polish. The original plan picked between iOS and Android at G1.
Options:
- A: iOS first (SwiftUI over UniFFI).
- B: Android first (Compose over UniFFI).
- C: Website first, with the Rust core compiled to WebAssembly.
Decision: C (owner, 2026-10-21). It is the fastest loop for testing the qirāʾāt engine:
no device builds, easy inspection, and screenshots via a desktop browser.
Consequences:
- Phase 8 becomes the web app (`apps/web`, `core/mushaf-wasm`); mobile apps move to Phase 9.
- Everything in `core/` must compile to `wasm32-unknown-unknown`. This is a constraint on
  D3 (justification): pure-Rust shaping compiles directly, while a C/C++ HarfBuzz fork
  needs an Emscripten or wasm-sysroot build. S3 must measure this.
- Rule 9 (no network at runtime) applies after the static files load; the site is
  cached for offline use and has no server-side logic.
- iOS and Android remain planned (owner, 2026-10-21). The core stays platform-neutral:
  `just core-targets` (in CI) checks the workspace for wasm32, aarch64-apple-ios(-sim) and
  aarch64/x86_64-linux-android on every push, and platform code lives only in
  `mushaf-wasm` and `mushaf-ffi`.

## D6 — Ḥafṣ edition: text + layout + font
Status: proposed (evidence from S1; the font half depends on S2 and S3)
Date: 2026-10-21
Context: CLAUDE.md rule 5 requires spine text, layout and font from one printed edition.
Evidence: `docs/spikes/s1-editions.md` (generated by `data/spikes/s1_editions.py` from pinned sources).
- KFGQPC's per-ayah `page`/`line_start`/`line_end` (v2.0 and v18 alike) match QUL's V4
  1441H layout on 6236/6236 ayahs (V2 1421H: 5367, V1 1405H: 4530). **The KFGQPC Ḥafṣ
  text belongs to the 1441H print.**
- Two independent 1441H line sources agree: `quran-qcf4` (word glyphs traced from the
  print) and QUL V4 have the same first/last word on 9042 of 9046 lines. The 4 that
  disagree are pages 92 (lines 12–13) and 254 (lines 6–7): one word sits at the end of one
  line in one source and at the start of the next in the other.
- DigitalKhatt's own texts follow older prints: Old Madina matches V1 1405H (9045/9046
  lines), Madina matches V2 1421H (9043/9046); QUL's "DigitalKhatt" layout is identical to
  V2. **No DigitalKhatt font follows 1441H.**
- Letters: KFGQPC v2.0, Tanzil 1.1, DigitalKhatt Madina and Old Madina have the same letter
  skeleton in all 6236 ayahs (ignoring spacing). Word splitting differs only at 15:7,
  27:20 and 36:22, where KFGQPC writes one word and the others two. Marks were not
  compared in S1 (S2).
- KFGQPC v2.0 vs v18: 33 ayahs differ, all the same change: كلا lost a tatweel in v2.0
  (as its changelog says). Page data identical. v2.0 encodes ayah numbers as U+FC00+(n−1),
  a presentation-form codepoint that only KFGQPC's font draws as an ayah sign; v18 used
  Arabic-Indic digits.
Options:
- A (1441H stack): spine KFGQPC v2.0; layout 1441H (QUL V4, cross-checked by qcf4 and the
  KFGQPC page fields); font KFGQPC Ḥafṣ v20 (made for this text) if S3 shows it can be
  justified, otherwise a DigitalKhatt font as a documented rule-5 exception.
- B (1421H stack): DigitalKhatt Madina font and its own text/lines (QUL V2); spine still
  KFGQPC (same letters, marks to be compared in S2). The KFGQPC 1441H page data is unused.
- C (1405H stack): DigitalKhatt Old Madina with QUL V1. The oldest print, furthest from
  KFGQPC's page data.
Leaning: spine = KFGQPC official v2.0 (newest; differs from v18 only in the tatweel of
كلا), which resolves G-010. Layout and font: A or B, decided at G0 from S2 and S3.

Decision (owner, GATE G0, 2026-10-21): spine = **KFGQPC official v2.0** (resolves G-010).
**The Ḥafṣ pages match the 1441H print** (option A for text and layout): KFGQPC's own page data, with word-level line breaks from QUL V4 and
`quran-qcf4`. Where those two disagree, the printed page decides. The owner checked the
two disagreements against printed 1441H pages (screenshots supplied 2026-10-21):
- page 92: line 12 ends with كُلَّ (4:91:8) and line 13 starts with مَا (4:91:9). QUL V4 is
  right; qcf4 is wrong.
- page 254: line 6 ends with بَعۡدَ (13:37:8) and line 7 starts with مَا (13:37:9). qcf4 is
  right; QUL V4 is wrong.
The font half of the stack follows D2: KFGQPC fonts keep the stack within one edition;
DigitalKhatt fonts (a 1421H design) on 1441H lines would be a documented rule-5 exception.

## D2 — Font per riwāyah
Status: proposed (evidence from S2; decided together with D3 at G0)
Date: 2026-10-21
Context: Each riwāyah's text must render with no `.notdef`, no dotted circles and no
unattached marks (rule 6), with letterforms that match its printed muṣḥaf.
Evidence: `docs/font-coverage.md` (generated by `data/spikes/s2_fonts.py`).
- Every KFGQPC text in its own KFGQPC font: 0 missing codepoints, 0 `.notdef`, 0 dotted
  circles. The anchor check flags 5–11 marks per riwāyah (60 Warsh, 142 Qālūn). Inspected
  visually: in Ḥafṣ-family fonts a pause sign that follows a small meem (e.g. 16:101)
  has no anchor and sits over the middle of the word, not its last letter (to compare
  with the print); in Warsh/Qālūn most flags are false positives (the pause sign is a
  pre-positioned substituted glyph and renders correctly).
- The KFGQPC Ḥafṣ font also renders Shuʿbah, Bazzi and Qunbul texts cleanly, but not
  Warsh/Qālūn (marks on U+06D2 unattached) and not Dūrī/Sūsī (one U+0622 missing).
- DigitalKhatt fonts cannot render KFGQPC text as stored: 6,720–18,459 `.notdef` per
  riwāyah. KFGQPC repurposes codepoints: open tanwīn is U+0657/U+065E/U+0656 (Unicode:
  inverted damma, fatha with two dots, subscript alef) where DigitalKhatt uses U+08F0–08F2;
  KFGQPC's U+0652 on alef is DigitalKhatt's U+06DF; ayah numbers are U+FC00+(n−1).
- For Ḥafṣ, a KFGQPC→DigitalKhatt conversion is a fixed table: 443 of 444 differing
  cluster forms have exactly one DigitalKhatt counterpart (the exception is a cluster-split
  artifact). The table is derivable from the two texts and verifiable by round-trip.
  No such data exists for marks used only by other riwāyāt.
- Letterforms: the KFGQPC Warsh/Qālūn fonts draw Maghribi dotting (qāf one dot above,
  fā' dot below) for the standard codepoints U+0642/U+0641; the Ḥafṣ font and DigitalKhatt
  fonts draw Mashriqi dotting. Warsh/Qālūn therefore need their own fonts.
- DigitalKhatt Madina is a variable font with axes `LTAT`/`RTAT` (stretching); KFGQPC fonts
  have no variation axes but carry contextual-variant features `zz01`… (purpose for
  justification to be tested in S3).
Options:
- A: KFGQPC font per riwāyah (official v2.x; Bazzi v7). Complete coverage and correct
  letterforms for all 8 today; justification capability unknown (S3).
- B: DigitalKhatt fonts with a data-derived KFGQPC→DigitalKhatt table. Calligraphic
  justification built in; 1405H/1421H letterforms (not 1441H); no Maghribi forms; marks
  outside Ḥafṣ need owner-approved table rows or font work.
- C: Hybrid: DigitalKhatt for the Mashriqi-script riwāyāt, KFGQPC fonts for Warsh/Qālūn.
Leaning: decide with D3 at G0. A is the only option complete for all 8 without new font
work; B/C win only if S3 shows KFGQPC fonts cannot be justified acceptably.
Decision (owner, GATE G0, 2026-10-21): **C, hybrid.**
- Ḥafṣ, Shuʿbah, Bazzi, Qunbul, Dūrī, Sūsī (Mashriqi script): DigitalKhatt Madina (`hb/`
  build from digitalkhatt-js), with calligraphic justification (D3). The text is converted
  from KFGQPC encoding by a table derived from the data, never written by hand, and verified
  by round-trip against DigitalKhatt's own text for every Ḥafṣ word. Marks that occur only
  outside Ḥafṣ get table rows only with owner approval.
- Warsh, Qālūn: their KFGQPC fonts (Maghribi letterforms), justified by spacing until a
  calligraphic option exists.
- **Rule-5 exception (recorded here as CLAUDE.md requires):** the Mashriqi stacks pair the
  1441H text and layout (D6) with a font of the 1421H design. Evidence that this is safe for
  the text: identical letters in all 6236 ayahs and a one-to-one mark mapping (S1, S2).
  What differs is letterform style and line metrics. The required word-level diff is
  produced when the conversion table is built (G-024) and reviewed by the owner.

## D3 — Justification implementation
Status: proposed (evidence from S3)
Date: 2026-10-21
Context: Lines must close calligraphically (by stretching letters, as in the print) and
re-justify at runtime after riwāyah changes, options and free edits, on web, iOS and
Android, deterministically (D10).
Evidence: `docs/spikes/s3-justify.md`.
- DigitalKhatt's justifier (MIT; its Python port in quran-madina-html used as the oracle)
  runs on **stock HarfBuzz**: it stretches letters by setting per-character values of the
  Madina font's `cv01`–`cv18` features, then adjusts spaces. No custom HarfBuzz is needed
  with the `hb/` fonts from `digitalkhatt-js`.
- On 162 lines from 12 pages every line closed within 0.1%. Adding a mark (U+06DC) does
  not change a word's width (marks have zero advance); inserting a letter widened the word
  by 246 units and the line still closed (spaces absorbed it).
- **harfrust 0.14.0 (pure Rust, MIT) reproduces HarfBuzz 14.6.0 exactly**: 1559/1559 words,
  13,915 glyphs with identical ids, clusters, advances and offsets, including all 124 words
  carrying stretch features. It builds for wasm32, iOS (device and simulator) and Android
  (arm64, x86_64). It needs one workaround: construct `Feature { start, end }` directly,
  because `Feature::new(…, s..e)` in 0.14.0 drops single-cluster ranges (off-by-one).
- Spacing alone cannot close lines like the print: word gaps would need a median 2.4×
  (KFGQPC font, 1441H lines) to 3.75× (DigitalKhatt, 1421H) normal width, p95 4.3×–7.7×.
- DigitalKhatt's HarfBuzz fork: 33 commits ahead, 6,173 behind upstream, +13.4k/−11.0k
  lines of C/C++ across 41 files.
Options:
- A: Vendor DigitalKhatt's HarfBuzz fork via FFI. Rejected: not needed for the `hb/` fonts,
  large divergence, C++ cross-builds for wasm/iOS/Android.
- B: Precomputed per-word features. Rejected: cannot follow runtime edits, options or
  riwāyah changes.
- C: Port DigitalKhatt's justifier to Rust in `mushaf-justify`, using the MIT TypeScript
  original and the quran-madina-html Python port as references, and the Python port as an
  executable oracle in tests.
- D: harfrust for shaping (pure Rust), with variable-axis support available if the
  variable `madinafont` is used later.
Recommendation: **C on D**: a Rust port of the DigitalKhatt justifier over harfrust
(pinned exactly), with the 1,559-word parity set kept as a regression test.
Decision (owner, GATE G0, 2026-10-21): accepted. Calligraphic
justification then requires DigitalKhatt fonts (D2 options B/C); with KFGQPC fonts only
spacing (and optionally flat kashida) is available.

## D1 — Line policy for non-Ḥafṣ riwāyāt
Status: proposed (evidence from S4; finalized in Phase 5)
Date: 2026-10-21
Context: The v1 plan defaulted to reusing Ḥafṣ line anchors for every riwāyah and absorbing
differences by justification.
Evidence: `docs/spikes/s4-layouts.md`; QUL has no non-Ḥafṣ layouts (T0.2).
- KFGQPC gives every riwāyah its own complete 604-page, 15-line layout, per ayah
  (`page`, `line_start`, `line_end`; an ayah crossing a page has `page` "a-b"). All 8 pass
  consistency checks (no missing pages, no out-of-range lines, no ordering errors).
- These are distinct prints: Shuʿbah's page assignments equal Ḥafṣ's (6236/6236); the
  others differ from Ḥafṣ for about 400 ayahs each.
- Granularity is the ayah: ~4,450 ayahs per riwāyah span several lines, and the word at
  which those lines break is not given.
Options:
- A: Reuse Ḥafṣ anchors for all riwāyāt (v1 default). Pages would not match the
  riwāyah's own print for ~400 ayahs, and overflow grows where texts differ.
- B: Native layout per riwāyah: KFGQPC page/line per ayah as hard constraints; word-level
  breaks inside multi-line ayahs fitted with the riwāyah's own font and flagged where
  ambiguous; owner spot-checks against the print. Each riwāyah gets its own `layout_id`.
- C: B where fitting is unambiguous, A as fallback, with every fallback line reported.
Leaning: B (C as a safety net), so each riwāyah looks like its own printed muṣḥaf. This
makes line anchors per-riwāyah data (T2.6 must map patched and inserted clusters to the
riwāyah layout, not only to Ḥafṣ anchors).
Decision (owner, GATE G0, 2026-10-21): B, native layouts per riwāyah, with C's reporting
for any line that falls back. Finalized in Phase 5 once fitting results are known.

## D7 — Licensing and distribution
Status: accepted
Date: 2026-10-21
Context: Rule 10 requires a license row before any source is copied into a tracked location.
Decision (owner, 2026-10-21): all sources are treated as open source and no license research
is done. `data/sources/LICENSES.md` records this per source group.
Recorded for the owner, as seen incidentally (not researched):
- Tanzil's file header says CC BY 3.0, verbatim copies only, text may not be changed. It is
  used read-only as a cross-check (T1.6) and is not shipped.
- quran-madina-html (Waqf GPL 2.0) is a reference and test oracle, cloned to /tmp only and
  never vendored. DigitalKhatt's justifier, which the Rust port (D3) follows, is MIT.
- visualmetafont and DigitalKhatt's LuaLaTeX package are AGPL-3.0 and are not used.
- harfrust (shaping, D3) is MIT.
Consequences: fonts and data may be tracked and bundled where the build needs them.
Revisit before public release (Phase 10) and before any free-edit feature ships, since some
upstream terms restrict changing the text.

## D11 — Spine file format and item IDs
Status: accepted (approved with the Phase 1 plan)
Date: 2026-10-21
Context: The spine must be tracked text (rule 3), lossless, reviewable in diffs, and give
every word, sign and cluster a stable ID.
Decision:
- Files under `data/spine/hafs/v1/` (`script_id: uthmani`, `spine_version: 1`):
  `items.jsonl` (one item per line, reading order) and `manifest.json` (source id and
  sha256, codepoint-table sha256, computed counts).
- Item kinds and IDs: `word` = `s:a:1..n`; `ayah_end` = `s:a:n+1` (as QUL numbers its
  marker); `hizb` = `s:a:0` (the rub el hizb sign always opens its ayah in the source).
  The sajdah sign stays inside its word as a `sign` cluster, as the source writes it.
- Separators are stored exactly: `sep_after` per item and, for listed source anomalies only,
  `sep_before` on an ayah's first item (2:6).
- Codepoints are written as `U+XXXX` strings (ASCII only), so no invisible or look-alike
  character can hide in tracked data or its diffs.
- Lossless invariant (tested): per ayah, `sep_before + cps + sep_after` over all items
  reproduces KFGQPC's `aya_text` byte for byte.

### D6 addendum — reconfirmed after the print comparison (2026-10-22)
The Phase 1 review found that the owner's printed muṣḥaf and KFGQPC's scanned "Medium Medina
Mushaf" follow the 1421H line layout (QUL V2), while the layout built in Phase 1 follows
KFGQPC's digital 1441H data (QUL V4, `quran-qcf4`, KFGQPC per-ayah page fields, which agree
with each other). They differ on 1,666 lines across 375 pages, mostly an ayah marker at the
end of a line versus the start of the next (`docs/review/layout-edition-check.md`).
Decision (owner, 2026-10-22): **keep the 1441H digital layout as built**, knowing it differs
from the owner's printed copy.
Consequences:
- Layout correctness is established by agreement of the three digital 1441H sources, not by
  comparison with the owner's print; where those sources disagree (G-011), the evidence is
  recorded per correction.
- Owner spot checks against a printed copy are expected to show these differences and are not
  an acceptance criterion for this layout.
- The rule-5 exception for DigitalKhatt Madina (a 1421H design) on 1441H lines stays (D2).


## D12 — Core model: file formats, patch semantics, DB access
Status: accepted (approved with the Phase 2 plan, 2026-10-22)
Date: 2026-10-22
Context: Phase 2 needs exact, deterministic semantics for every op in §3.2 and D9, file
formats for patches, option groups and ayah maps, and a way to read `mushaf.db` from the core
that still builds for wasm32, iOS and Android.
Decision:
- **DB reader is host-only.** `core/mushaf-db` (rusqlite, bundled SQLite) only runs SELECTs; all
  row-to-model logic is in `mushaf-model`, which is portable. `mushaf-db` is excluded from
  `just core-targets`, like `mushaf-render-cli`, because bundled SQLite needs a C toolchain per
  target (Android NDK, a wasm sysroot). Web DB access is decided in T8.2, mobile in Phase 9 (G-032).
- **File formats.** Patch, option-group and ayah-map files are YAML, ASCII only (option titles
  may be Arabic), codepoints as canonical `"U+XXXX"` strings. A patch file is
  `{spine: <spine id>, patches: [...]}`; the `spine` header pins the id space (D9). Ids and other
  strings are quoted: PyYAML (YAML 1.1) reads an unquoted `1:1:1:1` as a base-60 integer and
  `yes`/`off` as booleans. The Rust side reads YAML 1.2 strictly (no duplicate or merge keys,
  only `true`/`false` as booleans, no unquoted numbers in string fields); `compile_db` rejects
  any value whose type is not the one the schema expects, and the T2.2 parity test compares the
  two readers on the same files. Rust writes YAML with every string quoted.
- **Attributes are integers** (D10): `width_em_delta` in 1/1000 em, `stretch` in 1/1000 (its
  meaning is fixed in T3.2), `annotations` string to string. Non-integer numbers are rejected.
- **Ids** (`core/mushaf-model/src/ids.rs`): item `s:a:n`, cluster `s:a:w:c`, inserted
  `<anchor>+ins.<seq>` (nestable), the clusters of an inserted item `<item-ins-id>:<k>`, mark
  `<cluster>#m<n>`. Canonical decimal only, so each id has exactly one spelling. Order never
  comes from id strings.
- **Ops**: `delete` (item or cluster); `replace` (cluster: `{kind, cps}`; item:
  `{sep_before?, sep_after?}`, since an item's clusters change through cluster ops);
  `insert_before`/`insert_after` (`{seq, item}` next to an item, `{seq, cluster}` next to a
  cluster; new id `<target>+ins.<seq>`; siblings on one side are ordered by integer `seq`);
  `set_marks` (`remove: [cp]` removes the first occurrence, then `add` appends a bare `cp` or
  inserts `{cp, at}` at mark index `at`); `clear_marks`; `set_attr` (set
  `width_em_delta`/`stretch`/`annotations`, or `unset` them). "Marks" are a cluster's codepoints
  after its first one (a joiner counts as a mark position). Mark ids parse, but marks change
  only through `set_marks`.
- **Layers**: a patch with `when` is in the option layer; otherwise its layer is
  `owner.level` (`qiraah`, `riwayah`, `tariq`, `user`). Priority
  `qiraah < riwayah < tariq < option < user`.
- **Errors** (resolution fails): two patches with the same key in one layer (an insert's key is
  the id it creates); in one layer, a patch on a cluster of an item that layer deletes; a patch
  file for another spine; an unknown option group, an unknown choice, or no choice and no default.
- **Reported, never dropped**: *orphan* (target or anchor deleted by a lower layer: a
  modification is not applied; an insert is still placed at the deleted anchor's position and
  marked applied); *unresolved* (the id does not exist); *id collision* (an insert creates an id
  a lower layer already created; not applied); *ineffective* (removing an absent mark; an
  out-of-range `at` is appended). `Mode::Data` turns any report into an error; `Mode::Free`
  returns the result with the report (rule 11).
- **Engine**: a tree with tombstones. Deleted nodes keep their place, and each node keeps its
  before/after inserted children sorted by `seq`; the output is the in-order walk. Inside a
  layer the result therefore does not depend on the order patches are listed in.
- **Statuses in patch files**: `raw-derived`, `rule-derived`, and `user-edit` (user layer
  only, and required there). `cross-checked` and `muqri-verified` are rejected (rule 4).
  `single_source: true` is the optional §3.5 tag.
- **Option defaults**: a list of `{riwayah, tariq?, choice}`. The user's selection wins, then
  the ṭarīq's default, then the riwāyah's; with none of these, resolution fails rather
  than guess. `allowed_for` produces lint notes only. A patch with `when.option = g` must be
  listed in `g.affects`.
Consequences: the Phase 4 differ emits exactly these ops (word replacement = delete + insert,
separator changes = item `replace`). Riwāyah profiles (T5.1) choose which patch files form the
stack. Revisit together with D9 at the end of Phase 2.

### D9 / D12 addendum — revisited at the end of Phase 2 (2026-10-22)
Status: approved by the owner at the Phase 2 gate (2026-10-22), with one owner decision on
line attachment (below). Phase 2 is approved as built. No other change to D9 is made; the
other points record what building it showed.
- **Implemented as decided**, with these consequences found while building (all tested in
  `core/mushaf-resolve/tests/`): patch ids must be unique across the whole active stack (the
  Phase 4 differ must emit globally unique ids, e.g. prefixed with the riwāyah); user-level
  patches are accepted only from the edit overlay, never from data files; patches built in
  code go through the same validation as parsed ones; an item left with no clusters is
  reported (`EmptyItem`), so data builds fail on it.
- **Line attachment at a line start** (T2.6). The plan's first rule ("inserted clusters attach
  to the previous anchor") put an item inserted *before* a line's first item at the end of the
  previous line. **Owner decision (Phase 2 gate, 2026-10-22):** an insert-before joins the line
  of the word it precedes, and an insert-after joins the line of the word it follows; nested
  inserts follow the same rule relative to their own anchor. So every inserted item belongs to
  the line of the spine item its id starts with (`s:a:n` in `s:a:n+ins.k...`), whether or not
  that item is deleted. Deleted anchors still contribute nothing (their line starts or ends
  at the next surviving item). Implemented in `mushaf-resolve` (`Origin::Inserted { anchor }`)
  and `mushaf-layout`, with tests in `core/mushaf-layout/tests/mapping.rs`; PROJECT_PLAN T2.6
  updated. Native layouts per riwāyah (D1) still supply their own anchors in Phase 5.
- **Insert-id collisions after switching riwāyah** (G-034): an overlay insert with the same
  anchor and `seq` as a riwāyah's insert is reported (`IdCollision`) and not applied.
- **Whole-spine resolution** (G-035): every call rebuilds the tree for all 84k items. Fine for
  tests and data builds; per-page or incremental resolution belongs to T3.6/T7.4.

## D13 — Phase 4 inputs: Bazzi source, single-source texts, encoding differences
Status: accepted
Date: 2026-10-22
Context: Phase 4 derives raw patches for the 7 non-Ḥafṣ riwāyāt from KFGQPC. Three questions
had to be settled first (G-003, G-007, G-006).
Decisions (owner, 2026-10-22):
- **Bazzi source:** keep KFGQPC Bazzi v7 from the 2022 mirror (`kfgqpc-mirror-bazzi-data`,
  `kfgqpc-mirror-bazzi-font`). The official v2.0 zip was never archived. If it becomes
  available, Bazzi's patches are re-derived from it like any source change.
- **Single-source:** Shuʿbah, Warsh, Qālūn, Bazzi, Qunbul, Dūrī and Sūsī have no
  independent second text. All their derived data carries `single_source: true`. Their
  accuracy rests on exact reproduction of KFGQPC (round-trip, T4.6) and the owner's review
  sheets at GATE G2.
- **Encoding differences:** raw patches stay codepoint-exact against the Ḥafṣ spine, so the
  round-trip reproduces each KFGQPC text exactly. Each raw patch is additionally *tagged* with
  the kind of difference it carries (`encoding`, `spelling`, `reading`, or `unclassified`), so
  stats (T4.7) and review sheets (T4.8) can separate them. Tags are classifications for
  review, never applied as transformations, and only the owner confirms them. Any
  encoding conversion layer is deferred to Phase 6.
Consequences: the T4.4 differ and T4.5 patch format need a `difference_kind` tag (a
non-semantic field the resolver ignores); classification rules must be data-driven and
listed in the review sheets with their evidence, and anything they cannot classify is
`unclassified`, never guessed.

## D14 — KFGQPC to DigitalKhatt Madina encoding table (Ḥafṣ)
Status: accepted (approach approved with the Phase 3 plan; the two open points decided by the
owner during T3.1, 2026-10-22)
Date: 2026-10-22
Context: D2 renders the KFGQPC-encoded Ḥafṣ spine with DigitalKhatt Madina, which uses its own
encoding (G-006, G-018). The conversion must come from data, never from hand-written rows, and
must be checked on every word.
Method (`data/pipeline/dk_encoding.py`, run by `just data-build`):
- DigitalKhatt's own Madina text (`digitalkhatt-text-madina`) is split into ayahs; its lines
  are typed by QUL's 1421H layout, which it follows (S1). Its codepoints have their own class
  table (`data/encodings/digitalkhatt-madina-codepoint-classes.tsv`, Unicode category plus the
  font's GDEF class as evidence). Its hizb sign, glued to the next word, is split off.
- Words pair one to one per ayah with equal letter skeletons (`normalize.skeleton`); the three
  G-012 words pair with two DigitalKhatt words joined.
- Cluster rows from words with equally many clusters (one counterpart, or the identical one
  if there are several); sequence rows from the differing window of the other words, accepted
  only if they never break a word that cluster rows already convert. The core applies the
  table like the pipeline: the longest sequence first, else the cluster row, else an error.
- Every spine item is converted and compared with DigitalKhatt's word; the result is the G-024
  sheet `docs/review/g024-dk-text-diff.html`.
Result (from pinned sources): 1,354 rows (2 sequence rows, 1 owner row); all 83,865 spine items
convert; 83,863 equal DigitalKhatt's text; 2 differences (2:97:5, 17:7:12); no ambiguous
forms; 1 rejected context-dependent candidate (U+0633 U+064F in 17:7:12). The table also
records a sha256 over every converted item, which the Rust converter must reproduce
(`core/mushaf-shape/tests/encoding_spine.rs`).
Findings beyond S2: S2 counted one-to-one cluster forms, but a conversion can still merge two
KFGQPC forms into one output. 10 rows are such **lossy** rows (in each group of rows with the
same output, the identity row, else the most used one, is the reference; the others are lossy),
used by 33 words: 26 with U+06E4 small high madda (no glyph in DigitalKhatt Madina, absent from
its text) and 7 with yeh + hamza below, which DigitalKhatt writes with hamza above.
Decisions (owner, 2026-10-22):
- **Lossy words:** render DigitalKhatt's form for now; the exact 33 are listed (`lossy_items` in
  the table, G-040) and pinned by tests, so any new lossy word fails. To be resolved before
  release.
- **2:97:5:** its standalone tatweel cluster has no counterpart in DigitalKhatt's text, so no
  row can be derived. The owner approved one identity row (U+0640 to U+0640; the font has a
  tatweel glyph), kept in `data/encodings/hafs-v1.owner-rows.json` with the decision text.
  The 1441H text renders as stored.
Consequences: free-mode edits that create a cluster form with no row cannot be converted yet;
the edit API (Phase 7) must handle that (reported, never silently passed through). Marks that
occur only in other riwāyāt still need owner-approved rows (D2), in Phase 5.

## D15 — Page geometry, short lines and overflow (Ḥafṣ, 1441H)
Status: accepted (policies approved with the Phase 3 plan, 2026-10-22); the constants are for
the owner to judge on rendered pages at GATE G1
Date: 2026-10-22
Context: DigitalKhatt Madina's natural widths on the 1441H lines spread widely (p1 11,797,
median 15,530, p99 19,820 font units; the font's own 1421H lines spread the same way), so one
line width cannot fit every line without stretching some a lot and squeezing others (G-021).
DigitalKhatt gives target widths only for its 1421H short lines (G-020).
Decision:
- **Profile** `data/render/hafs-uthmani-1441h.json`: font, encoding table, line widths, surah
  names, and design constants: `line_width` 17,800 font units (about p90 of the natural
  widths), `line_pitch` 1,900, margins, `page_scale_floor` 900 per mille, `space_stretch` 1,000
  per mille (DigitalKhatt's reference value), `min_space` 600 per mille.
- **Short lines (G-020)** come from the print itself: `quran-qcf4` (pinned, 1441H) gives each
  line's glyph advances; a line under 90% of the median full line is short and gets its own
  ratio (`data/render/uthmani-1441h.line-widths.json`, `pipeline/line_widths.py`). Short lines
  are justified to that share of the line width and centred. This finds the layout's 29
  `centered` lines plus page 2 line 5, which QUL marks not centred but which prints at 0.83 of
  a full line (G-041).
- **Overflow (G-021):** one font scale per page, `clamp(target / widest line at minimum
  spacing, floor, 1)`; every line is justified to `target / scale` in font units. A line still
  too wide tightens its simple spaces to 60% and is then squeezed horizontally by exactly
  `target / width`; each such line is reported. Spaces next to ayah numbers keep their width.
- **Justifier** (T3.2): the DigitalKhatt reference (stops at the first overflow; not the
  "exhaustive" variant of quran-madina-html). Its overflow path is ours, as above.
- **Header lines**: surah names (KFGQPC `sura_name_ar`) and the basmalah (spine 1:1:1–4,
  converted; equal to all 112 DigitalKhatt basmalah lines) keep their natural width, centred.
Evidence (all 604 pages, `cargo run --release -p mushaf-layout --example fit_report`):

| line_width | pages scaled | at floor | squeezed lines | min squeeze | widest space, median / p95 / max (× normal) |
|---|---|---|---|---|---|
| **17,800 (chosen)** | 209 | 32 | 65 | 0.879 | 2.85 / 3.79 / 5.87 |
| 17,000 | 323 | 84 | 223 | 0.839 | 2.37 / 3.78 / 5.28 |
| 16,500 | 401 | 131 | 419 | 0.814 | 2.28 / 3.78 / 5.54 |

With 17,800, 6,785 of 8,820 text lines stretch letters (median gain 1,845 font units).
Line pitch (added 2026-10-22, at the owner's request after the G1 print check): an ink-collision
check (`mushaf-render-cli/src/collide.rs`, in `just golden`) rasterises each line's glyph
outlines and header frames and tests every pair of adjacent lines on all 604 pages (8,442
pairs). At 1,900 none touch. The tightest collision-free pitch is 1,890 (page 410, lines
14-15: a stretched letter's tail above a pause sign); at the print's pitch of about 1,780, 7
pairs would collide (1,800: 3; 1,850: 1). 1,900 is kept.

| Line pitch | Pairs that collide |
|---|---|
| 1,700 | 64 |
| 1,750 | 21 |
| 1,780 (the print's proportion) | 7 |
| 1,800 | 3 |
| 1,850 | 1 |
| **1,900 (chosen)** | **0** |

Consequences: page scale depends on the resolved page, so an edit or a riwāyah change can change
a whole page's size (deterministically). Wide spaces on loose lines are the price of few
squeezes; the owner judges the balance on the G1 pages. The constants are data: changing them
changes goldens, not code.

## D4 — Draw path
Status: accepted (approved with the Phase 3 plan, 2026-10-22); revisit for performance in
Phase 8
Date: 2026-10-22
Context: The core must return something every platform can draw identically (rule 7, rule 8):
the CLI and website now, iOS and Android later.
Options:
- A: positioned glyph outlines. The core returns each used glyph's outline once and a list of
  glyph placements; platforms fill paths (SVG, Canvas `Path2D`, CoreGraphics, Android `Path`).
- B: glyph ids and positions; platforms draw with their own font APIs. Browsers cannot draw a
  font's glyph by id, and platform rasterisers differ.
- C: the core rasterises. Blurry when zoomed, heavy, no hit-testing by cluster.
Decision: **A**. `mushaf-draw::DrawPage`: outlines in 26.6 font units (y up) for the glyphs
on the page; items in 26.6 page units (y down): `Glyph { glyph, x, y, sx, sy, line, cluster }`
(16.16 scales; `cluster` is the source cluster id) and `Stroke { role, width, path }`
(surah-header frames, the edited marker). Outlines come from the font's CFF2 table through
`read-fonts` (the version harfrust uses), whose charstring evaluator reports 16.16 fixed-point
coordinates; they are rounded to 26.6 with integers only, so no floating point is involved
anywhere in `core/` (better than the plan's proposal, which expected one float step via
skrifa). Checked against fontTools on 150 glyphs: identical within 1/64 unit. The SVG backend
writes every number as an exact decimal of its integer.
Consequences: a page carries its own glyph table (no font needed on the platform); per-glyph
colour, resize and hit-testing are straightforward. Size and speed of the draw list are
measured in T3.6 and revisited for the web in Phase 8.

## D16 — Manual overrides, header text, and the "edited" marker in draw output
Status: accepted (approved with the Phase 3 plan, 2026-10-22)
Date: 2026-10-22
Context: D12 left the meaning of `stretch` to T3.2; rule 11 needs the "edited" marker in every
render; header lines need text that is not spine text.
Decision:
- **`width_em_delta`** (1/1000 em) on a cluster: exactly that much extra advance after the
  cluster's last glyph, counted in the line's width. On an item: after its last cluster.
- **`stretch`** (1/1000 em) on a cluster: extra width made with the font's own stretch
  features on that letter, the smallest level that reaches it (a kashida towards the next
  joined letter, or the wider final form of a sub-word's last letter, up to the justifier's
  limits). Unreachable requests are reported (`StretchUnreachable`, with what was reached);
  `stretch` on a non-letter or on a whole item is reported (`StretchNotOnLetter`).
- Letters carrying an override are locked: the automatic justifier never changes them, and
  the line still closes around them.
- `width_em_delta` is a flat advance: inside a joined word it opens a visible gap (T3.7 demo
  on page 3); `stretch` is the joined way to widen a letter.
- **Canonical / edited:** `Resolved::canonical()` is false when the stack has any user-layer
  patch (`Resolved::user_patches`) or the resolver reported anything. A non-canonical page
  always gets the edited marker as its last draw items (a red frame and the word EDITED as
  strokes, no font needed); `draw_page` has no way to leave it out. Phase 7's lint refines what
  counts as non-canonical; the mechanism stays.
- **Header text:** the surah-name line shows KFGQPC's `sura_name_ar` (1441H source, metadata,
  shaped as stored: DigitalKhatt Madina maps its plain letters) inside a frame; the print's
  ornamental header and the word "surah" are not reproduced (no source). The basmalah line is
  the spine's 1:1:1 to 1:1:4, converted, which equals all 112 DigitalKhatt basmalah lines.

## D20 — Phase 4 derivation pipeline: ingest, aligner, differ, emitter
Status: accepted (approved with the Phase 4 plan, 2026-10-22)
Date: 2026-10-22
Context: T4.1–T4.5 turn each KFGQPC riwāyah text into raw patches on the Ḥafṣ spine that
reproduce it codepoint for codepoint (D13). Each step must be deterministic and must report,
never guess.
Decision:
- **Ingest** (`pipeline/ingest_riwayah.py`, T4.1): one parser for all 8 texts. `ingest_kfgqpc`
  stays the spine's parser, and a test checks that both give identical Ḥafṣ items. Ayah
  numbers are a U+FC00 sign or, for Bazzi v7, Arabic-Indic digits (`AYAH_NUMBER_STYLE`).
  Structural departures (`leading_separator`, `multi_separator`, `hizb_inside_ayah`,
  `attached_ayah_number`, `digit_ayah_number`) are accepted only from the exact per-riwāyah
  `ANOMALIES` list: found-but-unlisted and listed-but-not-found are both errors (G-070). The
  ingested texts are rebuilt from verified sources and not tracked;
  `docs/review/phase4/ingest.md` is tracked and checked by `data-verify`.
- **Skeleton**: the one normalization function (`normalize.skeleton`) also maps yeh barree
  U+06D2 to U+0649. It is used only for comparison, never on stored text. Without it, about
  3,000 Warsh/Qālūn words would not align.
- **Aligner** (T4.3): per surah, an LCS over tokens (word skeletons, ayah ends, hizb signs),
  so ayah ends act as anchors. Replace blocks pair words by position (n = m) or by skeleton
  edit distance (n ≠ m). Every pair with different skeletons and every unpaired word is listed
  in the alignment report.
- **Differ** (T4.4): paired words become cluster ops (`set_marks` when the base letter is
  equal, checked by simulating the resolver; `replace` otherwise; cluster `delete`/`insert`).
  Separator changes become an item `replace`. Unpaired words become an item `delete` plus an
  item insert anchored on the neighbouring Ḥafṣ item (D12 replacement). Ayah-end changes are
  ordinary item and cluster ops.
- **Emitter** (T4.5): `data/patches/raw/<riwayah>.yaml`, one flow-style patch per line, every
  string quoted, ASCII only. Patch ids are `<riwayah>.<key>.<op>`, unique across the stack.
  Every patch has `status: raw-derived`, `single_source: true` and a `difference_kind` (D21).
  Output is deterministic and rebuilt by `just data-build`.
Consequences: the patch files are large where encodings differ (Warsh/Qālūn), since every
codepoint difference is a patch (D13). Exactness is proved by the round-trip (T4.6), not by
the aligner: a poor alignment can only make patches less readable, never wrong.
- Measured sizes (T4.5): warsh 25.4 MB, qalun 24.3 MB, bazzi 10.6 MB, susi 8.6 MB,
  qunbul 7.1 MB, duri_abuamr 7.1 MB, shuba 0.26 MB. To read them in reasonable time,
  `compile_db` uses libyaml's parser with the same strict checks and parses each file once
  per process. The Rust YAML reader's resource budget is raised for size only, and
  third-party crates are optimized in dev/test builds (`core/Cargo.toml`).

## D21 — Difference-kind tags: what the data can and cannot classify
Status: accepted (owner, 2026-10-22, before GATE G2)
Date: 2026-10-22
Context: D13 asks for a `difference_kind` tag per raw patch (`encoding`, `spelling`,
`reading`, `unclassified`). The rules must be data-driven, evidence must be shown, and
anything uncertain stays `unclassified`. The Phase 4 plan proposed tagging `encoding` for a
Ḥafṣ form X that the riwāyah writes as Y when the substitution is **total** (every aligned X
becomes Y) and **exclusive** (X never occurs in the riwāyah's text).
Evidence (Phase 4 probe over the pinned texts and the T4.3 alignment):
- At the codepoint level, totality almost never holds, because reading differences touch the
  same codepoints. For example, Warsh never uses U+06E1. Of its 37,122 aligned occurrences in
  Ḥafṣ, 32,383 become U+0652, 1,996 become U+064E, and 1,473 are part of larger changes.
  The rule tags 1–2 patches per riwāyah.
- At the cluster-form level, total and exclusive substitutions with at least 50 occurrences
  include Bazzi `0645 06E1 06D7 → 0645 064F` (77×) and `0649 0670 0653 → 0649 0670` (378×).
  Whether such a change is an encoding, a spelling or a reading difference is exactly what the
  data cannot establish; frequency and consistency are not evidence of "same mark".
- The fonts give no independent evidence either: KFGQPC glyph names are derived from
  codepoints (`uni06E1`, `afii57458`), not from what a mark means.
Options:
- A: the planned total+exclusive rule, with a minimum count. It tags changes whose nature is
  unknown as `encoding`, which is a guess (rejected under D13).
- B: frequency-based "dominant counterpart" rules (e.g. X absent from the riwāyah and X→Y in
  most aligned places). Same objection, with an arbitrary threshold.
- C (implemented): no automatic `encoding`, `spelling` or `reading`. Rules are: `ayah-number`
  (any change to an ayah-end item) → `unclassified`; `confirmed` → the kind of an
  owner-confirmed substitution row in `data/approvals/difference_kinds.yaml` (riwāyah,
  `from`, `to`, kind, by, date, evidence), applied only when every codepoint difference of a
  cluster change is a confirmed one-for-one substitution of one kind; `default` →
  `unclassified`. The table has no rows. Stats (T4.7) list the candidate substitutions with
  their counts, so the owner can confirm rows.
Decision (owner, 2026-10-22): **C**, with these choices on the evidence in `stats.md`:
- Confirmed rows live in **`data/approvals/difference_kinds.yaml`**: only owner-signed commits
  may change it (CI, rule 4); the pipeline reads it and treats a missing file as no rows;
  Claude never writes it.
- **U+06E1 → U+0652 in Warsh and Qālūn: `encoding`.** Evidence: U+06E1 never occurs in either
  text; 32,383 of 37,122 (Warsh) and 36,538 of 37,121 (Qālūn) aligned occurrences are U+0652;
  G-006. With these rows, 31,906 Warsh and 35,962 Qālūn patches are tagged `encoding`; the
  5,238 / 1,182 changes that also alter something else stay `unclassified`.
- **Pause signs U+06DA, U+06D7, U+06D8 → U+06D6: left `unclassified`.** Several Ḥafṣ signs
  map to one, so it is not a one-to-one encoding; to be decided with a cited source.
- **U+0671 (alef waṣla): left `unclassified` for now.** Most occurrences become U+0627 plus
  added marks, which a one-for-one row cannot express; revisit with the encoding conversion
  (Phase 6), possibly with cluster-form rows.
- U+06ED → U+06E2, U+06E0 and U+06E4 were not confirmed.
Consequences: until rows are confirmed, every raw patch is `unclassified`, and the stats
separate differences by op shape (marks / letter / word / separator / ayah number), not by
kind. Adding a confirmed row changes only tags, never text, and is rebuilt by `data-build`.

## D22 — Round-trip check for raw patches
Status: accepted (approved with the Phase 4 plan, 2026-10-22)
Date: 2026-10-22
Context: T4.6 must prove that applying each riwāyah's raw patches to the Ḥafṣ spine gives
that riwāyah's KFGQPC text exactly, using the one resolver (rule 7).
Decision:
- `core/mushaf-roundtrip` (host-only binary, excluded from `core-targets` like `mushaf-db`)
  loads the data root, resolves `patches/raw/<riwayah>.yaml` on its spine with
  `mushaf_resolve::resolve` in `Mode::Data` (any report entry fails), and writes the resolved
  items as JSON lines (`build/roundtrip/`).
- `pipeline/roundtrip.py` compares them with the pinned source:
  1. the whole text, codepoint for codepoint;
  2. each ayah, splitting after every ayah-end item, in order;
  3. the tracked ayah map (T4.2), whose `first`/`last` must be each ayah's first and last
     resolved item.
  It prints the share of exact ayahs and exits 1 on any difference.
- `just roundtrip [RIWAYAH...]` runs both. CI runs it after the other checks.
- `pipeline/drift.py` writes `docs/review/phase4/version-drift.md`: official v2.x vs the 2022
  mirror for every riwāyah that has both, and official Warsh vs QUL `qpc-warsh` words. Ayah
  numbers are compared by value. Differences are grouped by codepoint change and not judged.
Result (2026-10-22): 100% for all 7 riwāyāt, with no change to the resolver.

## GATE G2 — Phase 4 review: passed (2026-10-22)
Owner decisions at GATE G2, given in the planning session and relayed to the Phase 4 session:
- **Phase 4 approved** (T4.1–T4.8, PR #2). The raw patches in `data/patches/raw/` are exact
  mirrors of the pinned KFGQPC texts (`just roundtrip`: 100% for all 7 riwāyāt). They are
  **not yet verified qirāʾāt knowledge**: every patch is `raw-derived` and `single_source`.
- **D21** was accepted before the gate. The owner-signed rows file
  `data/approvals/difference_kinds.yaml` (commit 775794f) confirms U+06E1 → U+0652 as
  `encoding` for Warsh and Qālūn.
- **Version drift (G-073):** the official v2.x releases are authoritative where they differ from
  the 2022 mirror, as D13 assumes. The mark-level differences stay in
  `docs/review/phase4/version-drift.md` for the muqriʾ review (Phase 6 / GATE G4).
- **Hypotheses (G-074):** the plan's expected cases (Warsh 1:4→1:3, 3:133, 2:10, 2:184) stay
  documented observations, not tests, until a cited source is available.

## D17 — Superscript alef after non-joining letters (G-045)
Status: rejected by the owner after reviewing the rendered pages; reverted (2026-10-22)
Date: 2026-10-22
Context: G-045. The pinned DigitalKhatt Madina font's `rclt` maps U+0670 after a non-joining
letter (1,377 places, 518 pages) to `glyph00103`, a spacing upright alef in a gap; the owner's
1441H reference shows a small alef over the join (صِرَٰطَ, page 1).
Tried (commit 1f5a0e5): a derived font with that one substitution changed to `glyph00102`, an
anchored superscript-alef mark (all 9,726 U+0670 anchored; rule 6, oracle parity, line fit and
collisions kept; line pitch 1,920). Rejected: on the rendered pages the anchored mark sits on the
stem of the following letter (e.g. on the ṭāʾ of صِرَٰطَ), which reads as a standing alif on top
of the ط. The comparison sheet used before had been looked at too small to show this. An
earlier option, `rclt`=0 on U+0670 at run time, leaves the alef unanchored (rule 6) and was not
taken.
Decision (owner): go back to the previous rendering (pinned font, line pitch 1,900). G-045 stays
open; any new fix must be judged on full-size rendered lines against the print before it lands.
Remaining options for G-045 (none chosen; resolve before public release, Phase 10):
(a) `rclt`=0 on U+0670 after non-joining letters at run time, with a documented rule-6
exception for those marks and a placement test (not yet shown to the owner); (b) proper font
work that anchors the mark to the *preceding* letter (D17 anchored it to the following one);
(c) an upstream report to DigitalKhatt (owner's call).

## D18 — Owner print corrections at G1: sajdah overline and owner word rows
Status: accepted (owner decisions at GATE G1, 2026-10-22)
Date: 2026-10-22
Context: the G1 check against the 1441H print (docs/review/phase3-g1.md §3a) found three places
where our pages differ from the print: the 26 KFGQPC words with U+06E4 (G-040), 17:7:12
(G-046) and 2:97:5 (G-047). Owner: "I want the sajdah overline", "I want the small waw and
madda" (17:7:12), "I don't want the tatweel" (2:97:5).
Evidence:
- KFGQPC's own font draws U+06E4 as a bar over the word, and the print (quran-qcf4 word glyphs,
  `qcf4-npm-1.1.0`) has a bar over exactly these 26 words. Measured on the print's glyphs: bar
  101–124 of 2,500 units thick (0.040–0.050 em); one height per line; its gap above the line's
  tallest overlined word 120–193 units (0.05–0.08 em); one continuous bar over adjacent
  overlined words (13:15:1–2, 17:107:16–18, 53:62:1–2). DigitalKhatt Madina has no such mark;
  the encoding table drops U+06E4 (lossy rows, D14).
- 17:7:12 and 2:97:5: the print and DigitalKhatt's text agree with each other and differ from
  KFGQPC v2.0 (§3a).
Options for the overline: (a) draw nothing (as before); (b) a derived font with an overline mark
(rejected: D17 showed derived-font changes are risky, and the bar spans words); (c) the core draws
the bar as a filled rectangle. Options for the two words: (a) edit the spine (rejected: rule 1,
and Phase 4's round-trip is against KFGQPC); (b) a per-cluster encoding row (rejected: the
change is word-level and would leak into other words); (c) an owner word row.
Decision:
- **Sajdah overline (c).** The render profile lists the source codepoints that mark a word
  (`overline_marks`: U+06E4) and the bar's `overline_thickness` (44) and `overline_clearance`
  (70, font units of a 1000-unit em). A word whose resolved clusters contain a mark gets
  `FitWord::overline`; `mushaf-draw` emits one `DrawItem::Fill { role: SajdahOverline }` per run
  of adjacent overlined words on a line, spanning their ink (outline on-curve and control
  points), at one height per line: clearance above the line's tallest overlined word. Integer
  arithmetic only. Because the marks are read from the resolved clusters, a free edit that adds
  or removes U+06E4 adds or removes the bar. The collision check counts the bar as ink (pitch
  1,900: no collisions).
- **Owner word rows (c).** `data/encodings/hafs-v1.owner-words.json` lists spine items the owner
  decided to render in DigitalKhatt's own form; `dk_encoding.py` takes that form from the
  aligned DigitalKhatt word (never typed) into the table's `word_rows` (item, KFGQPC clusters,
  DK word). `EncodingTable::convert_item` uses a word row only when the item id and its
  clusters both match exactly, else converts cluster by cluster, so an edited word still renders
  (rule 11); the build fails if a word row's clusters could not convert on their own. The
  spine is unchanged (KFGQPC); the G-024 sheet lists the word rows (§3a).
  The U+0640 identity row (T3.1) is kept for that fallback.
Consequences: text differences in the G-024 sheet drop to 0; the converted-spine digest changes;
golden pages are unchanged (none of them has these words). Hit-testing on a word-row word maps
every glyph to the word's first cluster (one span).

## GATE G1 — Phase 3 review: passed (2026-10-22)
Owner decisions at GATE G1, given in the planning session ("okay finish phase 3") and relayed to
the Phase 3 session:
- **Phase 3 approved** (T3.1–T3.7, T3.5a, PR #1) on the branch state at the gate: the pinned
  DigitalKhatt Madina font (`fonts/digitalkhatt/hb-madina.otf`, no derived font), line pitch
  1,900 (D15), and the goldens in `tests/golden/` as committed.
- **Surah headers stay plain** for Stage 1 (D16); the print's ornament is later polish.
- **D17 rejected**; **D18 accepted** (sajdah overline; owner word rows for 17:7:12 and 2:97:5).
- **G-045 stays open as a known deviation from the print:** after a non-joining letter, U+0670
  (1,377 places on 518 pages) is drawn as a spacing upright alef, not a small alef over the join.
  Resolve before public release (Phase 10). Options: (a) `rclt`=0 on U+0670 after non-joining letters at run time, with a documented rule-6
exception for those marks and a placement test (not yet shown to the owner); (b) proper font
work that anchors the mark to the *preceding* letter (D17 anchored it to the following one);
(c) an upstream report to DigitalKhatt (owner's call).
- No separate decision was recorded for G-041 (page 2 line 5) and G-043 (hizb sign spacing);
  they stay open with their current rendering.

## D36: Display styles, Ḥijāzī and Maghribi for every riwāyah
Status: accepted (scope and phase); details decided in Phase 6
Date: 2026-10-23
Context: The owner wants the script style to be independent of the riwāyah, so that a user
can show any muṣḥaf in Ḥijāzī (Mashriqi) or Maghribi style.
Evidence: the KFGQPC texts encode style-specific mark conventions in their codepoints (G-006;
e.g. sukūn U+06E1 vs U+0652); the Maghribi letterforms (fā'/qāf dotting) come from the KFGQPC
Warsh/Qālūn fonts (S2); DigitalKhatt has no Maghribi design; no other Maghribi font is pinned.
Decision (owner, 2026-10-23): build it in **Phase 6 (T6.7–T6.10)**, after the encoding conversion
layer it depends on. Phase 5 prepares for it by keying font, justification mode and encoding
table by **(riwāyah, style)**, with a default style per riwāyah (Maghribi for Warsh/Qālūn,
Ḥijāzī for the rest), and implements only the defaults.
Consequences: style conversion is a render-time transformation from owner-confirmed rows,
never a change to the spine or patches; a combination a font cannot render correctly is
reported as unavailable rather than drawn wrongly (rule 6); line breaks remain each
riwāyah's own (D1).

