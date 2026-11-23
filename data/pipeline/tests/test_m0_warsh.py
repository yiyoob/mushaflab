"""M0.6: the Warsh prototype's tracked outputs are current, and every difference is classified.

Needs al-Futuh volume 1 extracted to build/ (`just book-futuh --vol 1 --pages 47-63 --surah 1
--tag m0`; untracked, it holds book text) and the pinned al-Budur OCR text; skips otherwise.
"""

import pytest

from pipeline import m0_warsh

needs_inputs = pytest.mark.skipif(
    not (m0_warsh.FUTUH.is_file() and m0_warsh.BUDUR_TXT.is_file()),
    reason="al-Futuh volume 1 extraction or al-Budur text absent",
)


@needs_inputs
def test_no_difference_is_left_unexplained() -> None:
    hafs = m0_warsh.words()
    rows = m0_warsh.futuh_rows(m0_warsh.FUTUH)
    book: dict[str, list[dict]] = {}
    for r in rows:
        for w in r["words"]:
            if w.get("spine"):
                book.setdefault(w["spine"], []).append(r)
    results = [m0_warsh.classify(d, hafs, book) for d in m0_warsh.warsh_diffs()]
    assert results
    assert [
        d for d, (c, _) in zip(m0_warsh.warsh_diffs(), results, strict=True) if c == "unexplained"
    ] == []
