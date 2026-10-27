"""T6.7/T6.8: style conversion rows come only from owner-confirmed encoding rows (D21, D44)."""

from pipeline import classify, style_conversion


def test_rows_are_the_confirmed_rows_shared_by_every_maghribi_riwayah(monkeypatch) -> None:
    confirmed = {
        "warsh": {("\u06e1", "\u0652"): "encoding", ("a", "b"): "encoding"},
        "qalun": {("\u06e1", "\u0652"): "encoding", ("c", "d"): "spelling"},
    }
    monkeypatch.setattr(classify, "load_confirmed", lambda: confirmed)
    rows = style_conversion.rows()
    assert [(r["from"], r["to"]) for r in rows] == [("U+06E1", "U+0652")]
    assert rows[0]["from_style"] == "hijazi" and rows[0]["to_style"] == "maghribi"
    assert "owner-confirmed" in rows[0]["evidence"]


def test_no_confirmed_rows_no_conversion(monkeypatch) -> None:
    monkeypatch.setattr(classify, "load_confirmed", lambda: {"warsh": {}, "qalun": {}})
    assert style_conversion.rows() == []
