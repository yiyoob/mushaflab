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
| D1 | Line policy for non-Ḥafṣ riwāyāt | accepted: native layouts; final form D29 (GATE G3, 2026-10-23) |
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
| D26 | Riwāyah profiles and surah transitions | accepted with the Phase 5 plan (2026-10-22) |
| D27 | Warsh and Qālūn on their KFGQPC fonts: TrueType outlines, spacing-only justification | accepted with the Phase 5 plan (P4); per-glyph exceptions decided (2026-10-22) |
| D28 | DigitalKhatt rows for cluster forms outside Ḥafṣ | accepted (approach with the plan, P3); groups decided at the T5.3 checkpoint (2026-10-22) |
| D29 | Native layouts: Ḥafṣ breaks inside each riwāyah's KFGQPC ayah lines (D1 final form) | accepted at GATE G3 (2026-10-23); Warsh/Qālūn breaks unverified (G-104) |
| D30 | Short lines and header basmalah on native layouts | accepted at GATE G3 (2026-10-23), 60% threshold |
| D31 | Overflow per riwāyah | accepted (plan P6; numbers reviewed at GATE G3, 2026-10-23) |
| D32 | Rule-5 edition notes per riwāyah | accepted at GATE G3 (2026-10-23) |
| D33 | Display style as a render key: (riwāyah, style), default style only in Phase 5 | accepted (owner requirement, relayed 2026-10-23; confirmed at GATE G3); implements D36's Phase 5 part |
| D36 | Display styles: Ḥijāzī and Maghribi for every riwāyah | accepted (scope); built in Phase 6 |
| D37 | Highlighting differences from Ḥafṣ | accepted (scope); built in Phase 6, UI in Phase 8 |
| D38 | Rule DSL: declarative patterns expanded into ordinary patches in the core | accepted with the T6.1 plan (2026-10-26) |
| D39 | Owner corrections to the Ḥafṣ spine text as patches; G-142 (52:37 sīn below the ṣād) | accepted (owner decision, 2026-10-26) |
| D40 | Annotation channel: pronunciation-only features as `pron.<feature>` cluster annotations | accepted (T6.3, 2026-10-26) |
| D41 | مَالِيَهْ idghām wajh changes no glyph (owner decision, departs from p. ح) | accepted (owner decision, 2026-10-26) |
| D42 | T6.4: data-derived rules (hypotheses) plus residual patches per riwāyah | accepted (T6.4, 2026-10-26); every rule awaits a citation |
| D43 | Ṭarīq as the option group `<riwāyah>.tariq`; ṭarīq-level patches apply under their ṭarīq only | accepted (T6.6, 2026-10-26) |
| D44 | Display styles: render-time conversion from owner-confirmed rows; per-(riwāyah, style) audit decides availability | accepted (T6.7–T6.10, 2026-10-26) |
| D45 | Difference highlighting from rule provenance; cited rules by default, hypotheses opt-in and flagged | accepted (T6.11–T6.12, 2026-10-26); the owner chose to show hypotheses (2026-10-26) |
| D46 | Warsh and Qālūn in the Ḥijāzī style: owner-approved writing conventions, own pause signs, blocked pages listed | accepted (owner, 2026-10-26) |

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
Phase 5: the final form is proposed in D29 for GATE G3 (Ḥafṣ breaks inside each riwāyah's KFGQPC
ayah lines, width-fitted lines listed; no fallback was needed).

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
  tatweel glyph), kept in `data/encodings/hafs-v1.owner-rows.json` with the decision text
  (owner-signed commits only since G-130).
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
- **Owner word rows (c).** `data/encodings/hafs-v1.owner-words.json` (owner-signed commits only,
  G-130) lists spine items the owner
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

## D26 — Riwāyah profiles and surah transitions (T5.1)
Status: accepted (approved with the Phase 5 plan, 2026-10-22)
Date: 2026-10-22
Context: T5.1 asks for one profile per riwāyah that selects its patch stack, ayah map and
surah-transition policy, "as data, to be verified against the source".
Decision:
- `data/profiles/<riwayah>.yaml` (ASCII, generated by `pipeline.profiles`, checked current by
  `data-verify`): `riwayah`, `spine`, `script_id`, `text_source` (pinned source id),
  `single_source`, `patch_stack` (patch files in layer order; Ḥafṣ: none), `ayah_map` (absent
  for Ḥafṣ), `surah_headers`. Loaded by `mushaf-model` (`RiwayahProfile`) and compiled into
  `mushaf.db` (schema v3, table `profiles`), with file/DB parity tested as for the other inputs.
  `DataSet` validation checks that every listed patch file and ayah map exists on the
  profile's spine. `mushaf_resolve::resolve_profile` resolves a selection through its profile;
  the renderer uses it.
- **Surah transitions are page data.** Before each surah's first ayah the riwāyah's KFGQPC file
  leaves lines without an ayah (S4, T4.1). Each takes the kind of the 1441H Ḥafṣ layout's line at
  the same page and line (surah name, basmalah). Result: every riwāyah has the same 114 header
  positions as Ḥafṣ, except Fātiḥah in Warsh, Qālūn, Dūrī and Sūsī, where page 1 line 2 has no
  ayah and Ḥafṣ has text (its numbered basmalah). That line is the only listed hypothesis
  (`HYPOTHESES` in the pipeline, `basmalah_evidence` in the profile): drawn as an unnumbered
  basmalah line until the owner checks the print (G-023; owner's choice with the plan). Any other
  gap line without a Ḥafṣ header kind fails the build.
- Recitation choices between surahs (basmalah, sakt, waṣl) are not page layout; they belong to
  option groups in Phase 6.
Consequences: resolution of a riwāyah no longer hard-codes file paths (the round-trip tool still
reads `patches/raw/<r>.yaml` directly, which the profile test shows gives the same result).

## D27 — Warsh and Qālūn on their KFGQPC fonts (T5.3, plan P4)
Status: accepted (approved with the Phase 5 plan, 2026-10-22); the rule-6 exceptions it found
are pending the owner (G-100)
Date: 2026-10-22
Context: D2 renders Warsh and Qālūn with their own KFGQPC fonts (Maghribi letterforms), justified
by spacing only. Those fonts are TrueType (glyf, 2048 units per em, 843 of 1351 glyphs
composite, 221 components with a transform), while the renderer read only CFF2; and they come
from the official zips, not from a file of their own.
Decision:
- **Fonts tracked** as `fonts/kfgqpc/uthmanic_warsh_v21.ttf` and `uthmanic_qaloun_v21.ttf`,
  byte-identical to the font member of the pinned official v2.1 zips. `fonts/FONTS.json` lists
  them with `member` and `sha256`; `license_check` verifies the bytes against that sha256 and,
  when the pinned zip is present, against the member itself.
- **TrueType outlines (D4 addendum)**: `PathOp::Quad` (quadratic segment; SVG `Q`, Canvas
  `quadraticCurveTo`, CoreGraphics and Android `quadTo`). Contours start at their first on-curve
  point; the implied on-curve midpoint of two off-curve points is exact in 26.6; composite
  glyphs are flattened with their F2Dot14 transforms in integer arithmetic (rounded half up),
  offsets unscaled; point-matched components and scaled offsets are rejected (neither font uses
  them). Checked against fontTools on every 25th glyph of the Warsh font within 2/64 unit
  (fontTools transforms in floating point).
- **Identity encoding**: the fonts read KFGQPC's codepoints as stored (S2: no `.notdef` or dotted
  circle in their own text), so `EncodingTable::identity` passes every cluster through, one span
  per cluster. A render profile with no encoding table uses it.
- **Spacing-only justification**: `Params::letters = false` (render profile
  `justify.letters`) skips letter stretching, so spaces take the whole difference; the D15
  overflow path (tighten, then squeeze) is unchanged. A space next to a KFGQPC ayah sign
  (U+FC00 onwards) is an ayah space, as one next to DigitalKhatt's U+06DD.
- **Rule 6**: every resolved Warsh and Qālūn item shaped in its font has no `.notdef`, no
  inserted dotted circle and a valid cluster map (`just golden`). The fonts leave 60 (Warsh) and
  142 (Qālūn) marks unanchored, in four glyphs (the same counts S2 found): pinned in
  `tests/golden/shape/kfgqpc-rule6-pending.json`, so any change fails, until the owner decides
  per glyph (G-100). No blanket waiver.
- **Owner at the T5.3 checkpoint (2026-10-22), on the full-size sheet
  `build/review/phase5/kfgqpc-rule6.html`: "Exempt 3, flag damma".** `uni06D6`, `_.035` and `up`
  (glyphs 109, 410, 114 in both fonts) are per-glyph exceptions, listed with the decision in
  `data/render/kfgqpc-rule6-exceptions.json` (owner-signed commits only, G-130) and applied by
  `mushaf_shape::Exceptions`; they may be
  marks without an anchor, every other check still applies. `afii57455` (glyph 86), a damma after
  U+06E8 that the font does not show (Warsh and Qālūn 12:110:11 and 21:88:7), stays a defect
  (G-101): those four words render as the font draws them and remain pinned.
- U+200F (G-017), which Warsh and Qālūn keep at 14 places, shapes without a glyph or violation.

## D28 — DigitalKhatt rows for cluster forms outside Ḥafṣ (T5.3, plan P3)
Status: accepted (approach approved with the Phase 5 plan; groups decided by the owner at the
T5.3 checkpoint, 2026-10-22)
Date: 2026-10-22
Context: D2 draws Shuʿbah, Bazzi, Qunbul, al-Dūrī and al-Sūsī with DigitalKhatt Madina through
the data-derived Ḥafṣ table (D14). 494 cluster forms of these texts have no Ḥafṣ row (most of
them al-Dūrī/al-Sūsī: alef with a vowel and U+06EC or U+06EA; Bazzi's digit ayah numbers), and
no second text of these readings exists to derive rows from. D2 allows rows for such forms only
with the owner's approval and forbids inventing them.
Decision:
- `pipeline/dk_riwayat.py` lists every such form with its evidence and a candidate:
  `number` (Arabic-Indic ayah numbers, G-070: the DigitalKhatt form of the KFGQPC ayah sign of the
  same value, a Ḥafṣ row); `compose` (each codepoint has exactly one DigitalKhatt image in the
  equal-length Ḥafṣ rows; base+mark pairs no Ḥafṣ row has are named); `open` (a codepoint without
  a single image: an identity proposal shown for review only, marked "no evidence"). Forms are
  grouped by kind and by the mark that is new to a base. Output:
  `data/encodings/hafs-v1.riwayat-candidates.json` (tracked) and the sheet
  `build/review/phase5/encoding.html` (each example word in the riwāyah's KFGQPC font and in
  DigitalKhatt; every occurrence checked against rule 6).
- A form converts only when the owner approved every group it belongs to:
  `data/encodings/hafs-v1.riwayat-owner.json` records the decisions (written only after one,
  quoting it; owner-signed commits only since G-130); the pipeline writes the approved rows to `hafs-v1.riwayat-rows.json`, which the core
  loads next to the Ḥafṣ table (`EncodingTable::extend_from_json`; a row that repeats a Ḥafṣ row is
  an error).
- **Owner at the T5.3 checkpoint (2026-10-22): "All except U+06EC"** for the evidence-backed groups
  (15 groups: known pairs, numbers, and the marks U+064C, U+064E, U+064F, U+0650, U+0651, U+0653,
  U+0670, U+06D8, U+06DB, U+06E1, U+06E2, U+06EA, U+06ED on new bases; 458 rows), and **"Decide
  per group later"** for the nine open groups (U+0622, U+0623, U+0624, U+0654, U+0655, U+065C,
  U+066E, U+06DF, U+200D) and for U+06EC on new bases.
- Ayah numbers convert by value (Bazzi, and Shuʿbah's one digit number at 2:286, G-016).
Consequences:
- Words with a form from an undecided group cannot convert: 612 places, pinned in
  `tests/golden/shape/dk-riwayat-rule6-pending.json` and listed as blocked in the G3 report
  (G-102). Rule 6 still fails them; nothing is drawn silently.
- Shaping the whole texts found 16 words where an approved or Ḥafṣ row leaves a mark unattached in
  a context Ḥafṣ never has (alef + U+06EC after lām or hāʾ; the small nūn U+06E8 on a tatweel in
  12:110:11). The sheet the owner saw had checked six examples per form; it now checks every
  occurrence. These words are pinned (any change fails) and their pages are flagged in the
  overflow reports and contact sheets (G-103), for the owner at G3.
- **Walkthrough of the undecided groups (owner, 2026-10-23, group by group on focused sheets).**
  25 forms approved as owner rows in `hafs-v1.riwayat-owner.json`, each with its derivation:
  - U+06DF: the KFGQPC riwāyah fonts draw it as glyph `side`, a spacing dot on the line;
    DigitalKhatt has no such glyph. "U+06EC on bare alef only": alef + vowel + U+06DF becomes
    alef + vowel + U+06EC, which DigitalKhatt draws beside the alef (3 forms; al-Dūrī and al-Sūsī
    138 places each). Hamza-alef and madda forms stay blocked.
  - U+065C (Shuʿbah): the Shuʿbah font's U+065C has the same outline as the Ḥafṣ font's U+06EA, which
    the Ḥafṣ table converts to DigitalKhatt U+065C. "Approve all 11".
  - Hamza (U+0623, U+0624, U+0654, U+0655): each has a single pattern in the Ḥafṣ rows (8,900 / 705 /
    495 / 14 places), missed before because those rows differ in length. "Approve those passing":
    every form whose occurrences all pass rule 6.
  - U+0622 identity (no Ḥafṣ evidence; matches the KFGQPC drawing) and U+06EC on khāʾ.
  - "Keep blocked": 17:7:12 in al-Dūrī/al-Sūsī (U+200D; a tatweel carrier passes rule 6 but loses
    the visible hamza), U+066E (no DigitalKhatt glyph), U+06EC on dāl, hāʾ and wāw, Bazzi 2:71:4.
  Result: 483 rows; 169 places stay without a row (G-102); blocked pages Shuʿbah 2, Bazzi 33, Qunbul
  32, al-Dūrī 22, al-Sūsī 22 (were 55 / 41 / 40 / 159 / 159). No new rule-6 finding appeared.

## D29 — Native layouts: Ḥafṣ breaks inside each riwāyah's KFGQPC ayah lines (T5.2; D1 final form)
Status: accepted by the owner at GATE G3 (2026-10-23: "do whatever you recommend", on the recommendation listed in the G3 record below);
the breaks of Warsh and Qālūn stay unverified until a reference print exists (G-104, G-106)
Date: 2026-10-22
Context: D1 chose native layouts per riwāyah: KFGQPC's page and lines per ayah as hard
constraints, word-level breaks inside multi-line ayahs fitted, every fitted or fallback line
reported. The planning spikes measured what the data supports:
- Width alone is weak. On Ḥafṣ, where the true breaks are known, fitting the 7,037 breaks the
  ayah lines leave open by natural width finds about 54% of them, with the KFGQPC Ḥafṣ font or
  DigitalKhatt and for every cost shape tried (the print stretches letters, so natural widths do
  not show where it broke).
- The riwāyah layouts share the 1441H Ḥafṣ line grid: every ayah of Shuʿbah (6,236), Bazzi and
  Qunbul (6,216 plus four page-crossing ayahs) starts and ends on the same page and line as the
  aligned Ḥafṣ items; al-Dūrī and al-Sūsī differ only at two ayahs; Warsh and Qālūn differ at
  about 243 ayahs on about 250 pages.
Decision:
- `pipeline/ayah_lines.py` copies each riwāyah's KFGQPC page and lines per ayah to
  `data/layout_inputs/<r>.ayah-lines.yaml` (tracked, checked by `data-verify`).
- `mushaf_layout::native::fit_native` (pure, integer): one dynamic programme over all text lines
  of the riwāyah's resolved text. An item may sit only on its ayah's lines; an ayah's first item
  opens its first line and its last item closes its last line. Among the solutions it takes the
  fewest lines not starting at the 1441H Ḥafṣ break of the same page and line, then the smallest
  sum of squared differences between natural and target width (the target is the median natural
  width of lines whose two breaks the data fix), then the earliest start. Widths come from the
  riwāyah's own render font and encoding; an item without an encoding row yet (G-102) counts as
  unmeasured (it affects no fitted line).
- Every text line carries a `fit` tag in the layout file: `forced` (the ayah lines fix the
  break), `hafs` (the transferred Ḥafṣ break, a **hypothesis** for that print until spot-checked),
  `fitted` (width). No page needed a fallback: the programme always finds a solution, and fails
  loudly if it ever does not.
- Layouts `data/layouts/uthmani-<r>-kfgqpc.json` name items of the riwāyah's resolved text,
  inserted ones included (`Layout::riwayah`); `map_lines_on` maps any resolution of that riwāyah
  onto them (an overlay insert joins the line its insert chain starts from). Generated by
  `mushaf-fitlines` in `just data-build` (rule 7: the logic is in `core/`). Reports:
  `docs/review/phase5/layout-<r>.md` (every fitted line with its alternatives and margin).
- Self-test: from KFGQPC's own Ḥafṣ ayah lines the programme rebuilds the 1441H layout exactly
  (`core/mushaf-layout/tests/native.rs`), and every native layout keeps each ayah on its KFGQPC
  lines.
Result:

| Riwāyah | forced | hafs | fitted |
|---|---|---|---|
| shuba | 1783 | 7037 | 0 |
| warsh | 1749 | 6849 | 221 |
| qalun | 1749 | 6850 | 220 |
| bazzi | 1778 | 7042 | 0 |
| qunbul | 1778 | 7042 | 0 |
| duri_abuamr | 1775 | 7043 | 1 |
| susi | 1775 | 7043 | 1 |

Spot check (planning session, relayed 2026-10-23; G-106 has the method and details): Shuʿbah
pages 7, 300 and 501 (45 lines) match the QuranFlash `Shubah` scan at every word break, apart from
the ayah-marker convention already accepted for Ḥafṣ (D6 addendum): evidence for the transferred
breaks in a riwāyah on the Ḥafṣ grid. No scan exists for Bazzi, Qunbul and al-Sūsī, and the
`Douri`, `Qaloon` and `Warsh1` books are other printings. The `Warsh2` scan is another printing
too (different ayah lines and numbering on page 7); on its page 8, where its ayah lines agree with
KFGQPC, 2 of 9 transferred breaks differ: a warning for Warsh and Qālūn, not proof. Warsh and
Qālūn need KFGQPC's own printing to be checked (G-104, G-105).

D1 final form (owner at GATE G3, 2026-10-23): **this rule**. Proposed before the gate as: final = this rule, with the spot check of P7 deciding
whether the transferred breaks stand. Spot check: establish each QuranFlash book's edition on 5
pages (which ayah starts each line, against the KFGQPC data), then compare all fitted lines of
al-Dūrī and al-Sūsī, 30 fitted lines each of Warsh and Qālūn spread over the muṣḥaf, 20 random
`hafs` lines per riwāyah, every page-crossing ayah, and page 1 (G-106).

## D30 — Short lines and header basmalah on native layouts (T5.2, P5)
Status: accepted by the owner at GATE G3 (2026-10-23: "do whatever you recommend", on the recommendation listed in the G3 record below), 60% threshold;
the basmalah was decided by the owner at the T5.3 checkpoint
Date: 2026-10-22
Context: Ḥafṣ takes its short (centred) lines from the 1441H print's own widths (`quran-qcf4`,
G-020): under 90% of a full line. No such data exists for the other prints. P5 proposed: the
Ḥafṣ ratio where a line holds the same items as the Ḥafṣ line, otherwise natural width under 90%.
Evidence: on Ḥafṣ (KFGQPC Ḥafṣ font, the 30 short lines known) natural width under 90% of the
median full line would call 1,486 of 8,790 full lines short; the narrowest full line is at 63%;
at 60% no full line counts as short and 14 of the 30 short lines are found. The 90% rule is about
printed widths, not natural ones.
Decision (`mushaf_layout::native::short_lines`):
- a line that holds the same first and last item as the Ḥafṣ line at that page and line takes the
  Ḥafṣ print's width when that line is short;
- a line at a page and line where the Ḥafṣ print has a short line, but holding other items, is
  short at its own natural share of a full line (added after looking at Warsh page 1, where
  al-Fātiḥah's lines hold other words than Ḥafṣ's and would otherwise be stretched full; 4 lines
  each in Warsh and Qālūn);
- any other line is short when its natural width is under **60%** of the median full line
  (`NATURAL_SHORT_PPM`); its width is that share.
Written to `data/render/uthmani-<r>-kfgqpc.line-widths.json` with each line's basis. Result: 29 to
37 short lines per riwāyah (Ḥafṣ: 30). Lines that are short in a print but between 60% and 90% of
natural width are drawn full; the spot check covers page ends and surah ends (G-107).
- **Header basmalah (owner at the T5.3 checkpoint, 2026-10-22: "Own 27:30 words").** Header
  basmalah lines draw the riwāyah's own words 27:30:5 to 27:30:8 in its resolved text. In Ḥafṣ,
  Shuʿbah, Bazzi and Qunbul these equal 1:1:1 to 1:1:4 codepoint for codepoint (the Ḥafṣ goldens
  are unchanged); Warsh, Qālūn, al-Dūrī and al-Sūsī do not number the basmalah of al-Fātiḥah and
  keep 27:30 in their own spelling (D16 amended).

## D31 — Overflow per riwāyah (T5.2, P6)
Status: accepted with the Phase 5 plan; numbers reviewed at GATE G3 (2026-10-23)
Date: 2026-10-22
Decision: D15's policy for every riwāyah, in the plan's order: one font scale per page (floor
0.90), simple spaces tighten to 60%, a horizontal squeeze, each step reported. The DigitalKhatt
riwāyāt keep Ḥafṣ's constants: their natural full-line widths have the same p90 (17,796 to 18,005
against 17,800). Warsh and Qālūn (KFGQPC fonts, 2048 units per em, spacing only): `line_width`
39,000, about the p90 of their natural full-line widths, as D15 chose for Ḥafṣ; pitch and margins
keep the Ḥafṣ page proportions. `just golden` checks every page of every riwāyah: rule 6 (apart
from the pinned G-101/G-103 words), every line within 0.1% of its target, no ink of adjacent lines
touching, and writes `docs/review/phase5/overflow-<r>.md`.

| Riwāyah | Pages blocked (G-102) | Pages scaled | At floor | Squeezed lines | Widest space median / p95 / max | G-045 (spacing alef) |
|---|---|---|---|---|---|---|
| shuba | 2 | 209 | 32 | 66 | 2.85 / 3.79 / 5.87 | 1380 |
| warsh | 0 | 200 | 37 | 62 | 2.82 / 5.18 / 13.83 | n/a (KFGQPC font) |
| qalun | 0 | 191 | 39 | 67 | 2.83 / 5.18 / 12.38 | n/a (KFGQPC font) |
| bazzi | 33 | 216 | 34 | 84 | 2.73 / 3.79 / 5.69 | 1283 |
| qunbul | 32 | 216 | 34 | 84 | 2.72 / 3.79 / 5.69 | 1287 |
| duri_abuamr | 22 | 202 | 32 | 68 | 2.77 / 3.79 / 5.87 | 1343 |
| susi | 22 | 199 | 31 | 65 | 2.77 / 3.79 / 5.87 | 1343 |

Ḥafṣ for comparison (D15): 209 pages scaled, 32 at the floor, 65 squeezed lines, widest space
2.85 / 3.79 / 5.87, G-045 1,377. Spacing-only justification leaves wider gaps in Warsh and Qālūn,
as D3 measured.

## D32 — Rule-5 edition notes per riwāyah (T5.2)
Status: accepted by the owner at GATE G3 (2026-10-23: "do whatever you recommend", on the recommendation listed in the G3 record below)
Date: 2026-10-22
- **Shuʿbah, Bazzi, Qunbul, al-Dūrī, al-Sūsī:** the same documented exception as Ḥafṣ (D2):
  DigitalKhatt Madina, a 1421H design, draws the riwāyah's KFGQPC text (official v2.0; Bazzi v7
  from the 2022 mirror, D13) on the riwāyah's own KFGQPC layout. The word-level difference from
  the font's own text cannot be produced, since DigitalKhatt publishes no text for these readings;
  every form that needed a row beyond Ḥafṣ is listed with its evidence and the owner's decision
  (D28, G-102).
- **Warsh, Qālūn:** text, layout data and font all come from one KFGQPC release (official v2.1);
  no exception. Their line breaks inside ayahs are still transferred or fitted (D29).

## D33 — Display style as a render key (T5.2, D2/D1 in Phase 5; D36)
Status: accepted (owner requirement relayed by the planning session, 2026-10-23; confirmed at GATE G3); Phase 5 part of
D36
Date: 2026-10-23
Context: the owner wants every riwāyah displayable later in either script style, Ḥijāzī
(Mashriqi) or Maghribi. The feature itself is Phase 6 (T6.7–T6.10, D36). Phase 5 must not
hard-wire "riwāyah → font".
Decision:
- A render choice is keyed by **(riwāyah, style)**, `style` ∈ {`hijazi`, `maghribi`}. Each
  render profile under `data/render/` carries its `style`; font, justification mode
  (`justify.letters`: calligraphic or spacing only) and encoding table (or none, the identity
  table) are read from that profile, so all three are chosen by (riwāyah, style).
- Default style per riwāyah: its own print's, `maghribi` for Warsh and Qālūn (KFGQPC fonts,
  spacing only, D27), `hijazi` for Ḥafṣ, Shuʿbah, Bazzi, Qunbul, al-Dūrī and al-Sūsī (DigitalKhatt
  Madina, calligraphic, D2/D28). `mushaf_render::default_style`, `profile_path(riwāyah, style)`,
  `Renderer::load_style`, and the CLI's `--style`.
- Phase 5 implements and renders only the default style. Any other combination returns the error
  "style … is not available yet (D36 …)"; there is no silent fallback
  (`core/mushaf-render-cli/tests/cli.rs`).
- Line layouts stay each riwāyah's own (D29) whatever the style.
Consequences: Phase 6 adds a profile per new (riwāyah, style) and the conversion rows it needs
(T6.7–T6.9); nothing in the core assumes one font per riwāyah.

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
Phase 5 (2026-10-23, after merging this decision into the Phase 5 branch): render profiles carry
a `style` (`hijazi`, `maghribi`) and are looked up by (riwāyah, style)
(`mushaf_render::profile_path`, `default_style`, `Renderer::load_style`, `just render`'s CLI
`--style`). Each riwāyah's default style is the one rendered (Warsh and Qālūn `maghribi` with their
KFGQPC fonts and spacing-only justification; the others `hijazi` with DigitalKhatt Madina);
any other combination is refused with a reason naming this decision.

## GATE G3 — Phase 5 review: passed (2026-10-23)
The owner asked which decisions were needed, was given a list with a recommendation for each, and
answered "do whatever you recommend" (2026-10-23, in the Phase 5 session). Recorded as:
- **Accepted:** D29 (the final form of D1), D30 (short lines, 60% threshold), D31, D32; D33
  confirmed. The transferred Ḥafṣ breaks are supported by the Shuʿbah spot check and stay a
  hypothesis for the other riwāyāt, Warsh and Qālūn in particular (G-104, G-106).
- **G-103** (16 words with an unattached mark in DigitalKhatt): keep them drawn and flagged until a
  font fix or an escape exists; no rows withdrawn.
- **G-101** (missing damma, Warsh/Qālūn font): an open defect, to be fixed before public release
  (Phase 10), like G-045.
- **Not decided at the gate, by their nature:** the undecided encoding groups (G-102) needed the
  owner's per-group review on the sheet (D2 forbids approving a row without it); the owner then went
  through them the same day (D28, walkthrough), which leaves 169 places blocked; the Warsh/Qālūn reference print (G-106) and page 1 line 2 (G-023) wait for a print
  of KFGQPC's own edition.
- PR #3 is merged by the owner.

## D37: Highlighting differences from Ḥafṣ
Status: accepted (scope and phase)
Date: 2026-10-26
Decision (owner): every non-Ḥafṣ muṣḥaf can be shown plain or with its differences from Ḥafṣ
highlighted. A difference is a farsh difference or an uṣūl application (e.g. taghlīẓ al-lām,
ṣilah, ibdāl), never an encoding or spelling-convention difference. Highlights come from
Phase 6 rules and curated farsh (T6.11–T6.12), including pronunciation-only rules through
the annotation channel; each carries a label, the Ḥafṣ form and a citation. Text and layout
are identical in both modes. The toggle is in the Phase 8 UI.


## D38: Rule DSL (T6.1)
Status: accepted with the T6.1 plan (2026-10-26)
Context: Phase 6 replaces raw diffs with rules, so that choosing an option changes the text
through rules instead of a lookup table. Rules must be data in git (rule 3), must be
evaluated in the core (rule 7), and must be deterministic (rule 8). Each rule needs a source
and a status (rule 4), and each one must be checkable against a register.
Options:
- (a) Expand rules into patch files in the Python pipeline at build time. Simple, but the
  logic would live outside `core/`, and a free-mode edit could not re-run the rules.
- (b) A separate rule engine in the resolver with its own way of applying changes. This
  duplicates the patch semantics: layers, conflicts and orphans.
- (c) Rules as patterns whose actions are patch templates, expanded in the core into ordinary
  `Patch` values that the existing resolver applies.
Decision: (c).
- `mushaf-model::rules` holds the data types and their checks: `RuleFile`, `Rule`, `Step`,
  `ClusterMatch`, `Boundary`, `Scope`, `Exception`, `Action` and `Register`.
- `mushaf-rules::expand` matches on the spine and returns a `PatchFile`. Each patch gets the
  id `<rule>@<target>.<op>`, the source `{kind: rule, ref: <rule>}`, `rule_id`,
  `status: rule-derived`, and the rule's `when`. An action's payload is checked by the same
  parser as patch files.
- A profile lists its rule files in `rule_stack`. `resolve_profile` expands them before the
  patch stack. An option group's `affects` may name a rule id (§3.3).
- In data mode, a rule that matches nowhere and an exception that is never met are errors.
  In free mode they are lint notes.
- The register (`data/rules/register.yaml`) is checked in Rust (`DataSet` validation) and in
  Python (`pipeline/rules_register.py`, run by `just data-verify`). Its rules are:
  - every rule has exactly one entry, and every entry a rule;
  - owner and option agree with the rule file;
  - an uncited rule is a `hypothesis`;
  - `cross-checked` and `muqri-verified` are never allowed (rule 4).
- `mushaf.db` schema v4 adds the `rule_files` and `rule_register` tables. The parity tests
  cover them.
- Rules match the spine's own text, not the output of other rules or of farsh (G-141).
  Matches are leftmost and non-overlapping, in spine order. Codepoints are compared exactly.
- T6.2 addition: a rule and its register entry carry `single_source` (it must agree in both). The rule's patches inherit it (§3.5).
Consequences: T6.2 and T6.4 add real, cited rules to `data/rules/`. T6.4 compares each
rule's expansion with the raw patches it explains. If a rule ever needs to see another
rule's output, G-141 is revisited with a new ADR.

## D39: Owner corrections to the Ḥafṣ spine text (G-142)
Status: accepted (owner decision, 2026-10-26)
Context: at 52:37:7:4 (المصيطرون) the spine, which is KFGQPC Ḥafṣ v2.0, stores
`U+0635 U+06DC U+064E`, a small sīn *above* the ṣād. The KFGQPC Ḥafṣ muṣḥaf's explanation of
signs, p. و, says that in this word the sīn is placed *under* the ṣād, meaning that ṣād is
the more common reading. The owner decided: "sīn should be below the ṣāḍ on 52:37."
Rule 2 forbids editing the pinned source, and rule 3 makes the spine text in git
the source of truth.
Evidence for the codepoint, all from pinned data:
- U+06E3 ARABIC SMALL LOW SEEN appears nowhere in the eight KFGQPC texts.
- The pinned DigitalKhatt Madina and Old Madina texts (`quran_text_madina.ts`,
  `quran_text_old_madinah.ts`, line 8901) write this word with `U+0635 U+06E3 U+064E`. It is
  the only U+06E3 in each text.
- The data-derived Ḥafṣ encoding table (D14) already maps this exact cluster to that
  DigitalKhatt form, so the renderer drew the sīn below even before this correction
  (`docs/font-coverage.md`, S2 counts).
- DigitalKhatt Madina has a glyph for U+06E3 and anchors it on this ṣād. The rule-6 check
  passes in `just golden`.
Options:
- (a) Edit the spine. Rejected: rule 2 and rule 3.
- (b) Leave the text and rely on the renderer. Rejected: the stored text would still say
  "sīn above".
- (c) An owner correction patch on the Ḥafṣ profile.
Decision: (c).
- `data/patches/corrections/hafs.yaml` holds patch `hafs.g142.52.37.7.4.rep`. It replaces
  the cluster with `U+0635 U+06E3 U+064E`, with source = p. و plus the owner decision, and
  `single_source`.
- The Ḥafṣ profile's patch stack lists this file (`pipeline/profiles.py`, generated).
- The decision is recorded in `data/approvals/hafs.yaml` (`cross-checked`, owner's words
  quoted), written on the owner's explicit instruction.
- `pipeline/corrections.py` derives the encoding row for the corrected form: the DigitalKhatt
  image of the spine cluster it replaces, accepted only when that image equals the corrected
  form. The row is written to `data/encodings/hafs-v1.correction-rows.json` and loaded as
  the Ḥafṣ render profile's `encoding_extra`.
Effect on the other riwāyāt: none. All seven remove U+06DC at 52:37:7:4 in their own raw
patches, because their KFGQPC texts have a plain ṣād there. The correction applies only to
the Ḥafṣ profile, so their resolved text and `just roundtrip` are unchanged.
Consequences: the spine and KFGQPC still differ here; the difference is listed (G-142,
resolved). `data/approvals/` is still not joined into `mushaf.db` (G-033).

## D40: Annotation channel (T6.3)
Status: accepted (T6.3, 2026-10-26)
Context: some features change pronunciation but no glyph: madd lengths, taqlīl, tarqīq, and
(by D41) the idghām wajh of 69:28. Writing them into clusters would change the text and the
rendering. Leaving them out would lose them, and they are needed for highlighting (T6.11)
and for review.
Decision:
- A cluster's `attrs.annotations` (§3.1) is the channel. It is written by `set_attr` patches
  and by rule actions (D38), and the resolver already carries it to `Resolved`.
- Keys are `pron.<feature>`. The feature comes from a closed list of technical terms in
  `mushaf_model::PRONUNCIATION_FEATURES` (ibdāl, idghām, ikhfāʾ, imālah, iqlāb, ishmām, iẓhār,
  madd, naql, rawm, sakt, ṣilah, taghlīẓ, taḥqīq, taqlīl, tarqīq, tashīl).
- Values are short ASCII tokens (`[a-z0-9_-]+`).
- Rule validation rejects any other key or value. A new feature needs an ADR amendment.
- Annotations never change codepoints, shaping, line fit or the draw list. A test checks
  this: the 69:28 option pair must draw byte-identically.
Consequences: T6.4 writes pronunciation-only rules to this channel. T6.12 exposes
annotations in the draw output as highlight flags, without changing glyphs.

## D41: The idghām wajh of مَالِيَهْ changes no glyph (G-143)
Status: accepted (owner decision, 2026-10-26)
Context: the KFGQPC Ḥafṣ muṣḥaf's explanation of signs (p. ح) describes the idghām wajh at 69:28
in writing: the sukūn is removed from the first hāʾ and a shadda is placed on the hāʾ of
هَلَكَ. T6.2 implemented that, and also removed the sakt sign. The owner said: "No, keep the
sukun and sīn and dont have the shaddah."
Decision: the text is the same under both choices of `hafs.maliyah`. It keeps
`U+0647 U+06E1 U+06DC` and `U+0647 U+064E`. The rule `hafs.maliyah.idgham` writes only
`pron.idgham` annotations: `into_next` on 69:28:4:5 and `from_previous` on 69:29:1:1 (D40).
The golden test requires the two draw lists to be byte-identical. This departs from the
written form on p. ح; it is an owner decision, and the register entry says so.
Consequences: G-143 is closed. A reader who wants the idghām shown in writing has no
option for it; that would need a new owner decision.

## D42: Rule sets per riwāyah from the data (T6.4)
Status: accepted (T6.4, 2026-10-26). Every rule is a hypothesis until the owner or a muqriʾ
cites or rejects it.
Context: T6.4 asks for rules that explain the raw patches, qirāʾah by qirāʾah, with the
remainder kept as curated farsh, while `just roundtrip` stays at 100%. Rule 1 forbids
writing uṣūl from memory. The only sources in the repository are the KFGQPC texts (already
encoded as the raw patches) and the pinned book, whose text cannot be extracted reliably
(T6.0 deferred).
Options:
- (a) Write named uṣūl rules from knowledge. Rejected: rule 1.
- (b) Wait for cited sources for each rule. This would block T6.4 entirely.
- (c) Induce rules from the raw patches, accept only those the data supports exactly, and
  mark every one a hypothesis.
Decision: (c), in `pipeline/rule_mining.py`.
- A candidate is one cluster change, either a codepoint change (`replace`/`set_marks`) or a
  single-cluster insert with seq 1, plus at most one context feature:
  - the cluster's place in its word (first, last or only);
  - the letter before it or after it in the word;
  - the first letter of the next word, for a word-final cluster (boundary `any`).
- It becomes a rule when at least 3 places carry exactly that change and the other places its
  pattern matches are at most max(2, 5%) of the matches. Those other places are listed as
  exceptions, each with its reason from the data.
- The matcher is a port of `mushaf_rules::matches` (leftmost, non-overlapping), so each rule's
  exceptions are exactly what the core sees.
- Rules are written to `data/rules/<r>.yaml` and their register to
  `data/rules/<r>.register.yaml`. Every entry is `hypothesis`, `single_source`, with no source.
- The raw patches no rule explains are kept unchanged, with their ids, in
  `data/patches/curated/<r>.yaml`.
- Each riwāyah's profile now resolves `rule_stack: [rules/<r>.yaml]` and
  `patch_stack: [patches/curated/<r>.yaml]`. The raw files stay as evidence.
- `just roundtrip` resolves through the profiles. It, and a Rust test comparing with the raw
  file, require the same text codepoint for codepoint.
- Order: the plan's order of qirāʾāt (ʿĀṣim → Nāfiʿ → Ibn Kathīr → Abū ʿAmr) is kept for
  review. The mining itself is independent per riwāyah.
- Several register files make up one register: `rules/register.yaml` for hand-written rules
  and `rules/*.register.yaml` for generated ones.
Consequences:
- These rules are patterns in the KFGQPC data, not named uṣūl. Many are encoding or spelling
  conventions; some are coincidences of a few places. The review sheets (T6.5) show each one
  with its places and exceptions.
- At G4 the owner or a muqriʾ cites, renames, merges or rejects them. A rejected rule's
  places return to the residual by rerunning the miner with that rule excluded (to be built
  when the first rejection comes).
- Pronunciation-only uṣūl (madd, taqlīl …) are not in the KFGQPC codepoints, so no rule
  writes annotations yet (D40). They need cited sources.

## D43: The ṭarīq layer (T6.6)
Status: accepted (T6.6, 2026-10-26)
Context: §3.2 has a ṭarīq layer, and `Selection` has a `tariq`, but nothing chose between
ṭuruq. Every ṭarīq-level patch applied, whatever ṭarīq was selected. The plan asks to model
ṭuruq as an option group where the sources distinguish them.
Decision:
- A riwāyah's ṭuruq form the option group `<riwāyah>.tariq`. When it is loaded:
  - the selection's `tariq`, or a choice for the group, selects the ṭarīq; they must agree
    (`TariqMismatch`) and must be one of the group's choices (`UnknownChoice`);
  - without either, the group's default for the riwāyah applies;
  - the resolved ṭarīq is also the `tariq` used for other groups' per-ṭarīq defaults;
  - a patch (or a rule's patches) at owner level `tariq` applies only when its `owner.id` is
    the resolved ṭarīq;
  - patches that depend on a ṭarīq through `when {option: <r>.tariq, choice: …}` work as any
    other option.
- Without such a group, behaviour is unchanged.
- Ḥafṣ: the group `hafs.tariq` has the single choice `shatibiyyah`, cited to KFGQPC Ḥafṣ muṣḥaf,
  explanation of signs, p. ح. No second ṭarīq has a source in the repository, so none is
  modelled (G-145). The synthetic fixture tests two ṭuruq (`r3.tariq`).
Consequences: adding a ṭarīq means one more choice plus its cited rules or patches. The
"no sakt" wajh (G-143) waits for a cited second ṭarīq.

## D44: Display styles, conversion and availability (T6.7–T6.10)
Status: accepted (T6.7–T6.10, 2026-10-26)
Context: D36 wants every riwāyah shown in Ḥijāzī or Maghribi style. The KFGQPC texts encode
style-specific mark conventions in their codepoints (G-006). A conversion is allowed only
from owner-confirmed rows (the D21 pattern) and is applied only at render time. A combination
the fonts cannot render correctly is reported as unavailable (rule 6), never drawn wrongly.
Decision:
- T6.7, the conversion layer:
  - `pipeline/style_conversion.py` writes `data/encodings/style-conversion.json`. Each row maps
    one codepoint of one style to one codepoint of the other, and comes from an
    owner-confirmed `encoding` row in `data/approvals/difference_kinds.yaml` that holds for
    every riwāyah of the target style.
  - Today that is exactly one row, Ḥijāzī → Maghribi `U+06E1 → U+0652` (Warsh and Qālūn,
    D21).
  - Maghribi → Ḥijāzī has no row: Maghribi U+0652 stands for both Ḥijāzī U+06E1 and U+0652,
    so the data does not determine the inverse.
  - The core applies the rows as a codepoint pre-map in `EncodingTable` before the font's
    table (`with_style_conversion`). A render profile names them with `style_conversion` and
    `text_style`. The spine and patches are unchanged.
- T6.8, conventions: `docs/review/phase6/style-conventions.md` (generated) lists every
  codepoint whose use differs between the two families of KFGQPC texts by at least ten times,
  with its counts and conversion status. 22 differences have no confirmed row (e.g. U+0671
  alef waṣla, U+06DA, U+06ED) and stay unconverted.
- T6.9, coverage: an audit shapes every resolved word of each riwāyah in each non-default
  style with that style's font. Maghribi uses the KFGQPC Warsh font with the conversion rows.
  Ḥijāzī uses DigitalKhatt Madina through the Ḥafṣ table and the approved riwāyah rows.
  The audit counts words that cannot convert and words that fail rule 6. The result is
  pinned in `data/render/style-availability.json`, and `just golden` recomputes it.
  A combination is available only when both counts are zero.
- T6.10, the style switch: `Renderer::load_style` gives a non-default style only when the
  audit marks it available and a render profile for it exists. Otherwise the error states the
  audit's counts. Golden pages exist for every available combination.
Consequences: with one confirmed row, no non-default combination is available today (see
the audit). Each further owner-confirmed row, or a font, reduces the counts. The owner
reviews the convention report at G4.

## D45: Difference highlighting (T6.11–T6.12)
Status: accepted (T6.11–T6.12, 2026-10-26). Whether hypotheses show by default is the owner's
choice at G4.
Context: D37. A highlight must come from a rule application or a curated farsh entry,
never from a raw patch. It must never mark an encoding or spelling difference, and it must
carry a label, the Ḥafṣ form and a citation. Plain and highlighted modes draw identically.
Decision:
- Source: the resolver already records, for every cluster, the ids of the patches that
  changed it (`provenance`). A rule's patches are `<rule>@…`, so the rule and its register
  entry are known.
  - A cluster highlights if a rule changed it, and that rule's register `difference_kind` is
    neither `encoding` nor `spelling`, and the rule is cited (`rule-derived` with a source) or
    the policy shows hypotheses.
  - A curated farsh patch (`source.kind: farsh`) also highlights; none exists yet.
  - Raw and residual patches never highlight. Nothing highlights on Ḥafṣ.
- `difference_kind` for data-derived rules: `pipeline/rule_mining.py` gives each rule the D21
  kind of its change when every codepoint component is an owner-confirmed row of one kind,
  else `unclassified`. Today that is 60 Warsh and 38 Qālūn rules classed `encoding`, the
  confirmed sukūn row.
- Output: `DrawPage::highlights` holds, per highlighted cluster:
  - its line;
  - the label (rule or farsh id);
  - the Ḥafṣ form (`U+` codepoints; empty for an inserted cluster);
  - the citation and a `cited` flag (false means shown in a distinct style);
  - `annotation`, true for pronunciation-only differences (D40).
  `Renderer::render_highlighted(page, choices, HighlightPolicy)` fills it. The list is
  serialized only when non-empty, so plain pages and their goldens are unchanged.
- Policy: by the owner's decision (2026-10-26, "show unverified ones as well"),
  `HighlightPolicy::default()` also shows hypotheses, flagged `cited: false` for a distinct
  style. `HighlightPolicy::CITED_ONLY` shows cited rules only.
- Tests (`highlight_pairs_per_riwayah`, in `just golden`), per non-Ḥafṣ riwāyah on its first
  page with a highlight:
  - the highlighted page minus its highlights equals the plain page byte for byte;
  - the default policy equals plain;
  - no highlight comes from an encoding or spelling rule (for Warsh, encoding rules are
    applied on the page and none highlights);
  - the highlights are pinned in `tests/golden/pages/<r>-p<NNN>-highlights.json`.
Consequences: highlighting becomes useful by default once the owner cites rules (G4).
Pronunciation-only uṣūl highlight through annotations once cited rules write them (D40).
The UI toggle is Phase 8.

## D46: Warsh and Qālūn in the Ḥijāzī style
Status: accepted (owner, 2026-10-26)
Context: at G4 the owner asked for a Ḥijāzī option for the Maghribi riwāyāt; Ḥijāzī → Maghribi
is not wanted. The single Ḥijāzī → Maghribi row (D44) cannot be inverted: Maghribi U+0652
stands for both Ḥijāzī U+06E1 and U+0652. The owner reviewed an example sheet
(`build/review/phase6/sukun-examples.html`) and approved it: "the sheet looks good". The
owner also decided: "warsh and qalun should show their own pause signs". Both decisions are
recorded in `data/approvals/style-conventions.yaml`.
Evidence: each Warsh and Qālūn cluster aligned to the Ḥafṣ spine shows which sign Ḥafṣ uses:
- Warsh: 32,768 sukūn and 3,982 silent-letter signs;
- Qālūn: 37,028 sukūn and 3,981 silent-letter signs;
- Ḥafṣ U+0652 occurs only on alif (3,722), wāw (255) and yāʾ (11).
A rule based only on the vowel before the letter would have got 95 Warsh and 108 Qālūn
places wrong (e.g. yāʾ after kasra, 2:33).
Decision: `mushaf_resolve::maghribi_to_hijazi` rewrites a resolved Maghribi text at render
time. For each cluster, the aligned Ḥafṣ cluster decides:
- a difference only in writing takes Ḥafṣ's form: sukūn, alif waṣla, final ے, the hamza form
  ا+U+06DF;
- a reading difference keeps the riwāyah's form, with only its sukūn converted (from Ḥafṣ;
  where Ḥafṣ has neither sign, alif keeps U+0652 and any other letter takes U+06E1);
- a word with the same letters and marks as Ḥafṣ, with a mark stored on a neighbouring
  letter, takes Ḥafṣ's form;
- pause signs stay the riwāyah's own.
Cluster ids never change. The render profiles `data/render/{warsh,qalun}-hijazi.json` use
the riwāyah's own layout and DigitalKhatt Madina through the Ḥafṣ table plus the approved
riwāyah rows (D28). `text_style: maghribi` turns the rewrite on.
Consequences:
- Words that cannot convert block their pages: 1,756 Warsh and 1,290 Qālūn words, which
  leaves 53 and 94 drawable pages. These are forms with Maghribi reading marks that have no
  approved DigitalKhatt row (U+06EA under a letter, final ے where the reading differs,
  U+0657, some hamza forms). They are pinned in
  `tests/golden/shape/maghribi-hijazi-pending.json` and need rows approved as in D28 (G-148).
- Every drawn page passes rule 6.
- D44's availability rule changes: a combination is offered when its render profile exists,
  and its blocked pages are listed.

## D47: Approved Ḥijāzī drawings of Maghribi forms, with mark adjustments
Status: accepted (owner, 2026-10-26)
Context: after D46 the owner reviewed DigitalKhatt drawings of Warsh and Qālūn words against
print photos (IMG_2514–2517 and a 12:90:2 crop) and picked one per word
(`data/approvals/style-conventions.yaml`, `maghribi-to-hijazi.word-forms`). Three words
(5:18:16, 12:90:2, 2:225:2) could not be drawn right by any encoding: the font places the marks
wrongly. The owner approved placing those marks by hand.
Options: (a) per-word owner rows (D18), one per flagged word; (b) rows per form, applied wherever
the approved form occurs; (c) font changes (the font is a pinned third-party source: rejected).
Choice: (b). `data/encodings/hafs-v1.hijazi-forms.json` holds the rows, generated from the picks
(cluster or sequence rows; a row may replace a row of the table). Each row may carry adjustments in
font units: `gap` (extra advance after a character, counted in the word's width so justification
stays exact) and `shift` (moves the nominal glyph of one mark in its cluster; exactly one glyph
must match, else an error). Shifts are applied in `mushaf_layout::fit` right after shaping; all
integers, so output stays deterministic. The rows load only in the Ḥijāzī profiles of Warsh and
Qālūn (`hijazi_forms`).
Evidence: the picks; renders of the approved drawings; real pages of 12:90:2, 2:49:4, 20:58:14,
20:76:12 and 2:48:8 drawn through the core.
Consequence: Warsh draws 212 of 604 pages in the Ḥijāzī style (was 53), Qālūn 136 (was 94).
Applying each pick to every place of its form (2,025 places) is my reading of the picks: the owner
saw one word per form (G-149). Qālūn 12:90:2 with the standing alif was not seen by the owner as
drawn.

D45 addendum (owner, 2026-10-26, photo IMG_2513): highlighted clusters are drawn letter by letter
in red (#cc0000) in the SVG and PNG; `cited` stays in the draw list but no longer changes the
colour. `mushaf-render --highlight` renders a highlighted page.
D45 addendum 2 (owner, 2026-10-26: "Highlighting is very inaccurate", "You are highlighting
everywhere there is the ص waqf sign"): highlights no longer come from rule provenance. A word
cluster is highlighted when it differs from its aligned Ḥafṣ cluster after ignoring pause signs
(U+06D6–U+06DB), sukūn (present or not), the Maghribi bare lām for Ḥafṣ lām + shadda + fatḥa (1,674 places), the shapes of final yāʾ, hamza on alif written as plain alif with the same vowel, the shadda of idghām with ghunna, and the disjoined letters (owner: "Highlights should only be for Usul and furush differences"), and, for Warsh and Qālūn, the D46 writing conventions.
Āyah markers are never highlighted. The rule or patch that made the cluster is kept as the label;
differences with no rule (residual patches) are highlighted too, which closes that part of G-147.
## D50: Locating the book's words, and the convention projection (TB.4–TB.5)
Status: proposed (TB.4–TB.6, 2026-10-27). For the owner at the pilot review.
Context: al-Budur quotes Qurʾānic words almost without ḥarakāt and in imlāʾī spelling, and
gives no ayah numbers. Our eight texts are KFGQPC encodings whose conventions differ by family
(Ḥijāzī: hafs shuba bazzi qunbul duri_abuamr susi; Maghribī: warsh qalun; G-006, G-018,
`docs/review/phase6/style-conventions.md`). Only one convention row is owner-confirmed (D21).
Options:
1. Compare the book's text with ours letter for letter. Rejected: the book's quotes are not
   Qurʾānic text (rule 1) and carry no ḥarakāt.
2. Locate by letters only, and compare *our* eight forms with each other.
3. Locate by ayah numbers. Not possible: the book gives none.
Decision: option 2.
- **Letter key** (one function per side, `budur/match.py`): every alif form is dropped,
  including a spine wāw that carries the dagger alif. Hamza in any seat becomes one letter,
  yāʾ and alif maqṣūrah merge, tāʾ marbūṭah becomes hāʾ, and tatweel and the small letters
  are dropped. There are three match tiers: `exact`, `prefix` (a wāw/fāʾ on one side) and
  `no-hamza`. Below `exact` the match is marked for review, and rules never apply at
  `no-hamza`.
- **Placement**: farsh entries are aligned in book order by a longest-chain DP within their
  sūrah. An entry with one candidate is placed whatever the order, and is marked when the
  order conflicts. An entry with several candidates is placed only if every longest chain
  uses the same candidate. List entries (al-mumāl, al-mudgham) must match exactly their stated
  count within the rubʿ.
- **Projection**: `strict` = our text with the owner-confirmed D21 rows mapped back;
  `loose` = strict plus `data/books/al-budur-al-zahirah/conventions.yaml`. Every row there is
  a hypothesis with its evidence (family-only counts, or pilot counts). Written readings are
  compared in `loose`. A difference seen only in `strict` is never a reading and is reported.
  Claude never moves a row into `data/approvals/`.
Consequences: a wrong convention row could hide a real difference, so every row and every
explained pair is listed for the owner. The pilot measures the method's noise through the
Ḥafṣ false-flag rate (criteria file).

## D51: Parsing the book's statements (TB.2–TB.3)
Status: proposed (2026-10-27).
Context: the book states uṣūl at their first occurrence (page_1_0001–0002, pp. 5–6) inside
the farsh. It names readers by their names and its own group terms (مصطلح الكتاب, p. 11),
uses "الباقون" for the rest, and "بخلف عنه" for a reader with two wajhs.
Decision:
- **Readers**: `readers.yaml` holds names only. Each name cites a page and is checked against
  that page before every run. Contextual terms (unqualified الدوري) follow the book's rules on
  p. 11. Unknown or ambiguous terms (المدني) are reported as unresolved, never guessed.
- **Statements**: a block is a headword run plus its prose. A clause starts at a verb, a
  complement or a full stop. Feature classes (`textual`, `phonetic`), scope phrases,
  waqf/waṣl, exceptions, examples, awjuh and restrictions come from `features.yaml`. They are
  our own short terms and our interpretation (hypothesis).
- **Complement**: "الباقون" is read two ways: the complement of the previous clause, and the
  complement of every reader named so far. A verdict stands only when both readings agree.
  Otherwise the place is undecidable.
- **Implicit coverage**: once a statement names any of the 8 at a place, the others are
  covered implicitly. The book states each word's khilāf among the ten where it discusses the
  word (p. 5).
- Corrections (page_2, al-Sulamī, not the author) are indexed by printed page and shown next
  to their pages. They never change a status.

## D52: Statuses and what is applied mechanically (TB.5)
Status: proposed (2026-10-27). The per-word merge of statements (`run.merge_overlaps`), made
after the first pilot result, was accepted by the owner on 2026-10-27 ("Accept").
Decision:
- Statuses are `mentioned`, `rule-covered` and `not-mentioned` (never "confirmed"), each
  with a citation.
- A rule is applied mechanically only when its scope names the word: `حيث وقع` and the like
  (the whole Qurʾān), or `في هذه السورة` (the sūrah). Class scopes (`وهذا مذهبه في كل …`,
  `ما ماثله`, `وقس`) and the general chapters are listed as `scope: unclear` for the owner.
  They are not applied.
- Disagreements are attributed by minimum vertex cover. A riwāyah is `flagged` when it is in
  every minimum cover, and `possible` when it is in some.
- A data difference the book does not draw is explained in two cases, and both are listed:
  - when one riwāyah of the pair is named in a phonetic clause there;
  - when it is a Maghribī-family pattern that the book does not separate.

## D53: The free-edit API, and two patches per key in the user layer (T7.1, T7.2)
Status: accepted (owner, 2026-10-27: "Approve D53, D54 and D55")
Context: T7.1 needs editing operations (add, remove or replace a cluster; add or remove a mark;
set a word's or a glyph's width) that write the user overlay (D12), with undo and redo that
restore exact output (rule 8). D9 allows one patch per key per layer, so a cluster could not be
both replaced and widened by the user.
Options:
1. Keep D9 everywhere. A replaced and widened cluster would have to become delete + insert,
   which gives the user's cluster a new id, so later edits address something else.
2. Change `replace` to carry attributes. This changes the patch file format for every layer.
3. In the user layer only, a key holds one content patch and one `set_attr` patch.
Decision: option 3. Data layers keep one patch per key (tested), so Phase 2 to 6 data and
their conflict checks are unchanged.
- `Patch::slot()` is `(key, is user set_attr)`. `check()` and `EditOverlay` use it. Within a
  layer, the engine applies attribute patches after content patches. In data layers this
  changes no result, because one key never has two patches there.
- `EditSession` (`core/mushaf-resolve/src/edit.rs`):
  - Every operation takes the current `Resolved`.
  - A mark edit on a spine cluster becomes one `replace` holding the cluster's exact new
    codepoints. It is never a `set_marks`, whose remove-then-append order cannot express
    "remove a mark and put it back in place".
  - Edits to clusters the session inserted rewrite that insert's payload.
  - Deleting something the session inserted removes its insert, and the user's edits on and
    inside it. Deleting an item drops the user's edits inside it, which would otherwise be an
    overlap error.
  - `set_width` and `set_stretch` with `None` drop the user's value, so the lower layers'
    value applies again.
- **Rule 11.** No edit is refused for being non-canonical. The only errors are a target that is
  not in the current text (unknown or deleted), a mark the cluster does not carry, and an
  empty cluster.
- **Undo and redo** keep whole overlay snapshots. An edit that changes nothing adds no history,
  and a new edit clears the redo stack. Patch ids are `u<n>`, never reused in a session.
- **Consequence.** A user `replace` records absolute codepoints. Applied over another riwāyah,
  it gives the codepoints the user saw, not "this mark added". This is deliberate: an export
  means exactly what was on screen.

## D54: The free-edit lint, and the "edited" marker stays conservative (T7.3)
Status: accepted (owner, 2026-10-27: "Approve D53, D54 and D55")
Context: T7.3 asks for a lint that flags non-canonical states per riwāyah and option set,
advisory only, and an "edited" marker on every non-canonical page in every render and export.
D16 already draws the marker whenever a user patch or a resolver report entry is present.
Options for the marker:
1. Keep D16: any user patch, or any report entry, marks every page rendered from that stack.
2. Mark only pages whose items differ from the canonical text.
Decision: option 1. Option 2 would let a page that was edited back to its canonical form show
no marker, and it would make the marker depend on the lint, so a lint bug could hide an edit.
Over-marking is safe; under-marking breaks rule 11. The lint therefore never removes the
marker; it only explains it.
- `lint_edits(base, edited, alternatives)` and `lint_overlay` are in
  `core/mushaf-resolve/src/lint.rs`; the renderer exposes them as `Renderer::lint`.
  - The base is the stack without the overlay, for the same riwāyah and choices.
  - Items are compared by id, on content (kind, separators, attributes, and each cluster's id,
    kind, codepoints and attributes), not on provenance.
  - Each differing item is reported as `changed`, `inserted` or `deleted`.
  - Each report lists the alternatives (`group=choice`) whose canonical text has the item
    exactly as edited, or lacks it for a deletion. Each alternative changes one option group
    the base depends on, one at a time.
  - The lint never fails and never blocks an edit.
- The golden test `edited_page_shows_the_marker_in_every_export`
  (`tests/golden/pages/hafs-p003-lint.*`) checks the marker in:
  - the draw list (the last two items);
  - the SVG;
  - the JSON export;
  - the PNG (red pixels: none on the plain page).

  It also checks:
  - a no-op replace is not linted, yet still marks the page;
  - undo gives byte-identical draw output.
- Not covered (G-211): combinations of several groups, and matches against other riwāyāt.

## D55: Re-justifying only the lines an edit touches (T7.4)
Status: accepted (owner, 2026-10-27: "Approve D53, D54 and D55")
Context: T7.4 asks that an edit re-justify only the affected lines, with the cache keyed by
"(word, state hash)", and that undo restore the exact earlier output.
Decision:
- **`FitCache`** (`core/mushaf-layout/src/fit.rs`) memoises the three costly steps of
  `fit_page`:
  - a line's tight width;
  - a line's justification;
  - a word's shaping.

  `fit_page_cached` uses it; `Renderer::fit_cached` keeps it across edits. `fit_page` is
  `fit_page_cached` with a fresh cache, so there is one code path.
- **Keys are the exact inputs, not hashes.**
  - A line's key is its `LineInput` (the words' characters, locks, manual features and extra
    widths) with its target and parameters.
  - A word's key is its characters and features.
  - Keys are compared exactly in a `BTreeMap`, so a hash collision cannot return another
    line's result, and lookups are deterministic. This is the plan's "(word, state hash)"
    with the hash replaced by the state itself. A word's state decides its line's key, so a
    line whose words did not change is not justified again.
- **What is invalidated.** Nothing is ever invalidated: an entry is a pure function of its key.
  An edited line has a new key; undo brings back the old key, so the earlier result returns
  from the cache, byte for byte.
- **Limit.** The page's font scale depends on every line (G-021). If an edit changes it, every
  line's target changes and every line is justified again. This is needed for a correct
  result, not a cache miss to tune away.
- **Scope.** A cache belongs to one font (shaper). It grows without bound within a session
  (no eviction), and Phase 8 sizes it.
- Test (`just golden`, `an_edit_rejustifies_only_its_line_and_undo_is_exact`, page 3):
  - a repeat fit does no work;
  - narrowing one word re-justifies exactly one line, and only that line's fit changes;
  - cached and uncached fits are byte-identical at every step;
  - the edited line closes;
  - undo gives the earlier page byte for byte, with no new justification or shaping.

## D56: One platform-neutral page engine, read through a `Source` (T8.1)
Status: proposed
Context: Phase 8 needs the CLI's whole page pipeline (resolve, map, fit, draw, highlight) in
WebAssembly (D5, rule 7). It lived in `mushaf-render-cli`, which is host-only (files, resvg),
and the CLI read data through `mushaf-db`, which is host-only (bundled SQLite, G-032).
Options: (a) re-implement loading in `mushaf-wasm`; (b) move the pipeline into a new crate that
reads its inputs through a trait; (c) make `mushaf-render-cli` build for wasm.
Decision: (b).
- `core/mushaf-engine` holds `Renderer`, `highlights`, the style rules (D36, D44) and a new
  `session::Session`, moved without behaviour change. It reads through
  `trait Source { read(rel); data_files() }`. A checkout on disk (`Path`) and the web bundle
  (D57) are the two sources. It is in `just core-targets`.
- `read_tree` moved from `mushaf-db` to `mushaf-model` (plain `std::fs`); `mushaf-db`
  re-exports it.
- `mushaf-render-cli` re-exports the engine and keeps only SVG/PNG output and the binaries, so
  every existing test and golden is unchanged.
- `Session` adds what the apps show: option groups with data defaults, margin notes (D60), the
  engine inspector, the provenance summary and the Phase 7 edit session. It keeps the
  resolution and mapping for the current choices and edits, and a `FitCache` (D55).
- `mushaf-wasm` (wasm-bindgen `=0.2.129`; `wasm-bindgen-cli` the same version) wraps the
  session as JSON strings from the same serde code as the CLI.
Evidence: `just web-parity` — Node runs the WASM engine on the bundle: 58 of 58 golden pages
byte-identical (draw-list SHA-256 and highlight JSON), `hafs-p003-lint` skipped (overlay built
in the Rust test). `cargo test -p mushaf-engine --test session -- --ignored`: the session draws
what `Renderer::render`/`render_highlighted` draw.

## D57: The web app's static data is a bundle of the files the engine reads (T8.2)
Status: proposed
Context: The site is static (rule 9) and must use "the same content as the CLI". The CLI builds
its `DataSet` from the data root's files, not from `mushaf.db`. Measured on this machine
(`cargo run --release -p mushaf-engine --example load_timing`): reading the data root 0.1 s,
parsing all of it 7.6 s; without `patches/raw/` (79 MB) 2.9 s. No riwāyah profile stacks a raw
patch file: profiles stack `patches/curated/*` or `patches/corrections/hafs.yaml`; raw patches
are the round-trip's input (D22).
Options: (a) `mushaf.db` with SQLite in WASM; (b) a binary serialisation of the parsed
`DataSet`; (c) the source files themselves in one archive.
Decision: (c).
- `build/web/data/mushaf.bundle` (untracked): a length-prefixed archive (`MUSHAFB1`, path,
  bytes) of the repository files, byte for byte: the data root's data files except patch files
  no profile stacks; `data/render/*`, `data/encodings/*`, `data/approvals/*` (read-only, for the
  provenance screen); every font a render profile or the style audit names. Writer:
  `mushaf-bundle` (`just web-data`), with `manifest.json` (size and SHA-256).
- The engine loads it through the same `DataSet::from_files` as the CLI. (a) would add a second
  loader and a C toolchain for wasm (G-032); (b) would add a second format and serde for the
  whole model.
- Size 81 MB; parsing in Node's WASM 4.2 s (G-213).
Evidence: `cargo test -p mushaf-render-cli --test bundle -- --ignored`: every bundled file
equals the checkout's bytes, only unstacked patch files are left out, and pages of Ḥafṣ, Warsh
and Qālūn draw identically from the bundle and the checkout. `just web-parity` (D56).

## D58: Web stack: Vite + TypeScript, no UI framework, the engine in a Web Worker (T8.3)
Status: proposed
Context: The web app must only draw what the core returns (rule 7) and run offline (rule 9).
Options: (a) a UI framework (React, Svelte); (b) plain TypeScript with Vite as dev server and
bundler.
Decision: (b). The app is a handful of screens over JSON and SVG from the core; a framework
adds dependencies and no logic the app needs. The WASM engine runs in a module Web Worker
(`apps/web/src/worker.ts`), so a page's work never blocks the UI. Exact pins: vite 8.3.5,
typescript 5.9.3, @playwright/test 1.64.1 (`package-lock.json`). Vite serves `build/web/pkg`
and `build/web/data` through a middleware (`vite.config.ts`) on 127.0.0.1; `vite build` copies
them into `dist/`, so the site is plain static files (`just web-build`). A
service worker (`public/sw.js`) caches the data bundle by SHA-256 and everything else network
first with the cache as fallback; no other origin is fetched.

## D59: The page is the core's SVG (T8.3)
Status: proposed; the on-screen page is replaced by D63 (canvas). Exports, taps and the
interactive SVG below still hold.
Context: Draw commands can be painted by TypeScript (SVG or Canvas) or the core's own SVG
backend can be shown as is.
Decision: the core's SVG. `mushaf_draw::to_svg_interactive` is `to_svg` plus `data-c`
(cluster) and `data-l` (line) attributes on each glyph `<use>`, so a tap names its cluster. A
test in `mushaf-draw` checks that removing those attributes gives `to_svg`'s bytes. Exports use
`to_svg` itself (the CLI's bytes), the draw-list JSON, and a PNG the browser rasterises from
that SVG. The edited marker is part of the core's drawing, so it is in every view and export
(rule 11; Playwright `free edit` test). Margin words are drawn by the core too
(`DrawWord::to_svg`). TypeScript paints no glyph and computes no position. Canvas was not chosen:
it would need TypeScript code to replay draw commands, which the SVG already is.
T8.4 made the SVG writer allocation-free; a test compares it with the previous formatter on
about 180,000 values (`fast_formatter_writes_the_same_bytes`).

## D60: Margin khilāf notes are derived from option groups (T8.3)
Status: proposed
Context: Farsh khilāf goes in the margin where the data provides alternative awjuh. The data's
only alternatives are option groups (`data/options/*.yaml`); there is no margin data (G-151).
Decision: `Session::margin_notes(page)`: for each option group of the riwāyah and each choice
other than the current one, every cluster on the page whose codepoints or annotations (D40)
differ under that choice, with the word as the core draws it under that choice. Built once per
set of choices; user edits are not compared. A cluster only the other wajh has is placed at the
item it is inserted at. Notes sit beside their line, outside the page box, so line breaks never
change; on a phone they are tap-to-show. Today this gives the `hafs.maliyah` note on the page of
69:28 (idghām, annotation only, D41). The option sheet shows every group the riwāyah's data
defines, with the data's default; groups are not split into uṣūl and farsh because the data does
not say which is which (G-215). A dedicated margin data source (G-151) would add notes to the
same `MarginNote` list.

## D61: Browser screenshots are compared with a blur and a 1-pixel neighbourhood (T8.5)
Status: proposed. Since D63 the test compares the app's canvas painting (worst: 1 pixel in
1,577,000); the tolerance below is kept unchanged.
Context: The goldens are rasterised by resvg; Chromium rasterises the same SVG with Skia. On
`hafs-p001`, 1.2 % of pixels differed by more than 32 on a channel. Every difference was on an
edge (a diff image showed no moved or missing glyph); 1-px frame lines are grey in one image and
crisp in the other.
Decision: the browser paints the SVG at the golden's pixel size (a first version painted it at
its intrinsic size and resampled, which shifted lower lines by a fraction of a pixel; the app's
PNG export was fixed the same way). Both images are box-blurred 3×3; a pixel fails when no pixel
within 1 px of it in the other image is within 32 on every channel, checked both ways; at most
0.1 % may fail, as in `golden.rs`. On the 50 golden pages the worst is 0.12 per mille. A control test compares page 3 with the edit
golden (one line moved, plus the marker) and must fail (about 22 per mille). The drawing itself is
proven identical by bytes (D56, D59), not by pixels.

## D62: The page budget is met by drawing ahead; measured in throttled Chromium (T8.4)
Status: proposed
Context: The budget is 16 ms per page on a mid-range phone. In WASM on this machine (Node) a
page the reader has not seen costs about 23 ms (median) in the core: fitting (shaping and
justification) is about 9 ms of it natively, the rest drawing and SVG. On a phone this is
several times more.
Decision:
- The session keeps the resolution and mapping per set of choices and edits, a `FitCache`
  (D55), and the last 12 drawn pages nearest the current page.
- The UI asks the worker for the next and previous pages as soon as a page is shown, so a page
  turn only shows a page already drawn.
- Margin differences are indexed once per set of choices (it was about 200 ms per page).
- The SVG writer was made about 2.4× faster with identical bytes (D59).
- Measured by `just web-bench`: Chromium with the CPU throttled 4× as the proxy for a
  mid-range phone, 20 page turns per muṣḥaf, with the reader's pause between turns. The budget
  is on the turn (request to SVG on screen); the core's own time per page is reported beside it.
  The real device is the owner's check at G5 (G-214).
- The SVG of a page drawn ahead is parsed into a detached element when it arrives, so a turn
  only swaps it in. A turn ends at the first frame after the swap.
Result with SVG on screen (this Mac, 2026-10-27, `just web-bench`): **the budget was not met.** Turn median
42 ms / p95 59 ms for Ḥafṣ, median 28 ms / p95 34 ms for Warsh. The core's time per page in the
worker is median 17 ms (Ḥafṣ) and 7 ms (Warsh); the worker does not appear to be slowed by
CDP's throttle (Node, unthrottled, gives about the same), so these are desktop numbers. The turn
is dominated by the browser styling and painting about 2,000 `<use>` elements; CSS containment
did not help. A way to meet it would be to paint the core's draw list into an `OffscreenCanvas`
in the worker and show a bitmap, keeping the SVG only for taps. That changes D59 and is left to
the owner (G-214). The owner chose it (2026-10-27): D63. With D63 the budget is met.

## D63: The page on screen is painted into a canvas in the worker (T8.4)
Status: accepted (owner, 2026-10-27: "Lets make the switch to canvas painting")
Context: With the core's SVG on screen (D59), a page turn cost 28–42 ms median at CPU/4, almost
all of it the browser parsing, styling and painting about 2,000 SVG elements (D62).
Decision:
- The worker paints the core's draw list (`render` JSON) into an `OffscreenCanvas` at the page's
  pixel width and sends an `ImageBitmap` (`apps/web/src/paint.ts`). The painter replays the
  draw commands as given: each glyph outline under its transform, then fills, then strokes, in
  the colours of `mushaf-draw`'s SVG backend. It decides nothing (rule 7).
- Neighbouring pages are painted ahead (D62). A turn hands the bitmap to a `bitmaprenderer`
  canvas, with no copy.
- After the turn, the core's interactive SVG is laid on top, transparent, so a tap names its
  cluster; a selected or inspected cluster shows through it. Taps work from the frame after.
- A width or pixel-density change repaints. Exports are unchanged: the SVG and JSON are the
  core's bytes; the PNG export is the browser's raster of that SVG.
- The edited marker is in the draw list, so it is painted on screen (Playwright `free edit`
  checks red pixels in the canvas, and none after undo).
Evidence:
- `just web-golden`: all 50 golden pages, painted at 1000 px, match the golden PNGs; the worst
  differs on 1 pixel in 1,577,000. The control still fails.
- `just web-bench` (CPU/4, 20 turns each, two runs): turn median 9.7–10.4 ms / p95 15.1–15.3 ms
  (Ḥafṣ), median 7.8–8.5 ms / p95 10.5–12.6 ms (Warsh). The budget is met. The worker's time per
  page (core plus painting) is 22–34 ms median; it runs while the reader reads.
- Addendum (2026-10-27): with highlighting on, `highlights` compared the whole text on every
  page (about 200 ms per page in the worker, so quick turns waited). The session now computes
  the whole text's highlights once per text (`highlights_with`) and keeps those on the page; the
  output is unchanged (session test, `just web-parity` highlight goldens). Highlighted Warsh:
  worker 39 ms median, turn p95 15.6 ms. `web-bench` now includes it. A later Ḥafṣ run gave
  p95 18 ms while another process held a CPU core at 99 %; to be re-measured on a quiet machine.
- Addendum 2 (2026-10-27, owner: converted styles turned slowly): two causes of turns of about
  2 s in Warsh and Qālūn Ḥijāzī. (1) The margin index resolved the whole text once (2.4 s) even
  for a riwāyah with no option group; it is now skipped. (2) A page the core cannot draw in the
  converted style (G-102: 102 Warsh and 128 Qālūn pages) threw in the UI, so the page stayed
  put and its neighbours were never drawn ahead; it now shows a notice and paging goes on
  around it. The core's own time per page is about 12 ms for the converted styles against about
  5 ms for the Maghribi defaults (WASM, Node): DigitalKhatt with full justification against the
  KFGQPC font's spacing only. Drawing ahead hides it. `web-bench` now includes Warsh Ḥijāzī; its
  runs on 2026-10-27 were taken while another process held a CPU core and are not reliable.

## D64: The owner's highlight marks in the web editor (T8.3)
Status: accepted (owner, 2026-10-27: "add something where i can change highlights"; chose
"mark a highlighted word as not a difference, or an unhighlighted word as should be
highlighted, with an optional note")
Decision:
- `Session::set_highlight_mark(cluster, Some(HighlightMark { show, note }) | None)`.
  `show: false` drops the core's highlight on that cluster. `show: true` keeps or adds one; an
  added one has label `owner`. Either way the highlight carries `owner: Some(note)`.
  `Highlight::owner` is omitted from JSON when `None`, so every golden is unchanged.
- Display and feedback only: marks change no text and no drawing, so the page stays canonical
  and has no edited marker. They are not data: a mark becomes a rule or normalisation change
  only with a source (rules 1, 4).
- Exported and imported as YAML (`riwayah`, `style`, `spine`, `marks`) from the editor, apart
  from the text edits' overlay file.
Evidence: `cargo test -p mushaf-engine --test session -- --ignored owner_highlight` (marks change
only `highlights`; export/import round-trips); Playwright `highlight marks`; `just web-parity`.

D45 addendum 3 (owner, 2026-10-27: "do not higlights difference in mudūd (qasr, tawassut, ṭūl)"):
the maddah sign U+0653 is ignored in the highlight comparison, so differences in madd length are
never highlighted.

## D65: Free edits are saved in the browser (T8.3)
Status: accepted (owner, 2026-10-28, relayed by the planning session: a Netlify test deploy
"with edits saved")
Decision:
- After every edit, undo, redo, clear or import, the app stores the core's own export of the
  edits (`export_edits`, the overlay YAML) in `localStorage` under
  `mushaf.edits.<riwayah>/<style>`, or removes the key when there are no edits.
- When a muṣḥaf is opened, a saved copy is restored through `import_edits`. A restored page is
  the same edited page, so it carries the edited marker (rule 11). A copy that fails to import
  is left in place and reported in the status line.
- "Clear saved edits" in the editor clears the edits and the saved copy. Export and import
  of files are unchanged.
- Storage is per browser and per device. Nothing is sent anywhere (rule 9). Access is wrapped
  in try/catch, so private mode simply does not save.
Evidence: Playwright `free edits are saved in the browser` (edit, reload: the same edits and
the marker; clear, reload: canonical).
