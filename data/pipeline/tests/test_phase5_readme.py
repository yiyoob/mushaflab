"""T5.5: the GATE G3 review index is built from the tracked reports."""

from pipeline import phase5_readme


def test_index_lists_every_native_riwayah() -> None:
    text = phase5_readme.render()
    for r in phase5_readme.NATIVE:
        assert f"\n| {r} | " in text


def test_tracked_index_is_current() -> None:
    assert phase5_readme.main(["", "check"]) == 0
