"""T1.3: attach the 1441H line layout (QUL V4) to the Hafs spine.

    python -m pipeline.ingest_layout build   # write data/layouts/uthmani-1441h.json
    python -m pipeline.ingest_layout check   # exit 1 if it differs from a rebuild

QUL's layout names lines by QUL word ids. Each QUL word is mapped to the spine words whose
letters it covers (normalize.skeleton), which handles QUL's different word splitting
without special cases. Every mapped word's text is then compared exactly; differences must
fall in KNOWN_DIFFERENCES or the build stops. Owner-verified corrections are applied from
uthmani-1441h.corrections.json, each with its evidence and a guard on the old value.
"""

import json
import sqlite3
import sys
import tarfile
from collections import Counter
from pathlib import Path

from pipeline import ingest_kfgqpc, sources, spine
from pipeline.normalize import skeleton

LAYOUT_ID = "uthmani-1441h"
LAYOUTS_DIR = Path(__file__).resolve().parents[1] / "layouts"
LAYOUT_PATH = LAYOUTS_DIR / f"{LAYOUT_ID}.json"
CORRECTIONS_PATH = LAYOUTS_DIR / f"{LAYOUT_ID}.corrections.json"
QUL_LAYOUT = "qul-layout-19-v4-1441h"
QUL_WORDS = "qul-script-312-kfgqpc-hafs-wbw"
QCF4 = "qcf4-npm-1.1.0"
HIZB, SAJDAH, TATWEEL = "\u06de", "\u06e9", "\u0640"
KNOWN_DIFFERENCES = ("hizb_prefix", "sajdah_space", "v18_tatweel", "segmentation")
LINE_TYPES = {"surah_name": "surah_name", "basmallah": "basmalah", "ayah": "text"}


class LayoutError(ValueError):
    pass


# --- alignment ------------------------------------------------------------------------


def align_ayah(spine_words: list[tuple[str, str]], qul_words: list[tuple[int, str]]) -> dict:
    """Map each QUL word id to the spine word ids whose skeleton letters it covers."""
    sk_spine = [skeleton(t) for _, t in spine_words]
    sk_qul = [skeleton(t) for _, t in qul_words]
    if "".join(sk_spine) != "".join(sk_qul) or not all(sk_spine) or not all(sk_qul):
        raise LayoutError("skeleton mismatch between QUL and spine words")
    spans, start = [], 0
    for (sid, _), sk in zip(spine_words, sk_spine, strict=True):
        spans.append((start, start + len(sk), sid))
        start += len(sk)
    mapping, start = {}, 0
    for (qid, _), sk in zip(qul_words, sk_qul, strict=True):
        end = start + len(sk)
        mapping[qid] = [sid for a, b, sid in spans if a < end and start < b]
        start = end
    return mapping


def check_boundary(mapping: dict, last_qid: int, next_qid: int) -> None:
    if set(mapping.get(last_qid, [])) & set(mapping.get(next_qid, [])):
        raise LayoutError(f"line break inside a spine word (QUL {last_qid}/{next_qid})")


# --- inputs -----------------------------------------------------------------------------


def spine_records() -> list[dict]:
    return spine.load_records()


def _spine_by_ayah() -> dict[tuple[int, int], dict]:
    ayahs: dict[tuple[int, int], dict] = {}
    for r in spine_records():
        s, a, _ = (int(x) for x in r["id"].split(":"))
        entry = ayahs.setdefault((s, a), {"hizb": None, "words": [], "end": None})
        text = "".join(spine.from_cps(c["cps"]) for c in r["clusters"])
        if r["kind"] == "hizb":
            entry["hizb"] = r["id"]
        elif r["kind"] == "word":
            entry["words"].append((r["id"], text))
        else:
            entry["end"] = r["id"]
    return ayahs


def _qul_by_ayah() -> dict[tuple[int, int], dict]:
    ayahs: dict[tuple[int, int], dict] = {}
    for qid, (s, a, pos, text) in sorted(sources.qul_hafs_words().items()):
        entry = ayahs.setdefault((s, a), {"words": [], "marker": None})
        if sources.is_ayah_number(text):
            entry["marker"] = qid
        else:
            entry["words"].append((pos, qid, text))
    for entry in ayahs.values():
        entry["words"] = [(qid, text) for _, qid, text in sorted(entry["words"])]
    return ayahs


def qul_layout_rows() -> list[tuple]:
    con = sqlite3.connect(":memory:")
    con.deserialize(sources.zip_member(QUL_LAYOUT, ".db"))
    rows = con.execute(
        "select page_number, line_number, line_type, is_centered, first_word_id,"
        " last_word_id, surah_number from pages order by page_number, line_number"
    ).fetchall()
    con.close()
    return [tuple(None if v == "" else v for v in row) for row in rows]


# --- mapping and text check -------------------------------------------------------------


def _mappings() -> tuple[dict, dict, dict]:
    """(QUL id -> (first item, last item), QUL id -> spine word ids, per-ayah groups)."""
    spine_ayahs, qul_ayahs = _spine_by_ayah(), _qul_by_ayah()
    if set(spine_ayahs) != set(qul_ayahs):
        raise LayoutError("QUL and spine cover different ayahs")
    bounds: dict[int, tuple[str, str]] = {}
    word_map: dict[int, list[str]] = {}
    for key, sp in spine_ayahs.items():
        q = qul_ayahs[key]
        mapping = align_ayah(sp["words"], q["words"])
        word_map.update(mapping)
        for n, (qid, _) in enumerate(q["words"]):
            first = sp["hizb"] if n == 0 and sp["hizb"] else mapping[qid][0]
            bounds[qid] = (first, mapping[qid][-1])
        bounds[q["marker"]] = (sp["end"], sp["end"])
    return bounds, word_map, spine_ayahs


def _explain(qul_texts: list[str], spine_texts: list[str]) -> list[str] | None:
    """Categories that explain the difference, [] if identical, None if unexplained."""
    cats = []
    q = list(qul_texts)
    if q[0].startswith(HIZB + " "):
        q[0] = q[0][2:]
        cats.append("hizb_prefix")
    if any(" " + SAJDAH in t for t in q):
        q = [t.replace(" " + SAJDAH, SAJDAH) for t in q]
        cats.append("sajdah_space")
    if len(q) != len(spine_texts) or any(" " in t for t in q):
        if "".join(t.replace(" ", "") for t in q) != "".join(spine_texts):
            return None
        cats.append("segmentation")
        return cats
    if q != spine_texts:
        if [t.replace(TATWEEL, "") for t in q] != spine_texts:
            return None
        cats.append("v18_tatweel")
    return cats


def text_report() -> dict:
    _, word_map, spine_ayahs = _mappings()
    spine_text = {sid: t for sp in spine_ayahs.values() for sid, t in sp["words"]}
    qul_text = {qid: t for qid, (_s, _a, _p, t) in sources.qul_hafs_words().items()}
    groups: dict[tuple, list[int]] = {}
    for qid, sids in word_map.items():
        groups.setdefault(tuple(sids), []).append(qid)
    # merge groups that share spine words (QUL words split from one spine word)
    merged: dict[str, set] = {}
    for sids, qids in groups.items():
        key = sids[0]
        merged.setdefault(key, set()).update(qids)
    categories: Counter = Counter()
    unexplained = []
    for qids in merged.values():
        qids_sorted = sorted(qids)
        sids = sorted({s for q in qids_sorted for s in word_map[q]}, key=_id_key)
        cats = _explain([qul_text[q] for q in qids_sorted], [spine_text[s] for s in sids])
        if cats is None:
            unexplained.append(sids)
        else:
            categories.update(cats or ["exact"])
    return {"categories": dict(categories), "unexplained": unexplained}


def _id_key(item_id: str) -> tuple[int, ...]:
    return tuple(int(x) for x in item_id.split(":"))


# --- layout -----------------------------------------------------------------------------


def _corrections() -> list[dict]:
    return json.loads(CORRECTIONS_PATH.read_text(encoding="utf-8"))["corrections"]


def build_layout() -> dict:
    bounds, word_map, _ = _mappings()
    lines = []
    prev_last_qid = None
    for page, line, kind, centered, first_id, last_id, surah in qul_layout_rows():
        rec: dict = {"page": page, "line": line, "type": LINE_TYPES[kind]}
        if kind == "surah_name":
            rec["surah"] = surah
        if kind == "ayah":
            first, last = int(first_id), int(last_id)
            if prev_last_qid is not None:
                check_boundary(word_map, prev_last_qid, first)
            rec["first"], rec["last"] = bounds[first][0], bounds[last][1]
            prev_last_qid = last
        rec["centered"] = bool(centered)
        lines.append(rec)
    by_key = {(r["page"], r["line"]): r for r in lines}
    for c in _corrections():
        rec = by_key[(c["page"], c["line"])]
        if rec[c["end"]] != c["expect"]:
            raise LayoutError(f"correction guard failed at {c['page']}:{c['line']}")
        rec[c["end"]] = c["set"]
        rec["correction"] = c["evidence"]
    return {
        "layout_id": LAYOUT_ID,
        "script_id": spine.SCRIPT_ID,
        "spine": f"hafs/v{spine.SPINE_VERSION}",
        "pages": max(r["page"] for r in lines),
        "lines_per_page": max(r["line"] for r in lines),
        "sources": {"layout": QUL_LAYOUT, "words": QUL_WORDS},
        "lines": lines,
    }


def write_layout(layout: dict) -> None:
    head = {k: v for k, v in layout.items() if k != "lines"}
    body = ",\n".join("    " + json.dumps(r, ensure_ascii=True) for r in layout["lines"])
    text = json.dumps(head, ensure_ascii=True, indent=2)[:-2]
    LAYOUT_PATH.write_text(text + ',\n  "lines": [\n' + body + "\n  ]\n}\n", encoding="ascii")


def load_layout() -> dict:
    return json.loads(LAYOUT_PATH.read_text(encoding="ascii"))


def _items_on_lines(layout: dict) -> dict[str, tuple[int, int]]:
    order = [r["id"] for r in spine_records()]
    index = {item_id: n for n, item_id in enumerate(order)}
    where = {}
    for rec in layout["lines"]:
        if rec["type"] == "text":
            for n in range(index[rec["first"]], index[rec["last"]] + 1):
                where[order[n]] = (rec["page"], rec["line"])
    return where


# --- cross-checks -----------------------------------------------------------------------


def qcf4_disagreements() -> list[tuple[int, int]]:
    """Lines where quran-qcf4 (1441H print glyph data) puts different first/last items."""
    bounds, _, spine_ayahs = _mappings()
    loc = {(s, a, p): qid for qid, (s, a, p, _t) in sources.qul_hafs_words().items()}
    mine = {(r["page"], r["line"]): r for r in load_layout()["lines"]}
    out = []
    with tarfile.open(sources.path(QCF4)) as tf:
        for page in range(1, len({p for p, _ in mine}) + 1):
            member = tf.extractfile(f"package/pages/{page:03d}.json")
            if member is None:
                raise LayoutError(f"qcf4 page {page} missing")
            for line in json.load(member)["lines"]:
                ends = _qcf4_line_items(line["words"], bounds, loc, spine_ayahs)
                rec = mine[(page, line["line"])]
                if (ends is None) != (rec["type"] != "text") or (
                    ends and ends != (rec["first"], rec["last"])
                ):
                    out.append((page, line["line"]))
    return out


def _qcf4_line_items(words: list[dict], bounds: dict, loc: dict, spine_ayahs: dict):  # noqa: ANN202
    items = []
    for w in words:
        if w["type"] in ("surah_header", "bismillah"):
            return None
        s, a = (int(x) for x in w["verse_key"].split(":"))
        if w["type"] == "quarter":
            items.append((f"{s}:{a}:0", f"{s}:{a}:0"))
        elif w["type"] == "end":
            end = spine_ayahs[(s, a)]["end"]
            items.append((end, end))
        elif not w["text"].startswith("#"):  # '#' marks qcf4's separate sajdah glyph
            items.append(bounds[loc[(s, a, int(w["position"]))]])
    return (items[0][0], items[-1][1]) if items else None


def kfgqpc_page_disagreements() -> list[str]:
    """Ayahs whose KFGQPC page/line_start/line_end differ from this layout."""
    where = _items_on_lines(load_layout())
    by_ayah: dict[tuple[int, int], list[str]] = {}
    for r in spine_records():
        s, a, _ = (int(x) for x in r["id"].split(":"))
        by_ayah.setdefault((s, a), []).append(r["id"])
    out = []
    for row in ingest_kfgqpc.load_rows():
        ids = by_ayah[(int(row["sura_no"]), int(row["aya_no"]))]
        (p0, l0), (p1, l1) = where[ids[0]], where[ids[-1]]
        theirs = (str(row["page"]), int(row["line_start"]), int(row["line_end"]))
        mine = (str(p0) if p0 == p1 else f"{p0}-{p1}", l0, l1)
        if mine != theirs:
            out.append(f"{row['sura_no']}:{row['aya_no']} kfgqpc={theirs} layout={mine}")
    return out


def main(argv: list[str]) -> int:
    layout = build_layout()
    if argv[1:] == ["build"]:
        write_layout(layout)
        print(f"layout: {len(layout['lines'])} lines written to {LAYOUT_PATH}")
        return 0
    if load_layout() != layout:
        print("layout: committed file differs from a rebuild", file=sys.stderr)
        return 1
    print("layout: up to date")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
