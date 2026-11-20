"""Anchor the book's Quranic text to the Hafs spine and grade every word.

Grades (per book word):
  hafs-strict / hafs-loose        same as the Hafs spine word (exact code points / loose key)
  riwayah-strict / riwayah-loose  same as the word one of the 7 other KFGQPC riwayat print at
                                  that spine position (which ones are listed)
  unattested                      none of the 8 published texts has it (expected for the 12
                                  riwayat KFGQPC does not publish, and for decoding errors)
  unaligned                       no spine word could be paired with it
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .glyphs import glyph_text
from .page import Row, Token
from .quran import Canon, align_words, loose_key, strict_key

DEFAULT_COLORS = {"#231f20", "#000000"}


@dataclass
class BookWord:
    text: str
    colors: list[tuple[int, int, str]]  # (start, end) char offsets in text, colour hex
    spine: str | None = None
    sim: float = 0.0
    grade: str = "unaligned"
    riwayat: list[str] = field(default_factory=list)
    hafs: str | None = None  # the Hafs spine word (reference only, never substituted)
    gids: list[int] = field(default_factory=list)
    tok: Token | None = None  # source token (glyphs), kept for calibration
    bbox: tuple[float, float, float, float] | None = None  # PDF points (x0, y0, x1, y1)
    split: bool = False  # one of two book words that together make one spine word

    @property
    def colored(self) -> bool:
        return any(c not in DEFAULT_COLORS for _, _, c in self.colors)


def book_word(tok: Token) -> BookWord:
    text, runs = "", []
    for g in tok.glyphs:
        t = glyph_text(g)
        s = len(text)
        text += t
        if runs and runs[-1][2] == g.color and runs[-1][1] == s:
            runs[-1] = (runs[-1][0], len(text), g.color)
        else:
            runs.append((s, len(text), g.color))
        # a mark may carry its own colour (e.g. a red fatha on a black letter)
        for m in g.marks:
            if m.color != g.color:
                runs.append((s, len(text), m.color))
    gl = tok.glyphs
    bbox = None
    if gl:
        size = max(g.size for g in gl)
        bbox = (
            round(min(g.x0 for g in gl), 1),
            round(min(g.y for g in gl) - size * 1.05, 1),
            round(max(g.x1 for g in gl), 1),
            round(max(g.y for g in gl) + size * 0.5, 1),
        )
    return BookWord(text, runs, gids=[g.gid for g in gl], tok=tok, bbox=bbox)


@dataclass
class Chunk:
    words: list[BookWord]
    marker: int | None  # ayah-end number closing this chunk, if any
    surah: int | None = None
    ayah: int | None = None
    cost: float = 0.0


def chunks_of(tokens: list[Token]) -> list[Chunk]:
    out: list[Chunk] = []
    cur: list[BookWord] = []
    for t in tokens:
        if t.kind == "quran":
            cur.append(book_word(t))
        elif t.kind == "ayah":
            out.append(Chunk(cur, t.num))
            cur = []
    if cur:
        out.append(Chunk(cur, None))
    return out


class Anchorer:
    def __init__(self, canon: Canon, surah: int | None = None) -> None:
        self.c = canon
        self.surah = surah
        self.ayah = 0
        self.pending: list[Chunk] = []  # chunks seen before any context; anchored later

    def _score(self, ch: Chunk, s: int, a: int) -> tuple[float, list]:
        ref = self.c.ayahs.get((s, a))
        if not ref:
            return 1e9, []
        cost, pairs = align_words(
            [w.text for w in ch.words], [w.text for w in ref], anchored_end=ch.marker is not None
        )
        return cost / max(len(ch.words), 1), pairs

    def anchor(self, ch: Chunk) -> None:
        if not ch.words:
            if ch.marker:
                self.ayah = ch.marker
            return
        cands: list[tuple[int, int]] = []
        if ch.marker is not None:
            if self.surah is not None:
                # previous surah too: surah-junction tables step back (e.g. 1:7 before 2:1)
                cands = [
                    (s, ch.marker)
                    for s in (self.surah - 1, self.surah, self.surah + 1, self.surah + 2)
                ]
        elif self.surah is not None:
            cands = [(self.surah, a) for a in range(max(1, self.ayah - 2), self.ayah + 8)]
            cands += [(self.surah + 1, a) for a in (1, 2, 3)]
        best = min(
            ((self._score(ch, s, a), s, a) for s, a in cands), default=None, key=lambda x: x[0][0]
        )
        if best is None or best[0][0] > 0.9:
            # lost: search everywhere (start of a volume, or an unexpected jump)
            if ch.marker is not None:
                pool = [(s, ch.marker) for s in range(1, 115) if (s, ch.marker) in self.c.ayahs]
            elif self.surah is not None:
                pool = [(self.surah, a) for a in range(1, self.c.n_ayahs[self.surah] + 1)]
            else:
                self.pending.append(ch)  # no marker, no context: retry once context exists
                return
            if not pool:
                return  # e.g. a marker number no surah has: leave unaligned, reported
            g = min(((self._score(ch, s, a), s, a) for s, a in pool), key=lambda x: x[0][0])
            if best is None or g[0][0] < best[0][0]:
                best = g
        (cost, pairs), s, a = best
        if cost > 1.2:
            return  # leave unaligned; reported
        self._apply(ch, s, a, cost, pairs)
        self.surah, self.ayah = s, a
        if self.pending:
            pend, self.pending = self.pending, []
            for p in pend:
                cand = [(s, x) for x in range(max(1, a - 8), a + 1)]
                (pc, pp), ps, pa = min(
                    ((self._score(p, s2, a2), s2, a2) for s2, a2 in cand), key=lambda x: x[0][0]
                )
                if pc <= 0.9:
                    self._apply(p, ps, pa, pc, pp)

    def _apply(
        self, ch: Chunk, s: int, a: int, cost: float, pairs: list[tuple[int, int | None]]
    ) -> None:
        ch.surah, ch.ayah, ch.cost = s, a, cost
        ref = self.c.ayahs[(s, a)]
        prev_pair: dict[int, BookWord] = {}
        for bi, ri in pairs:
            w = ch.words[bi]
            if ri is None:
                continue
            if ri < 0:  # second half of a word the book prints in two parts
                first = prev_pair[-ri - 1]
                joined = BookWord(first.text + w.text, [], tok=None)
                joined.spine, joined.hafs = first.spine, first.hafs
                grade(joined, self.c)
                for part in (first, w):
                    part.spine, part.hafs = first.spine, first.hafs
                    part.grade, part.riwayat, part.sim = joined.grade, joined.riwayat, joined.sim
                    part.split = True
                continue
            sw = ref[ri]
            w.spine, w.hafs = sw.id, sw.text
            grade(w, self.c)
            prev_pair[ri] = w


def grade(w: BookWord, c: Canon) -> None:
    from .quran import sim

    w.sim = round(sim(w.text, w.hafs), 3)
    if strict_key(w.text) == strict_key(w.hafs):
        w.grade = "hafs-strict"
        return
    if loose_key(w.text) == loose_key(w.hafs):
        w.grade = "hafs-loose"
        return
    hits_s = [
        r
        for r, m in c.riwayat.items()
        if any(strict_key(x) == strict_key(w.text) for x in m.get(w.spine, []))
    ]
    if hits_s:
        w.grade, w.riwayat = "riwayah-strict", hits_s
        return
    hits_l = [
        r
        for r, m in c.riwayat.items()
        if any(loose_key(x) == loose_key(w.text) for x in m.get(w.spine, []))
    ]
    if hits_l:
        w.grade, w.riwayat = "riwayah-loose", hits_l
        return
    w.grade = "unattested"


def anchor_row(anc: Anchorer, row: Row) -> list[Chunk]:
    chs = chunks_of(row.reading)
    for ch in chs:
        anc.anchor(ch)
    return chs
