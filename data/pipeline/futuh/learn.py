"""Self-calibration of the Quran fonts' glyph decoding.

Some ligature glyphs in the book's KFGQPC fonts carry incomplete ToUnicode (e.g. lam-hah says
only "\u0644"), and a ligature's vowel marks must be distributed over its letters. Neither can be
read from the PDF alone, and the exact old font file is not available. So we learn both from
the book itself, using its uncoloured Quranic text (which follows the Hafs spine) as ground truth:

  letters: for glyph G, how its letters must change so that words containing G match Hafs.
           Accepted only when consistent (>= 80% of G's aligned occurrences, >= 3 of them).
  marks:   for a multi-letter glyph, where (relative x across the glyph) each component's marks
           sit, from occurrences whose letters match Hafs; marks are then assigned to the
           nearest component.

The result (data/books/al-futuh-al-bahira/glyphfix.json) is data, keyed by font and glyph id,
with its evidence counts.
The book's coloured (variant) words are decoded with the same per-glyph rules and are never
compared letter-by-letter to Hafs, so learning cannot pull a variant towards Hafs.

    uv run python -m pipeline.futuh.learn   # sample all six volumes, write glyphfix.json
"""

from __future__ import annotations

import collections
import json
import statistics
import sys
from pathlib import Path

import pymupdf

from .glyphs import Glyph, InkBounds, is_arabic
from .paths import BOOK, OUT, font_path
from .quran import clusters, is_mark

FIXFILE = BOOK / "glyphfix.json"


def letters(t: str) -> str:
    return "".join(c for c in t if not is_mark(c) and c != "\u0640")


def _edit_ops(a: str, b: str) -> list[tuple[str, int, int]]:
    """Levenshtein alignment a->b. Returns list of (op, i, j) with op in m/s/d/i."""
    n, m = len(a), len(b)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n + 1):
        dp[i][0] = i
    for j in range(m + 1):
        dp[0][j] = j
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            dp[i][j] = min(
                dp[i - 1][j] + 1, dp[i][j - 1] + 1, dp[i - 1][j - 1] + (a[i - 1] != b[j - 1])
            )
    ops = []
    i, j = n, m
    while i or j:
        # ties: take the insertion first (walking backwards), so missing letters land as late as
        # possible, i.e. on the ligature glyph that drew them (ٱ + [لله] for ٱلله, not [ٱل] + [له])
        if j and dp[i][j] == dp[i][j - 1] + 1:
            ops.append(("i", i, j - 1))
            j -= 1
        elif i and j and dp[i][j] == dp[i - 1][j - 1] + (a[i - 1] != b[j - 1]):
            ops.append(("m" if a[i - 1] == b[j - 1] else "s", i - 1, j - 1))
            i, j = i - 1, j - 1
        elif i and dp[i][j] == dp[i - 1][j] + 1:
            ops.append(("d", i - 1, j))
            i -= 1
        else:
            ops.append(("i", i, j - 1))
            j -= 1
    return ops[::-1]


def corrected_letters(glyphs: list[Glyph], hafs: str) -> list[str] | None:
    """Per-glyph letters that make the word's letters equal Hafs's, or None if no clean fix."""
    own = []
    dl = ""
    for k, g in enumerate(glyphs):
        lt = letters(g.text)
        dl += lt
        own += [k] * len(lt)
    hl = letters(hafs)
    if not glyphs:
        return None
    new = [""] * len(glyphs)
    last = 0
    for op, i, j in _edit_ops(dl, hl):
        if op in ("m", "s"):
            last = own[i]
            new[last] += hl[j]
        elif op == "d":
            last = own[i]
        else:  # insertion: belongs to the glyph that precedes it (ligature tail)
            k = own[i - 1] if i > 0 else 0
            new[k] += hl[j]
    return new


def _norm(t: str) -> str:
    """Letters compared for consistency: final ya / alif maqsura is an encoding convention."""
    return t.replace("\u0649", "\u064a")


def collect(vol: int, step: int, block: int, out: Path) -> None:
    """Gather evidence from one volume (sampled blocks of pages) into a JSON file."""
    from .anchor import Anchorer, anchor_row
    from .fontmap import FontMap
    from .page import parse_page
    from .quran import canon
    from .run import volume_pdf

    fm = FontMap(font_path())
    c = canon()
    votes: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    pos: dict[str, list] = collections.defaultdict(list)
    doc = pymupdf.open(volume_pdf(vol))
    ink = InkBounds(doc)
    for start in range(20, len(doc), step):
        anc = Anchorer(c, None)
        chunks = []
        for pn in range(start, min(start + block, len(doc) + 1)):
            m = parse_page(doc, pn - 1, ink, {"SakkalMajalla": fm})
            for r in m.rows:
                if r.number.strip().isdigit():
                    chunks.extend(anchor_row(anc, r))
        for ch in chunks:
            for w in ch.words:
                # low-similarity words are still usable when the ayah around them aligned well
                # (a 3-letter ligature decoding as 1 letter scores 0.33)
                if (
                    not w.hafs
                    or w.colored
                    or w.tok is None
                    or w.sim < 0.3
                    or (w.sim < 0.5 and ch.cost > 0.5)
                ):
                    continue
                gl = w.tok.glyphs
                new = corrected_letters(gl, w.hafs)
                if new is None:
                    continue
                prev = "^"
                for g, nl in zip(gl, new, strict=False):
                    if g.text and not is_arabic(g.text):
                        continue  # punctuation drawn in the Quran font
                    orig = letters(g.text)
                    # context-free key, and a key with the preceding glyph's letters: the same
                    # glyph can cover different letters by context
                    for okey in (orig, orig + "|" + prev):
                        votes[f"{g.font}\t{g.gid}\t{okey}"][nl] += 1
                    prev = letters(g.text) or prev
                # where each mark sits on a multi-letter glyph, per mark character
                if "".join(new) != letters(w.hafs):
                    continue
                hcl = [cl for cl in clusters(w.hafs) if not is_mark(cl[0])]
                jj = 0
                for g, nl in zip(gl, new, strict=False):
                    span = hcl[jj : jj + len(nl)]
                    jj += len(nl)
                    if len(nl) < 2 or not g.marks or g.x1 - g.x0 < 0.5:
                        continue
                    # per mark character: the components that carry it (in Hafs) and the marks
                    # of that character on the glyph; equal counts pair up in order (right to
                    # left), so a fatha on each of two letters is learned too (عَلَى)
                    bych: dict[str, list[float]] = collections.defaultdict(list)
                    for mk in g.marks:
                        r = (g.x1 - mk.mx) / (g.x1 - g.x0)
                        for ch_ in mk.text:
                            bych[ch_].append(r)
                    for ch_, rs_ in bych.items():
                        comps = [k for k, cl in enumerate(span) if ch_ in cl[1:]]
                        if comps and len(comps) == len(rs_):
                            for r, k in zip(sorted(rs_), comps, strict=False):
                                pos[f"{g.font}\t{g.gid}\t{_norm(nl)}"].append((round(r, 4), k, ch_))
        print(f"vol {vol} p{start}", file=sys.stderr, flush=True)
    out.write_text(json.dumps({"votes": votes, "pos": pos}, ensure_ascii=False))


def merge(files: list[Path]) -> None:
    votes: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    pos: dict[str, list] = collections.defaultdict(list)
    for f in files:
        d = json.loads(f.read_text())
        for k, v in d["votes"].items():
            votes[k].update(v)
        for k, v in d["pos"].items():
            pos[k].extend(v)
    fixes: dict[str, dict] = {}
    proven: list[str] = []  # glyphs whose decoding (as is, or with its fix) is consistent with Hafs
    for key, cnt in votes.items():
        font, gid, okey = key.split("\t")
        gid = int(gid)
        orig = okey.split("|")[0]
        groups: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
        for nl, n in cnt.items():
            groups[_norm(nl)][nl] += n
        gnorm, gcnt = max(groups.items(), key=lambda kv: sum(kv[1].values()))
        n = sum(gcnt.values())
        total = sum(cnt.values())
        new = gcnt.most_common(1)[0][0]
        if n < 3 or n / total < 0.8:
            continue  # too little or inconsistent evidence: leave the glyph as the PDF says
        proven.append(f"{font}:{gid}:{okey}")
        entry = {
            "font": font,
            "gid": gid,
            "tounicode_letters": orig,
            "letters": new,
            "n": n,
            "of": total,
        }
        if "|" in okey:
            entry["context"] = okey.split("|")[1]
        p = pos.get(f"{font}\t{gid}\t{gnorm}", [])
        if len(new) >= 2 and p:
            by = collections.defaultdict(list)
            byc: dict[str, dict[int, list]] = collections.defaultdict(
                lambda: collections.defaultdict(list)
            )
            for r, k, ch_ in p:
                by[k].append(r)
                byc[ch_][k].append(r)
            entry["comp_r"] = {
                str(k): round(statistics.median(v), 3) for k, v in sorted(by.items())
            }
            entry["comp_n"] = {str(k): len(v) for k, v in sorted(by.items())}
            # per mark character: which component it lands on, by position
            entry["comp_rc"] = {
                ch_: {
                    str(k): round(statistics.median(v), 3)
                    for k, v in sorted(d.items())
                    if len(v) >= 2
                }
                for ch_, d in byc.items()
            }
        if _norm(new) == _norm(orig) and "comp_r" not in entry:
            continue
        fixes[f"{font}:{gid}:{okey}"] = entry
    FIXFILE.write_text(
        json.dumps(
            {"method": __doc__.strip().splitlines()[0], "glyphs": fixes, "proven": sorted(proven)},
            ensure_ascii=False,
            indent=1,
            sort_keys=True,
        )
    )
    changed = sum(1 for e in fixes.values() if e["letters"] != e["tounicode_letters"])
    print(
        f"wrote {FIXFILE}: {len(fixes)} glyph entries, {changed} with letter changes",
        file=sys.stderr,
    )


def main(argv: list[str]) -> None:
    """No args: collect from all six volumes in parallel (step 35, blocks of 6), then merge."""
    import subprocess

    if argv and argv[0] == "collect":
        collect(int(argv[1]), int(argv[2]), int(argv[3]), Path(argv[4]))
        return
    outdir = OUT / "learn"
    outdir.mkdir(parents=True, exist_ok=True)
    files = [outdir / f"votes_v{v}.json" for v in range(1, 7)]
    procs = [
        subprocess.Popen(
            [sys.executable, "-m", "pipeline.futuh.learn", "collect", str(v), "35", "6", str(f)]
        )
        for v, f in zip(range(1, 7), files, strict=False)
    ]
    if any(p.wait() for p in procs):
        raise SystemExit("a collect process failed")
    merge(files)


if __name__ == "__main__":
    main(sys.argv[1:])
