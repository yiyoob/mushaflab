"""Verified, read-only access to pinned sources (CLAUDE.md rule 2).

Every accessor verifies all checksums in MANIFEST.json first (once per process), and reads
archives in place: nothing is ever written under data/sources/raw/.
"""

import functools
import io
import json
import zipfile
from collections import Counter
from pathlib import Path

from pipeline.fetch import MANIFEST_PATH, RAW_ROOT, verify
from pipeline.manifest import load_manifest


class SourceError(RuntimeError):
    pass


def available() -> bool:
    """True if every pinned source is present and verifies."""
    try:
        verified()
    except SourceError:
        return False
    return True


@functools.cache
def verified() -> dict[str, Path]:
    manifest = load_manifest(MANIFEST_PATH)
    problems = verify(manifest, RAW_ROOT)
    if problems:
        raise SourceError("; ".join(p.message for p in problems))
    return {s.id: RAW_ROOT / s.file for s in manifest.sources}


def path(source_id: str) -> Path:
    return verified()[source_id]


def zip_member(source_id: str, suffix: str) -> bytes:
    """The bytes of the single member of a pinned zip whose name ends with `suffix`."""
    with zipfile.ZipFile(path(source_id)) as zf:
        names = [n for n in zf.namelist() if n.endswith(suffix)]
        if len(names) != 1:
            raise SourceError(f"{source_id}: expected one member ending {suffix!r}, got {names}")
        return zf.read(names[0])


def zip_json(source_id: str, suffix: str) -> object:
    raw = zip_member(source_id, suffix)
    return json.load(io.TextIOWrapper(io.BytesIO(raw), encoding="utf-8-sig"))


@functools.cache
def qul_hafs_words() -> dict[int, tuple[int, int, int, str]]:
    """QUL KFGQPC Hafs word list: word id -> (surah, ayah, position, text)."""
    data = zip_json("qul-script-312-kfgqpc-hafs-wbw", ".json")
    return {
        int(w["id"]): (int(w["surah"]), int(w["ayah"]), int(w["word"]), w["text"])
        for w in data.values()  # type: ignore[union-attr]
    }


def is_ayah_number(text: str) -> bool:
    """QUL writes ayah markers as Arabic-Indic digits."""
    return bool(text) and all("\u0660" <= c <= "\u0669" for c in text)


def qul_ayah_counts_per_surah() -> Counter:
    """Ayahs per surah, counted as QUL ayah markers (one per ayah)."""
    return Counter(s for s, _a, _w, t in qul_hafs_words().values() if is_ayah_number(t))


@functools.cache
def tanzil_ayahs() -> dict[tuple[int, int], str]:
    """Tanzil `surah|ayah|text` lines; the trailing comment block is not data."""
    ayahs = {}
    for line in path("tanzil-uthmani-1.1").read_text(encoding="utf-8").splitlines():
        surah, _, rest = line.partition("|")
        ayah, _, text = rest.partition("|")
        if surah.isdigit() and ayah.isdigit():
            ayahs[(int(surah), int(ayah))] = text
    return ayahs


def tanzil_ayah_counts_per_surah() -> Counter:
    return Counter(s for s, _a in tanzil_ayahs())
