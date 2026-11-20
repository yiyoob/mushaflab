"""Command line: extract a page range of one volume.

    uv run python -m pipeline.futuh.run --vol 1 --pages 48-55 [--surah 1] [--qa]

Writes out/vol{N}[-{range}].jsonl, .shamela.txt, a summary .json, and with --qa an HTML review
page plus page images under out/qa/.
"""

from __future__ import annotations

import argparse
import collections
import json
import time
from pathlib import Path

import pymupdf

from .anchor import Anchorer
from .emit import finalize, page_records, qa_html, shamela, write_jsonl
from .fontmap import FontMap
from .glyphs import InkBounds
from .page import parse_page
from .paths import OUT, font_path, pdf_dir
from .quran import canon

# al-Futuh al-Bahira
BOOK_TITLE = "\u0627\u0644\u0641\u062a\u0648\u062d \u0627\u0644\u0628\u0627\u0647\u0631\u0629"
VOLUME_LABEL = "\u0627\u0644\u0645\u062c\u0644\u062f"  # al-mujallad
VOLUME_WORDS = {
    1: "\u0627\u0644\u0623\u0648\u0644",
    2: "\u0627\u0644\u062b\u0627\u0646\u064a",
    3: "\u0627\u0644\u062b\u0627\u0644\u062b",
    4: "\u0627\u0644\u0631\u0627\u0628\u0639",
    5: "\u0627\u0644\u062e\u0627\u0645\u0633",
    6: "\u0627\u0644\u0633\u0627\u062f\u0633",
}


def volume_pdf(vol: int) -> Path:
    import unicodedata

    named = pdf_dir() / f"vol{vol}.pdf"  # MANIFEST.json file names
    if named.exists():
        return named
    want = unicodedata.normalize("NFC", VOLUME_WORDS[vol])
    for p in sorted(pdf_dir().glob("*.pdf")):
        if unicodedata.normalize("NFC", Path(p).stem).endswith(want):
            return Path(p)
    raise SystemExit(f"no PDF for volume {vol} in {pdf_dir()} (set FUTUH_PDF_DIR)")


def page_list(spec: str | None, n: int) -> list[int]:
    if not spec:
        return list(range(1, n + 1))
    out = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-")
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return [p for p in out if 1 <= p <= n]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--vol", type=int, required=True)
    ap.add_argument("--pages", help="pdf page numbers, e.g. 48-55,100")
    ap.add_argument("--surah", type=int, help="surah in progress at the first page (else searched)")
    ap.add_argument("--qa", action="store_true", help="also write the HTML review page")
    ap.add_argument("--tag", default="")
    a = ap.parse_args()

    pdf = volume_pdf(a.vol)
    doc = pymupdf.open(pdf)
    pages = page_list(a.pages, len(doc))
    fm = FontMap(font_path())
    fontmaps = {"SakkalMajalla": fm}
    c = canon()
    ink = InkBounds(doc)
    anc = Anchorer(c, a.surah)
    state: dict = {}
    records: list[dict] = []
    t0 = time.time()
    for pn in pages:
        m = parse_page(doc, pn - 1, ink, fontmaps)
        records.extend(page_records(a.vol, m, anc, c, state))
    finalize(records, c)
    from .readers import Lexicon, resolve_rows

    resolve_rows(Lexicon(), [r for r in records if r["type"] == "row"], c)
    tag = a.tag or (f"-p{pages[0]}-{pages[-1]}" if a.pages else "")
    stem = OUT / f"vol{a.vol}{tag}"
    OUT.mkdir(parents=True, exist_ok=True)
    write_jsonl(stem.with_suffix(".jsonl"), records)
    title = f"{BOOK_TITLE} \u2014 {VOLUME_LABEL} {VOLUME_WORDS[a.vol]}"
    stem.with_suffix(".shamela.txt").write_text(shamela(records, c, title))

    # summary
    grades = collections.Counter()
    colored = collections.Counter()
    prose = collections.Counter()
    unknown_colors = collections.Counter()
    for r in records:
        if r["type"] == "row":
            for w in r["words"]:
                grades[w["grade"]] += 1
                if w.get("colors"):
                    colored[w["grade"]] += 1
                for cr in w.get("colors", []):
                    if cr["class"] is None:
                        unknown_colors[cr["hex"]] += 1
        if r["type"] == "page":
            prose.update(r["prose_roundtrip"])
    rstat = collections.Counter(r["readers_parse"]["status"] for r in records if r["type"] == "row")
    summary = {
        "readers": dict(rstat),
        "volume": a.vol,
        "pages": [pages[0], pages[-1]],
        "seconds": round(time.time() - t0, 1),
        "rows": sum(r["type"] == "row" for r in records),
        "rows_unanchored": sum(
            bool(r["type"] == "row" and r["words"] and not any(x["surah"] for x in r["ayahs"]))
            for r in records
        ),
        "words_by_grade": dict(grades),
        "colored_words_by_grade": dict(colored),
        "prose_roundtrip": dict(prose),
        "unmapped_colors": dict(unknown_colors),
    }
    stem.with_suffix(".summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1))

    if a.qa:
        qa = OUT / "qa"
        imgdir = qa / f"img-v{a.vol}"
        imgdir.mkdir(parents=True, exist_ok=True)
        images = {}
        for pn in pages:
            f = imgdir / f"p{pn:03d}.png"
            if not f.exists():
                doc[pn - 1].get_pixmap(dpi=110).save(f)
            images[pn] = f"img-v{a.vol}/{f.name}"
        page = qa / f"vol{a.vol}{tag}.html"
        page.write_text(
            qa_html(
                records,
                images,
                c,
                title + (f" \u2014 pp. {pages[0]}\u2013{pages[-1]}" if a.pages else ""),
            )
        )
        print("QA:", page)


if __name__ == "__main__":
    main()
