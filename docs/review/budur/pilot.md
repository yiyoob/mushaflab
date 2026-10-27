# al-Budur al-Zahirah pilot: results (TB.6)

Run: `BOOKS_DIR=<main>/data/sources/raw/books just budur-verify` (about 2.5 min). The report is
`build/budur/pilot/report.html` (untracked: it holds book text). Criteria:
[pilot-criteria.md](pilot-criteria.md), committed before the run (2cf0b5c).

Range: 1:1:1 to 2:25:34, 360 words, ending at the book's first rubʿ end ("خالدون", page_1_0021
p. 25). The spine's rubʿ sign follows it directly (2:26:0), as an independent cross-check.

## Against the criteria

| # | Criterion | Result |
|---|---|---|
| 1 | Integrity preflight | Pass: EPUB matches MANIFEST, and all 75 cited lexicon forms occur on their pages |
| 2 | Citations | Pass: 0 uncited `mentioned`/`rule-covered` statuses |
| 3 | Ḥafṣ false flags ≤ 0.5% | **0 / 360 words (0.0000%)**, after one method fix (below). 5 words (1.39%) are *possible* only |
| 4 | Undecidable counted | 3 words undecidable (complement scope); placements in the rubʿ: 54 matched, 7 ambiguous, 2 unmatched |
| 5 | No uncited fact decides a status | Pass: statuses come only from placed entries and page-checked names |
| 6 | Both directions | See the per-riwāyah table |
| 7 | Rule application listed | 399 rules indexed; 11 applications in range, all at their own entries; 380 class/general rules listed, not applied |

**Disclosure on criterion 3.** The first run gave Ḥafṣ 2/360 (0.556%), over the target. Both
flags came from one artefact. The al-mudgham al-kabīr entry for 1:3:2–1:4:1 was judged
without the farsh entry that states the مالك/ملك difference at 1:4:1. The fix judges each
word against everything the book says at that word (`run.merge_overlaps`), as D52 intends.
It was made after seeing the result. The owner should weigh the 0% accordingly.

The 5 *possible* Ḥafṣ words are not flags; in each, all 8 riwāyāt tie:
- 1:1:1–1:1:4, the basmalah: absent as an ayah from 4 of the KFGQPC texts (G-206);
- 2:15:2: a hamza-seat encoding (ئ against ي + U+0654) in warsh and susi (G-208).

## Per riwāyah (360 words)

| riwāyah | mentioned | not-mentioned | flagged | main cause of flags |
|---|---|---|---|---|
| hafs | 63 | 297 | 0 | — |
| shuba | 63 | 297 | 0 | — |
| warsh | 63 | 297 | 18 | unstated differences at entries (10); patches without a statement (7) |
| qalun | 63 | 297 | 2 | one missing, one unstated difference |
| bazzi | 63 | 297 | 49 | 48 patches without a statement: ṣilat mīm al-jamʿ (G-207) |
| qunbul | 63 | 297 | 51 | the same 48, plus 3 at entries |
| duri_abuamr | 63 | 297 | 1 | one patch without a statement |
| susi | 63 | 297 | 10 | patches without a statement (7); unstated differences (3) |

`rule-covered` is 0 for all: every application of the 7 whole-Qurʾān rules and 12 surah
rules that falls in the range is at the rule's own entry. Explained pairs, listed in the
report: 65 phonetic (KFGQPC writes that delivery), and 200 Maghribī-family convention
candidates.

## Phase 6 rules in the range

Of 182 data-derived rules: 35 supported, 22 partly, 65 unsupported, 60 encoding-only.
"Unsupported" here means "not mentioned at these places", not "wrong"; most rows need the
class rules mapped (G-203).

## Decisions for the owner

1. Accept or revise the method fix above (D52), and the three proposed ADRs D50–D52.
2. Confirm or remove each `conventions.yaml` row and each feature class (G-202).
3. Map the class rules, starting with ṣilat mīm al-jamʿ for Ibn Kathīr (G-207), which
   alone accounts for 96 flags.
4. Choose whether the full run goes ahead, and with which changes.
