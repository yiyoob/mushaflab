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
