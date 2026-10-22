"""T1.2: write and read the Hafs spine files (D11).

    python -m pipeline.spine build   # (re)write data/spine/hafs/v1/
    python -m pipeline.spine check   # exit 1 if the committed spine differs from a rebuild

Codepoints are stored as "U+XXXX" strings so tracked data is ASCII only.
"""

import hashlib
import json
import sys
import unicodedata
from pathlib import Path

from pipeline import ingest_kfgqpc, sources
from pipeline.clusterize import clusterize
from pipeline.codepoints import TABLE_PATH, Row, load_table
from pipeline.ingest_kfgqpc import Item

SPINE_DIR = Path(__file__).resolve().parents[1] / "spine" / "hafs" / "v1"
ITEMS = SPINE_DIR / "items.jsonl"
MANIFEST = SPINE_DIR / "manifest.json"
SCRIPT_ID = "uthmani"
SPINE_VERSION = 1


def to_cps(text: str) -> str:
    return " ".join(f"U+{ord(c):04X}" for c in text)


def from_cps(cps: str) -> str:
    return "".join(chr(int(part[2:], 16)) for part in cps.split(" ") if part)


def records(items: list[Item], table: dict[int, Row]) -> list[dict]:
    out = []
    for item in items:
        rec: dict = {"id": item.id, "kind": item.kind}
        if item.sep_before:
            rec["sep_before"] = to_cps(item.sep_before)
        rec["sep_after"] = to_cps(item.sep_after)
        rec["clusters"] = [
            {"id": f"{item.id}:{n}", "kind": c.kind, "cps": to_cps(c.text)}
            for n, c in enumerate(clusterize(item.text, table), start=1)
        ]
        out.append(rec)
    return out


def rebuild_ayahs(recs: list[dict]) -> dict[tuple[int, int], str]:
    ayahs: dict[tuple[int, int], str] = {}
    for r in recs:
        surah, ayah, _ = (int(x) for x in r["id"].split(":"))
        text = from_cps(r.get("sep_before", ""))
        text += "".join(from_cps(c["cps"]) for c in r["clusters"])
        text += from_cps(r["sep_after"])
        ayahs[(surah, ayah)] = ayahs.get((surah, ayah), "") + text
    return ayahs


def source_rows() -> list[dict]:
    return ingest_kfgqpc.load_rows()


def build_records() -> list[dict]:
    return records(ingest_kfgqpc.ingest(source_rows()), load_table())


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_manifest(recs: list[dict]) -> dict:
    kinds: dict[str, int] = {}
    clusters: dict[str, int] = {}
    for r in recs:
        kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
        for c in r["clusters"]:
            clusters[c["kind"]] = clusters.get(c["kind"], 0) + 1
    return {
        "script_id": SCRIPT_ID,
        "spine_version": SPINE_VERSION,
        "source": {
            "id": ingest_kfgqpc.SOURCE_ID,
            "member": ingest_kfgqpc.SOURCE_MEMBER,
            "sha256": _sha256(sources.path(ingest_kfgqpc.SOURCE_ID)),
        },
        "codepoint_table_sha256": _sha256(TABLE_PATH),
        "unicode_version": unicodedata.unidata_version,
        "counts": {
            "items": dict(sorted(kinds.items())),
            "clusters": dict(sorted(clusters.items())),
        },
    }


def write(recs: list[dict]) -> None:
    SPINE_DIR.mkdir(parents=True, exist_ok=True)
    lines = [json.dumps(r, ensure_ascii=True, separators=(",", ":")) for r in recs]
    ITEMS.write_text("\n".join(lines) + "\n", encoding="ascii")
    MANIFEST.write_text(
        json.dumps(build_manifest(recs), ensure_ascii=True, indent=2) + "\n", encoding="ascii"
    )


def load_records() -> list[dict]:
    return [json.loads(line) for line in ITEMS.read_text(encoding="ascii").splitlines()]


def load_manifest() -> dict:
    return json.loads(MANIFEST.read_text(encoding="ascii"))


def main(argv: list[str]) -> int:
    recs = build_records()
    if argv[1:] == ["build"]:
        write(recs)
        print(f"spine: {len(recs)} items written to {SPINE_DIR}")
        return 0
    if load_records() != recs or load_manifest() != build_manifest(recs):
        print("spine: committed files differ from a rebuild", file=sys.stderr)
        return 1
    print("spine: up to date")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
