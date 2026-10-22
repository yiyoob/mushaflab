"""String literals in pipeline code must be ASCII, so no invisible or look-alike character
(e.g. U+00A0, U+0640, Arabic presentation forms) can hide in a comparison. Use \\u escapes.
Comments may contain Arabic (CLAUDE.md conventions)."""

import tokenize
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parents[2]
STRING_TOKENS = (tokenize.STRING, tokenize.FSTRING_MIDDLE)


def non_ascii_string_lines(path: Path) -> list[int]:
    with path.open("rb") as fh:
        return [
            tok.start[0]
            for tok in tokenize.tokenize(fh.readline)
            if tok.type in STRING_TOKENS and not tok.string.isascii()
        ]


def test_string_literals_are_ascii() -> None:
    files = sorted(p for p in DATA_DIR.rglob("*.py") if ".venv" not in p.parts)
    problems = [
        f"{p.relative_to(DATA_DIR)}:{line}" for p in files for line in non_ascii_string_lines(p)
    ]
    assert problems == []


def test_flags_literal_but_not_comment(tmp_path: Path) -> None:
    src = tmp_path / "sample.py"
    src.write_text('x = "a\u00a0b"\n# \u0627 in a comment is fine\ny = "\\u00a0"\n', "utf-8")
    assert non_ascii_string_lines(src) == [1]
