"""M0.5: the derived mark order is current and explains every letter but the listed ones."""

from pipeline import corpus_dabt


def test_tracked_style_is_current() -> None:
    assert corpus_dabt.OUT.read_text(encoding="utf-8") == corpus_dabt.render()


def test_order_covers_every_mark_and_exceptions_are_rare() -> None:
    order, exceptions, n = corpus_dabt.derive()
    assert len(order) == len(set(order))
    # Exceptions are measured, not expected values about the Quran: the test only requires
    # that the order explains all but a tiny share of letters, so a regression shows up.
    assert len(exceptions) * 10_000 < n


def _words() -> dict[str, str]:
    out = {}
    for line in corpus_dabt.corpus_words.OUT.read_text(encoding="utf-8").split("\n"):
        if line and not line.startswith(("#", "word_id\t")):
            wid, _kind, text = line.split("\t")[:3]
            out[wid] = text
    return out


def test_signs_section_survives_a_rewrite() -> None:
    text = corpus_dabt.OUT.read_text(encoding="utf-8")
    assert corpus_dabt.SIGNS_MARK in text
    assert text.endswith(corpus_dabt._signs_section())


def test_sajdah_overline_sits_by_a_sajdah_sign() -> None:
    # M1.2, row sajdah_word: the overline (U+06E4) is on the word that calls for the sajdah,
    # so every ayah carrying it has the sajdah sign (U+06E9) in it or within the next two ayahs
    # (measured: the sign follows in the next ayah at four places and two ayahs on at one).
    words = _words()
    ayah = lambda wid: tuple(int(x) for x in wid.split(":")[:2])  # noqa: E731
    sign = {ayah(w) for w, t in words.items() if chr(0x06E9) in t}
    line = {ayah(w) for w, t in words.items() if chr(0x06E4) in t}
    assert line
    assert all(any((a[0], a[1] + k) in sign for k in (0, 1, 2)) for a in line)
    assert len(line) == len(sign)


def test_review_sheet_is_current() -> None:
    assert corpus_dabt.SHEET.read_text(encoding="utf-8") == corpus_dabt.render_sheet()
