# Architecture Decision Records

One file per decision (`D<n>.md`), plus the phase gate records (`GATE-G<n>.md`). A decision
is never made silently. Only the owner moves an ADR from `proposed` to `accepted`.
Addenda go at the end of their ADR's file. Add a row to the index below with every new ADR.
Until 2026-11-21 all ADRs were in `docs/decisions.md`; that file now only points here.

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
| [D1](D1.md) | Line policy for non-Ḥafṣ riwāyāt | accepted: native layouts; final form D29 (GATE G3, 2026-10-23) |
| [D2](D2.md) | Font per riwāyah | accepted: hybrid (DigitalKhatt + KFGQPC) |
| [D3](D3.md) | Justification implementation | accepted: Rust port on harfrust |
| [D4](D4.md) | Draw path | accepted with the Phase 3 plan: positioned glyph outlines |
| [D5](D5.md) | First platform | accepted (website first) |
| [D6](D6.md) | Ḥafṣ edition: text + layout + font | accepted: 1441H digital layout (reconfirmed 2026-10-22) |
| [D7](D7.md) | Licensing and distribution | accepted (owner-declared) |
| [D8](D8.md) | Script/layout scope | accepted |
| [D9](D9.md) | ID and patch model details | accepted; P2 addendum approved at the Phase 2 gate (2026-10-22) |
| [D10](D10.md) | Numeric determinism | accepted |
| [D11](D11.md) | Spine file format and item IDs | accepted (Phase 1 plan) |
| [D12](D12.md) | Core model: formats, patch semantics, DB access | accepted (Phase 2 plan); P2 addendum approved (2026-10-22) |
| [D13](D13.md) | Phase 4 inputs: Bazzi source, single-source, encoding handling | accepted (2026-10-22) |
| [D14](D14.md) | KFGQPC to DigitalKhatt Madina encoding table (Ḥafṣ) | accepted (approach with the Phase 3 plan; lossy and owner rows decided 2026-10-22) |
| [D15](D15.md) | Page geometry, short lines and overflow (Ḥafṣ, 1441H) | accepted with the Phase 3 plan; numbers for review at G1 |
| [D16](D16.md) | Manual overrides, header text, and the "edited" marker in draw output | accepted with the Phase 3 plan |
| [D17](D17.md) | Superscript alef after non-joining letters (G-045) | rejected by the owner after review; reverted (2026-10-22) |
| [D18](D18.md) | Owner print corrections at G1: sajdah overline and owner word rows | accepted (owner decisions at GATE G1, 2026-10-22) |
| [D20](D20.md) | Phase 4 derivation pipeline: ingest, aligner, differ, emitter | accepted (Phase 4 plan, 2026-10-22) |
| [D21](D21.md) | Difference-kind tags: what the data can and cannot classify | accepted: C, rows owner-signed (2026-10-22) |
| [D22](D22.md) | Round-trip check for raw patches | accepted (Phase 4 plan, 2026-10-22) |
| [D26](D26.md) | Riwāyah profiles and surah transitions | accepted with the Phase 5 plan (2026-10-22) |
| [D27](D27.md) | Warsh and Qālūn on their KFGQPC fonts: TrueType outlines, spacing-only justification | accepted with the Phase 5 plan (P4); per-glyph exceptions decided (2026-10-22) |
| [D28](D28.md) | DigitalKhatt rows for cluster forms outside Ḥafṣ | accepted (approach with the plan, P3); groups decided at the T5.3 checkpoint (2026-10-22) |
| [D29](D29.md) | Native layouts: Ḥafṣ breaks inside each riwāyah's KFGQPC ayah lines (D1 final form) | accepted at GATE G3 (2026-10-23); Warsh/Qālūn breaks unverified (G-104) |
| [D30](D30.md) | Short lines and header basmalah on native layouts | accepted at GATE G3 (2026-10-23), 60% threshold |
| [D31](D31.md) | Overflow per riwāyah | accepted (plan P6; numbers reviewed at GATE G3, 2026-10-23) |
| [D32](D32.md) | Rule-5 edition notes per riwāyah | accepted at GATE G3 (2026-10-23) |
| [D33](D33.md) | Display style as a render key: (riwāyah, style), default style only in Phase 5 | accepted (owner requirement, relayed 2026-10-23; confirmed at GATE G3); implements D36's Phase 5 part |
| [D36](D36.md) | Display styles: Ḥijāzī and Maghribi for every riwāyah | accepted (scope); built in Phase 6 |
| [D37](D37.md) | Highlighting differences from Ḥafṣ | accepted (scope); built in Phase 6, UI in Phase 8 |
| [D38](D38.md) | Rule DSL: declarative patterns expanded into ordinary patches in the core | accepted with the T6.1 plan (2026-10-26) |
| [D39](D39.md) | Owner corrections to the Ḥafṣ spine text as patches; G-142 (52:37 sīn below the ṣād) | accepted (owner decision, 2026-10-26) |
| [D40](D40.md) | Annotation channel: pronunciation-only features as `pron.<feature>` cluster annotations | accepted (T6.3, 2026-10-26) |
| [D41](D41.md) | مَالِيَهْ idghām wajh changes no glyph (owner decision, departs from p. ح) | accepted (owner decision, 2026-10-26) |
| [D42](D42.md) | T6.4: data-derived rules (hypotheses) plus residual patches per riwāyah | accepted (T6.4, 2026-10-26); every rule awaits a citation |
| [D43](D43.md) | Ṭarīq as the option group `<riwāyah>.tariq`; ṭarīq-level patches apply under their ṭarīq only | accepted (T6.6, 2026-10-26) |
| [D44](D44.md) | Display styles: render-time conversion from owner-confirmed rows; per-(riwāyah, style) audit decides availability | accepted (T6.7–T6.10, 2026-10-26) |
| [D45](D45.md) | Difference highlighting from rule provenance; cited rules by default, hypotheses opt-in and flagged | accepted (T6.11–T6.12, 2026-10-26); the owner chose to show hypotheses (2026-10-26) |
| [D46](D46.md) | Warsh and Qālūn in the Ḥijāzī style: owner-approved writing conventions, own pause signs, blocked pages listed | accepted (owner, 2026-10-26) |
| [D47](D47.md) | Approved Ḥijāzī drawings of Maghribi forms, with mark adjustments | accepted (owner, 2026-10-26) |
| [D50](D50.md) | Locating the book's words, and the convention projection (TB.4–TB.5) | proposed (TB.4–TB.6, 2026-10-27). For the owner at the pilot review. |
| [D51](D51.md) | Parsing the book's statements (TB.2–TB.3) | proposed (2026-10-27). |
| [D52](D52.md) | Statuses and what is applied mechanically (TB.5) | proposed (2026-10-27). The per-word merge of statements (`run.merge_overlaps`), made after the first pilot result, was accepted by the … |
| [D53](D53.md) | The free-edit API, and two patches per key in the user layer (T7.1, T7.2) | accepted (owner, 2026-10-27: "Approve D53, D54 and D55") |
| [D54](D54.md) | The free-edit lint, and the "edited" marker stays conservative (T7.3) | accepted (owner, 2026-10-27: "Approve D53, D54 and D55") |
| [D55](D55.md) | Re-justifying only the lines an edit touches (T7.4) | accepted (owner, 2026-10-27: "Approve D53, D54 and D55") |
| [D56](D56.md) | One platform-neutral page engine, read through a `Source` (T8.1) | proposed |
| [D57](D57.md) | The web app's static data is a bundle of the files the engine reads (T8.2) | proposed |
| [D58](D58.md) | Web stack: Vite + TypeScript, no UI framework, the engine in a Web Worker (T8.3) | proposed |
| [D59](D59.md) | The page is the core's SVG (T8.3) | proposed; the on-screen page is replaced by D63 (canvas). Exports, taps and the interactive SVG below still hold. |
| [D60](D60.md) | Margin khilāf notes are derived from option groups (T8.3) | proposed |
| [D61](D61.md) | Browser screenshots are compared with a blur and a 1-pixel neighbourhood (T8.5) | proposed. Since D63 the test compares the app's canvas painting (worst: 1 pixel in 1,577,000); the tolerance below is kept unchanged. |
| [D62](D62.md) | The page budget is met by drawing ahead; measured in throttled Chromium (T8.4) | proposed |
| [D63](D63.md) | The page on screen is painted into a canvas in the worker (T8.4) | accepted (owner, 2026-10-27: "Lets make the switch to canvas painting") |
| [D64](D64.md) | The owner's highlight marks in the web editor (T8.3) | accepted (owner, 2026-10-27: "add something where i can change highlights"; chose "mark a highlighted word as not a difference, or an … |
| [D65](D65.md) | Free edits are saved in the browser (T8.3) | accepted (owner, 2026-10-28, relayed by the planning session: a Netlify test deploy "with edits saved") |
| [D66](D66.md) | al-Futūḥ al-Bāhira as structured text, extracted glyph by glyph (prerequisite for T6.0) | proposed (contributor PR, 2026-11-20); the owner decides. Nothing in the app or the patch stack depends on it. |
| [GATE-G1](GATE-G1.md) | GATE G1 — Phase 3 review: passed (2026-10-22) |  |
| [GATE-G2](GATE-G2.md) | GATE G2 — Phase 4 review: passed (2026-10-22) |  |
| [GATE-G3](GATE-G3.md) | GATE G3 — Phase 5 review: passed (2026-10-23) |  |
