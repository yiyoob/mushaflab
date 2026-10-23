"""T5.3: KFGQPC -> DigitalKhatt Madina rows for the cluster forms of the other Mashriqi riwayat.

    python -m pipeline.dk_riwayat write    # candidates + approved rows + review sheet
    python -m pipeline.dk_riwayat check    # exit 1 if a tracked file is stale

The Hafs table (D14) is derived from two texts of the same reading. No such pair exists for
Shu'bah, Bazzi, Qunbul, al-Duri and al-Susi, so a cluster form that occurs only in them has no
derivable row. Nothing here invents one (D2, D28). Each such form becomes a *candidate*, with
the evidence for it:

- `number`: an ayah number written in Arabic-Indic digits (Bazzi v7, G-070). Candidate: the
  DigitalKhatt form of the KFGQPC ayah sign with the same value (a Hafs row).
- `compose`: every codepoint of the form has exactly one DigitalKhatt image in the Hafs rows
  of equal length. Candidate: the codepoints converted one by one. Each base+mark pair that no
  Hafs row contains is listed; such a pair is new to the evidence.
- `open`: a codepoint has no single image in the Hafs rows, so no candidate follows from the
  data. The sheet shows an *identity proposal* (that codepoint unchanged, the others converted
  as above) only so the owner can see what it would draw; it is marked as having no evidence,
  and becomes a row only if the owner approves its group (D28).

Candidates are grouped for review: one group per mark that appears on a base letter no Hafs
row has it on, one per open codepoint,
one for number rows and one for forms built only from pairs Hafs already has. A form is
converted only when every group it belongs to is approved in `hafs-v1.riwayat-owner.json`,
which records the owner's decisions; approved rows are written to `hafs-v1.riwayat-rows.json`,
which the core loads next to the Hafs table.
"""

import base64
import html
import json
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field

import uharfbuzz as hb
from fontTools.ttLib import TTFont

from pipeline import clusterize, codepoints, dk_encoding, ingest_riwayah, kfgqpc, spine
from pipeline.dk_encoding import Table, cps

RIWAYAT = ("shuba", "bazzi", "qunbul", "duri_abuamr", "susi")
ENC = dk_encoding.ENCODINGS_DIR
CANDIDATES_PATH = ENC / "hafs-v1.riwayat-candidates.json"
OWNER_PATH = ENC / "hafs-v1.riwayat-owner.json"
ROWS_PATH = ENC / "hafs-v1.riwayat-rows.json"
SHEET_PATH = dk_encoding.DATA_DIR.parent / "build" / "review" / "phase5" / "encoding.html"
DK_FONT_PATH = dk_encoding.DATA_DIR.parent / "fonts" / "digitalkhatt" / "hb-madina.otf"
EXAMPLES = 6
NUMBERS_SHOWN = 12
MARK_CLASS = 3


@dataclass
class Form:
    src: tuple[str, ...]  # KFGQPC clusters (one, or the digit clusters of an ayah number)
    kind: str  # number | compose | open
    dst: str | None  # candidate DigitalKhatt text
    groups: tuple[str, ...]
    evidence: str
    counts: Counter = field(default_factory=Counter)
    examples: list = field(default_factory=list)  # (riwayah, item ref, clusters)
    words: set = field(default_factory=set)  # every word (clusters) the form occurs in


def hafs_table() -> Table:
    return Table.from_json(json.loads(dk_encoding.TABLE_PATH.read_text(encoding="ascii")))


def images(table: Table) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    """Per KFGQPC codepoint, its DigitalKhatt images in equal-length cluster rows; per mark,
    the base letters it occurs with in those rows."""
    img: dict[str, set[str]] = defaultdict(set)
    bases: dict[str, set[str]] = defaultdict(set)
    for r in table.rows:
        if r.kind != "cluster" or len(r.src[0]) != len(r.dst):
            continue
        for a, b in zip(r.src[0], r.dst, strict=True):
            img[a].add(b)
        for m in r.src[0][1:]:
            bases[m].add(r.src[0][0])
    return img, bases


def number_dst(table: Table, n: int) -> str:
    return table.cluster_rows[chr(0xFC00 + n - 1)]


def uncovered(clusters: tuple[str, ...], table: Table) -> list[int]:
    """Indices of clusters the Hafs table cannot convert (sequence rows first, as convert)."""
    out, i = [], 0
    while i < len(clusters):
        for n in range(min(table.max_seq, len(clusters) - i), 0, -1):
            if clusters[i : i + n] in table.sequences:
                i += n
                break
        else:
            if clusters[i] not in table.cluster_rows:
                out.append(i)
            i += 1
    return out


def classify(src: tuple[str, ...], kind_hint: str, table: Table, img: dict, bases: dict) -> Form:
    if kind_hint == "number":
        n = int("".join(str(ord(c) - 0x0660) for c in "".join(src)))
        return Form(
            src,
            "number",
            number_dst(table, n),
            ("number",),
            f"ayah number {n}: the DigitalKhatt form of the KFGQPC ayah sign for {n} (Hafs row)",
        )
    cl = src[0]
    unseen = sorted({c for c in cl if len(img.get(c, ())) != 1})
    if unseen:
        proposal = "".join(c if c in unseen else next(iter(img[c])) for c in cl)
        return Form(
            src,
            "open",
            proposal,
            tuple(f"open {cps(c)}" for c in unseen),
            "NO EVIDENCE (identity proposal for review only): no single DigitalKhatt image in "
            "the Hafs rows for "
            + ", ".join(f"{cps(c)} ({len(img.get(c, ()))} images)" for c in unseen),
        )
    dst = "".join(next(iter(img[c])) for c in cl)
    new_pairs = [m for m in cl[1:] if cl[0] not in bases[m]]
    groups = tuple(f"mark {cps(m)} on new bases" for m in dict.fromkeys(new_pairs))
    evidence = "each codepoint has one DigitalKhatt image in the Hafs rows"
    if new_pairs:
        evidence += "; base+mark pairs not in any Hafs row: " + ", ".join(
            f"{cps(cl[0])}+{cps(m)}" for m in dict.fromkeys(new_pairs)
        )
    return Form(src, "compose", dst, groups or ("compose: known pairs",), evidence)


def collect() -> dict[tuple[str, ...], Form]:
    table = hafs_table()
    img, bases = images(table)
    ctable = codepoints.load_table()
    forms: dict[tuple[str, ...], Form] = {}
    for r in RIWAYAT:
        for it in ingest_riwayah.load(r).items:
            clusters = tuple(c.text for c in clusterize.clusterize(it.text, ctable))
            if it.kind == "ayah_end" and all("\u0660" <= c <= "\u0669" for c in it.text):
                keys = [(clusters, "number")] if uncovered(clusters, table) else []
            else:
                keys = [((clusters[i],), "cluster") for i in uncovered(clusters, table)]
            for src, hint in keys:
                f = forms.get(src) or classify(src, hint, table, img, bases)
                forms[src] = f
                f.counts[r] += 1
                f.words.add(clusters)
                if len(f.examples) < EXAMPLES:
                    f.examples.append((r, it.ref, clusters))
    return dict(sorted(forms.items()))


class Checker:
    """Rule 6 in DigitalKhatt Madina, as S2 and `mushaf-shape::check_word` test it."""

    def __init__(self) -> None:
        data = DK_FONT_PATH.read_bytes()
        self.font = hb.Font(hb.Face(data))
        tt = TTFont(DK_FONT_PATH)
        self.classes = tt["GDEF"].table.GlyphClassDef.classDefs
        self.order = tt.getGlyphOrder()

    def _shape(self, text: str, feats: dict) -> hb.Buffer:
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(self.font, buf, feats)
        return buf

    def violations(self, text: str) -> list[str]:
        a, z = self._shape(text, {}), self._shape(text, {"mark": False, "mkmk": False})
        out = []
        for g, p, q in zip(a.glyph_infos, a.glyph_positions, z.glyph_positions, strict=True):
            name = self.order[g.codepoint]
            if g.codepoint == 0:
                out.append("notdef")
            elif self.classes.get(name) == MARK_CLASS and (p.x_offset, p.y_offset) == (
                q.x_offset,
                q.y_offset,
            ):
                out.append(f"unattached {name}")
        return out


def candidate_word(clusters: tuple[str, ...], table: Table, forms: dict) -> str | None:
    """The word converted with the Hafs table plus every candidate; None if a form has none."""
    rows = {**table.cluster_rows, **{f.src[0]: f.dst for f in forms.values() if len(f.src) == 1}}
    seqs = {**table.sequences, **{f.src: f.dst for f in forms.values() if len(f.src) > 1}}
    out, i = "", 0
    longest = max(len(s) for s in seqs)
    while i < len(clusters):
        for n in range(min(longest, len(clusters) - i), 0, -1):
            dst = seqs.get(clusters[i : i + n])
            if dst is not None:
                break
        else:
            n, dst = 1, rows.get(clusters[i])
        if dst is None:
            return None
        out += dst
        i += n
    return out


def candidates_json(forms: dict[tuple[str, ...], Form]) -> dict:
    table = hafs_table()
    check = Checker()
    with_dst = {k: v for k, v in forms.items() if v.dst}
    groups: dict[str, list[dict]] = defaultdict(list)
    for f in forms.values():
        rec = {
            "src": [cps(c) for c in f.src],
            "kind": f.kind,
            "dst": cps(f.dst) if f.dst is not None else None,
            "evidence": f.evidence,
            "counts": dict(sorted(f.counts.items())),
            "examples": [f"{r} {ref}" for r, ref, _ in f.examples],
            "groups": list(f.groups),
            "rule6": [],
        }
        failing = 0
        for clusters in sorted(f.words):
            word = candidate_word(clusters, table, with_dst)
            if word is not None:
                found = check.violations(word)
                failing += bool(found)
                rec["rule6"] += [v for v in found if v not in rec["rule6"]]
        if failing:
            rec["rule6"].append(f"{failing} of {len(f.words)} distinct words fail")
        for g in f.groups:
            groups[g].append(rec)
    return {
        "schema": 1,
        "spine": "hafs/v1",
        "note": (
            "Candidates only: cluster forms of Shu'bah, Bazzi, Qunbul, al-Duri and al-Susi that "
            "the Hafs table (D14) cannot convert, with the evidence for each. Generated by "
            "`python -m pipeline.dk_riwayat write` (T5.3, D28). A form is converted only when "
            "the owner approves every group it belongs to (hafs-v1.riwayat-owner.json)."
        ),
        "groups": [
            {
                "group": g,
                "forms": len(recs),
                "clusters": {
                    r: sum(rec["counts"].get(r, 0) for rec in recs)
                    for r in RIWAYAT
                    if any(r in rec["counts"] for rec in recs)
                },
                "members": recs,
            }
            for g, recs in sorted(groups.items())
        ],
    }


def load_owner() -> dict:
    """The owner's decisions: approved groups, and owner rows for open forms."""
    if not OWNER_PATH.exists():
        return {"groups": [], "rows": []}
    return json.loads(OWNER_PATH.read_text(encoding="ascii"))


def rows_json(forms: dict[tuple[str, ...], Form], owner: dict) -> dict:
    """Rows for the forms whose groups are all approved, plus the owner's own rows."""
    approved = {g["group"] for g in owner["groups"]}
    rows = []
    for f in forms.values():
        if f.dst is not None and all(g in approved for g in f.groups):
            rows.append(
                {
                    "src": [cps(c) for c in f.src],
                    "dst": cps(f.dst),
                    "kind": "sequence" if len(f.src) > 1 else "cluster",
                    "groups": list(f.groups),
                }
            )
    for r in owner["rows"]:
        rows.append({"src": r["src"], "dst": r["dst"], "kind": "owner", "groups": []})
    return {
        "schema": 1,
        "spine": "hafs/v1",
        "from": "kfgqpc",
        "to": "digitalkhatt-madina",
        "note": (
            "Rows for cluster forms of the other Mashriqi riwayat, each from a group the owner "
            "approved or an owner row (hafs-v1.riwayat-owner.json, D28). Generated by "
            "`python -m pipeline.dk_riwayat write`; loaded by the core next to the Hafs table."
        ),
        "rows": rows,
    }


CSS = """
body { font-family: system-ui, sans-serif; margin: 2rem; max-width: 90rem; }
table { border-collapse: collapse; margin: .5rem 0 1.5rem; }
td, th { border: 1px solid #ccc; padding: .3rem .6rem; vertical-align: middle; }
.k { font-size: 2.6rem; direction: rtl; }
.dk { font-family: dk; font-size: 2.6rem; direction: rtl; }
code { font-size: .8rem; }
h2 { margin-top: 2.5rem; }
"""


def _font_face(name: str, data: bytes) -> str:
    b64 = base64.b64encode(data).decode("ascii")
    return f"@font-face {{ font-family: {name}; src: url(data:font/ttf;base64,{b64}); }}\n"


def sheet(forms: dict[tuple[str, ...], Form], cand: dict, owner: dict) -> str:
    table = hafs_table()
    approved = {g["group"] for g in owner["groups"]}
    faces = _font_face("dk", DK_FONT_PATH.read_bytes())
    faces += "".join(_font_face(f"k_{r}", kfgqpc.font_bytes(r)) for r in RIWAYAT)
    w = [
        "<!doctype html>\n<meta charset='utf-8'>\n<title>Phase 5 encoding candidates</title>\n",
        f"<style>{faces}{CSS}</style>\n",
        "<h1>T5.3: DigitalKhatt rows for the other Mashriqi riwayat (candidates)</h1>\n",
        "<p>Generated by <code>python -m pipeline.dk_riwayat write</code> from the pinned "
        "KFGQPC texts and the Hafs encoding table (D14). Each row below is a cluster form that "
        "occurs only outside Hafs. The left column draws example words in the riwayah's own "
        "KFGQPC font, as KFGQPC publishes them; the right column draws the same words in "
        "DigitalKhatt Madina with the candidate row. Nothing here is approved until the owner "
        "approves its group; approve a group only if every right-hand word shows the same "
        "marks as its left-hand word.</p>\n",
        "<h2>Groups</h2>\n<table><tr><th>Group</th><th>Forms</th><th>Clusters per riwayah</th>"
        "<th>Status</th></tr>\n",
    ]
    for g in cand["groups"]:
        counts = ", ".join(f"{r} {n}" for r, n in g["clusters"].items())
        status = "approved" if g["group"] in approved else "pending"
        w.append(
            f"<tr><td><a href='#{html.escape(g['group'])}'>{html.escape(g['group'])}</a></td>"
            f"<td>{g['forms']}</td><td>{counts}</td><td>{status}</td></tr>\n"
        )
    w.append("</table>\n")
    rows_all = {k: v for k, v in forms.items() if v.dst}
    for g in cand["groups"]:
        w.append(f"<h2 id='{html.escape(g['group'])}'>{html.escape(g['group'])}</h2>\n")
        w.append(
            "<table><tr><th>Form (KFGQPC)</th><th>Candidate (DigitalKhatt)</th><th>Evidence</th>"
            "<th>Rule 6</th><th>Example</th><th>KFGQPC font</th><th>DigitalKhatt</th></tr>\n"
        )
        members = g["members"]
        if g["group"] == "number":  # one rule for every value: show the first few
            w.append(f"<p>{len(members)} values; the first {NUMBERS_SHOWN} are shown.</p>\n")
            members = members[:NUMBERS_SHOWN]
        for rec in members:
            f = forms[tuple(spine.from_cps(c) for c in rec["src"])]
            examples = f.examples[:1] if g["group"] == "number" else f.examples
            for k, (r, ref, clusters) in enumerate(examples):
                word = candidate_word(clusters, table, rows_all)
                head = ""
                if k == 0:
                    n = len(examples)
                    head = (
                        f"<td rowspan={n}>{'<br>'.join(f'<code>{c}</code>' for c in rec['src'])}"
                        f"</td><td rowspan={n}><code>{rec['dst'] or '-'}</code></td>"
                        f"<td rowspan={n}>{html.escape(f.evidence)}<br>"
                        f"{html.escape(str(rec['counts']))}</td>"
                        f"<td rowspan={n}>{html.escape(', '.join(rec['rule6']) or 'ok')}</td>"
                    )
                w.append(
                    f"<tr>{head}<td>{r} {ref}</td>"
                    f"<td class='k' style='font-family:k_{r}'>{html.escape(''.join(clusters))}</td>"
                    f"<td class='dk'>{html.escape(word or '')}</td></tr>\n"
                )
        w.append("</table>\n")
    return "".join(w)


def render_json(data: dict) -> str:
    return json.dumps(data, ensure_ascii=True, indent=1) + "\n"


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else "write"
    forms = collect()
    owner = load_owner()
    cand = candidates_json(forms)
    outputs = {CANDIDATES_PATH: render_json(cand), ROWS_PATH: render_json(rows_json(forms, owner))}
    if cmd == "write":
        for path, text in outputs.items():
            path.write_text(text, encoding="ascii")
        SHEET_PATH.parent.mkdir(parents=True, exist_ok=True)
        SHEET_PATH.write_text(sheet(forms, cand, owner), encoding="utf-8")
        print(f"dk_riwayat: {len(forms)} forms in {len(cand['groups'])} groups -> {SHEET_PATH}")
        return 0
    stale = [p.name for p, t in outputs.items() if not p.exists() or p.read_text("ascii") != t]
    for name in stale:
        print(f"dk_riwayat: {name} differs from a rebuild", file=sys.stderr)
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
