"""Surah names, read from the pinned KFGQPC Hafs data (never typed by hand)."""

import json
import zipfile

from .quran import OFFICIAL


def _load() -> list[str]:
    z = zipfile.ZipFile(OFFICIAL / "UthmanicHafs_v2-0.zip")
    rows = json.loads(z.read(next(n for n in z.namelist() if n.endswith(".json"))))
    names: dict[int, str] = {}
    for r in rows:
        s = int(r.get("sura_no") or r.get("sora"))
        names.setdefault(s, (r.get("sura_name_ar") or r.get("sora_name_ar")).strip())
    return [names[i] for i in range(1, 115)]


SURAH_NAMES = _load()
