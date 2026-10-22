"""T3.3: surah names for the surah-header lines, from KFGQPC's own data (D16).

KFGQPC v2.0 Hafs (the spine's source, 1441H) gives `sura_name_ar` on every ayah row. A surah's
rows must all carry the same name; it is written to data/render/surah-names.json as canonical
`U+XXXX` codepoints. Header names are metadata, not spine text: they are shaped as stored,
without the spine's encoding table, and the shaping-integrity test checks them like any line.

    python -m pipeline.surah_names build | check
"""

import json
import sys
from pathlib import Path

from pipeline import ingest_kfgqpc, sources, spine

OUT = Path(__file__).resolve().parents[1] / "render" / "surah-names.json"


def build() -> dict:
    names: dict[int, set[str]] = {}
    for row in ingest_kfgqpc.load_rows():
        names.setdefault(int(row["sura_no"]), set()).add(row["sura_name_ar"])
    if sorted(names) != list(range(1, 115)):
        raise ValueError("surah numbers are not 1..114")
    bad = {s: v for s, v in names.items() if len(v) != 1}
    if bad:
        raise ValueError(f"surahs with more than one name: {sorted(bad)}")
    manifest = {s.id: s.sha256 for s in sources.load_manifest(sources.MANIFEST_PATH).sources}
    return {
        "source": {
            "id": ingest_kfgqpc.SOURCE_ID,
            "member": ingest_kfgqpc.SOURCE_MEMBER,
            "field": "sura_name_ar",
            "sha256": manifest[ingest_kfgqpc.SOURCE_ID],
        },
        "names": [
            {"surah": s, "name": spine.to_cps(next(iter(v)))} for s, v in sorted(names.items())
        ],
    }


def text(data: dict) -> str:
    rows = data.pop("names")
    head = json.dumps(data, ensure_ascii=True, indent=1)[:-2]
    body = ",\n".join("  " + json.dumps(r) for r in rows)
    return f'{head},\n "names": [\n{body}\n ]\n}}\n'


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd not in ("build", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    out = text(build())
    if cmd == "check":
        ok = OUT.exists() and OUT.read_text(encoding="ascii") == out
        if not ok:
            print(f"surah_names: stale {OUT}", file=sys.stderr)
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(out, encoding="ascii")
    print(f"surah_names: 114 names -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
