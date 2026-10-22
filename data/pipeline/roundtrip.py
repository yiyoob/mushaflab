"""T4.6 round-trip (D22): compare the Rust resolver's output with the pinned KFGQPC texts.

    python -m pipeline.roundtrip DIR [RIWAYAH...]   # DIR holds <riwayah>.jsonl (mushaf-roundtrip)

For each riwayah:
  1. the whole resolved text equals the source's `aya_text`s concatenated, codepoint for
     codepoint;
  2. splitting the resolved items after each ayah-end item gives the source's ayahs one by
     one, in order, each exactly equal;
  3. the tracked ayah map (T4.2) names, for every ayah in order, its first and last
     resolved item.
Prints the share of exact ayahs and the first problems; exit 1 unless everything is exact.
"""

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

from pipeline import kfgqpc
from pipeline.compile_db import load_yaml
from pipeline.spine import from_cps

AYAH_MAP_DIR = Path(__file__).resolve().parents[1] / "ayah_maps"
SHOWN = 10


@dataclass(frozen=True)
class RItem:
    id: str
    kind: str
    text: str


@dataclass
class Result:
    riwayah: str
    ayahs: int = 0
    exact: int = 0
    problems: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.problems and self.exact == self.ayahs


def load_resolved(path: Path) -> list[RItem]:
    out = []
    for line in path.read_text(encoding="ascii").splitlines():
        d = json.loads(line)
        out.append(RItem(d["id"], d["kind"], from_cps(d["text"])))
    return out


def split_ayahs(items: list[RItem]) -> list[list[RItem]]:
    """Ayahs as runs of items, each ending at an ayah-end item."""
    out: list[list[RItem]] = [[]]
    for i in items:
        out[-1].append(i)
        if i.kind == "ayah_end":
            out.append([])
    if not out[-1]:
        out.pop()
    return out


def _first_diff(a: str, b: str) -> str:
    n = next((k for k, (x, y) in enumerate(zip(a, b, strict=False)) if x != y), min(len(a), len(b)))

    def show(s: str) -> str:
        return " ".join(f"U+{ord(c):04X}" for c in s[max(0, n - 2) : n + 3])

    return f"first difference at codepoint {n}: resolved [{show(a)}] source [{show(b)}]"


def check(riwayah: str, items: list[RItem], rows: list[dict], amap: dict | None) -> Result:
    res = Result(riwayah, ayahs=len(rows))
    whole = "".join(i.text for i in items)
    source = "".join(r["aya_text"] for r in rows)
    if whole != source:
        res.problems.append(f"whole text differs: {_first_diff(whole, source)}")
    groups = split_ayahs(items)
    if len(groups) != len(rows):
        res.problems.append(f"{len(groups)} resolved ayahs, source has {len(rows)}")
    for g, row in zip(groups, rows, strict=False):
        text = "".join(i.text for i in g)
        if text == row["aya_text"]:
            res.exact += 1
        elif len(res.problems) < SHOWN:
            where = f"{row['sura_no']}:{row['aya_no']}"
            res.problems.append(f"ayah {where}: {_first_diff(text, row['aya_text'])}")
    if amap is not None:
        res.problems += _check_map(amap, groups, rows)
    return res


def _check_map(amap: dict, groups: list[list[RItem]], rows: list[dict]) -> list[str]:
    entries = amap["entries"]
    want = [(int(r["sura_no"]), int(r["aya_no"])) for r in rows]
    got = [(e["surah"], e["ayah"]) for e in entries]
    if got != want:
        return ["ayah map entries do not list the source's ayahs in order"]
    bad = [
        f"ayah map {e['surah']}:{e['ayah']}: {e['first']}..{e['last']} is not {g[0].id}..{g[-1].id}"
        for e, g in zip(entries, groups, strict=False)
        if (e["first"], e["last"]) != (g[0].id, g[-1].id)
    ]
    return bad[:SHOWN]


def main(argv: list[str]) -> int:
    if not argv[1:]:
        print(__doc__, file=sys.stderr)
        return 2
    folder = Path(argv[1])
    riwayat = argv[2:] or list(kfgqpc.RIWAYAT[1:])
    ok = True
    print("| Riwayah | Ayahs | Exact | % |")
    print("|---|---|---|---|")
    results = []
    for r in riwayat:
        amap = load_yaml(AYAH_MAP_DIR / f"{r}.yaml")
        res = check(r, load_resolved(folder / f"{r}.jsonl"), kfgqpc.rows(r), amap)  # type: ignore[arg-type]
        results.append(res)
        pct = 100 * res.exact / res.ayahs if res.ayahs else 0
        print(f"| {r} | {res.ayahs} | {res.exact} | {pct:.2f} |")
        ok &= res.ok
    for res in results:
        for p in res.problems:
            print(f"{res.riwayah}: {p}", file=sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
