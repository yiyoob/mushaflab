"""Spike S1 (T0.3): which edition do the text, layouts and fonts follow?

    uv run --directory data python -m spikes.s1_editions > docs/spikes/s1-editions.md

Everything reported here is computed from pinned sources. Nothing is asserted from memory.
"""

import re
import sys
from collections import Counter
from itertools import combinations

from spikes.common import (
    extracted,
    kfgqpc_official,
    load_json,
    only,
    qul_layout,
    verified_sources,
)

LAYOUTS = {
    "V1 1405H": "qul-layout-15-v1-1405h",
    "V2 1421H": "qul-layout-10-v2-1421h",
    "V4 1441H": "qul-layout-19-v4-1441h",
    "DK V2": "qul-layout-21-digitalkhatt-v2",
}
NBSP = "\u00a0"
HIZB = "\u06de"
EXAMPLES = 8


def out(line: str = "") -> None:
    print(line)


def qul_words() -> dict[int, tuple[int, int, int, str]]:
    path = only(sorted(extracted("qul-script-312-kfgqpc-hafs-wbw").glob("*.json")))
    raw = load_json(path)
    return {
        w["id"]: (int(w["surah"]), int(w["ayah"]), int(w["word"]), w["text"])
        for w in raw.values()  # type: ignore[union-attr]
    }


def ayah_number_like(text: str) -> bool:
    return all("\u0660" <= c <= "\u0669" for c in text)


def kfgqpc_mirror_hafs() -> list[dict]:
    rows = load_json(verified_sources()["kfgqpc-mirror-hafs-data"])
    return [
        {**r, "sura_no": r["sora"]}
        for r in rows  # type: ignore[union-attr]
    ]


def layouts_section(layouts: dict[str, list[tuple]]) -> None:
    out("## 1. QUL layouts compared line by line")
    out()
    out("| Layout | Lines | Pages | Line types |")
    out("|---|---|---|---|")
    for name, rows in layouts.items():
        types = Counter(r[2] for r in rows)
        pages = len({r[0] for r in rows})
        out(f"| {name} | {len(rows)} | {pages} | {dict(sorted(types.items()))} |")
    out()
    out("Lines whose (type, first word, last word) differ, per pair:")
    out()
    out("| Pair | Differing lines |")
    out("|---|---|")
    for a, b in combinations(layouts, 2):
        ka = {(r[0], r[1]): (r[2], r[4], r[5]) for r in layouts[a]}
        kb = {(r[0], r[1]): (r[2], r[4], r[5]) for r in layouts[b]}
        diff = sum(1 for k in ka.keys() | kb.keys() if ka.get(k) != kb.get(k))
        out(f"| {a} vs {b} | {diff} |")
    out()


def ayah_positions(
    rows: list[tuple], words: dict[int, tuple[int, int, int, str]]
) -> dict[tuple[int, int], tuple[int, int, int]]:
    """(surah, ayah) -> (page, first line, last line) according to a layout."""
    where: dict[int, tuple[int, int]] = {}
    for page, line, kind, _c, first, last, _s in rows:
        if kind == "ayah":
            for wid in range(first, last + 1):
                where[wid] = (page, line)
    pos: dict[tuple[int, int], tuple[int, int, int]] = {}
    for wid in sorted(where):
        s, a, _w, _t = words[wid]
        page, line = where[wid]
        if (s, a) not in pos:
            pos[(s, a)] = (page, line, line)
        else:
            p0, l0, _ = pos[(s, a)]
            pos[(s, a)] = (p0, l0, line)
    return pos


def kfgqpc_page_section(
    layouts: dict[str, list[tuple]],
    words: dict[int, tuple[int, int, int, str]],
    texts: dict[str, list[dict]],
) -> None:
    out("## 2. KFGQPC per-ayah page/line fields vs each layout")
    out()
    out(
        "For each ayah, KFGQPC gives `page`, `line_start`, `line_end`. Each QUL layout gives "
        "the page and line of the ayah's first and last word. An exact match on all 6236 "
        "ayahs identifies the printing the KFGQPC page data follows."
    )
    out()
    out("| KFGQPC text | " + " | ".join(layouts) + " |")
    out("|---|" + "---|" * len(layouts))
    positions = {name: ayah_positions(rows, words) for name, rows in layouts.items()}
    mismatches: dict[tuple[str, str], list[str]] = {}
    for tname, rows in texts.items():
        cells = []
        for lname, pos in positions.items():
            ok = 0
            bad = []
            for r in rows:
                key = (int(r["sura_no"]), int(r["aya_no"]))
                mine = (int(r["page"]), int(r["line_start"]), int(r["line_end"]))
                if pos.get(key) == mine:
                    ok += 1
                else:
                    bad.append(f"{key[0]}:{key[1]} kfgqpc={mine} layout={pos.get(key)}")
            cells.append(f"{ok}/{len(rows)}")
            mismatches[(tname, lname)] = bad
        out(f"| {tname} | " + " | ".join(cells) + " |")
    out()
    for (tname, lname), bad in mismatches.items():
        if bad and len(bad) <= 200:
            out(f"First mismatches, {tname} vs {lname}: " + "; ".join(bad[:EXAMPLES]))
            out()


def qcf4_section(layouts: dict[str, list[tuple]], words: dict[int, tuple]) -> None:
    out("## 3. quran-qcf4 (1441H print, word glyphs) vs QUL layouts")
    out()
    loc_to_id = {(s, a, w): wid for wid, (s, a, w, _t) in words.items()}
    pages_dir = extracted("qcf4-npm-1.1.0") / "package" / "pages"
    qcf_lines: dict[tuple[int, int], tuple[str, int | None, int | None]] = {}
    unmapped = 0
    for page in range(1, 605):
        data = load_json(pages_dir / f"{page:03d}.json")
        for line in data["lines"]:  # type: ignore[index]
            kinds = {w["type"] for w in line["words"]}
            if "surah_header" in kinds:
                qcf_lines[(page, line["line"])] = ("surah_name", None, None)
                continue
            if "bismillah" in kinds:
                qcf_lines[(page, line["line"])] = ("basmallah", None, None)
                continue
            ids = []
            for w in line["words"]:
                if w["type"] not in ("word", "end"):
                    continue
                s, a = (int(x) for x in w["verse_key"].split(":"))
                wid = loc_to_id.get((s, a, int(w["position"])))
                if wid is None:
                    unmapped += 1
                else:
                    ids.append(wid)
            first, last = (min(ids), max(ids)) if ids else (None, None)
            qcf_lines[(page, line["line"])] = ("ayah", first, last)
    out(f"qcf4 lines: {len(qcf_lines)}; word entries not found in the QUL word list: {unmapped}")
    out()
    out("| Layout | Lines identical to qcf4 | Differing |")
    out("|---|---|---|")
    for name, rows in layouts.items():
        mine = {(r[0], r[1]): (r[2], r[4], r[5]) for r in rows}
        same = sum(1 for k, v in qcf_lines.items() if mine.get(k) == v)
        differing = len(mine.keys() | qcf_lines.keys()) - same
        out(f"| {name} | {same} | {differing} |")
    out()


def dk_text_pages(source_id: str) -> list[list[str]]:
    text = verified_sources()[source_id].read_text(encoding="utf-8")
    body = text[text.index("[") : text.rindex("]") + 1]
    pages = []
    for page in re.findall(r"\[\s*((?:'[^']*',?\s*)+)\]", body):
        pages.append(re.findall(r"'([^']*)'", page))
    return pages


def dk_section(layouts: dict[str, list[tuple]]) -> None:
    out("## 4. DigitalKhatt's own text: line breaks vs QUL layouts")
    out()
    out(
        "DigitalKhatt ships its text as pages of lines. Per text line we count words "
        "(space-separated tokens, ayah-number tokens included, a standalone \u06de excluded) "
        "and compare with the layout's word count for that line."
    )
    out()
    out(
        "| DigitalKhatt text | Pages | Lines | " + " | ".join(f"same count as {n}" for n in layouts)
    )
    out("|---|---|---|" + "---|" * len(layouts))
    for sid in ("digitalkhatt-text-oldmadina", "digitalkhatt-text-madina"):
        pages = dk_text_pages(sid)
        total = sum(len(p) for p in pages)
        cells = []
        for rows in layouts.values():
            by_key = {(r[0], r[1]): r for r in rows}
            same = 0
            for pno, lines in enumerate(pages, start=1):
                for lno, line in enumerate(lines, start=1):
                    r = by_key.get((pno, lno))
                    if r is None:
                        continue
                    if r[2] != "ayah":
                        same += 1  # header/basmalah lines are not word-counted
                        continue
                    n = len([t for t in line.split(" ") if t and t != HIZB])
                    same += n == r[5] - r[4] + 1
            cells.append(f"{same}/{total}")
        out(f"| {sid} | {len(pages)} | {total} | " + " | ".join(cells) + " |")
    out()


def kfgqpc_words(row: dict) -> list[str]:
    """Words of a KFGQPC ayah without the ayah-number token or a standalone \u06de."""
    tokens = [t for t in re.split(f"[ {NBSP}]", row["aya_text"]) if t]
    if tokens and (
        ayah_number_like(tokens[-1]) or all("\ufc00" <= c <= "\ufdff" for c in tokens[-1])
    ):
        tokens = tokens[:-1]
    return [t for t in tokens if t != HIZB]


def words_section(words: dict[int, tuple], texts: dict[str, list[dict]]) -> None:
    out("## 5. Word segmentation and text: KFGQPC vs the QUL word list")
    out()
    by_ayah: dict[tuple[int, int], list[str]] = {}
    for _wid, (s, a, _w, t) in sorted(words.items()):
        if ayah_number_like(t):
            continue
        by_ayah.setdefault((s, a), []).append(t.removeprefix(HIZB + " "))
    out(f"QUL word list: {len(words)} entries, {len(by_ayah)} ayahs")
    out()
    out("| KFGQPC text | Ayahs | Same word count | Identical words (codepoint-exact) |")
    out("|---|---|---|---|")
    examples: dict[str, list[str]] = {}
    for tname, rows in texts.items():
        same_count = identical = 0
        ex = []
        for r in rows:
            key = (int(r["sura_no"]), int(r["aya_no"]))
            mine, theirs = kfgqpc_words(r), by_ayah.get(key, [])
            same_count += len(mine) == len(theirs)
            identical += mine == theirs
            if mine != theirs and len(ex) < EXAMPLES:
                diffs = [
                    f"w{i + 1} {x!r}\u2260{y!r}"
                    for i, (x, y) in enumerate(zip(mine, theirs, strict=False))
                    if x != y
                ]
                ex.append(f"{key[0]}:{key[1]} ({len(mine)} vs {len(theirs)} words) {diffs[:2]}")
        examples[tname] = ex
        out(f"| {tname} | {len(rows)} | {same_count} | {identical} |")
    out()
    for tname, ex in examples.items():
        if ex:
            out(f"Examples, {tname}: " + "; ".join(ex))
            out()


def versions_section(texts: dict[str, list[dict]]) -> None:
    out("## 6. KFGQPC official v2.0 vs 2022 mirror v18 (\u1e24af\u1e63)")
    out()
    v2 = {(int(r["sura_no"]), int(r["aya_no"])): r for r in texts["official v2.0"]}
    v18 = {(int(r["sura_no"]), int(r["aya_no"])): r for r in texts["mirror v18"]}
    out(f"Ayahs: v2.0 {len(v2)}, v18 {len(v18)}")
    word_diff = [k for k in v2 if kfgqpc_words(v2[k]) != kfgqpc_words(v18.get(k, {"aya_text": ""}))]
    page_diff = [
        k
        for k in v2
        if (v2[k]["page"], v2[k]["line_start"], v2[k]["line_end"])
        != (int(v18[k]["page"]), int(v18[k]["line_start"]), int(v18[k]["line_end"]))
    ]
    out(f"Ayahs whose words differ (ayah-number token excluded): {len(word_diff)}")
    out()
    out("| Ayah | Word | v2.0 | v18 |")
    out("|---|---|---|---|")
    for k in word_diff:
        a, b = kfgqpc_words(v2[k]), kfgqpc_words(v18[k])
        for i, (x, y) in enumerate(zip(a, b, strict=False)):
            if x != y:
                out(f"| {k[0]}:{k[1]} | {i + 1} | {cps(x)} | {cps(y)} |")
        if len(a) != len(b):
            out(f"| {k[0]}:{k[1]} | count | {len(a)} words | {len(b)} words |")
    out()
    out(f"Ayahs whose page/line fields differ: {len(page_diff)}")
    out()
    for tname, rows in texts.items():
        seps = Counter()
        marks = Counter()
        for r in rows:
            t = r["aya_text"]
            cut = max(t.rfind(" "), t.rfind(NBSP))
            seps[f"U+{ord(t[cut]):04X}"] += 1
            last = t[cut + 1 :]
            marks[
                "U+FC00 block"
                if all("\ufc00" <= c <= "\ufdff" for c in last)
                else "digits"
                if ayah_number_like(last)
                else "other"
            ] += 1
        out(f"{tname}: separator before ayah-number token {dict(seps)}; token kind {dict(marks)}")
        out()


def cps(word: str) -> str:
    """A word with every non-letter shown as its codepoint, so mark differences are visible."""
    return "".join(
        c if "\u0621" <= c <= "\u064a" or c == "\u0671" else f"\u27e8{ord(c):04X}\u27e9"
        for c in word
    )


# Skeleton normalization for this spike only (the pipeline gets its one documented function in
# T1.2). Keeps base letters, maps hamza seats and alif variants to their carrier letter, and
# maps ya/alif maqsura to one form, because the four sources encode these differently.
SKELETON_MAP = str.maketrans(
    {
        "\u0623": "\u0627",  # \u0623
        "\u0625": "\u0627",  # \u0625
        "\u0622": "\u0627",  # \u0622
        "\u0671": "\u0627",  # \u0671
        "\u0624": "\u0648",  # \u0624
        "\u0626": "\u0649",  # \u0626
        "\u064a": "\u0649",  # \u064a
        "\u06cc": "\u0649",  # \u06cc
    }
)


def skeleton(word: str) -> str:
    w = word.translate(SKELETON_MAP)
    # Base letters only; standalone hamza (U+0621) and tatweel (U+0640) are not rasm.
    return "".join(c for c in w if "\u0622" <= c <= "\u064a" and c != "\u0640")


def dk_ayahs(source_id: str, layout: list[tuple], counts: list[int]) -> dict:
    """Split DigitalKhatt's page/line text into ayahs using its \u06ddN markers."""
    types = {(r[0], r[1]): r[2] for r in layout}
    tokens: list[str] = []
    for pno, lines in enumerate(dk_text_pages(source_id), start=1):
        for lno, line in enumerate(lines, start=1):
            if types.get((pno, lno)) == "ayah":
                tokens.extend(t for t in line.split(" ") if t and t != HIZB)
    ayahs: dict[tuple[int, int], list[str]] = {}
    keys = [(s, a) for s, n in enumerate(counts, start=1) for a in range(1, n + 1)]
    current: list[str] = []
    for t in tokens:
        if "\u06dd" not in t:
            current.append(t)
            continue
        # The marker may share a token with a preceding sign, e.g. sajdah + marker.
        if t.index("\u06dd") > 0:
            current.append(t[: t.index("\u06dd")])
        if not keys:
            break
        ayahs[keys.pop(0)] = current
        current = []
    return ayahs


def skeleton_section(texts: dict[str, list[dict]]) -> None:
    out("## 7. Letter skeleton (rasm) per word: KFGQPC v2.0 vs other texts")
    out()
    out(
        "Marks removed; hamza seats, alif variants and ya/alif maqsura unified (spike-only "
        "normalization, see `skeleton()`). Same skeleton = same letters in the same words; "
        "differences need owner review and may still be encoding artifacts."
    )
    out()
    v2 = {(int(r["sura_no"]), int(r["aya_no"])): kfgqpc_words(r) for r in texts["official v2.0"]}
    counts = [0] * 114
    for s, a in v2:
        counts[s - 1] = max(counts[s - 1], a)
    tanzil: dict[tuple[int, int], list[str]] = {}
    for line in verified_sources()["tanzil-uthmani-1.1"].read_text(encoding="utf-8").splitlines():
        parts = line.split("|")
        if len(parts) == 3 and parts[0].isdigit():
            tanzil[(int(parts[0]), int(parts[1]))] = [t for t in parts[2].split(" ") if t != HIZB]
    # Tanzil prefixes ayah 1 of most surahs with the basmalah; take its words from Tanzil 1:1.
    # Compared by skeleton, because Tanzil marks it differently in some surahs (e.g. a shadda).
    basmalah = [skeleton(w) for w in tanzil[(1, 1)]]
    for (s, a), words in tanzil.items():
        if s != 1 and a == 1 and [skeleton(w) for w in words[: len(basmalah)]] == basmalah:
            tanzil[(s, a)] = words[len(basmalah) :]
    others = {
        "Tanzil 1.1": tanzil,
        "DK madina (1421H)": dk_ayahs(
            "digitalkhatt-text-madina", qul_layout("qul-layout-10-v2-1421h"), counts
        ),
        "DK oldmadina (1405H)": dk_ayahs(
            "digitalkhatt-text-oldmadina", qul_layout("qul-layout-15-v1-1405h"), counts
        ),
    }
    out(
        "| Compared with | Ayahs found | Same word count | Same skeleton, every word "
        "| Same letters ignoring word spacing |"
    )
    out("|---|---|---|---|---|")
    examples = {}
    for name, ayahs in others.items():
        same_n = same_sk = same_joined = 0
        ex = []
        letter_ex = []
        for key, mine in v2.items():
            theirs = ayahs.get(key)
            if theirs is None:
                continue
            same_n += len(mine) == len(theirs)
            a = [skeleton(w) for w in mine]
            b = [skeleton(w) for w in theirs]
            if "".join(a) == "".join(b):
                same_joined += 1
            elif len(letter_ex) < 12:
                letter_ex.append(f"{key[0]}:{key[1]}")
            if a == b:
                same_sk += 1
            elif len(ex) < 12:
                d = [f"{x}\u2260{y}" for x, y in zip(a, b, strict=False) if x != y][:2]
                ex.append(f"{key[0]}:{key[1]} ({len(a)}/{len(b)}) {d}")
        examples[name] = ex
        examples[name + ", letters differ"] = letter_ex
        out(f"| {name} | {len(ayahs)} | {same_n} | {same_sk} | {same_joined} |")
    out()
    for name, ex in examples.items():
        if ex:
            out(f"Examples vs {name}: " + "; ".join(ex))
            out()


def main() -> int:
    verified_sources()
    words = qul_words()
    layouts = {name: qul_layout(sid) for name, sid in LAYOUTS.items()}
    texts = {"official v2.0": kfgqpc_official("hafs"), "mirror v18": kfgqpc_mirror_hafs()}
    out("# Spike S1: editions, layouts and word lists")
    out()
    out("Generated by `data/spikes/s1_editions.py` from pinned sources (`MANIFEST.json`).")
    out()
    layouts_section(layouts)
    kfgqpc_page_section(layouts, words, texts)
    qcf4_section(layouts, words)
    dk_section(layouts)
    words_section(words, texts)
    versions_section(texts)
    skeleton_section(texts)
    return 0


if __name__ == "__main__":
    sys.exit(main())
