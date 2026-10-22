"""T4.8: review sheets (synthetic folding rules; one real sheet built from pinned sources)."""

from pipeline import emit_patches, review_sheets
from pipeline.differ import Change
from pipeline.review_sheets import pattern
from pipeline.tests.markers import needs_sources


def change(kind: str, item_kind: str = "word", level: str = "cluster") -> Change:
    return Change("1:1:1:1", "replace", {}, level, kind, item_kind, "a", "b", "1:1:1", 1)


def test_only_cluster_and_separator_changes_fold_into_patterns() -> None:
    assert pattern(change("marks")) == ("mark-only", "a", "b")
    assert pattern(change("separator", level="item")) == ("separator", "a", "b")
    assert pattern(change("item-insert", level="item")) is None
    assert pattern(change("letter", item_kind="ayah_end")) is None
    assert pattern(change("separator", item_kind="hizb", level="item")) is None


@needs_sources
def test_shuba_sheet_shows_every_change_once() -> None:
    _, changes = emit_patches.derive("shuba")
    page = review_sheets.sheet("shuba")
    assert page.startswith("<!doctype html>")
    assert "hypotheses" not in page.split("<h2>Frequent")[0].lower()  # Warsh-only cases
    shown = page.count("<code>set_marks ") + page.count("<code>replace ")
    shown += page.count("<code>delete ") + page.count("<code>insert_")
    folded = sum(1 for c in changes if review_sheets.pattern(c) in _frequent(changes))
    ayah = sum(1 for c in changes if c.item_kind == "ayah_end")
    assert shown + folded + ayah == len(changes)


def _frequent(changes: list[Change]) -> set:
    from collections import Counter  # noqa: PLC0415 - test helper

    counts = Counter(pattern(c) for c in changes if pattern(c))
    return {k for k, n in counts.items() if n >= review_sheets.FREQUENT}
