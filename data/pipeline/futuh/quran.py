"""Reference Quran texts (Hafs spine + 7 other KFGQPC riwayat) and alignment of book text.

Reference text is only ever read from mushaflab's pinned sources; nothing Quranic is typed here.
The book's own Quranic text is never "corrected" from these: they are used to locate (surah,
ayah, word) and to report whether a printed word is attested in a published riwayah.
"""

from __future__ import annotations

import json
import re
import unicodedata
import zipfile
from dataclasses import dataclass, field
from functools import lru_cache

import yaml

from .paths import CACHE as CACHE_DIR
from .paths import DATA as MUSHAFLAB

SPINE = MUSHAFLAB / "spine" / "hafs" / "v1" / "items.jsonl"
OFFICIAL = MUSHAFLAB / "sources" / "raw" / "kfgqpc" / "official"
AYAH_MAPS = MUSHAFLAB / "ayah_maps"

RIWAYAT_ZIPS = {
    "shuba": "UthmanicShuba_v2-0.zip",
    "warsh": "UthmanicWarsh_v2-1.zip",
    "qalun": "UthmanicQaloun_v2-1.zip",
    "qunbul": "UthmanicQunbul_v2-0.zip",
    "duri_abuamr": "UthmanicDouri_v2-0.zip",
    "susi": "UthmanicSousi_v2-0.zip",
}
# bazzi is published by KFGQPC only through the mirror (no official zip pinned in mushaflab)
BAZZI_MIRROR = MUSHAFLAB / "sources" / "raw" / "kfgqpc" / "mirror" / "bazzi" / "BazziData_v07.json"


# ---------------------------------------------------------------------------------------------
# Comparison keys

_MARKS_DROP = set(
    "\u0640"  # tatweel
    "\u06d6\u06d7\u06d8\u06d9\u06da\u06db\u06dc\u06dd\u06de\u06e9"  # pause signs, ayah/rub' signs
    "\u06e0\u06e4\u06ed"  # small zero, small high madda, small low meem (encoding conventions)
    "\u200f\u200e\u00a0"
    "\u2022"  # the book's bullet for a hamza place (page.TASHIL_DOT)
)
_MERGE = {
    "\u06e1": "\u0652",  # dotless head of khah (Hijazi sukun) -> sukun
    "\u0671": "\u0627",  # alef wasla -> alef
    "\u06d2": "\u064a",  # yeh barree -> yeh
    "\u08f0": "\u064b",  # open tanwin forms -> tanwin
    "\u08f1": "\u064c",
    "\u08f2": "\u064d",
    "\u0657": "\u064c",  # KFGQPC open-dammatan codepoint (see mushaflab known-gaps G-018)
    "\u065e": "\u064b",
    "\u0656": "\u064d",
}
LETTER_NORMALIZE = {"\u0649": "\u064a"}  # used only for the skeleton


def is_mark(c: str) -> bool:
    return unicodedata.category(c) in ("Mn", "Me")


def clusters(word: str) -> list[str]:
    """Base letter + following marks."""
    out: list[str] = []
    for c in word:
        if is_mark(c) and out:
            out[-1] += c
        else:
            out.append(c)
    return out


def strict_key(word: str) -> str:
    """Exact code points, marks sorted within each letter (Unicode gives them no order)."""
    return "".join(cl[0] + "".join(sorted(cl[1:])) for cl in clusters(word))


def loose_key(word: str) -> str:
    """Encoding-convention-insensitive key (mirrors the spirit of mushaflab's `loose` form)."""
    w = "".join(_MERGE.get(c, c) for c in word if c not in _MARKS_DROP)
    w = w.replace("\u0653", "")  # maddah: madd length is delivery, not text
    w = w.replace("\u200d", "")  # ZWJ, like tatweel (dropped above), only seats a hamza (بِـَٔايَٰتِ)
    w = w.replace("\u0649", "\u064a")  # final ya drawn dotless: encoding convention
    w = w.replace("\u06e5", "").replace("\u06e6", "")  # small waw/yeh (sila signs vary by encoding)
    out = []
    for cl in clusters(w):
        base, ms = cl[0], cl[1:]
        if base == "\u0627":
            ms = "".join(m for m in ms if m not in "\u064e\u064f\u0650\u06ec\u065c\u0670")
        out.append(base + "".join(sorted(ms)))
    return "".join(out)


@lru_cache(maxsize=200_000)
def skeleton(word: str) -> str:
    w = "".join(_MERGE.get(c, c) for c in word)
    w = "".join(LETTER_NORMALIZE.get(c, c) for c in w if not is_mark(c) and c not in _MARKS_DROP)
    for a in "\u0623\u0625\u0622\u0671":
        w = w.replace(a, "\u0627")
    return w.replace("\u0670", "")


def lev(a: str, b: str) -> int:
    if a == b:
        return 0
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


@lru_cache(maxsize=500_000)
def sim(a: str, b: str) -> float:
    sa, sb = skeleton(a), skeleton(b)
    if not sa and not sb:
        return 1.0
    return 1.0 - lev(sa, sb) / max(len(sa), len(sb))


# ---------------------------------------------------------------------------------------------
# Reference data


@dataclass
class Word:
    id: str  # spine word id "s:a:w"
    text: str
    clusters: list[tuple[str, str]] = field(default_factory=list)  # (cluster id, text)


class Canon:
    def __init__(self) -> None:
        self.ayahs: dict[tuple[int, int], list[Word]] = {}
        with SPINE.open() as f:
            for line in f:
                it = json.loads(line)
                if it["kind"] != "word":
                    continue
                s, a, _w = (int(x) for x in it["id"].split(":"))
                cls = [
                    (c["id"], "".join(chr(int(p[2:], 16)) for p in c["cps"].split()))
                    for c in it["clusters"]
                ]
                self.ayahs.setdefault((s, a), []).append(
                    Word(it["id"], "".join(t for _, t in cls), cls)
                )
        self.n_ayahs = {}
        for s, a in self.ayahs:
            self.n_ayahs[s] = max(self.n_ayahs.get(s, 0), a)
        self._riwayat: dict[str, dict[str, list[str]]] | None = None

    @property
    def riwayat(self) -> dict[str, dict[str, list[str]]]:
        """riwayah -> spine word id -> words; loaded on first use (needs the raw KFGQPC zips)."""
        if self._riwayat is None:
            cache = CACHE_DIR / "riwayat.json"
            if cache.exists():
                self._riwayat = json.loads(cache.read_text())
            else:
                self._riwayat = {r: self._load_riwayah(r) for r in [*RIWAYAT_ZIPS, "bazzi"]}
                CACHE_DIR.mkdir(parents=True, exist_ok=True)
                cache.write_text(json.dumps(self._riwayat, ensure_ascii=False))
        return self._riwayat

    def _rows(self, r: str) -> list[dict]:
        if r == "bazzi":
            return json.loads(BAZZI_MIRROR.read_text())
        z = zipfile.ZipFile(OFFICIAL / RIWAYAT_ZIPS[r])
        name = next(n for n in z.namelist() if n.endswith(".json"))
        return json.loads(z.read(name))

    def _load_riwayah(self, r: str) -> dict[str, list[str]]:
        """Map each Hafs spine word id to the riwayah word(s) at that place.

        Uses mushaflab's ayah map (riwayah ayah -> spine word range). Inside a range the words
        are paired by skeleton alignment, so an extra or missing word does not shift the rest."""
        rows = self._rows(r)
        text = {}
        for row in rows:
            s = int(row.get("sura_no") or row.get("sora"))
            a = int(row["aya_no"])
            words = [w for w in row["aya_text"].split() if not _is_ayah_sign(w)]
            text[(s, a)] = words
        amap = yaml.safe_load((AYAH_MAPS / f"{r}.yaml").read_text())["entries"]
        out: dict[str, list[str]] = {}
        for e in amap:
            rw = text.get((e["surah"], e["ayah"]), [])
            spine_ids = self._spine_range(e["first"], e["last"])
            sw = [(i, self.word(i).text) for i in spine_ids]
            for i, w in _pair(sw, rw):
                out.setdefault(i, []).append(w)
        return out

    def _spine_range(self, first: str, last: str) -> list[str]:
        import bisect

        if not hasattr(self, "_order"):
            self._order = sorted(
                (tuple(int(x) for x in w.id.split(":")), w.id)
                for ws in self.ayahs.values()
                for w in ws
            )
            self._keys = [k for k, _ in self._order]

        def base(x: str) -> tuple[int, int, int]:
            s, a, w = x.split("+", maxsplit=1)[0].split(":")
            return int(s), int(a), max(int(w), 1)

        lo = bisect.bisect_left(self._keys, base(first))
        hi = bisect.bisect_right(self._keys, base(last))
        return [i for _, i in self._order[lo:hi]]

    def word(self, wid: str) -> Word:
        s, a, w = (int(x) for x in wid.split(":"))
        return self.ayahs[(s, a)][w - 1]


def _is_ayah_sign(w: str) -> bool:
    return all(
        unicodedata.category(c) in ("Nd", "No")
        or (0xFC00 <= ord(c) <= 0xFFFF and unicodedata.category(c) in ("Co", "Cn", "So"))
        or c == "\u06dd"
        for c in w
    ) or bool(re.fullmatch("[\ue000-\uf8ff\U000f0000-\U000fffff]+", w))


def _pair(sw: list[tuple[str, str]], rw: list[str]) -> list[tuple[str, str]]:
    """Pair spine words with riwayah words by skeleton similarity (monotone alignment)."""
    if len(sw) == len(rw):
        return [(i, w) for (i, _), w in zip(sw, rw, strict=False)]
    n, m = len(sw), len(rw)
    inf = 1e9
    dp = [[inf] * (m + 1) for _ in range(n + 1)]
    bt = [[None] * (m + 1) for _ in range(n + 1)]
    dp[0][0] = 0
    for i in range(n + 1):
        for j in range(m + 1):
            if dp[i][j] >= inf:
                continue
            if i < n and j < m:
                c = dp[i][j] + (1 - sim(sw[i][1], rw[j]))
                if c < dp[i + 1][j + 1]:
                    dp[i + 1][j + 1], bt[i + 1][j + 1] = c, (i, j, "m")
            if i < n and dp[i][j] + 0.8 < dp[i + 1][j]:
                dp[i + 1][j], bt[i + 1][j] = dp[i][j] + 0.8, (i, j, "s")
            if j < m and dp[i][j] + 0.8 < dp[i][j + 1]:
                dp[i][j + 1], bt[i][j + 1] = dp[i][j] + 0.8, (i, j, "r")
    out = []
    i, j = n, m
    while (i, j) != (0, 0):
        pi, pj, op = bt[i][j]
        if op == "m":
            out.append((sw[pi][0], rw[pj]))
        i, j = pi, pj
    return out[::-1]


@lru_cache(maxsize=1)
def canon() -> Canon:
    return Canon()


# ---------------------------------------------------------------------------------------------
# Aligning a run of book words to an ayah


def align_words(
    book: list[str],
    ref: list[str],
    anchored_end: bool,
    gap_ref: float = 0.15,
    gap_book: float = 1.2,
) -> tuple[float, list[tuple[int, int | None]]]:
    """Semi-global alignment: every book word is placed; reference words may be skipped
    (elisions "..." and partial quotations), cheaply at either end and a bit dearer inside.

    Returns (cost, [(book index, ref index or None)]). A reference index -(j + 1) marks the
    second of two book words that together make reference word j."""
    n, m = len(book), len(ref)
    inf = 1e18
    dp = [[inf] * (m + 1) for _ in range(n + 1)]
    bt: list[list] = [[None] * (m + 1) for _ in range(n + 1)]
    for j in range(m + 1):
        dp[0][j] = 0.0  # free leading reference gap
    for i in range(n + 1):
        for j in range(m + 1):
            c0 = dp[i][j]
            if c0 >= inf:
                continue
            if i < n and j < m:
                c = c0 + (1 - sim(book[i], ref[j])) * 2
                if c < dp[i + 1][j + 1]:
                    dp[i + 1][j + 1], bt[i + 1][j + 1] = c, (i, j, "m")
            if i + 1 < n and j < m:
                # two book words for one reference word (the book prints a space inside it,
                # e.g. أَوَ لَمۡ for أَوَلَمۡ, or a glyph quirk splits a word); small extra cost
                c = c0 + (1 - sim(book[i] + book[i + 1], ref[j])) * 2 + 0.3
                if c < dp[i + 2][j + 1]:
                    dp[i + 2][j + 1], bt[i + 2][j + 1] = c, (i, j, "m2")
            if i < n:
                c = c0 + gap_book
                if c < dp[i + 1][j]:
                    dp[i + 1][j], bt[i + 1][j] = c, (i, j, "b")
            if j < m and i > 0 and i < n:
                c = c0 + gap_ref
                if c < dp[i][j + 1]:
                    dp[i][j + 1], bt[i][j + 1] = c, (i, j, "r")
    # trailing reference gap: free for a partial quotation, charged when the book run ends at
    # an ayah-end marker (the ayah must end there)
    tail = 0.6 if anchored_end else 0.0
    jbest = min(range(m + 1), key=lambda j: dp[n][j] + (m - j) * tail)
    cost = dp[n][jbest] + (m - jbest) * tail
    pairs = []
    i, j = n, jbest
    while i > 0:
        pi, pj, op = bt[i][j]
        if op == "m":
            pairs.append((pi, pj))
        elif op == "m2":
            pairs.append((pi + 1, -(pj + 1)))  # second half: negative = "continues ref word pj"
            pairs.append((pi, pj))
        elif op == "b":
            pairs.append((pi, None))
        i, j = pi, pj
    return cost, pairs[::-1]
