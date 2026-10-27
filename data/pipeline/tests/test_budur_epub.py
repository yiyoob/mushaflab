"""TB.2: the al-Budur EPUB reader, on a synthetic EPUB (placeholder text, no book content)."""

import zipfile
from pathlib import Path

import pytest

from pipeline.budur import epub

SURAH = "\u0633\u0648\u0631\u0629"
FOOT = "\u0627\u0644\u062c\u0632\u0621: 1 - \u0627\u0644\u0635\u0641\u062d\u0629: {}"


def _page(body: str) -> str:
    return (
        '<?xml version="1.0" encoding="utf-8"?><!DOCTYPE html>'
        '<html xmlns="http://www.w3.org/1999/xhtml"><head><title>t</title></head>'
        f"<body>{body}</body></html>"
    )


def _epub(tmp_path: Path, pages: list[str]) -> Path:
    path = tmp_path / "b.epub"
    items = "".join(
        f'<item id="p{i}" href="text/page_{n}" media-type="application/xhtml+xml"/>'
        for i, n in enumerate(_names(pages))
    )
    refs = "".join(f'<itemref idref="p{i}"/>' for i in range(len(pages)))
    opf = (
        '<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0">'
        f"<manifest>{items}</manifest><spine>{refs}</spine></package>"
    )
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("OEBPS/content.opf", opf)
        for n, body in zip(_names(pages), pages, strict=True):
            z.writestr(f"OEBPS/text/page_{n}", _page(body))
    return path


def _names(pages: list[str]) -> list[str]:
    return [
        f"1_{i + 1:04d}.xhtml" if "CORR" not in p else f"2_{i + 1:04d}.xhtml"
        for i, p in enumerate(pages)
    ]


def test_pages_paragraphs_printed_pages_and_zones(tmp_path: Path) -> None:
    pages = [
        f'<h2 id="toc-1">bab</h2><p>intro</p><p class="text-center">{FOOT.format(5)}</p>',
        f'<h2 id="toc-2">{SURAH} a</h2><p>text</p><p>text 2</p><p>{FOOT.format(6)}</p>',
        f'<p>cont</p><h2 id="toc-3">{SURAH} b</h2><p>text</p><p>{FOOT.format(7)}</p>',
        '<h2 id="toc-4">CORR</h2><p>correction</p>',
    ]
    paras = epub.read(_epub(tmp_path, pages), n_surahs=2, inline=[])
    got = [(p.page_id, p.printed, p.index, p.zone, p.surah, p.is_heading) for p in paras]
    assert got == [
        ("page_1_0001", 5, 1, "general", None, True),
        ("page_1_0001", 5, 2, "general", None, False),
        ("page_1_0002", 6, 1, "surahs", 1, True),
        ("page_1_0002", 6, 2, "surahs", 1, False),
        ("page_1_0002", 6, 3, "surahs", 1, False),
        ("page_1_0003", 7, 1, "surahs", 1, False),
        ("page_1_0003", 7, 2, "surahs", 2, True),
        ("page_1_0003", 7, 3, "surahs", 2, False),
        ("page_2_0004", None, 1, "corrections", None, True),
        ("page_2_0004", None, 2, "corrections", None, False),
    ]
    assert paras[3].cite == "page_1_0002 p. 6"
    assert paras[3].anchor == "page_1_0002#p2"


def test_inline_surah_start(tmp_path: Path) -> None:
    pages = [
        f'<h2 id="t1">{SURAH} a</h2><p>text " {SURAH} b "</p><p>after</p>'
        f'<h2 id="t2">{SURAH} c</h2><p>text</p><p>{FOOT.format(5)}</p>',
    ]
    inline = [{"surah": 2, "page_id": "page_1_0001", "after_para": 2, "title": f"{SURAH} b"}]
    paras = epub.read(_epub(tmp_path, pages), n_surahs=3, inline=inline)
    assert [p.surah for p in paras] == [1, 1, 2, 3, 3]


def test_inline_start_must_be_where_declared(tmp_path: Path) -> None:
    pages = [
        f'<h2 id="t1">{SURAH} \u0623</h2><p>\u0643\u0644\u0627\u0645</p><p>{FOOT.format(5)}</p>'
    ]
    inline = [{"surah": 2, "page_id": "page_1_0001", "after_para": 2, "title": f"{SURAH} b"}]
    with pytest.raises(epub.BookError, match="inline start"):
        epub.read(_epub(tmp_path, pages), n_surahs=2, inline=inline)


def test_wrong_surah_count_is_an_error(tmp_path: Path) -> None:
    pages = [f'<h2 id="t1">{SURAH} \u0623</h2><p>\u0643\u0644\u0627\u0645</p>']
    with pytest.raises(epub.BookError, match="surah headings"):
        epub.read(_epub(tmp_path, pages), n_surahs=2, inline=[])


def test_book_path_uses_books_dir(tmp_path: Path) -> None:
    assert epub.book_path({"BOOKS_DIR": str(tmp_path)}) == (
        tmp_path / "al-budur-al-zahirah" / "al-budur.epub"
    )


def test_verify_rejects_a_changed_file(tmp_path: Path) -> None:
    fake = tmp_path / "al-budur.epub"
    fake.write_bytes(b"not the book")
    with pytest.raises(epub.BookError, match="does not match"):
        epub.verify(fake)


def test_structure_file_declares_al_anam() -> None:
    starts = epub.inline_starts()
    assert [(s["surah"], s["page_id"], s["after_para"]) for s in starts] == [(6, "page_1_0095", 13)]
