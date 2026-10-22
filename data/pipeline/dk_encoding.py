"""T3.1: KFGQPC -> DigitalKhatt Madina encoding table, derived from data (D2, D14, G-024).

The Hafs spine is KFGQPC-encoded (G-006, G-018); DigitalKhatt Madina draws its own encoding.
Nothing here is written by hand: every row comes from aligning the spine with DigitalKhatt's
own Madina text (`digitalkhatt-text-madina`, pinned), word by word.

  1. Both texts are split into words per ayah. DigitalKhatt's lines are typed by QUL's 1421H
     layout (the text follows it, S1); only `ayah` lines carry spine words. DigitalKhatt
     writes the hizb sign glued to the next word; it is split off to match the spine item.
  2. Words pair one to one inside an ayah and must have the same letter skeleton
     (normalize.skeleton). G-012 words (one KFGQPC word, two DigitalKhatt words) pair with
     the two DigitalKhatt words joined; they are listed as segmentation differences.
  3. Cluster rows: from words with equally many clusters, cluster i maps to cluster i. A form
     with one counterpart gets a row. With several, the codepoint-identical one wins if
     present and the other words are text differences; otherwise there is no row.
  4. Sequence rows: in the remaining words, after cluster rows, the differing window (common
     prefix and suffix removed) gives a candidate `KFGQPC clusters -> DigitalKhatt text`. A
     candidate with one counterpart is accepted unless applying it breaks a word that cluster
     rows alone convert correctly (then it is context dependent: rejected and reported).
  5. Every aligned word is converted with the table and compared with DigitalKhatt's word.
     Mismatches are the G-024 differences. Rows whose output another row also produces are
     `lossy` (the conversion merges two KFGQPC forms) and are listed for the owner.

    python -m pipeline.dk_encoding build      # write the table and the G-024 review sheet
    python -m pipeline.dk_encoding check      # exit 1 if either is stale
    python -m pipeline.dk_encoding classes    # write the DigitalKhatt codepoint class table
"""

import hashlib
import html
import io
import json
import re
import sqlite3
import sys
import unicodedata
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

from fontTools.ttLib import TTFont

from pipeline import sources, spine
from pipeline.codepoints import TABLE_PATH as KFGQPC_CLASSES_PATH
from pipeline.normalize import skeleton

DATA_DIR = Path(__file__).resolve().parents[1]
ENCODINGS_DIR = DATA_DIR / "encodings"
TABLE_PATH = ENCODINGS_DIR / "hafs-v1.kfgqpc-to-dk-madina.json"
OWNER_ROWS_PATH = ENCODINGS_DIR / "hafs-v1.owner-rows.json"
DK_CLASSES_PATH = ENCODINGS_DIR / "digitalkhatt-madina-codepoint-classes.tsv"
REVIEW_PATH = DATA_DIR.parent / "docs" / "review" / "g024-dk-text-diff.html"
DK_TEXT = "digitalkhatt-text-madina"
DK_FONT = "digitalkhatt-hb-madina-font"
QUL_1421H = "qul-layout-10-v2-1421h"
# The basmalah line (surah headers) is drawn from these spine words (D16).
BASMALAH_ITEMS = ("1:1:1", "1:1:2", "1:1:3", "1:1:4")
HIZB, AYAH_END, SPACE = "\u06de", "\u06dd", " "
DK_CLASSES = ("base", "mark", "sign", "ayah_number", "digit")
DK_HEADER = ("codepoint", "class", "category", "name", "gdef", "count", "note")
GDEF_NAMES = {1: "base", 2: "ligature", 3: "mark", 4: "component"}


class EncodingError(ValueError):
    pass


def cps(text: str) -> str:
    return spine.to_cps(text)


# --- DigitalKhatt codepoint classes ---------------------------------------------------------


@dataclass(frozen=True)
class DkRow:
    cp: int
    cls: str
    category: str
    name: str
    gdef: str
    count: int
    note: str


def dk_proposed_class(cp: int, category: str) -> str:
    if cp == ord(AYAH_END):
        return "ayah_number"  # Unicode Cf; in DigitalKhatt's text it opens the ayah-number token
    if category in ("Mn", "Me"):
        return "mark"
    if category in ("Lo", "Lm"):
        return "base"
    if category in ("So", "Nd"):
        return {"So": "sign", "Nd": "digit"}[category]
    raise EncodingError(f"U+{cp:04X}: no class for Unicode category {category}")


def generate_dk_classes() -> dict[int, DkRow]:
    text = sources.path(DK_TEXT).read_text(encoding="utf-8")
    counts = Counter(c for page in dk_pages(text) for line in page for c in line if c != SPACE)
    font = TTFont(sources.path(DK_FONT))
    cmap, classes = font.getBestCmap(), font["GDEF"].table.GlyphClassDef.classDefs
    rows = {}
    for cp_, n in counts.items():
        cp = ord(cp_)
        category = unicodedata.category(cp_)
        cls = dk_proposed_class(cp, category)
        glyph = cmap.get(cp)
        gdef = "nocmap" if glyph is None else GDEF_NAMES.get(classes.get(glyph), "none")
        note = "" if gdef == ("mark" if cls == "mark" else "base") else f"font GDEF: {gdef}"
        rows[cp] = DkRow(cp, cls, category, unicodedata.name(cp_, "UNNAMED"), gdef, n, note)
    return rows


def write_dk_classes(rows: dict[int, DkRow]) -> None:
    out = ["\t".join(DK_HEADER)]
    for cp in sorted(rows):
        r = rows[cp]
        out.append(
            "\t".join((f"U+{cp:04X}", r.cls, r.category, r.name, r.gdef, str(r.count), r.note))
        )
    DK_CLASSES_PATH.parent.mkdir(parents=True, exist_ok=True)
    DK_CLASSES_PATH.write_text("\n".join(out) + "\n", encoding="ascii")


def load_dk_classes() -> dict[int, DkRow]:
    lines = DK_CLASSES_PATH.read_text(encoding="ascii").splitlines()
    if tuple(lines[0].split("\t")) != DK_HEADER:
        raise EncodingError("unexpected header in DigitalKhatt class table")
    rows = {}
    for line in lines[1:]:
        f = line.split("\t")
        if len(f) != len(DK_HEADER) or f[1] not in DK_CLASSES:
            raise EncodingError(f"malformed class row {line!r}")
        rows[int(f[0][2:], 16)] = DkRow(int(f[0][2:], 16), f[1], f[2], f[3], f[4], int(f[5]), f[6])
    return rows


def dk_clusterize(word: str, table: dict[int, DkRow]) -> tuple[str, ...]:
    """Base + following marks; an ayah-number token (U+06DD + digits) stays whole."""
    out: list[str] = []
    open_base = open_ayah = False
    for c in word:
        row = table.get(ord(c))
        if row is None:
            raise EncodingError(f"U+{ord(c):04X} not in the DigitalKhatt class table")
        if row.cls == "mark":
            if not open_base:
                raise EncodingError(f"mark U+{ord(c):04X} has no base")
            out[-1] += c
        elif row.cls == "digit" and open_ayah:
            out[-1] += c
        else:
            out.append(c)
            open_base, open_ayah = row.cls == "base", row.cls == "ayah_number"
    return tuple(out)


# --- reading the two texts -----------------------------------------------------------------


def dk_pages(text: str) -> list[list[str]]:
    """DigitalKhatt's `quranText`: an array of pages, each an array of quoted lines."""
    body = text[text.index("[") : text.rindex("]") + 1]
    pages = [
        re.findall(r"'([^']*)'", page) for page in re.findall(r"\[\s*((?:'[^']*',?\s*)+)\]", body)
    ]
    if len(pages) != 604:  # noqa: PLR2004 - the Madina mushaf's page count
        raise EncodingError(f"expected 604 pages, found {len(pages)}")
    return pages


def qul_line_types(source_id: str) -> dict[tuple[int, int], str]:
    con = sqlite3.connect(":memory:")
    con.deserialize(sources.zip_member(source_id, ".db"))
    rows = con.execute("select page_number, line_number, line_type from pages").fetchall()
    con.close()
    return {(p, n): t for p, n, t in rows}


def dk_ayahs(pages: list[list[str]], types: dict[tuple[int, int], str]) -> list[list[str]]:
    """Tokens per ayah, in order; the hizb sign is split off the word it is glued to."""
    ayahs, cur = [], []
    for pno, page in enumerate(pages, 1):
        for lno, line in enumerate(page, 1):
            kind = types.get((pno, lno))
            if kind is None:
                raise EncodingError(f"DigitalKhatt line {pno}:{lno} has no QUL 1421H line")
            if kind != "ayah":
                continue
            for tok in line.split(SPACE):
                if not tok:
                    raise EncodingError(f"empty token on line {pno}:{lno}")
                if tok.startswith(HIZB) and len(tok) > 1:
                    cur.append(HIZB)
                    tok = tok[1:]  # noqa: PLW2901 - the rest of the glued token
                cur.append(tok)
                if tok.startswith(AYAH_END):
                    ayahs.append(cur)
                    cur = []
    if cur:
        raise EncodingError("tokens after the last ayah number")
    return ayahs


# --- alignment ------------------------------------------------------------------------------


@dataclass(frozen=True)
class WordPair:
    word: str  # spine item id
    k: tuple[str, ...]  # KFGQPC clusters
    d: tuple[str, ...]  # DigitalKhatt clusters


def spine_clusters(rec: dict) -> tuple[str, ...]:
    return tuple(spine.from_cps(c["cps"]) for c in rec["clusters"])


def align(recs: list[dict], ayahs: list[list[str]], dk_table: dict) -> tuple[list, list]:
    """(word pairs, segmentation differences) for every spine item."""
    by_ayah: dict[tuple[str, str], list[dict]] = {}
    for r in recs:
        s, a, _ = r["id"].split(":")
        by_ayah.setdefault((s, a), []).append(r)
    if len(by_ayah) != len(ayahs):
        raise EncodingError(f"{len(by_ayah)} spine ayahs vs {len(ayahs)} DigitalKhatt ayahs")
    pairs, segmentation = [], []
    for items, toks in zip(by_ayah.values(), ayahs, strict=True):
        i = 0
        for rec in items:
            k = spine_clusters(rec)
            tok = toks[i]
            if rec["kind"] == "word" and skeleton("".join(k)) != skeleton(tok):
                joined = tok + toks[i + 1] if i + 1 < len(toks) else ""
                if skeleton("".join(k)) != skeleton(joined):
                    raise EncodingError(f"{rec['id']}: no DigitalKhatt word with its skeleton")
                segmentation.append({"word": rec["id"], "dk_words": [cps(tok), cps(toks[i + 1])]})
                tok, i = joined, i + 1
            if (rec["kind"] == "ayah_end") != tok.startswith(AYAH_END) or (
                rec["kind"] == "hizb"
            ) != (tok == HIZB):
                raise EncodingError(f"{rec['id']}: kind {rec['kind']} vs DigitalKhatt {cps(tok)}")
            pairs.append(WordPair(rec["id"], k, dk_clusterize(tok, dk_table)))
            i += 1
        if i != len(toks):
            raise EncodingError(f"ayah of {items[0]['id']}: DigitalKhatt has extra words")
    return pairs, segmentation


# --- the table ------------------------------------------------------------------------------


@dataclass(frozen=True)
class Row:
    src: tuple[str, ...]  # KFGQPC clusters
    dst: str  # DigitalKhatt text
    count: int
    kind: str  # cluster | sequence
    lossy: bool = False


@dataclass(frozen=True)
class Difference:
    word: str
    k: tuple[str, ...]
    d: tuple[str, ...]
    converted: str | None


@dataclass
class Table:
    rows: list[Row]
    differences: list[Difference] = field(default_factory=list)
    ambiguous: dict = field(default_factory=dict)
    rejected: dict = field(default_factory=dict)
    unmapped: dict = field(default_factory=dict)

    def __post_init__(self) -> None:
        self.cluster_rows = {r.src[0]: r.dst for r in self.rows if r.kind in ("cluster", "owner")}
        self.sequences: dict[tuple[str, ...], str] = {
            r.src: r.dst for r in self.rows if r.kind == "sequence"
        }
        self.max_seq = max((len(s) for s in self.sequences), default=0)

    def to_json(self, provenance: dict) -> dict:
        return {
            "schema": 1,
            "spine": "hafs/v1",
            "from": "kfgqpc",
            "to": "digitalkhatt-madina",
            "provenance": provenance,
            "rows": [
                {
                    "src": [cps(c) for c in r.src],
                    "dst": cps(r.dst),
                    "count": r.count,
                    "kind": r.kind,
                    "lossy": r.lossy,
                }
                for r in self.rows
            ],
        }

    @staticmethod
    def from_json(data: dict) -> "Table":
        return Table(
            [
                Row(
                    tuple(spine.from_cps(c) for c in r["src"]),
                    spine.from_cps(r["dst"]),
                    r["count"],
                    r["kind"],
                    r["lossy"],
                )
                for r in data["rows"]
            ]
        )


@dataclass(frozen=True)
class Converted:
    text: str
    # (first cluster, end cluster, first char, end char) per row applied, in order
    spans: list[tuple[int, int, int, int]]


def convert(k: tuple[str, ...], table: Table) -> Converted:
    out, spans, i = "", [], 0
    while i < len(k):
        for n in range(min(table.max_seq, len(k) - i), 0, -1):
            dst = table.sequences.get(k[i : i + n])
            if dst is not None:
                break
        else:
            n, dst = 1, table.cluster_rows.get(k[i])
            if dst is None:
                raise EncodingError(f"no row for cluster {cps(k[i])}")
        spans.append((i, i + n, len(out), len(out) + len(dst)))
        out += dst
        i += n
    return Converted(out, spans)


def _try(k: tuple[str, ...], table: Table) -> str | None:
    try:
        return convert(k, table).text
    except EncodingError:
        return None


def _cluster_rows(pairs: list[WordPair]) -> tuple[list[Row], dict, set]:
    seen: dict[str, Counter] = defaultdict(Counter)
    for p in pairs:
        if len(p.k) == len(p.d):
            for a, b in zip(p.k, p.d, strict=True):
                seen[a][b] += 1
    rows, ambiguous, conflicted = [], {}, set()
    for a, counter in seen.items():
        if len(counter) == 1:
            ((b, n),) = counter.items()
            rows.append(Row((a,), b, n, "cluster"))
        elif a in counter:
            rows.append(Row((a,), a, counter[a], "cluster"))
            conflicted.add(a)
        else:
            ambiguous[(a,)] = dict(sorted(counter.items()))
    return rows, ambiguous, conflicted


def _window(conv: list[str | None], d: tuple[str, ...]) -> tuple[int, int]:
    p = 0
    while p < min(len(conv), len(d)) and conv[p] == d[p]:
        p += 1
    s = 0
    while s < min(len(conv), len(d)) - p and conv[-1 - s] == d[-1 - s]:
        s += 1
    return p, s


def _candidates(pairs: list[WordPair], base: Table) -> tuple[dict, dict]:
    found: dict[tuple[str, ...], Counter] = defaultdict(Counter)
    for p in pairs:
        conv = [base.cluster_rows.get(c) for c in p.k]
        if conv == list(p.d):
            continue
        pre, suf = _window(conv, p.d)
        win_k, win_d = p.k[pre : len(p.k) - suf], p.d[pre : len(p.d) - suf]
        if win_k and win_d:
            found[win_k]["".join(win_d)] += 1
    unique = {k: next(iter(c)) for k, c in found.items() if len(c) == 1}
    multi = {k: dict(sorted(c.items())) for k, c in found.items() if len(c) > 1}
    return unique, multi


def _contains(seq: tuple[str, ...], sub: tuple[str, ...]) -> bool:
    n = len(sub)
    return any(seq[i : i + n] == sub for i in range(len(seq) - n + 1))


def _mark_lossy(rows: list[Row]) -> list[Row]:
    """In each group of rows with the same output, the reference row is the one whose input
    is the output itself (identity), else the most used one (ties: first by codepoints).
    The other rows of a group lose a distinction KFGQPC makes: they are `lossy`."""
    groups: dict[str, list[Row]] = defaultdict(list)
    for r in rows:
        groups[r.dst].append(r)
    lossy = set()
    for dst, group in groups.items():
        if len(group) < 2:  # noqa: PLR2004 - a group of one loses nothing
            continue
        ident = [r for r in group if r.src == (dst,)]
        order = sorted(group, key=lambda r: (-r.count, [cps(c) for c in r.src]))
        ref = ident[0] if ident else order[0]
        lossy.update(r.src for r in group if r is not ref)
    return [Row(r.src, r.dst, r.count, r.kind, r.src in lossy) for r in rows]


def derive(pairs: list[WordPair], owner_rows: tuple[Row, ...] = ()) -> Table:
    """Derive the table from aligned words. `owner_rows` are single-cluster rows the owner
    approved for forms the data cannot give a row for (kind "owner", D14)."""
    rows, ambiguous, _ = _cluster_rows(pairs)
    derived = {r.src for r in rows}
    for r in owner_rows:
        if r.kind != "owner" or len(r.src) != 1 or r.src in derived:
            raise EncodingError(f"owner row {[cps(c) for c in r.src]}: not a new single form")
    rows = [*rows, *owner_rows]
    base = Table(list(rows))
    unique, multi = _candidates(pairs, base)
    ambiguous.update(multi)
    accepted, rejected = [], {}
    for src, dst in sorted(unique.items()):
        trial = Table([*rows, Row(src, dst, 0, "sequence")])
        breaks = any(
            _try(p.k, base) == "".join(p.d) and _try(p.k, trial) != "".join(p.d)
            for p in pairs
            if _contains(p.k, src)
        )
        if breaks:
            rejected[src] = dst
        else:
            accepted.append((src, dst))
    table = Table([*rows, *(Row(s, d, 0, "sequence") for s, d in accepted)])
    used: Counter = Counter()
    differences, unmapped = [], Counter()
    for p in pairs:
        try:
            conv = convert(p.k, table)
        except EncodingError:
            unmapped.update((c,) for c in p.k if c not in table.cluster_rows)
            differences.append(Difference(p.word, p.k, p.d, None))
            continue
        used.update(p.k[a:b] for a, b, _, _ in conv.spans)
        if conv.text != "".join(p.d):
            differences.append(Difference(p.word, p.k, p.d, conv.text))
    final = [
        Row(r.src, r.dst, used[r.src], r.kind)
        for r in table.rows
        if used[r.src] or r.kind == "owner"
    ]
    final = _mark_lossy(final)
    final.sort(key=lambda r: (r.kind, [cps(c) for c in r.src]))
    return Table(final, differences, ambiguous, rejected, dict(unmapped))


# --- build ------------------------------------------------------------------------------------


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def provenance() -> dict:
    manifest = {s.id: s.sha256 for s in sources.load_manifest(sources.MANIFEST_PATH).sources}
    return {
        "sources": {sid: manifest[sid] for sid in (DK_TEXT, QUL_1421H, DK_FONT)},
        "spine_items_sha256": _sha256(spine.ITEMS),
        "kfgqpc_codepoint_table_sha256": _sha256(KFGQPC_CLASSES_PATH),
        "dk_codepoint_table_sha256": _sha256(DK_CLASSES_PATH),
        "owner_rows_sha256": _sha256(OWNER_ROWS_PATH),
    }


def load_owner_rows() -> tuple[Row, ...]:
    data = json.loads(OWNER_ROWS_PATH.read_text(encoding="ascii"))
    return tuple(
        Row(tuple(spine.from_cps(c) for c in r["src"]), spine.from_cps(r["dst"]), 0, "owner")
        for r in data["rows"]
    )


def build() -> tuple[Table, dict]:
    dk_table = load_dk_classes()
    text = sources.path(DK_TEXT).read_text(encoding="utf-8")
    ayahs = dk_ayahs(dk_pages(text), qul_line_types(QUL_1421H))
    recs = spine.load_records()
    pairs, segmentation = align(recs, ayahs, dk_table)
    table = derive(pairs, load_owner_rows())
    font_cmap = TTFont(sources.path(DK_FONT)).getBestCmap()
    no_glyph = sorted({c for r in table.rows for c in r.dst if ord(c) not in font_cmap})
    if no_glyph:
        raise EncodingError(f"rows produce codepoints the font cannot draw: {no_glyph}")
    unmapped: Counter = Counter()
    converted = 0
    for r in recs:
        try:
            convert(spine_clusters(r), table)
            converted += 1
        except EncodingError:
            unmapped.update(c for c in spine_clusters(r) if c not in table.cluster_rows)
    kfgqpc_cps = {c for r in recs for k in spine_clusters(r) for c in k}
    basmalah = SPACE.join(
        convert(spine_clusters(r), table).text for r in recs if r["id"] in BASMALAH_ITEMS
    )
    types = qul_line_types(QUL_1421H)
    dk_basmalah = [
        line
        for pno, page in enumerate(dk_pages(text), 1)
        for lno, line in enumerate(page, 1)
        if types[(pno, lno)] == "basmallah"
    ]
    report = {
        "spine_words": len(recs),
        "converted_words": converted,
        "aligned_words": len(pairs),
        "identical_after_conversion": len(pairs) - len(table.differences),
        "differences": [_diff_json(d) for d in table.differences],
        "segmentation": segmentation,
        "unmapped": {cps(c): n for c, n in sorted(unmapped.items())},
        "ambiguous": {" | ".join(cps(c) for c in k): v for k, v in table.ambiguous.items()},
        "rejected": {" | ".join(cps(c) for c in k): cps(v) for k, v in table.rejected.items()},
        "kfgqpc_codepoints_without_dk_glyph": [
            f"U+{ord(c):04X}" for c in sorted(kfgqpc_cps) if ord(c) not in font_cmap
        ],
        "lossy_rows": [r for r in table.rows if r.lossy],
        "lossy_words": _lossy_words(pairs, table),
        "basmalah_lines": len(dk_basmalah),
        "basmalah_lines_equal": sum(line == basmalah for line in dk_basmalah),
    }
    return table, report


def _diff_json(d: Difference) -> dict:
    return {
        "word": d.word,
        "kfgqpc": " | ".join(cps(c) for c in d.k),
        "dk": " | ".join(cps(c) for c in d.d),
        "converted": None if d.converted is None else cps(d.converted),
    }


def _lossy_words(pairs: list[WordPair], table: Table) -> dict[tuple[str, ...], list[str]]:
    lossy = {r.src for r in table.rows if r.lossy}
    out: dict[tuple[str, ...], list[str]] = defaultdict(list)
    for p in pairs:
        try:
            spans = convert(p.k, table).spans
        except EncodingError:
            continue
        for a, b, _, _ in spans:
            if p.k[a:b] in lossy:
                out[p.k[a:b]].append(p.word)
    return dict(out)


def converted_digest(recs: list[dict], table: Table) -> dict:
    """sha256 over `id TAB converted cps LF` for every spine item: the core's converter
    (mushaf-shape) must reproduce it exactly."""
    h = hashlib.sha256()
    for r in recs:
        h.update(f"{r['id']}\t{cps(convert(spine_clusters(r), table).text)}\n".encode("ascii"))
    return {"items": len(recs), "sha256": h.hexdigest()}


def lossy_items(recs: list[dict], table: Table) -> list[str]:
    """Spine items whose conversion uses a lossy row (G-040): the font shows another form."""
    lossy = {r.src for r in table.rows if r.lossy}
    out = []
    for r in recs:
        k = spine_clusters(r)
        if any(k[a:b] in lossy for a, b, _, _ in convert(k, table).spans):
            out.append(r["id"])
    return out


def table_text(table: Table) -> str:
    """JSON with one row per line, so diffs of the table stay reviewable."""
    data = table.to_json(provenance())
    recs = spine.load_records()
    data["converted_spine"] = converted_digest(recs, table)
    data["lossy_items"] = lossy_items(recs, table)
    rows = data.pop("rows")
    head = json.dumps(data, ensure_ascii=True, indent=1)[:-2]
    body = ",\n".join("  " + json.dumps(r, ensure_ascii=True) for r in rows)
    return f'{head},\n "rows": [\n{body}\n ]\n}}\n'


# --- review sheet (G-024) ---------------------------------------------------------------------

CSS = """
@font-face { font-family: dk; src: url("../../fonts/digitalkhatt/hb-madina.otf"); }
body { font-family: system-ui, sans-serif; margin: 2rem; max-width: 80rem; }
table { border-collapse: collapse; margin: 1rem 0; }
td, th { border: 1px solid #ccc; padding: .3rem .5rem; vertical-align: top; }
.ar { font-size: 1.6rem; direction: rtl; }
.dk { font-family: dk; font-size: 1.8rem; direction: rtl; }
code { font-size: .8rem; }
"""


def _ar(text: str, cls: str = "ar") -> str:
    return f'<span class="{cls}">{html.escape(text)}</span>'


def _cl(clusters: tuple[str, ...] | list[str]) -> str:
    return "<br>".join(f"<code>{cps(c)}</code>" for c in clusters)


def review_html(table: Table, report: dict) -> str:
    out = io.StringIO()
    w = out.write
    w("<!doctype html>\n<meta charset='utf-8'>\n<title>G-024 KFGQPC vs DigitalKhatt</title>\n")
    w(f"<style>{CSS}</style>\n")
    w("<h1>G-024: KFGQPC 1441H text vs DigitalKhatt Madina 1421H text</h1>\n")
    w(
        "<p>Generated by <code>data/pipeline/dk_encoding.py</code> from pinned sources "
        "(D2 rule-5 exception). The spine (KFGQPC v2.0 Hafs) is converted to DigitalKhatt's "
        "encoding with a table derived from the two texts, then compared word by word with "
        "DigitalKhatt's own text. Nothing here is a judgement about which text is right: it "
        "lists what the data says, for the owner to review.</p>\n"
    )
    w("<h2>Summary</h2>\n<table>\n")
    summary = [
        ("Spine items (words, ayah numbers, hizb signs)", report["spine_words"]),
        ("Items the table converts", report["converted_words"]),
        ("Items aligned with DigitalKhatt's text", report["aligned_words"]),
        ("Identical to DigitalKhatt after conversion", report["identical_after_conversion"]),
        ("Text differences (section 3)", len(report["differences"])),
        ("Segmentation differences, G-012 (section 4)", len(report["segmentation"])),
        ("Table rows: cluster", sum(r.kind == "cluster" for r in table.rows)),
        ("Table rows: sequence", sum(r.kind == "sequence" for r in table.rows)),
        ("Lossy rows: two KFGQPC forms give one output (section 2)", len(report["lossy_rows"])),
        ("Words that use a lossy row", sum(len(v) for v in report["lossy_words"].values())),
        ("Rejected (context-dependent) candidates", len(report["rejected"])),
        ("Ambiguous forms (no row)", len(report["ambiguous"])),
        ("Unmapped spine cluster forms", len(report["unmapped"])),
        (
            "DigitalKhatt basmalah lines equal to converted 1:1:1-4",
            f"{report['basmalah_lines_equal']} of {report['basmalah_lines']}",
        ),
    ]
    for label, n in summary:
        w(f"<tr><td>{label}</td><td>{n}</td></tr>\n")
    w("</table>\n")
    nog = ", ".join(report["kfgqpc_codepoints_without_dk_glyph"])
    w(f"<p>KFGQPC codepoints with no glyph in DigitalKhatt Madina: <code>{nog}</code>.</p>\n")

    w("<h2>1. Rows that change the encoding</h2>\n")
    w("<p>Identity rows (same codepoints on both sides) are omitted here.</p>\n<table>\n")
    w("<tr><th>KFGQPC</th><th>DigitalKhatt</th><th>Kind</th><th>Uses</th><th>Lossy</th></tr>\n")
    for r in table.rows:
        if r.src == (r.dst,):
            continue
        w(
            f"<tr><td>{_cl(r.src)}</td><td><code>{cps(r.dst)}</code></td><td>{r.kind}</td>"
            f"<td>{r.count}</td><td>{'yes' if r.lossy else ''}</td></tr>\n"
        )
    w("</table>\n")

    w("<h2>2. Lossy rows and every word that uses them</h2>\n")
    w(
        "<p>Each output below is produced by more than one KFGQPC form, so DigitalKhatt's "
        "encoding does not distinguish what KFGQPC distinguishes. Rendering these words "
        "with DigitalKhatt Madina shows DigitalKhatt's form.</p>\n<table>\n"
    )
    w("<tr><th>KFGQPC form</th><th>DigitalKhatt output</th><th>Words</th></tr>\n")
    for r in report["lossy_rows"]:
        words = report["lossy_words"].get(r.src, [])
        shown = ", ".join(words) if len(words) <= 40 else ", ".join(words[:40]) + ", ..."  # noqa: PLR2004
        w(
            f"<tr><td>{_cl(r.src)}</td><td><code>{cps(r.dst)}</code></td>"
            f"<td>{len(words)}: {shown}</td></tr>\n"
        )
    w("</table>\n")

    w("<h2>3. Text differences</h2>\n")
    w("<p>Words whose converted KFGQPC form is not DigitalKhatt's word.</p>\n<table>\n")
    w("<tr><th>Item</th><th>KFGQPC</th><th>Converted</th><th>DigitalKhatt</th></tr>\n")
    for d in table.differences:
        conv = "(no row)" if d.converted is None else _ar(d.converted, "dk")
        w(
            f"<tr><td>{d.word}</td><td>{_ar(''.join(d.k))}<br>{_cl(d.k)}</td>"
            f"<td>{conv}</td><td>{_ar(''.join(d.d), 'dk')}<br>{_cl(d.d)}</td></tr>\n"
        )
    w("</table>\n")

    w("<h2>4. Segmentation (G-012)</h2>\n<table>\n")
    for s in report["segmentation"]:
        w(f"<tr><td>{s['word']}</td><td><code>{' / '.join(s['dk_words'])}</code></td></tr>\n")
    w("</table>\n")

    w("<h2>5. Rejected and ambiguous candidates</h2>\n<table>\n")
    for k, v in report["rejected"].items():
        w(f"<tr><td>rejected</td><td><code>{k}</code></td><td><code>{v}</code></td></tr>\n")
    for k, v in report["ambiguous"].items():
        alts = "; ".join(f"{cps(a)} x{n}" for a, n in v.items())
        w(f"<tr><td>ambiguous</td><td><code>{k}</code></td><td><code>{alts}</code></td></tr>\n")
    w("</table>\n")
    return out.getvalue()


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd == "classes":
        write_dk_classes(generate_dk_classes())
        print(f"dk_encoding: wrote {DK_CLASSES_PATH}")
        return 0
    if cmd not in ("build", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    if generate_dk_classes() != load_dk_classes():
        print("dk_encoding: DigitalKhatt class table is stale (run `classes`)", file=sys.stderr)
        return 1
    table, report = build()
    if report["unmapped"]:
        print(f"dk_encoding: unmapped spine clusters: {report['unmapped']}", file=sys.stderr)
        return 1
    outputs = {TABLE_PATH: table_text(table), REVIEW_PATH: review_html(table, report)}
    if cmd == "check":
        stale = [p for p, t in outputs.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in stale:
            print(f"dk_encoding: stale {p}", file=sys.stderr)
        return 1 if stale else 0
    for p, t in outputs.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(t, encoding="utf-8")
    print(
        f"dk_encoding: {len(table.rows)} rows, {len(report['differences'])} differences, "
        f"{len(report['lossy_rows'])} lossy rows -> {TABLE_PATH}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
