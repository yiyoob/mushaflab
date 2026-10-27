# al-Budur al-Zahirah pilot: success criteria (TB.6)

Written before the pilot run and committed before it, so the order is visible in git. The
pilot is judged against these criteria only. Nothing in this file is a claim about any
reading.

## Source and range

- Source: `book-al-budur-al-zahirah` (MANIFEST.json, sha256 `d2043ac9…dae7b`). It is the
  turath.io EPUB of the Dār al-Kitāb al-ʿArabī first edition (1401/1981), and its page
  numbering matches the print.
- The book's ṭarīq is al-Shāṭibiyyah and al-Durrah. Evidence: the title on `info.xhtml`
  ("من طريقَي الشاطبية والدُّرة"), and page_1_0001 p. 5, which names al-Taysīr, al-Taḥbīr,
  al-Shāṭibiyyah and al-Durrah.
- Range: al-Fātiḥah and al-Baqarah up to the book's first rubʿ end. That end is the entry the
  book marks "منتهى الربع", whose place is fixed by matching, not by us. The pilot reports
  whether the spine's own rubʿ sign follows it; this is a cross-check only.
- All 8 riwāyāt: hafs, shuba, warsh, qalun, bazzi, qunbul, duri_abuamr, susi.
- The rule index is built from the whole book, and every rule is applied within the range.

## Criteria

1. **Integrity.** The run stops before judging anything if the EPUB does not match its
   MANIFEST row, or if any cited lexicon form (`readers.yaml`) is missing from its cited page.
2. **Citations: 100%.** Every `mentioned` and `rule-covered` status carries a citation
   (EPUB page id and printed page). The run reports the number of uncited statuses, and it
   must be 0.
3. **Ḥafṣ false flags: near zero.**
   - A Ḥafṣ false flag is a pilot word where Ḥafṣ is in every minimum attribution of a
     disagreement between the book and our data. Our Ḥafṣ text is the spine (KFGQPC Ḥafṣ,
     al-Shāṭibiyyah), so any such flag is presumed to come from the method.
   - The exact rate is reported twice: over all words in the range, and over the words that
     are `mentioned` or `rule-covered`.
   - **Target: ≤ 0.5% of the words in the range, with every Ḥafṣ flag explained one by one.**
   - Words where Ḥafṣ is only *possibly* attributed (in some minimum attributions, not all)
     are reported as a separate rate.
4. **Undecidable places are counted, not hidden.** Each of the following is reported as a
   count, and none of them may raise a flag or produce a `mentioned` status:
   - ambiguous placements;
   - unmatched placements;
   - entries whose complement ("الباقون") has two readings that give different verdicts.
5. **No uncited fact decides a status.**
   - Reader names come only from `readers.yaml`, where each name is cited and checked by
     criterion 1.
   - Feature, scope and convention classes (`features.yaml`, `conventions.yaml`) are our
     hypotheses (D50, D51). They decide only whether a difference is flagged or explained,
     never a status. Every explained pair is listed in the report.
6. **Both directions are reported.**
   - Book → ours: missing and unstated differences.
   - Ours → book: patches without a statement.
   - Phase 6 rules in the range, each marked supported, partly, unsupported or encoding-only.
7. **Rule application is listed, never silent.** Every application of every applied rule is
   listed. Rules of unclear scope (class or general) are listed and are not applied.

The pilot passes when criteria 1, 2 and 5 hold exactly, criterion 3 meets its target, and
the counts for 4, 6 and 7 are reported. The full run waits for the owner whatever the result.
