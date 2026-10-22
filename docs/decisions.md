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
| D4 | Draw path | open (P3) |
| D5 | First platform | accepted (website first) |
| D6 | Ḥafṣ edition: text + layout + font | accepted: 1441H digital layout (reconfirmed 2026-10-22) |
| D7 | Licensing and distribution | accepted (owner-declared) |
| D8 | Script/layout scope | accepted |
| D9 | ID and patch model details | accepted (revisited end of P2: no change proposed) |
| D10 | Numeric determinism | accepted |
| D11 | Spine file format and item IDs | accepted (Phase 1 plan) |
| D12 | Core model: formats, patch semantics, DB access | accepted (Phase 2 plan) |

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
Status: for owner review at the end of Phase 2. No change to D9 is proposed; these are what
building it showed.
- **Implemented as decided**, with these consequences found while building (all tested in
  `core/mushaf-resolve/tests/`): patch ids must be unique across the whole active stack (the
  Phase 4 differ must emit globally unique ids, e.g. prefixed with the riwāyah); user-level
  patches are accepted only from the edit overlay, never from data files; patches built in
  code go through the same validation as parsed ones; an item left with no clusters is
  reported (`EmptyItem`), so data builds fail on it.
- **Line attachment at a line start** (T2.6, plan rule "inserted clusters attach to the
  previous anchor"): an item inserted *before* the first item of a line lands at the end of the
  previous line. This follows the plan, but a riwāyah word that belongs at a line start will
  sit on the previous line on the Ḥafṣ anchors. Native layouts per riwāyah (D1) will
  supply their own anchors in Phase 5; the owner may prefer a different rule for that case.
- **Insert-id collisions after switching riwāyah** (G-034): an overlay insert with the same
  anchor and `seq` as a riwāyah's insert is reported (`IdCollision`) and not applied.
- **Whole-spine resolution** (G-035): every call rebuilds the tree for all 84k items. Fine for
  tests and data builds; per-page or incremental resolution belongs to T3.6/T7.4.
