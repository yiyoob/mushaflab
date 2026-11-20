"""Stage D: visual review of the words stage A could not verify.

    uv run python -m pipeline.futuh.review sheets <run>.jsonl   # contact sheets + queue
    uv run python -m pipeline.futuh.review apply  <run>.jsonl   # merge verdicts

`sheets` crops every flagged word (grade unattested/unaligned) straight from the PDF (vector,
no re-rasterised OCR input) into numbered contact sheets, and writes queue.json with the
decoded text of each crop. A reviewer (a person, or a vision model instructed to transcribe
exactly what is printed and never to normalise towards any riwayah) writes verdicts.json:

    {"<word key>": {"verdict": "ok" | "fix", "text": "<as printed, if fix>", "by": "...",
                    "note": "..."}}

`apply` copies verdicts onto the words as w["review"]; it never overwrites w["t"], so the
extracted text and the reviewer's reading stay side by side.
"""

from __future__ import annotations

import json
import sys
from collections.abc import Iterator
from pathlib import Path

import pymupdf

from .paths import OUT
from .run import volume_pdf

FLAG = ("unattested", "unaligned")
PER_SHEET = 12


def _words(records: list[dict]) -> Iterator[tuple[str, dict, dict]]:
    for r in records:
        if r["type"] != "row":
            continue
        for i, w in enumerate(r["words"]):
            yield f"{r['id']}-w{i + 1}", r, w


def sheets(jsonl: Path, only: str | None = None) -> Path:
    records = [json.loads(ln) for ln in jsonl.read_text().splitlines()]
    vol = records[0]["vol"]
    doc = pymupdf.open(volume_pdf(vol))
    outdir = OUT / "review" / jsonl.stem
    outdir.mkdir(parents=True, exist_ok=True)
    queue = []
    for key, r, w in _words(records):
        if w["grade"] in FLAG and w.get("bbox"):
            colored = bool(w.get("colors"))
            # high: an uncoloured word the book prints like Hafs (so a mismatch smells of
            # extraction), or a word with glyphs never confirmed against Hafs. low: a coloured
            # variant whose every glyph was confirmed elsewhere (pilot: all such words reviewed
            # were faithful readings).
            prio = "high" if (not colored or w.get("decode") != "proven") else "low"
            queue.append(
                {
                    "key": key,
                    "priority": prio,
                    "pdf_page": r["pdf_page"],
                    "bbox": w["bbox"],
                    "decoded": w["t"],
                    "grade": w["grade"],
                    "spine": w.get("spine"),
                    "colored": colored,
                    "context": r["reading"],
                }
            )
    queue.sort(key=lambda q: q["priority"] != "high")  # stable: high first, book order within
    render = [q for q in queue if only is None or q["priority"] == only]
    cell_w, cell_h, cols = 300, 70, 3
    for s in range(0, len(render), PER_SHEET):
        batch = render[s : s + PER_SHEET]
        rows = (len(batch) + cols - 1) // cols
        out = pymupdf.open()
        pg = out.new_page(width=cols * cell_w, height=rows * cell_h)
        for k, q in enumerate(batch):
            q["sheet"], q["n"] = s // PER_SHEET + 1, s + k + 1
            x0, y0, x1, y1 = q["bbox"]
            pad = 6
            clip = pymupdf.Rect(x0 - pad, y0 - pad, x1 + pad, y1 + pad)
            cx, cy = (k % cols) * cell_w, (k // cols) * cell_h
            pg.draw_rect(
                pymupdf.Rect(cx + 2, cy + 2, cx + cell_w - 2, cy + cell_h - 2),
                color=(0.8, 0.8, 0.8),
                width=0.5,
            )
            pg.insert_text((cx + 6, cy + 16), str(q["n"]), fontsize=11, color=(0.8, 0.1, 0.1))
            target = pymupdf.Rect(cx + 30, cy + 6, cx + cell_w - 6, cy + cell_h - 6)
            pg.show_pdf_page(target, doc, q["pdf_page"] - 1, clip=clip, keep_proportion=True)
        pix = pg.get_pixmap(dpi=200)
        pix.save(outdir / f"sheet-{s // PER_SHEET + 1:03d}.png")
    (outdir / "queue.json").write_text(json.dumps(queue, ensure_ascii=False, indent=1))
    print(
        f"{len(queue)} flagged words ({len(render)} rendered) -> {outdir} "
        f"({(len(render) + PER_SHEET - 1) // PER_SHEET} sheets)"
    )
    return outdir


def apply(jsonl: Path) -> None:
    outdir = OUT / "review" / jsonl.stem
    verdicts = json.loads((outdir / "verdicts.json").read_text())
    records = [json.loads(ln) for ln in jsonl.read_text().splitlines()]
    n = 0
    for key, _r, w in _words(records):
        if key in verdicts:
            w["review"] = verdicts[key]
            n += 1
    jsonl.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in records))
    print(f"applied {n} verdicts to {jsonl}")


if __name__ == "__main__":
    cmd, path = sys.argv[1], Path(sys.argv[2])
    if cmd == "sheets":
        sheets(
            path, sys.argv[3] if len(sys.argv) > 3 else None
        )  # optional: "high" renders only those
    else:
        apply(path)
