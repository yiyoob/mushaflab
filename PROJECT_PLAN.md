# Multi-Qirāʾāt Muṣḥaf — Project Plan v3

Status: accepted 2026-11-23 (architecture reset, D67) · Owner: Yusuf · Maintainers: Yusuf, Eisa ·
Executor: Claude Code, one milestone at a time, with a human gate after each.

The muṣḥaf displays any of the **20 riwāyāt of al-Shāṭibiyyah and al-Durrah**, with proper
rendering. Every wajh the books allow can be toggled (e.g. Warsh's madd al-badal), and the
related uṣūl stay within the combinations the books allow. Layouts are independent of the
riwāyah, so any riwāyah can be drawn on any muṣḥaf layout (15-line Madinah first; Indo-Pak
13-line and others later).

The architecture is in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). The plan it replaces
(v2, the patch model, Phases 0–8) is in [`docs/history/PROJECT_PLAN-v2.md`](docs/history/PROJECT_PLAN-v2.md).
References to `PROJECT_PLAN §n`, phases or `T<phase>.<n>` tasks in legacy code and
review documents mean v2.

## 0. Scope

**In scope.**
- **Riwāyāt:** all 20 riwāyāt of the ten qurrāʾ by the ṭuruq of al-Shāṭibiyyah (the seven) and
  al-Durrah (the three). Al-Ṭayyibah is out of scope until the owner says otherwise.
- **Wujūh:** every wajh those ṭuruq allow, as a reader option, constrained by the combination
  tables the books give (D68).
- **Layouts:** the Madinah 15-line ʿUthmānī layout first. Further layouts (Warsh's Maghribī
  muṣḥaf, Indo-Pak 13-line, …) arrive in M4; `layout_id`, `script_id` and the writing style are
  explicit everywhere.
- **Platforms:** web first, then iOS and Android, all on one Rust engine.

**Sources of truth (D68).**

| Source | Role |
| --- | --- |
| *al-Budūr al-Zāhira*, ʿAbd al-Fattāḥ al-Qāḍī (Dār al-Kitāb al-ʿArabī, 1401/1981) | source of truth |
| *al-Futūḥ al-Bāhira* (6 volumes) | source of truth |
| KFGQPC muṣḥafs (8 riwāyāt) | check only: each prints one wajh per place |
| Ḥafṣ KFGQPC text | the word table's text and the base the readings change |

A fact both books state is `cross-source`; a fact one states is `single-source`; a
contradiction goes to the owner. Excluded for ṭarīq reasons: Ibn Mihrān's *al-Mabsūṭ* and
al-Nashshār's *al-Budūr al-Zāhira*.

**Out of scope for now:** al-Ṭayyibah, shawādh, audio, tafsīr.

## 1. Milestones

Each milestone is one PR to `main`, and each ends at a gate the owner checks. The patch-model
app (legacy) is frozen: it keeps building and rendering the 8 KFGQPC riwāyāt until the new
model reproduces each of them, and is deleted in M3 (D67).

| Milestone | Delivers | Gate |
| --- | --- | --- |
| **M0** Agree the model | plan, rules, glossary; `corpus/` formats; `mushaf-corpus` (load, check, build); Ḥafṣ words table; letter-feature round trip; Warsh prototype for al-Fātiḥa + al-Baqarah 1–20 | owner approves the formats (D69, D70, D72) and reviews the Warsh residual report |
| **M1** Ḥafṣ from the new model | letter features for the whole text; Madinah writing style; the current renderer draws from the new model | byte-identical to KFGQPC Ḥafṣ on all 604 pages; goldens unchanged |
| **M2** Warsh and Qālūn | farsh and uṣūl of Nāfiʿ from the books; Maghribī style; reader toggles with combination tables | at KFGQPC's wajh, output matches KFGQPC; every other wajh cited |
| **M3** The other five KFGQPC riwāyāt | Shuʿbah, al-Bazzī, Qunbul, al-Dūrī ʿan Abī ʿAmr, al-Sūsī | all 8 match at KFGQPC's wajh; legacy patch model deleted |
| **M4** Layouts | layout tables, reflow rules, a second muṣḥaf layout (owner's choice) | line-fit and font-coverage report for every (riwāyah, layout, style) |
| **M5** The 12 without a KFGQPC text | farsh and uṣūl from the books; disagreements to the review queue | owner or muqriʾ sign-off per riwāyah |
| **M6** Apps | web app on per-page data chunks, then iOS and Android | 16 ms per page; under 5 MB first load per riwāyah and layout |

M4 may start once M1 passes, alongside M2 and M3. M5 depends on the error rate M2 and M3
measure.

### M0 tasks

| Task | Delivers | Accept |
| --- | --- | --- |
| M0.1 | this plan, `CLAUDE.md`, `docs/ARCHITECTURE.md`, `docs/GLOSSARY.md`, ADRs D67–D72 | `just lint` |
| M0.2 | `corpus/` tree, `docs/formats.md` | `just corpus-check` on the empty tree |
| M0.3 | `core/mushaf-corpus`: loaders, validator, `mushaf check` / `mushaf build` | `just corpus-check`, crate tests |
| M0.4 | `corpus/text/words.tsv` from the Ḥafṣ spine | words ↔ spine identical, item for item |
| M0.5 | `core/mushaf-text`: letter features, `decode` / `encode` | byte-identical round trip on every word |
| M0.6 | Warsh prototype for 1:1–2:20 from the books | `docs/review/m0/warsh-residuals.md`, every residual classified |

## 2. Test matrix

| Test | From | What it proves |
| --- | --- | --- |
| Source checksums | now | the pipeline runs on pinned data |
| Corpus check | M0 | every row has a source and status; every reference resolves |
| Words ↔ spine | M0 | the word table holds the KFGQPC Ḥafṣ text exactly |
| Letter-feature round trip | M0 | decode + encode reproduce every word byte for byte |
| KFGQPC check | M1–M3 | at KFGQPC's wajh, our output equals its text |
| Cross-source report | M2 | where the two books agree, disagree, or only one speaks |
| Combination tables | M2 | no reader setting reaches a combination the books exclude |
| Shaping integrity | now | no `.notdef`, dotted circle or unattached mark on any page (rule 6) |
| Line-fit and goldens | now | justified lines close; no visual regressions |
| Edited marker | now | non-canonical pages always show it |
| Approvals integrity | now | owner statuses only in `data/approvals/` |
| Device performance | M6 | 16 ms per page |

## 3. Risks

| Risk | Mitigation |
| --- | --- |
| Importers misread the books | page reference on every row; two books cross-checked; KFGQPC check on the 8; owner review |
| Letter features cannot express a reading | round trip on all of Ḥafṣ first (M0.5); unknown combinations are hard failures; D72 is revised, not worked around |
| Fonts lack signs a riwāyah or style needs | coverage audit per (riwāyah, layout, style); unavailable combinations are shown as unavailable (rule 6) |
| A layout from another printing splits words differently | per-layout split table (M4) |
| LLM-generated religious content creeping in | CLAUDE.md rule 1; provenance on every row; owner-only statuses |
| The rebuild stalls and the legacy app rots | legacy frozen but built by `just gate` until M3 |
