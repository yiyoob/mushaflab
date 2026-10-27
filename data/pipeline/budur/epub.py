"""Read the al-Budur EPUB (TB.2): pages in spine order, paragraphs, printed pages, sections.

The EPUB is checked against its MANIFEST row (sha256, size) before it is opened. Its location
is `$BOOKS_DIR/al-budur-al-zahirah/al-budur.epub` when BOOKS_DIR is set (the raw folder lives
only in the main checkout), else the MANIFEST path under `data/sources/raw/`.

Each page is one xhtml file `OEBPS/text/page_<part>_<nnnn>.xhtml`. Its last paragraph is the
printed-page line ("al-juz': N - al-safhah: M"; info.xhtml: numbering matches the print), which
is taken off as `printed` and not returned as a paragraph. `<h2>` elements are headings. The
`<h2>` headings that start with "surah " (in Arabic) are counted in order and must number 114;
paragraphs before the first are `general` (introduction, isti'adhah, basmalah), as are the
chapters after the last surah, and `page_2_*` pages are the appended corrections
(`corrections`). Headings that the EPUB gives only as `<span>` anchors carry no text.
"""

import hashlib
import os
import re
import zipfile
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree as ET

import yaml

from pipeline.fetch import MANIFEST_PATH, RAW_ROOT
from pipeline.manifest import load_manifest

SOURCE_ID = "book-al-budur-al-zahirah"
BOOK_DATA = Path(__file__).resolve().parents[2] / "books" / "al-budur-al-zahirah"
BOOK_SUBPATH = Path("al-budur-al-zahirah") / "al-budur.epub"
XHTML = "{http://www.w3.org/1999/xhtml}"
OPF = "{http://www.idpf.org/2007/opf}"
N_SURAHS = 114

# "\u0627\u0644\u062c\u0632\u0621: 1 - \u0627\u0644\u0635\u0641\u062d\u0629: 15"
PRINTED_RE = re.compile(
    "\u0627\u0644\u062c\u0632\u0621\\s*:\\s*([^\\s-]+)\\s*-\\s*"
    "\u0627\u0644\u0635\u0641\u062d\u0629\\s*:\\s*(\\d+)"
)
SURAH_PREFIX = "\u0633\u0648\u0631\u0629"
PAGE_RE = re.compile(r"page_(\d+)_(\d+)\.xhtml$")


class BookError(ValueError):
    pass


@dataclass(frozen=True)
class Para:
    page_id: str  # "page_1_0011"
    part: str | None  # printed part (al-juz'): "1", or "mulhaq" (in Arabic) in the appendix
    printed: int | None  # printed page ("\u0627\u0644\u0635\u0641\u062d\u0629")
    index: int  # paragraph number on the page, 1-based (headings count)
    heading: str | None  # text of the latest heading at or before this paragraph
    surah: int | None  # 1..114 inside the surah sections, else None
    zone: str  # "general" | "surahs" | "corrections"
    is_heading: bool
    text: str

    @property
    def anchor(self) -> str:
        return f"{self.page_id}#p{self.index}"

    @property
    def cite(self) -> str:
        return f"{self.page_id} p. {self.printed}" if self.printed is not None else self.page_id


def book_path(env: dict[str, str] | None = None) -> Path:
    env = os.environ if env is None else env
    if env.get("BOOKS_DIR"):
        return Path(env["BOOKS_DIR"]) / BOOK_SUBPATH
    src = next(s for s in load_manifest(MANIFEST_PATH).sources if s.id == SOURCE_ID)
    return RAW_ROOT / src.file


def verify(path: Path) -> None:
    src = next(s for s in load_manifest(MANIFEST_PATH).sources if s.id == SOURCE_ID)
    if src.sha256 is None:
        raise BookError(f"{SOURCE_ID} is not pinned in MANIFEST.json (run just data-pin)")
    if not path.exists():
        raise BookError(f"{SOURCE_ID}: {path} missing (set BOOKS_DIR to the main checkout's books)")
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != src.sha256 or len(data) != src.size:
        raise BookError(f"{SOURCE_ID}: {path} does not match its pinned sha256/size")


def _text(el: ET.Element) -> str:
    return " ".join("".join(el.itertext()).split())


def _spine_order(z: zipfile.ZipFile) -> list[str]:
    opf_name = next(n for n in z.namelist() if n.endswith(".opf"))
    opf = ET.fromstring(z.read(opf_name))  # noqa: S314 (pinned file, checksum verified)
    base = str(Path(opf_name).parent)
    hrefs = {i.get("id"): i.get("href") for i in opf.iter(f"{OPF}item")}
    names = [f"{base}/{hrefs[r.get('idref')]}" for r in opf.iter(f"{OPF}itemref")]
    return [n for n in names if PAGE_RE.search(n)]


def inline_starts(path: Path = BOOK_DATA / "structure.yaml") -> list[dict]:
    """Surahs whose start the EPUB marks only inside a paragraph (structure.yaml)."""
    return yaml.safe_load(path.read_text(encoding="utf-8"))["inline_surah_starts"]


def _page(name: str, data: bytes) -> tuple[str, str | None, int | None, list[ET.Element]]:
    """Page id, printed part and page (taken off the last paragraph), and its blocks."""
    page_id = Path(name).stem
    body = ET.fromstring(data).find(f"{XHTML}body")  # noqa: S314 (pinned file, checksum verified)
    if body is None:
        raise BookError(f"{page_id}: no body")
    blocks = [el for el in body if el.tag in (f"{XHTML}p", f"{XHTML}h2")]
    if blocks and blocks[-1].tag == f"{XHTML}p" and (m := PRINTED_RE.search(_text(blocks[-1]))):
        return page_id, m.group(1), int(m.group(2)), blocks[:-1]
    return page_id, None, None, blocks


def _zone(corrections: bool, surah: int, n_surahs: int) -> tuple[str, int | None]:
    if corrections:
        return "corrections", None
    if 1 <= surah <= n_surahs:
        return "surahs", surah
    return "general", None


def read(path: Path, n_surahs: int = N_SURAHS, inline: list[dict] | None = None) -> list[Para]:
    """Every paragraph and heading of the book, in reading order."""
    inline = inline_starts() if inline is None else inline
    starts = {(s["page_id"], s["after_para"]): s for s in inline}
    with zipfile.ZipFile(path) as z:
        raw_pages = [(n, z.read(n)) for n in _spine_order(z)]
    out: list[Para] = []
    heading: str | None = None
    surah = 0
    for name, data in raw_pages:
        page_id, part, printed, blocks = _page(name, data)
        corrections = PAGE_RE.search(name).group(1) != "1"  # type: ignore[union-attr]
        for i, el in enumerate(blocks, start=1):
            text = _text(el)
            is_h = el.tag == f"{XHTML}h2"
            if is_h:
                heading = text
                if not corrections and text.startswith(SURAH_PREFIX + " ") and surah < n_surahs:
                    surah += 1
                elif surah == n_surahs:
                    surah = n_surahs + 1  # after the last surah: later chapters are general
            zone, s = _zone(corrections, surah, n_surahs)
            if text:
                out.append(Para(page_id, part, printed, i, heading, s, zone, is_h, text))
            if (start := starts.pop((page_id, i), None)) is not None:
                if start["title"] not in text or surah != start["surah"] - 1:
                    raise BookError(
                        f"{page_id}#p{i}: inline start of surah {start['surah']} not found"
                    )
                surah, heading = start["surah"], start["title"]
    if starts:
        raise BookError(f"inline surah starts not found: {sorted(starts)}")
    found = max((p.surah or 0) for p in out) if out else 0
    if found != n_surahs:
        raise BookError(f"found {found} surah headings, expected {n_surahs}")
    return out
