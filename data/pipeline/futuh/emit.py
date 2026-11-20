"""Output: JSONL (machine), Shamela-style text (human), HTML QA pages (review).

JSONL record types, one per line, in book order:
  {"type": "page", ...}       page-level facts: printed number, prose round-trip tally
  {"type": "row", ...}        one table row: reader label, reading words anchored to the spine
  {"type": "footnote", ...}   one footnote paragraph (its Quranic words graded too)
  {"type": "text", ...}       running text outside tables (introductions, notes)
"""

from __future__ import annotations

import html
import json
from pathlib import Path

import yaml

from .anchor import Anchorer, BookWord, anchor_row, book_word
from .glyphs import Glyph
from .page import PageModel, Token, tokens_text
from .paths import BOOK
from .quran import Canon, clusters, is_mark

COLORS_FILE = BOOK / "colors.yaml"


def load_colors() -> dict[str, dict]:
    if not COLORS_FILE.exists():
        return {}
    d = yaml.safe_load(COLORS_FILE.read_text())
    out = {}
    for cls in d["classes"]:
        for h in cls["hex"]:
            out[h.lower()] = cls
    return out


COLORS = load_colors()
AR_DIGITS = str.maketrans(
    "0123456789", "\u0660\u0661\u0662\u0663\u0664\u0665\u0666\u0667\u0668\u0669"
)


def color_class(hexc: str) -> str | None:
    c = COLORS.get(hexc.lower())
    return c["id"] if c else None


def word_json(w: BookWord, canon: Canon) -> dict:
    d = {"t": w.text, "grade": w.grade}
    if w.bbox:
        d["bbox"] = list(w.bbox)
    if w.spine:
        d["spine"] = w.spine
        d["sim"] = w.sim
    if w.riwayat:
        d["riwayat"] = w.riwayat
    if w.split:
        d["split"] = True  # printed as two words; both carry the same spine id and grade
    if w.tok is not None and not w.grade.startswith("hafs"):
        from .glyphs import proven_glyphs

        pg = proven_glyphs()
        import unicodedata as ud

        def gkey(g: Glyph) -> str:
            return f"{g.font}:{g.gid}:" + "".join(
                c for c in g.text if ud.category(c) != "Mn" and c != "\u0640"
            )

        unproven = sorted(
            {
                g.gid
                for g in w.tok.glyphs
                if not g.is_space and gkey(g) not in pg and f"{gkey(g)}|{g.prev_letter}" not in pg
            }
        )
        # "proven": every glyph's decoding was confirmed elsewhere against Hafs, so a mismatch
        # is about the reading, not about extraction. "unproven" lists glyph ids never confirmed.
        d["decode"] = "proven" if not unproven else {"unproven_gids": unproven}
    runs = []
    bcl = clusters(w.text)
    starts = []
    pos = 0
    for cl in bcl:
        starts.append(pos)
        pos += len(cl)
    spine_cl = canon.word(w.spine).clusters if w.spine else None
    for s, e, hexc in w.colors:
        if hexc in ("#231f20", "#000000"):
            continue
        r = {"s": s, "e": e, "hex": hexc, "class": color_class(hexc)}
        # letters covered, and their spine cluster ids when the word has the same letter count
        li = [i for i, st in enumerate(starts) if s <= st < e]
        if li:
            r["letters"] = [li[0], li[-1]]
            if spine_cl and len(spine_cl) == len(bcl):
                r["clusters"] = [spine_cl[i][0] for i in li]
        runs.append(r)
    if runs:
        d["colors"] = runs
    return d


def _tok_json(toks: list[Token]) -> tuple[str, list[dict], list[int]]:
    """Plain text of a token list + its Quranic words (unanchored here) + footnote refs."""
    text = tokens_text(toks)
    qw = [book_word(t) for t in toks if t.kind == "quran"]
    fns = [t.num for t in toks if t.kind == "fn"]
    return text, qw, fns


def page_records(vol: int, m: PageModel, anc: Anchorer, canon: Canon, state: dict) -> list[dict]:
    pid = f"v{vol}-p{m.pdf_page:03d}"
    recs: list[dict] = [
        {
            "type": "page",
            "id": pid,
            "vol": vol,
            "pdf_page": m.pdf_page,
            "page": m.printed_page,
            "prose_roundtrip": {k: v for k, v in m.roundtrip.items() if isinstance(v, int)},
            "prose_corrections": m.roundtrip.get("corrections", []),
            "prose_failures": m.roundtrip.get("examples", []),
            "orphan_marks": m.orphan_marks,
        }
    ]
    for i, line in enumerate(m.body_before):
        recs.append(
            {
                "type": "text",
                "id": f"{pid}-t{i + 1}",
                "vol": vol,
                "pdf_page": m.pdf_page,
                "where": "before_table",
                "text": tokens_text(line),
            }
        )
    for i, r in enumerate(m.rows):
        if r.group_start or "group" not in state:
            state["group"] = f"{pid}-r{i + 1}"
        if not r.number.strip().isdigit():
            continue  # column-header row (رقم / القراءة / الشرح)
        chs = anchor_row(anc, r)
        if not any(ch.words for ch in chs) and not r.reading:
            continue
        rec = {
            "type": "row",
            "id": f"{pid}-r{i + 1}",
            "group": state["group"],
            "vol": vol,
            "pdf_page": m.pdf_page,
            "page": m.printed_page,
            "n": r.number,
            "reading": tokens_text(r.reading).strip(),
            "readers": r.explanation,
            "fn_refs": [t.num for t in r.reading if t.kind == "fn"],
            "labels": [t.text for t in r.reading if t.kind == "label"],
            "_chunks": chs,  # converted by finalize(), after later rows may have anchored these
        }
        if "..." in rec["reading"] or "\u2026" in rec["reading"]:
            rec["elided"] = True
        recs.append(rec)
    for i, line in enumerate(m.body_after):
        recs.append(
            {
                "type": "text",
                "id": f"{pid}-a{i + 1}",
                "vol": vol,
                "pdf_page": m.pdf_page,
                "where": "after_table",
                "text": tokens_text(line),
            }
        )
    # footnotes: a line starting with "(n)" opens footnote n; following lines continue it
    cur = None
    for line in m.footnotes:
        lead = next((t for t in line if t.kind != "space"), None)
        text, qw, _ = _tok_json(line)
        if (
            lead is not None
            and lead.kind == "text"
            and lead.text.startswith("(")
            and lead.text[1:].split(")")[0].isdigit()
            and lead is line[0]
        ) or (lead is not None and lead.kind == "fn"):
            num = lead.num if lead.kind == "fn" else int(lead.text[1:].split(")")[0])
            cur = {
                "type": "footnote",
                "id": f"{pid}-fn{num}",
                "vol": vol,
                "pdf_page": m.pdf_page,
                "n": num,
                "lines": [],
                "words": [],
            }
            recs.append(cur)
        if cur is None:
            cur = {
                "type": "footnote",
                "id": f"{pid}-fn0",
                "vol": vol,
                "pdf_page": m.pdf_page,
                "n": None,
                "lines": [],
                "words": [],
                "continued": True,
            }
            recs.append(cur)
        cur["lines"].append(text.strip())
        cur["words"].extend(
            {"t": w.text, "colors": [c for c in w.colors if c[2] not in ("#231f20", "#000000")]}
            for w in qw
        )
    for r in recs:
        if r["type"] == "footnote":
            r["text"] = " ".join(r["lines"])
    return recs


def finalize(records: list[dict], canon: Canon) -> list[dict]:
    """Turn anchored chunks into JSON (run once the whole range has been anchored)."""
    for r in records:
        chs = r.pop("_chunks", None)
        if chs is None:
            continue
        r["ayahs"] = [
            {"surah": ch.surah, "ayah": ch.ayah, "marker": ch.marker, "cost": round(ch.cost, 3)}
            for ch in chs
        ]
        r["words"] = [word_json(w, canon) for ch in chs for w in ch.words]
    return records


# ---------------------------------------------------------------------------------------------
# Shamela-style text


def _tagged(rec: dict) -> str:
    """Reading text with coloured runs wrapped as {class:letters}."""
    out = []
    for w in rec["words"]:
        t = w["t"]
        runs = sorted(w.get("colors", []), key=lambda r: r["s"])
        if not runs:
            out.append(t)
            continue
        s, pos = "", 0
        for r in runs:
            if r["s"] < pos:
                continue
            s += t[pos : r["s"]]
            label = COLORS.get(r["hex"], {}).get("label", r["hex"])
            s += "{" + label + ":" + t[r["s"] : r["e"]] + "}"
            pos = r["e"]
        out.append(s + t[pos:])
    return " ".join(out)


def shamela(records: list[dict], canon: Canon, title: str) -> str:
    from .names import SURAH_NAMES

    lines = [title, ""]
    for r in records:
        if r["type"] == "page":
            lines.append(f"\n[\u0635:{(r['page'] or str(r['pdf_page'])).translate(AR_DIGITS)}]")
        elif r["type"] == "text":
            lines.append(r["text"].strip())
        elif r["type"] == "row":
            by: dict[int, list[int]] = {}
            for a in r["ayahs"]:
                if a["surah"]:
                    by.setdefault(a["surah"], [])
                    if a["ayah"] not in by[a["surah"]]:
                        by[a["surah"]].append(a["ayah"])
            refs = []
            for su, ays in by.items():
                name = "".join(ch for ch in SURAH_NAMES[su - 1] if not is_mark(ch))
                rng = str(ays[0]) if len(ays) == 1 else f"{min(ays)}-{max(ays)}"
                refs.append(f"{name}: {rng.translate(AR_DIGITS)}")
            ref = f" [{'\u060c '.join(refs)}]" if refs else ""
            sep = "\n" if r["id"] == r["group"] else ""
            fn = "".join(f"({str(n).translate(AR_DIGITS)})" for n in r["fn_refs"])
            lines.append(
                f"{sep}{r['n'].translate(AR_DIGITS)}- \ufd3f{_tagged(r)}\ufd3e{ref}{fn} \u2014 {r['readers']}"
            )
        elif r["type"] == "footnote":
            if r.get("n") == 1 or (r.get("n") is None):
                lines.append(
                    "\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640\u0640"
                )
            lines.append(r["text"])
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------------------------
# HTML QA


GRADE_STYLE = {
    "hafs-strict": "",
    "hafs-loose": "g-loose",
    "riwayah-strict": "g-riw",
    "riwayah-loose": "g-riw",
    "unattested": "g-un",
    "unaligned": "g-na",
}


def qa_html(records: list[dict], images: dict[int, str], canon: Canon, title: str) -> str:
    pages: dict[int, list[dict]] = {}
    for r in records:
        pages.setdefault(r["pdf_page"], []).append(r)
    css = """
@font-face{font-family:'KFGQPC Hafs Local';src:url('fonts/uthmanic_hafs_v20.ttf')}
:root{--bg:#fbfaf7;--fg:#1d1b18;--mut:#6b665e;--line:#e3ded4;--card:#fff}
@media (prefers-color-scheme:dark){:root{--bg:#161513;--fg:#ece8e1;--mut:#a29b90;--line:#34312c;--card:#1f1d1a}}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.6 system-ui,sans-serif}
header{padding:16px 20px;border-bottom:1px solid var(--line)}
h1{font-size:18px;margin:0}
.legend span{margin-inline-end:14px;font-size:13px;color:var(--mut)}
.page{display:grid;grid-template-columns:minmax(300px,520px) 1fr;gap:20px;padding:20px;border-bottom:1px solid var(--line)}
.page img{width:100%;border:1px solid var(--line);background:#fff;position:sticky;top:10px}
.ext{direction:rtl;font-family:'KFGQPC Hafs Local','KFGQPC HAFS Uthmanic Script','Amiri Quran','Scheherazade New',serif;font-size:20px}
.row{background:var(--card);border:1px solid var(--line);border-radius:8px;padding:6px 10px;margin:6px 0}
.row.gs{margin-top:16px;border-top:3px double var(--mut)}
.n{color:var(--mut);font-size:14px;margin-inline-end:8px}
.rd{font-family:system-ui,sans-serif;font-size:14px;color:var(--mut)}
.w{cursor:help;border-radius:3px}
.g-loose{background:color-mix(in srgb,#d9a400 14%,transparent)}
.g-riw{box-shadow:inset 0 -2px #2a9d8f}
.g-un{box-shadow:inset 0 -2px #e76f51;background:color-mix(in srgb,#e76f51 12%,transparent)}
.g-na{outline:1px dashed #e76f51}
.fn,.tx{font-family:system-ui,sans-serif;font-size:14px;color:var(--fg);margin:4px 0}
.meta{font-family:ui-monospace,monospace;font-size:12px;color:var(--mut);direction:ltr;text-align:left}
@media (max-width:800px){.page{grid-template-columns:1fr}.page img{position:static}}
"""
    out = [
        f"<!doctype html><html lang=ar><head><meta charset=utf-8><meta name=viewport content='width=device-width,initial-scale=1'><title>{html.escape(title)}</title><style>{css}</style></head><body>"
    ]
    out.append(
        f"<header><h1>{html.escape(title)}</h1><div class=legend><span>no mark: matches \u1e24af\u1e63 exactly</span><span class=g-loose>yellow: matches \u1e24af\u1e63 up to encoding conventions</span><span class=g-riw>teal underline: attested in another KFGQPC riw\u0101yah</span><span class=g-un>red: unattested \u2014 review</span><span class=g-na>dashed: not aligned</span></div></header>"
    )
    for pno in sorted(pages):
        recs = pages[pno]
        pg = recs[0]
        out.append(
            f"<section class=page id=p{pno}><div><img loading=lazy src='{images.get(pno, '')}' alt='page {pno}'><div class=meta>pdf p{pno} \u00b7 printed {pg.get('page')} \u00b7 prose round-trip {pg.get('prose_roundtrip')}</div></div><div class=ext>"
        )
        for r in recs:
            if r["type"] == "row":
                ws = []
                for w in r["words"]:
                    t = w["t"]
                    runs = sorted(w.get("colors", []), key=lambda x: x["s"])
                    s, pos = "", 0
                    for c in runs:
                        if c["s"] < pos:
                            continue
                        s += (
                            html.escape(t[pos : c["s"]])
                            + f"<span style='color:{c['hex']}'>{html.escape(t[c['s'] : c['e']])}</span>"
                        )
                        pos = c["e"]
                    s += html.escape(t[pos:])
                    hafs = canon.word(w["spine"]).text if w.get("spine") else ""
                    tip = (
                        f"{w.get('spine', '\u2014')} \u00b7 {w['grade']}"
                        + (f" \u00b7 {','.join(w['riwayat'])}" if w.get("riwayat") else "")
                        + (
                            f" \u00b7 \u1e24af\u1e63: {hafs}"
                            if hafs and not w["grade"].startswith("hafs-strict")
                            else ""
                        )
                    )
                    ws.append(
                        f"<span class='w {GRADE_STYLE.get(w['grade'], '')}' title='{html.escape(tip)}'>{s}</span>"
                    )
                anchors = "; ".join(f"{a['surah']}:{a['ayah']}" for a in r["ayahs"] if a["surah"])
                out.append(
                    f"<div class='row{' gs' if r['id'] == r['group'] else ''}'><span class=n>{html.escape(r['n'])}</span>{' '.join(ws)}<div class=rd>{html.escape(r['readers'])} <span class=meta>[{anchors}]</span></div></div>"
                )
            elif r["type"] == "footnote":
                out.append(f"<div class=fn>{html.escape(r['text'])}</div>")
            elif r["type"] == "text":
                out.append(f"<div class=tx>{html.escape(r['text'])}</div>")
        out.append("</div></section>")
    out.append("</body></html>")
    return "\n".join(out)


def write_jsonl(path: Path, records: list[dict]) -> None:
    with path.open("w") as f:
        for r in records:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
