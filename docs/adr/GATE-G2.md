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
