"""Shared helpers for Phase 0 spikes: verified, read-only access to pinned sources.

Spikes read only pinned files (CLAUDE.md rule 1). Every run verifies checksums first,
then extracts archives into build/extracted/ (gitignored), never into raw/.
"""

import json
import sqlite3
import tarfile
import zipfile
from pathlib import Path

from pipeline.fetch import MANIFEST_PATH, RAW_ROOT, verify
from pipeline.manifest import load_manifest

REPO = Path(__file__).resolve().parents[2]
EXTRACTED = REPO / "build" / "extracted"


def verified_sources() -> dict[str, Path]:
    manifest = load_manifest(MANIFEST_PATH)
    problems = verify(manifest, RAW_ROOT)
    if problems:
        raise SystemExit("sources do not verify: " + "; ".join(p.message for p in problems))
    return {s.id: RAW_ROOT / s.file for s in manifest.sources}


def extracted(source_id: str) -> Path:
    """Extract a pinned zip/tgz once into build/extracted/<id>/ and return that directory."""
    src = verified_sources()[source_id]
    out = EXTRACTED / source_id
    if not out.exists():
        tmp = out.with_name(out.name + ".tmp")
        tmp.mkdir(parents=True)
        if src.suffix == ".zip":
            with zipfile.ZipFile(src) as zf:
                zf.extractall(tmp)  # noqa: S202 - zipfile strips absolute and .. paths
        else:
            with tarfile.open(src) as tf:
                tf.extractall(tmp, filter="data")
        tmp.rename(out)
    return out


def only(paths: list[Path]) -> Path:
    if len(paths) != 1:
        raise SystemExit(f"expected exactly one match, got {paths}")
    return paths[0]


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def kfgqpc_official(riwayah_id: str) -> list[dict]:
    root = extracted(f"kfgqpc-official-{riwayah_id}")
    return load_json(only(sorted(root.glob("*/*.json"))))  # type: ignore[return-value]


def qul_layout(source_id: str) -> list[tuple]:
    db = only(sorted(extracted(source_id).glob("*.db")))
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        rows = con.execute(
            "select page_number, line_number, line_type, is_centered, first_word_id,"
            " last_word_id, surah_number from pages order by page_number, line_number"
        ).fetchall()
    finally:
        con.close()
    # QUL stores absent word ids on header/basmalah lines as '' rather than NULL.
    return [tuple(None if v == "" else v for v in row) for row in rows]
