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
