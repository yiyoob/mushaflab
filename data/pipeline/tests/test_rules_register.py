"""T6.1: register and rule files must agree (synthetic fixture, Latin letters only)."""

import shutil
from pathlib import Path

from pipeline import rules_register

FIXTURE = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "synthetic"


def _copy(tmp_path: Path) -> Path:
    root = tmp_path / "root"
    shutil.copytree(FIXTURE, root)
    return root


def test_fixture_and_real_data_agree() -> None:
    assert rules_register.check(FIXTURE) == []
    assert rules_register.check() == []


def _edit(root: Path, name: str, old: str, new: str) -> None:
    p = root / "rules" / name
    text = p.read_text(encoding="utf-8")
    assert old in text
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


def test_disagreements_fail(tmp_path: Path) -> None:
    cases = [
        ("register.yaml", 'id: "r3.mark"', 'id: "r3.other"', "has no register entry"),
        ("register.yaml", 'option: "test.toggle"', "option: null", "option differs"),
        (
            "r3.yaml",
            'id: "r3"}\n    pattern:\n      - {base: ["U+0063"]',
            'id: "q"}\n    pattern:\n      - {base: ["U+0063"]',
            "owner differs",
        ),
        (
            "register.yaml",
            'source: {book: "synthetic", page: "1"}',
            "source: null",
            "without a source",
        ),
        ("register.yaml", '"rule-derived"', '"muqri-verified"', "has status"),
    ]
    for n, (name, old, new, want) in enumerate(cases):
        root = _copy(tmp_path / str(n))
        _edit(root, name, old, new)
        problems = rules_register.check(root)
        assert any(want in p for p in problems), (want, problems)
