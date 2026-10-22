"""T4.3: align a riwayah text on the Hafs spine, word by word, per surah (D20).

    python -m pipeline.align write     # docs/review/phase4/alignment-<riwayah>.md for all 7

Tokens are items: words keyed by their letter skeleton (`normalize.skeleton`), ayah ends and
hizb signs by kind only, so ayah ends act as anchors. difflib's matcher (deterministic,
`autojunk=False`) finds the equal runs. Inside a replace block, items of the same length and
kinds pair by position; otherwise a small dynamic program pairs items of the same kind by
skeleton edit distance and leaves the rest unpaired (deleted Hafs items, inserted riwayah
items). Items never pair across kinds or surahs.

The alignment only decides which patches describe a difference; exactness is proved by the
round-trip (T4.6), so a poor pairing can make patches less readable but never wrong.
"""

import difflib
import sys
from collections import Counter
from collections.abc import Callable
from dataclasses import dataclass
from itertools import groupby
from pathlib import Path

from pipeline import ingest_riwayah, kfgqpc
from pipeline.ingest_riwayah import Item
from pipeline.normalize import skeleton

REPORT_DIR = Path(__file__).resolve().parents[2] / "docs" / "review" / "phase4"
GAP = 0.6  # cost of leaving one item unpaired; a pair costs its normalized edit distance


@dataclass(frozen=True)
class Pair:
    h: Item | None
    r: Item | None
    how: str  # equal | position | fuzzy | unpaired


def default_key(item: Item) -> str:
    return skeleton(item.text) if item.kind == "word" else item.kind


def _distance(a: str, b: str) -> float:
    if a == b:
        return 0.0
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, start=1):
        cur = [i]
        for j, cb in enumerate(b, start=1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1] / max(len(a), len(b), 1)


def _pair(h: Item, r: Item, kh: str, kr: str, how: str) -> Pair:
    return Pair(h, r, "equal" if kh == kr else how)


def _dp(h: list[Item], r: list[Item], key: Callable[[Item], str]) -> list[Pair]:
    """Minimum-cost global alignment; ties prefer pair, then delete, then insert."""
    kh, kr = [key(i) for i in h], [key(i) for i in r]
    n, m = len(h), len(r)
    inf = float("inf")
    cost = [[0.0] * (m + 1) for _ in range(n + 1)]
    move = [[""] * (m + 1) for _ in range(n + 1)]
    for i in range(n + 1):
        for j in range(m + 1):
            if i == j == 0:
                continue
            options = []
            if i and j:
                same = h[i - 1].kind == r[j - 1].kind
                c = _distance(kh[i - 1], kr[j - 1]) if same else inf
                options.append((cost[i - 1][j - 1] + c, 0, "pair"))
            if i:
                options.append((cost[i - 1][j] + GAP, 1, "del"))
            if j:
                options.append((cost[i][j - 1] + GAP, 2, "ins"))
            best = min(options)
            cost[i][j], move[i][j] = best[0], best[2]
    out: list[Pair] = []
    i, j = n, m
    while i or j:
        mv = move[i][j]
        if mv == "pair":
            out.append(_pair(h[i - 1], r[j - 1], kh[i - 1], kr[j - 1], "fuzzy"))
            i, j = i - 1, j - 1
        elif mv == "del":
            out.append(Pair(h[i - 1], None, "unpaired"))
            i -= 1
        else:
            out.append(Pair(None, r[j - 1], "unpaired"))
            j -= 1
    return out[::-1]


def align_surah(
    h: list[Item], r: list[Item], key: Callable[[Item], str] = default_key
) -> list[Pair]:
    kh, kr = [key(i) for i in h], [key(i) for i in r]
    matcher = difflib.SequenceMatcher(None, kh, kr, autojunk=False)
    out: list[Pair] = []
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        hs, rs = h[i1:i2], r[j1:j2]
        if tag == "equal":
            out += [Pair(a, b, "equal") for a, b in zip(hs, rs, strict=True)]
        elif len(hs) == len(rs) and [a.kind for a in hs] == [b.kind for b in rs]:
            out += [_pair(a, b, key(a), key(b), "position") for a, b in zip(hs, rs, strict=True)]
        else:
            out += _dp(hs, rs, key)
    return out


def align_text(h: list[Item], r: list[Item]) -> list[Pair]:
    """Both texts must have the same surahs; pairs are in reading order."""
    hs = {s: list(g) for s, g in groupby(h, key=lambda i: i.surah)}
    rs = {s: list(g) for s, g in groupby(r, key=lambda i: i.surah)}
    if list(hs) != list(rs):
        raise ValueError("the two texts do not have the same surahs in the same order")
    return [p for s in hs for p in align_surah(hs[s], rs[s])]


def _blocks(pairs: list[Pair]) -> list[list[Pair]]:
    """Maximal runs of pairs that are not `equal`."""
    out: list[list[Pair]] = []
    run: list[Pair] = []
    for p in pairs:
        if p.how == "equal":
            if run:
                out.append(run)
                run = []
        else:
            run.append(p)
    if run:
        out.append(run)
    return out


def block_kind(block: list[Pair]) -> str:
    kinds = {i.kind for p in block for i in (p.h, p.r) if i}
    return "word" if "word" in kinds else "+".join(sorted(kinds))


def _cell(item: Item | None) -> str:
    if item is None:
        return "-"
    shown = item.text if item.kind == "word" else item.kind
    return f"{item.ref} {shown}"


def report(riwayah: str, pairs: list[Pair]) -> str:
    counts = Counter(p.how for p in pairs)
    unpaired_h = sum(1 for p in pairs if p.how == "unpaired" and p.h)
    unpaired_r = sum(1 for p in pairs if p.how == "unpaired" and p.r)
    blocks = _blocks(pairs)
    by_kind = Counter(block_kind(b) for b in blocks)
    lines = [
        f"# Alignment report: {riwayah} on the Hafs spine (T4.3)",
        "",
        "Generated by `python -m pipeline.align write` from the pinned KFGQPC texts. Words are"
        " compared by letter skeleton (`normalize.skeleton`); ayah ends and hizb signs by kind."
        " Every segment that is not an exact skeleton match is listed below; nothing is"
        " dropped. Text shown is copied from the sources, never typed.",
        "",
        f"- equal pairs: {counts['equal']}",
        f"- paired by position (same block length, different skeleton): {counts['position']}",
        f"- paired by edit distance (fuzzy): {counts['fuzzy']}",
        f"- unpaired Hafs items (deleted): {unpaired_h}",
        f"- unpaired {riwayah} items (inserted): {unpaired_r}",
        f"- segments: {len(blocks)} ("
        + ", ".join(f"{k}: {v}" for k, v in sorted(by_kind.items()))
        + ")",
        "",
        "| # | Kind | Hafs | " + riwayah + " | How |",
        "|---|---|---|---|---|",
    ]
    for n, block in enumerate(blocks, start=1):
        kind = block_kind(block)
        for p in block:
            lines.append(f"| {n} | {kind} | {_cell(p.h)} | {_cell(p.r)} | {p.how} |")
    return "\n".join(lines) + "\n"


def report_path(riwayah: str) -> Path:
    return REPORT_DIR / f"alignment-{riwayah}.md"


def build_reports() -> dict[Path, str]:
    hafs = ingest_riwayah.load("hafs").items
    out = {}
    for r in kfgqpc.RIWAYAT[1:]:
        out[report_path(r)] = report(r, align_text(hafs, ingest_riwayah.load(r).items))
    return out


def main(argv: list[str]) -> int:
    reports = build_reports()
    if argv[1:] == ["write"]:
        REPORT_DIR.mkdir(parents=True, exist_ok=True)
        for path, text in reports.items():
            path.write_text(text, encoding="utf-8")
        print(f"alignment reports written to {REPORT_DIR}")
        return 0
    stale = [p.name for p, t in reports.items() if not p.exists() or p.read_text("utf-8") != t]
    print("alignment reports: " + (f"stale {stale}" if stale else "up to date"))
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
