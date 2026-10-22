"""Rewrite non-ASCII characters in Python source files as escapes.

    uv run python devtools/escape_non_ascii.py FILE...

The editing tools used on this repo can turn written escape sequences into the literal
characters (including invisible ones such as U+00A0). Pipeline code must keep string
literals ASCII (pipeline/tests/test_source_hygiene.py); this restores that.
"""

import sys
from pathlib import Path

ASCII_END = 0x80
BMP_END = 0xFFFF


def escape(text: str) -> str:
    return "".join(
        c
        if ord(c) < ASCII_END
        else (f"\\u{ord(c):04x}" if ord(c) <= BMP_END else f"\\U{ord(c):08x}")
        for c in text
    )


def main(paths: list[str]) -> int:
    for p in map(Path, paths):
        text = p.read_text(encoding="utf-8")
        fixed = escape(text)
        if fixed != text:
            p.write_text(fixed, encoding="utf-8")
            print(f"escaped: {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
