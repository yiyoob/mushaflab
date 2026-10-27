"""Locate the book's quoted words in the spine by letters (TB.4, D50).

The book quotes Quranic words almost without harakat and in ordinary (imla'i) spelling. Both
sides are reduced to a letter key with one function each, built on the same letter map:
  * every alif form is dropped (alif, alif wasla, and in the spine also a waw or ya that
    carries the dagger alif U+0670, which the spine writes for an alif);
  * hamza in any seat becomes one hamza letter; ya and alif maqsura merge; ta marbuta
    becomes ha; tatweel and the small waw/ya/high ya (U+06E5-U+06E7) are dropped.
A key is matched against one spine word or a run of consecutive words (spaces ignored).
Match tiers, tried in order and recorded: `exact`; `prefix` (a leading waw or fa on one
side only); `no-hamza` (keys equal once hamza is removed). A match below `exact` is marked
for review.

Farsh entries are aligned in book order: each entry's candidates lie in its surah, and a
dynamic program finds the longest chain of entries with increasing spine positions. An entry
is `matched` when exactly one of its candidates lies on some longest chain, `ambiguous` when
several do, and `unmatched` when none does (no candidate, or out of order). List entries
(al-mumal, al-mudgham) match within their rub' in any order; their expected number of places
is the entry's count (2 for "ma'an"), else 1, and any other number is `ambiguous`.
"""

from dataclasses import dataclass

from pipeline.budur.forms import Word, letters
from pipeline.codepoints import Row

HAMZA = "\u0621"
_ALIFS = frozenset("\u0627\u0671")
_HAMZAS = frozenset("\u0621\u0623\u0625\u0622\u0624\u0626")
_DROP = frozenset("\u0640\u06e5\u06e6\u06e7")  # tatweel, small waw, small ya, small high ya
_MAP = {"\u0649": "\u064a", "\u0629": "\u0647"}  # alif maqsura -> ya, ta marbuta -> ha
_HAMZA_MARKS = frozenset("\u0654\u0655")  # hamza above / below as a mark
_DAGGER = "\u0670"
_WAW, _YA, _MAQSURA, _FA = "\u0648", "\u064a", "\u0649", "\u0641"
_PREFIXES = (_WAW, _FA)


def _letter(c: str) -> str:
    if c in _ALIFS or c in _DROP:
        return ""
    if c in _HAMZAS:
        return HAMZA
    return _MAP.get(c, c)


def book_key(text: str) -> str:
    """Letters of a book quote (its few harakat and any punctuation are ignored)."""
    return "".join(_letter(c) for c in text if "\u0621" <= c <= "\u064a")


def spine_key(text: str, table: dict[int, Row]) -> str:
    out = []
    for base, marks in letters(text, table):
        if table[ord(base)].cls != "base":
            continue
        if base == _WAW and _DAGGER in marks:
            continue  # a waw written for an alif (e.g. in al-salah)
        if _HAMZA_MARKS & set(marks):
            out.append(HAMZA)  # a seat (alif, ya, waw, tatweel) carrying hamza is the hamza
            continue
        out.append(_letter(base))
    return "".join(out)


@dataclass(frozen=True)
class Candidate:
    spans: tuple[tuple[int, int], ...]  # (first, last) spine word positions of each place
    tier: str  # exact | prefix | no-hamza (the weakest tier of its places)

    @property
    def start(self) -> int:
        return self.spans[0][0]

    @property
    def end(self) -> int:
        return self.spans[-1][1]


TIERS = ("exact", "prefix", "no-hamza")


def _tier(book: str, spine: str) -> str | None:
    if book == spine:
        return "exact"
    for p in _PREFIXES:
        if p + book == spine or book == p + spine:
            return "prefix"
    if book and book.replace(HAMZA, "") == spine.replace(HAMZA, ""):
        return "no-hamza"
    return None


def _weakest(tiers: list[str]) -> str:
    return max(tiers, key=TIERS.index)


class Index:
    """Spine word keys and surah ranges, for candidate search."""

    def __init__(self, words: list[Word], table: dict[int, Row]) -> None:
        self.words = words
        self.keys = [spine_key(w.text, table) for w in words]
        self.surah_range: dict[int, tuple[int, int]] = {}
        for w in words:
            lo, hi = self.surah_range.get(w.surah, (w.pos, w.pos))
            self.surah_range[w.surah] = (min(lo, w.pos), max(hi, w.pos))

    def _at(self, parts: list[str], start: int, hi: int) -> tuple[int, str] | None:
        """Word by word (each word may differ by a prefix), else the joined letters."""
        n = len(parts)
        if start + n - 1 <= hi:
            tiers = [_tier(b, self.keys[start + i]) for i, b in enumerate(parts)]
            if all(tiers):
                return start + n - 1, _weakest(tiers)  # type: ignore[arg-type]
        key = "".join(parts)
        joined = ""
        for end in range(start, min(hi, start + n) + 1):
            joined += self.keys[end]
            if (tier := _tier(key, joined)) is not None:
                return end, tier
            if len(joined) > len(key) + 1:
                break
        return None

    def candidates(self, quote: str, lo: int, hi: int) -> list[Candidate]:
        """Places in [lo, hi] whose consecutive words have the quote's letters."""
        parts = [k for k in (book_key(w) for w in quote.split()) if k]
        if not parts:
            return []
        out = []
        for start in range(lo, hi + 1):
            if (hit := self._at(parts, start, hi)) is not None:
                out.append(Candidate(((start, hit[0]),), hit[1]))
        return out


def combine(cands: list[Candidate], count: int) -> list[Candidate]:
    """For an entry naming `count` places ("ma'an"): runs of `count` consecutive candidates."""
    return [
        Candidate(
            tuple(s for c in cands[i : i + count] for s in c.spans),
            _weakest([c.tier for c in cands[i : i + count]]),
        )
        for i in range(len(cands) - count + 1)
    ]


@dataclass
class Placement:
    status: str  # matched | ambiguous | unmatched
    places: list[Candidate]  # the chosen candidate; all chain candidates when ambiguous
    candidates: int  # how many candidates there were
    out_of_order: bool = False  # a single candidate that not every longest chain uses


class _MaxCountTree:
    """Fenwick tree over spine positions: prefix (longest chain, number of such chains)."""

    def __init__(self, n: int) -> None:
        self.n = n
        self.t = [(0, 1)] * (n + 1)

    @staticmethod
    def _join(a: tuple[int, int], b: tuple[int, int]) -> tuple[int, int]:
        if a[0] != b[0]:
            return a if a[0] > b[0] else b
        return a[0], a[1] + b[1]

    def update(self, i: int, v: tuple[int, int]) -> None:
        i += 1
        while i <= self.n:
            self.t[i] = self._join(self.t[i], v) if self.t[i][0] else v
            i += i & -i

    def query(self, i: int) -> tuple[int, int]:  # over positions 0..i-1
        out = (0, 1)
        while i > 0:
            if self.t[i][0]:
                out = self._join(out, self.t[i]) if out[0] else self.t[i]
            i -= i & -i
        return out


def _chains(entries: list[list[Candidate]], n: int) -> list[list[tuple[int, int]]]:
    """(longest chain ending at the candidate, number of such chains), in entry order."""
    tree = _MaxCountTree(n)
    out = []
    for cs in entries:
        got = []
        for c in cs:
            length, count = tree.query(c.start)
            got.append((length + 1, count))
        for c, v in zip(cs, got, strict=True):
            tree.update(c.end, v)
        out.append(got)
    return out


def align(entries: list[list[Candidate]], n_positions: int) -> list[Placement]:
    """Order-constrained placement of farsh entries (each a list of candidates).

    An entry with exactly one candidate is matched whatever the order, and marked
    `out_of_order` when not every longest chain uses it (when the book's order conflicts,
    every entry in the conflict is marked: the order alone cannot say which one moved). An
    entry with several candidates is matched only when every longest chain uses it, all
    with the same candidate.
    """
    fwd = _chains(entries, n_positions)
    flipped = [
        [Candidate(((n_positions - 1 - c.end, n_positions - 1 - c.start),), c.tier) for c in cs]
        for cs in reversed(entries)
    ]
    bwd = list(reversed(_chains(flipped, n_positions)))
    best, total = 0, 0
    for fs, bs in zip(fwd, bwd, strict=True):
        for (fl, _), (bl, _) in zip(fs, bs, strict=True):
            best = max(best, fl + bl - 1)
    for fs, bs in zip(fwd, bwd, strict=True):
        for (fl, fc), (bl, _) in zip(fs, bs, strict=True):
            if fl + bl - 1 == best and fl == best:
                total += fc  # chains end somewhere: count them once, at their last entry
    out = []
    for cs, fs, bs in zip(entries, fwd, bwd, strict=True):
        through = [
            (c, fc * bc)
            for c, (fl, fc), (bl, bc) in zip(cs, fs, bs, strict=True)
            if fl + bl - 1 == best
        ]
        if len(cs) == 1:
            used = through[0][1] if through else 0
            out.append(Placement("matched", cs, 1, out_of_order=used != total))
        elif len(through) == 1 and through[0][1] == total:
            out.append(Placement("matched", [through[0][0]], len(cs)))
        elif cs:
            out.append(Placement("ambiguous", [c for c, _ in through] or cs, len(cs)))
        else:
            out.append(Placement("unmatched", [], 0))
    return out


def place_list(cands: list[Candidate], count: int | None) -> Placement:
    """A list entry (al-mumal, al-mudgham) in its rub': exactly `count` (else 1) places."""
    expected = count or 1
    if len(cands) == expected:
        return Placement(
            "matched",
            [
                Candidate(
                    tuple(s for c in cands for s in c.spans), _weakest([c.tier for c in cands])
                )
            ],
            len(cands),
        )
    return Placement("ambiguous" if cands else "unmatched", cands, len(cands))
