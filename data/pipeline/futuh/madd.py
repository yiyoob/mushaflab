"""Which madds a row's text contains, read from the Hafs spine words it is aligned to.

In the KFGQPC Hafs text a madd letter that is lengthened carries U+0653 (maddah above):
  munfasil  the maddah is on the word's last letter and the next spine word starts with a hamza
  muttasil  a hamza follows the maddah letter inside the same word
  lazim     a shadda or sukun follows the maddah letter (or the word is a muqatta'at letter name)
Only the pinned spine text is read; nothing is typed in."""

from __future__ import annotations

from .quran import Canon, clusters, is_mark

MADDAH = "\u0653"
HAMZA_LETTERS = set("\u0621\u0623\u0625\u0624\u0626\u0622")
HAMZA_MARKS = set("\u0654\u0655")
SUKUN = set("\u0652\u06e1\u0651")


def _starts_with_hamza(word: str) -> bool:
    cl = [c for c in clusters(word) if not is_mark(c[0])]
    if not cl:
        return False
    first = cl[0]
    return first[0] in HAMZA_LETTERS or any(m in HAMZA_MARKS for m in first[1:])


def madd_types(c: Canon, spine_ids: list[str]) -> set[str]:
    out: set[str] = set()
    for wid in spine_ids:
        s, a, w = (int(x) for x in wid.split(":"))
        words = c.ayahs.get((s, a), [])
        if w < 1 or w > len(words):
            continue
        cl = [x for x in clusters(words[w - 1].text) if not is_mark(x[0])]
        for i, letter in enumerate(cl):
            if MADDAH not in letter:
                continue
            rest = cl[i + 1 :]
            if not rest:
                nxt = words[w] if w < len(words) else None
                if nxt is not None and _starts_with_hamza(nxt.text):
                    out.add("munfasil")
                continue
            n = rest[0]
            if n[0] in HAMZA_LETTERS or any(m in HAMZA_MARKS for m in n[1:]):
                out.add("muttasil")
            elif any(m in SUKUN for m in n[1:]) or any(m in SUKUN for m in letter[1:]):
                out.add("lazim")
    return out
