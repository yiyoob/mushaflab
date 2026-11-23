"""M0.6: the Warsh prototype for al-Fatihah and al-Baqarah 1-20 (PROJECT_PLAN v3, M0).

What it measures: every place where KFGQPC's Warsh text differs from KFGQPC's Hafs text in the
range, and whether the books explain it. The differences are read from the legacy raw patches
(data/patches/raw/warsh.yaml, D20: an exact codepoint diff of the two pinned KFGQPC texts); they
are only read here, never extended (rule 12). Each one is classified:

  numbering    an ayah-number sign, an ayah-end item, or a word separator (adad, layout)
  convention   only how a sign is written: the vowelless sign's codepoint, hamzat al-wasl
               written as alef plus marks, or a waqf sign (writing style, D72)
  book         the word is in a row of al-Futuh that names Warsh among its readers
               (the row's word agrees, or disagrees, with what KFGQPC Warsh prints)
  candidate    a change of signs or of a letter's written form at a place al-Futuh lists no
               khilaf for: most likely a writing convention, labelled by its observable pattern
               only (what a sign means is never asserted here, G-226); for review
  unexplained  none of the above

It also writes the range's farsh entries, built from al-Futuh's khilaf segments
(corpus/farsh/001.yaml, 002.yaml), and looks for each entry's Hafs word on al-Budur's pages for
the range as a candidate corroboration (OCR text; candidates are listed, never asserted).
Nothing here is typed from memory (rule 1): every reading comes from a book row or a pinned file.

    python -m pipeline.m0_warsh [<al-futuh jsonl>]
"""

import json
import re
import sys
from collections import Counter, defaultdict
from difflib import SequenceMatcher
from pathlib import Path

import yaml

from pipeline import corpus_words
from pipeline.clusterize import clusterize
from pipeline.codepoints import load_table

REPO = corpus_words.REPO
FIRST, LAST = (1, 1), (2, 20)
RAW_WARSH = REPO / "data" / "patches" / "raw" / "warsh.yaml"
FUTUH = REPO / "build" / "books" / "al-futuh-al-bahira" / "vol1m0.jsonl"
BUDUR_TXT = REPO / "data" / "sources" / "raw" / "books" / "al-budur-al-zahirah" / "aljam3-3480.txt"
BUDUR_PAGES = range(14, 20)
REPORT = REPO / "docs" / "review" / "m0" / "warsh-residuals.md"
FARSH_DIR = REPO / "corpus" / "farsh"

VOWELLESS = {"U+0652", "U+06E1"}
STOPS = {"U+06D6", "U+06D7", "U+06D8", "U+06DA", "U+06DB"}
# Codepoints ignored when comparing a word's letters (rasm-level comparison, OCR search).
_MARKS = re.compile("[ً-ٰٟۖ-ۭـ]")
_ALEFS = str.maketrans(dict.fromkeys("ٱأإآ", chr(0x0627)))
MIN_ROWS = 2  # a segment with one row has no khilaf
MIN_LETTERS = 3  # shorter words match too much OCR text to be worth a candidate


def in_range(sa: tuple[int, int]) -> bool:
    return FIRST <= sa <= LAST


def sa_of(word_id: str) -> tuple[int, int]:
    s, a = word_id.split(":")[:2]
    return int(s), int(a)


def word_of(target: str) -> str:
    """The word id (s:a:w) of a patch target (an item or a cluster, maybe +ins)."""
    return ":".join(target.split("+", 1)[0].split(":")[:3])


def bare(text: str) -> str:
    return _MARKS.sub("", text).translate(_ALEFS)


def words() -> dict[str, tuple[str, str]]:
    out = {}
    for line in corpus_words.OUT.read_text(encoding="utf-8").split("\n"):
        if not line or line.startswith(("#", "word_id\t")):
            continue
        wid, kind, text = line.split("\t")[:3]
        if in_range(sa_of(wid)):
            out[wid] = (kind, text)
    return out


def warsh_diffs() -> list[dict]:
    pat = re.compile(r'target: "(\d+):(\d+):')
    out = []
    with RAW_WARSH.open(encoding="utf-8") as f:
        for line in f:
            m = pat.search(line)
            if m and in_range((int(m.group(1)), int(m.group(2)))):
                out.append(yaml.safe_load(line.strip()[2:]))
    return out


def futuh_rows(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").split("\n"):
        if not line:
            continue
        r = json.loads(line)
        if r["type"] == "row" and any(
            w.get("spine") and in_range(sa_of(w["spine"])) for w in r["words"]
        ):
            rows.append(r)
    return rows


# One return per class, in order of precedence.
def classify(  # noqa: PLR0911
    d: dict, hafs: dict[str, tuple[str, str]], book: dict[str, list[dict]]
) -> tuple[str, str]:
    op, target, payload = d["op"], d["target"], d.get("payload") or {}
    wid = word_of(target)
    item_kind = hafs.get(wid, ("", ""))[0]
    if item_kind == "ayah_end" or payload.get("kind") == "ayah_number":
        return "numbering", "ayah-end item or ayah-number sign"
    if op == "insert_after" and (payload.get("item") or {}).get("kind") == "ayah_end":
        return "numbering", "ayah end inserted"
    if op == "replace" and set(payload) <= {"sep_after", "sep_before"} and payload:
        return "numbering", "word separator"
    if op == "set_marks":
        rem = {m if isinstance(m, str) else m["cp"] for m in payload.get("remove", [])}
        add = {m if isinstance(m, str) else m["cp"] for m in payload.get("add", [])}
        if rem | add and (rem | add) <= VOWELLESS:
            return "convention", "vowelless sign written as another codepoint"
        if rem | add and (rem | add) <= STOPS:
            return "convention", "waqf sign"
    if op == "replace" and payload.get("kind") == "letter":
        cps = payload.get("cps", "").split()
        old = hafs.get(wid, ("", ""))[1]
        if cps[:1] == ["U+0627"] and "U+06EC" in cps and "ٱ" in old:
            return "convention", "hamzat al-wasl written as alef + vowel + U+06EC"
    rows = book.get(wid, [])
    named = [r for r in rows if "warsh" in (r.get("ruwat") or [])]
    if named:
        agree = any(
            "warsh" in (w.get("riwayat") or [])
            for r in named
            for w in r["words"]
            if w.get("spine") == wid
        )
        ids = ", ".join(sorted({r["id"] for r in named}))
        return "book", (
            "agrees with KFGQPC" if agree else "row names Warsh; word differs from KFGQPC"
        ) + f" ({ids})"
    bounded = [r for r in rows if "warsh" in (r.get("ruwat_max") or [])]
    if bounded:
        return "book", "readers bounded, Warsh possible (" + ", ".join(
            sorted({r["id"] for r in bounded})
        ) + ")"
    return candidate(d, hafs)


def candidate(d: dict, hafs: dict[str, tuple[str, str]]) -> tuple[str, str]:
    """A sign-level or letter-form change with no book row at the word, by observable pattern."""
    op, payload = d["op"], d.get("payload") or {}
    if op == "set_marks":
        rem = sorted({m if isinstance(m, str) else m["cp"] for m in payload.get("remove", [])})
        add = sorted({m if isinstance(m, str) else m["cp"] for m in payload.get("add", [])})
        parts = ([f"removes {' '.join(rem)}"] if rem else []) + (
            [f"adds {' '.join(add)}"] if add else []
        )
        return "candidate", "marks: " + ", ".join(parts)
    if op == "replace" and payload.get("kind") == "letter":
        wid = word_of(d["target"])
        n = int(d["target"].split("+")[0].split(":")[3])
        try:
            old = clusterize(hafs[wid][1], load_table())[n - 1].text
        except (KeyError, IndexError, ValueError):
            return "unexplained", ""
        return (
            "candidate",
            f"letter U+{ord(old[0]):04X} written as {payload.get('cps', '').split()[0]}",
        )
    return "unexplained", ""


def edits(hafs_text: str, printed: str, wid: str, table: dict) -> list[dict] | None:
    """Letter edits that turn the Hafs word into the printed variant (codepoints exactly as each
    text has them). None when the printed word cannot be clustered."""
    try:
        a = [c.text for c in clusterize(hafs_text, table)]
        b = [c.text for c in clusterize(printed, table)]
    except ValueError:
        return None
    cp = lambda t: " ".join(f"U+{ord(c):04X}" for c in t)  # noqa: E731
    out: list[dict] = []
    for tag, i1, i2, j1, j2 in SequenceMatcher(a=a, b=b, autojunk=False).get_opcodes():
        if tag == "equal":
            continue
        if tag == "replace" and i2 - i1 == j2 - j1:
            out += [
                {"set": {"letter": f"{wid}:{i1 + k + 1}", "cps": cp(b[j1 + k])}}
                for k in range(i2 - i1)
            ]
            continue
        out += [{"delete": f"{wid}:{i + 1}"} for i in range(i1, i2)]
        anchor = f"{wid}:{i1}" if i1 else f"{wid}:0"
        out += [{"insert": {"after": anchor, "cps": cp(b[j])}} for j in range(j1, j2)]
    return out


def budur_pages() -> dict[int, str]:
    pages = BUDUR_TXT.read_text(encoding="utf-8").split("PAGE_SEPARATOR")
    return {n: bare(pages[n - 1]) for n in BUDUR_PAGES}


def farsh_entries(
    rows: list[dict], hafs: dict, table: dict, budur: dict[int, str]
) -> dict[int, list[dict]]:
    groups: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        groups[r["group"]].append(r)
    by_surah: dict[int, list[dict]] = defaultdict(list)
    for gid, rs in groups.items():
        if len(rs) < MIN_ROWS:
            continue  # one row for all readers: no khilaf at this place
        spine = sorted(
            {
                w["spine"]
                for r in rs
                for w in r["words"]
                if w.get("spine") and in_range(sa_of(w["spine"]))
            },
            key=lambda x: tuple(int(p) for p in x.split(":")),
        )
        if not spine:
            continue
        variants = []
        for r in rs:
            ed: list[dict] = []
            for w in r["words"]:
                sid = w.get("spine")
                if not sid or sid not in hafs or w.get("grade") == "hafs-strict":
                    continue
                e = edits(hafs[sid][1], w.get("t", ""), sid, table)
                ed += e if e is not None else [{"unreadable": sid}]
            loc = f"al-futuh:v{r['vol']}p{r['pdf_page']}r{r['n']}"
            variants.append(
                {
                    "ruwat": sorted(r.get("ruwat") or []),
                    **(
                        {"ruwat_max": sorted(r["ruwat_max"])}
                        if r.get("ruwat_max") and r.get("ruwat_max") != r.get("ruwat")
                        else {}
                    ),
                    "reading": ed or "hafs",
                    "source": loc,
                    "readers_status": (r.get("readers_parse") or {}).get("status"),
                }
            )
        cands = sorted(
            {
                p
                for p, t in budur.items()
                for sid in spine
                if len(bare(hafs[sid][1])) >= MIN_LETTERS and bare(hafs[sid][1]) in t
            }
        )
        s, a = sa_of(spine[0])
        notes = []
        if all(v["reading"] == "hafs" for v in variants):
            notes.append(
                "the variants differ in ways not written as letters (e.g. sakt, wasl, basmalah)"
            )
        if cands:
            notes.append(
                "al-Budur candidate pages (OCR, unverified): " + ", ".join(f"p{p}" for p in cands)
            )
        by_surah[s].append(
            {
                "id": f"farsh.{s}.{a}.{gid}",
                "words": spine,
                "variants": variants,
                "status": "raw-derived",
                "support": "single-source",
                "note": "; ".join(notes),
            }
        )
    return by_surah


def write_farsh(by_surah: dict[int, list[dict]]) -> None:
    FARSH_DIR.mkdir(parents=True, exist_ok=True)
    for s, entries in sorted(by_surah.items()):
        head = (
            f"# Farsh entries for surah {s}, al-Fatihah and al-Baqarah 1-20 only (M0.6).\n"
            "# Generated by `python -m pipeline.m0_warsh` from al-Futuh volume 1 (D66);\n"
            "# readings are letter edits on the Hafs word, from the book's printed word.\n"
            "# Format still provisional (docs/formats.md, M0.6). Do not edit.\n"
        )
        body = yaml.safe_dump(
            entries, allow_unicode=True, sort_keys=False, width=100, default_flow_style=None
        )
        (FARSH_DIR / f"{s:03d}.yaml").write_text(head + body, encoding="utf-8")


def report(diffs: list[dict], results: list[tuple[str, str]], hafs: dict, by_surah: dict) -> str:
    n = len(diffs)
    cls = Counter(c for c, _ in results)
    detail = Counter((c, re.sub(r" \(.*\)$", "", why)) for c, why in results)
    entries = [e for es in by_surah.values() for e in es]
    unreadable = sorted(
        {
            x["unreadable"]
            for e in entries
            for v in e["variants"]
            if isinstance(v["reading"], list)
            for x in v["reading"]
            if "unreadable" in x
        }
    )
    with_warsh = [e for e in entries if any("warsh" in v["ruwat"] for v in e["variants"])]
    lines = [
        "# M0.6: Warsh prototype, al-Fātiḥah and al-Baqarah 1\u201320",
        "",
        "Generated by `python -m pipeline.m0_warsh`. Every difference between KFGQPC's Warsh",
        f"and Ḥafṣ texts in the range ({n}, from the legacy raw patches, D20) is classified.",
        "",
        "| Class | Differences | Share |",
        "| --- | --- | --- |",
    ]
    for c in ("numbering", "convention", "book", "candidate", "unexplained"):
        lines.append(f"| {c} | {cls[c]} | {100 * cls[c] / n:.0f}% |")
    lines += ["", "## By reason", "", "| Class | Reason | Differences |", "| --- | --- | --- |"]
    lines += [
        f"| {c} | {why or '—'} | {k} |"
        for (c, why), k in sorted(detail.items(), key=lambda x: (x[0][0], -x[1]))
    ]
    lines += [
        "",
        "## Farsh entries written",
        "",
        f"{len(entries)} khilāf places in the range from al-Futūḥ (`corpus/farsh/00*.yaml`),",
        f"{len(with_warsh)} of them naming Warsh in a variant; an al-Futūḥ row per variant.",
        "al-Budūr pages where the place's Ḥafṣ word appears (OCR) are candidates only.",
        f"{len(unreadable)} printed words could not be split into letters with the codepoint",
        "table and are marked `unreadable` (al-Futūḥ's glyph decoding, G-219): "
        + (", ".join(f"`{w}`" for w in unreadable) or "none")
        + ".",
        "",
        "Book rows whose word differs from KFGQPC are expected at this stage: al-Futūḥ prints its",
        "variants in Madinah ḍabṭ, KFGQPC's Warsh muṣḥaf in Maghribī ḍabṭ. Closing that gap is the",
        "Warsh writing-style table's job (M2), not the readings'.",
        "",
        "## Convention candidates and unexplained differences",
        "",
        "| Target | Class | Op | Ḥafṣ word | Change |",
        "| --- | --- | --- | --- | --- |",
    ]
    for d, (c, _) in zip(diffs, results, strict=True):
        if c in ("candidate", "unexplained"):
            wid = word_of(d["target"])
            change = json.dumps(d.get("payload"), ensure_ascii=False)
            word = hafs.get(wid, ("", ""))[1]
            lines.append(f"| `{d['target']}` | {c} | {d['op']} | {word} | `{change}` |")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    futuh = Path(argv[0]) if argv else FUTUH
    table = load_table()
    hafs = words()
    diffs = warsh_diffs()
    rows = futuh_rows(futuh)
    book: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        for w in r["words"]:
            if w.get("spine"):
                book[w["spine"]].append(r)
    results = [classify(d, hafs, book) for d in diffs]
    by_surah = farsh_entries(rows, hafs, table, budur_pages())
    write_farsh(by_surah)
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(report(diffs, results, hafs, by_surah), encoding="utf-8")
    print(f"m0_warsh: {len(diffs)} differences: {dict(Counter(c for c, _ in results))}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
