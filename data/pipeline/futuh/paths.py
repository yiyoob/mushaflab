"""Where the al-Futuh pipeline reads and writes.

Sources are the pinned, owner-supplied files (MANIFEST.json, group `books` and `owner-fonts`),
under data/sources/raw/ and never tracked. FUTUH_PDF_DIR / FUTUH_FONT point elsewhere when the
files are kept outside the repository. Outputs (book text) go to build/, never to git (rule 10).
"""

import os
from pathlib import Path

DATA = Path(__file__).resolve().parents[2]
REPO = DATA.parent
RAW = DATA / "sources" / "raw"
BOOK = DATA / "books" / "al-futuh-al-bahira"  # tracked: lexicons and learned glyph data only
OUT = REPO / "build" / "books" / "al-futuh-al-bahira"
CACHE = OUT / "cache"


def pdf_dir() -> Path:
    return Path(os.environ.get("FUTUH_PDF_DIR", RAW / "books" / "al-futuh-al-bahira"))


def font_path() -> Path:
    return Path(os.environ.get("FUTUH_FONT", RAW / "owner-fonts" / "majalla.ttf"))
