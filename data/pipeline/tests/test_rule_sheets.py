"""T6.5: rule review sheets (synthetic values, Latin letters only)."""

from pipeline import rule_sheets


def test_changed_follows_the_action() -> None:
    assert rule_sheets._changed("a", {"op": "replace", "payload": {"cps": "U+0062"}}) == "b"
    add = {"op": "set_marks", "payload": {"add": ["U+0301"]}}
    assert rule_sheets._changed("a", add) == "a\u0301"
    ins = {
        "op": "insert_after",
        "payload": {"seq": 1, "cluster": {"kind": "letter", "cps": "U+0062"}},
    }
    assert rule_sheets._changed("a", ins) == "ab"
    attr = {"op": "set_attr", "payload": {"annotations": {"pron.madd": "4"}}}
    assert rule_sheets._changed("a", attr) == "a"


def test_summary_counts_statuses() -> None:
    text = rule_sheets.summary(
        {
            "x": [
                {"id": "x.1", "places": 3, "exceptions": 1, "status": "hypothesis"},
                {"id": "x.2", "places": 2, "exceptions": 0, "status": "rule-derived"},
            ]
        }
    )
    assert "| x | 2 | 5 | 1 | 1 | 1 |" in text
