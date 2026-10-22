"""The 8 KFGQPC riwayah texts and fonts in scope, read from pinned sources.

Seven come from the official v2.x zips; Bazzi from the 2022 mirror (v7), because its
official zip was never archived (G-003).
"""

import io

from fontTools.ttLib import TTFont

from pipeline import sources

RIWAYAT = ("hafs", "shuba", "warsh", "qalun", "bazzi", "qunbul", "duri_abuamr", "susi")
OFFICIAL = {
    "hafs": "kfgqpc-official-hafs",
    "shuba": "kfgqpc-official-shuba",
    "warsh": "kfgqpc-official-warsh",
    "qalun": "kfgqpc-official-qalun",
    "qunbul": "kfgqpc-official-qunbul",
    "duri_abuamr": "kfgqpc-official-duri_abuamr",
    "susi": "kfgqpc-official-susi",
}


BAZZI_DATA = "kfgqpc-mirror-bazzi-data"


def source_id(riwayah: str) -> str:
    """The pinned source id of a riwayah's text."""
    return BAZZI_DATA if riwayah == "bazzi" else OFFICIAL[riwayah]


def rows(riwayah: str) -> list[dict]:
    if riwayah == "bazzi":
        return sources.json_file(BAZZI_DATA)  # type: ignore[return-value]
    return sources.zip_json(OFFICIAL[riwayah], ".json")  # type: ignore[return-value]


def font_bytes(riwayah: str) -> bytes:
    if riwayah == "bazzi":
        return sources.path("kfgqpc-mirror-bazzi-font").read_bytes()
    return sources.zip_member(OFFICIAL[riwayah], ".ttf")


def font(riwayah: str) -> TTFont:
    return TTFont(io.BytesIO(font_bytes(riwayah)))
