"""Resolve each table row's reader label to the 20 ruwat, and check each segment's coverage.

    uv run python -m pipeline.futuh.readers out/vol1.jsonl [out/vol2.jsonl ...]

Adds to every row record:
  "ruwat":      sorted rawi ids reading this row, when fully determined (else null)
  "ruwat_min":  ruwat that certainly read this row
  "ruwat_max":  ruwat that may read it (equal to ruwat_min when resolved)
  "readers_parse": how the label was read (named / included / excluded, flags, qualifiers such
                as khulf or waqf-only) and a status: resolved | partial | contextual | unresolved
and to the first row of each segment (thick rule in the table):
  "coverage": {all_certain, not_listed (certainly absent from the segment), uncertain}

The lexicon is data/books/al-futuh-al-bahira/readers.yaml (every term cited to the book).
Nothing is guessed: a term whose members depend on the passage (ashab al-tawassut) is
bounded or stays contextual.

Label grammar (the head of the sharh cell, before ":"), in transliteration:
  NAMES                           X, X wa-Y ...
  NAMES wa-indaraja NAMES         the second group's reading is folded into this row
  NAMES wa-indaraja man indaraja  X with whoever shares the reading (unnamed)
  al-jami'                        all 20
  al-jami' 'ada NAMES             all except
  al-jami' 'ada man sa-ya'ti      all except those in the following rows of the segment
  al-baqun                        all except those in the other rows of the segment
  ... waqfan / bi-khulf           qualifiers on the preceding name(s)
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

import yaml

from .paths import BOOK

if TYPE_CHECKING:
    from .quran import Canon

LEXICON = BOOK / "readers.yaml"

_HARAKAT = re.compile("[\u064b-\u065f\u0670\u0640\u06d6-\u06ed]")
_UNIFY = str.maketrans(
    {
        "\u0623": "\u0627",
        "\u0625": "\u0627",
        "\u0622": "\u0627",
        "\u0671": "\u0627",
        "\u0649": "\u064a",
    }
)
_SPLIT = re.compile("[\\s\u060c\u061b\u061f.,;()\\[\\]\"'\u00ab\u00bb\\-]+")


def name_key(text: str) -> str:
    return " ".join(_HARAKAT.sub("", text).translate(_UNIFY).split())


def toks(text: str) -> list[str]:
    return [t for t in _SPLIT.split(name_key(text)) if t]


class Lexicon:
    def __init__(self, path: Path = LEXICON) -> None:
        d = yaml.safe_load(path.read_text())
        self.qurra: dict[str, list[str]] = d["qurra"]
        self.all_ruwat = sorted(r for rs in self.qurra.values() for r in rs)
        self.forms: list[tuple[tuple[str, ...], dict]] = []
        for f in d["forms"]:
            self.forms.append((tuple(toks(f["form"])), f))
        self.forms.sort(key=lambda kv: -len(kv[0]))
        self.phrases = {
            k: sorted((tuple(toks(p)) for p in v), key=len, reverse=True)
            for k, v in d["phrases"].items()
        }
        allr = set(self.all_ruwat)
        self.madd: dict[str, dict[str, set[str]]] = {}
        self.madd_cite: dict[str, str] = {}
        for madd, t in d.get("madd_tables", {}).items():
            self.madd_cite[madd] = t["cite"]
            self.madd[madd] = {}
            for wajh in ("qasr", "tawassut", "ishba"):
                if wajh not in t:
                    continue
                e = t[wajh]
                if "all_except" in e:
                    self.madd[madd][wajh] = allr - self.ruwat_of(e["all_except"])
                else:
                    self.madd[madd][wajh] = self.ruwat_of(e)

    def ruwat_of(self, f: dict) -> set[str]:
        out = set(f.get("ruwat", []))
        for q in f.get("qurra", []):
            out |= set(self.qurra[q])
        return out


def _match(tk: list[str], i: int, key: tuple[str, ...], prefix: bool = True) -> bool:
    if not key or i + len(key) > len(tk):
        return False
    if tuple(tk[i + 1 : i + len(key)]) != key[1:]:
        return False
    return tk[i] == key[0] or (prefix and tk[i] == "\u0648" + key[0])


def parse_head(lex: Lexicon, head: str) -> dict:
    """Read one label head. Returns the parse (sets are lists for JSON)."""
    tk = toks(head)
    p: dict = {
        "named": set(),
        "included": set(),
        "excluded": set(),
        "all": False,
        "except_following": False,
        "rest": False,
        "include_previous": False,
        "khulf": set(),
        "waqf_only": set(),
        "contextual": [],
        "contextual_excluded": [],
        "unparsed": [],
        "forms": [],
    }
    mode = "named"
    last: set[str] = set()
    i = 0
    while i < len(tk):
        hit = None
        for kind in (
            "include_previous",
            "following",
            "all",
            "rest",
            "except",
            "include",
            "with",
            "khulf",
            "waqf_only",
        ):
            for key in lex.phrases[kind]:
                if _match(tk, i, key, prefix=kind in ("all", "rest")):
                    hit = (kind, len(key))
                    break
            if hit:
                break
        if hit:
            kind, n = hit
            if kind == "include_previous":
                p["include_previous"] = True
            elif kind == "following":
                p["except_following"] = True
            elif kind == "all":
                p["all"] = True
            elif kind == "rest":
                p["rest"] = True
            elif kind == "except":
                mode = "excluded"
            elif kind in ("include", "with"):
                mode = "included"
            elif kind == "khulf":
                p["khulf"] |= last
            elif kind == "waqf_only":
                p["waqf_only"] |= last
            i += n
            continue
        for key, f in lex.forms:
            if _match(tk, i, key):
                p["forms"].append(f["form"])
                if "contextual" in f:
                    # a madd group after "عدا" is subtracted ("الجميع عدا أصحاب الإشباع")
                    (p["contextual_excluded"] if mode == "excluded" else p["contextual"]).append(
                        f["contextual"]
                    )
                    last = set()
                else:
                    last = lex.ruwat_of(f)
                    p[mode] |= last
                hit = ("form", len(key))
                break
        if hit:
            i += hit[1]
            continue
        # anything else: the label has run into its description (no ":" in the cell)
        p["unparsed"] = tk[i:]
        break
    return p


def head_of(readers: str) -> str:
    return readers.split(":", 1)[0] if ":" in readers else readers


def _madd_set(
    lex: Lexicon, canon: Canon | None, row: dict, kind: str
) -> tuple[set[str] | None, str | None]:
    """Members of a madd group (madd_qasr / madd_tawassut / madd_ishba) for this row's text.

    The separated-madd table decides when the row has a separated madd (its sets agree with the
    connected-madd table on every reader); otherwise the connected-madd table; otherwise none."""
    if canon is None:
        return None, None
    from .madd import madd_types

    wajh = kind.split("_", 1)[1]
    types = madd_types(canon, [w["spine"] for w in row.get("words", []) if w.get("spine")])
    for madd in ("munfasil", "muttasil"):
        if madd in types:
            t = lex.madd.get(madd, {})
            return (set(t[wajh]), madd) if wajh in t else (None, madd)
    return None, None


def resolve_rows(lex: Lexicon, rows: list[dict], canon: Canon | None = None) -> None:
    """Resolve rows in book order (mutates them). Rows carry "group" ids from emit.py.

    Every row gets bounds: ruwat_min (certainly read this row) and ruwat_max (may read it);
    "ruwat" is set only when they are equal. Nothing is guessed: "man indaraja" (whoever
    shares the reading, unnamed) and the madd groups (ashab al-tawassut ...) widen the upper
    bound, and "al-jami' 'ada man sa-ya'ti" / "al-baqun" take their bounds from the other rows
    of the segment."""
    every = frozenset(lex.all_ruwat)
    parses = [parse_head(lex, head_of(r.get("readers", ""))) for r in rows]
    lo: list[set[str]] = [set() for _ in rows]
    hi: list[set[str]] = [set(every) for _ in rows]
    rel: list[bool] = [False] * len(rows)  # bounds depend on the segment's other rows
    madd_rows: dict[int, str] = {}
    for k, p in enumerate(parses):
        if p["contextual"]:
            # madd groups: upper bound from the book's madd schools when the row's madd is known
            kinds = [c for c in p["contextual"] if c.startswith("madd_")]
            if len(kinds) == 1 and len(p["contextual"]) == 1:
                ms, madd = _madd_set(lex, canon, rows[k], kinds[0])
                if ms is not None:
                    hi[k] = ms - p["excluded"]  # "أصحاب التوسط عدا هشام"
                    madd_rows[k] = madd
            continue  # otherwise unknown: [{}, ALL]
        if p["contextual_excluded"]:
            # "الجميع عدا أصحاب الإشباع": all minus the madd group, when its madd is known
            subtract: set[str] = set()
            for kind in p["contextual_excluded"]:
                ms, madd = (
                    _madd_set(lex, canon, rows[k], kind)
                    if kind.startswith("madd_")
                    else (None, None)
                )
                if ms is None:
                    subtract = None
                    break
                subtract |= ms
                madd_rows[k] = madd
            if subtract is None:
                p["contextual"] = list(p["contextual_excluded"])
                continue
            p["excluded"] = set(p["excluded"]) | subtract
        if p["all"]:
            if p["except_following"] or p["rest"]:
                rel[k] = True
            else:
                lo[k] = hi[k] = set(every - p["excluded"])
        elif p["rest"]:
            rel[k] = True
        elif p["named"] or p["included"]:
            lo[k] = set(p["named"]) | set(p["included"])
            hi[k] = set(lo[k]) if not p["include_previous"] else set(every)
    groups: dict[str, list[int]] = {}
    for k, r in enumerate(rows):
        groups.setdefault(r.get("group") or r["id"], []).append(k)
    for _g, ks in groups.items():
        for _ in range(3):  # propagate (rows depending on rows)
            for k in ks:
                if k in madd_rows and parses[k]["contextual"]:
                    # by elimination: a member of the madd school listed in no other row of a
                    # segment that accounts for all 20 can only be in this row
                    others = [j for j in ks if j != k]
                    if set().union(*(hi[j] for j in ks)) == every:
                        lo[k] = (
                            hi[k] - set().union(*(hi[j] for j in others)) if others else set(hi[k])
                        )
                    continue
                if not rel[k]:
                    continue
                p = parses[k]
                others = [j for j in ks if (j > k if p["except_following"] else j != k)]
                lo[k] = (
                    set(every - p["excluded"] - set().union(*(hi[j] for j in others)))
                    if others
                    else set(every - p["excluded"])
                )
                hi[k] = (
                    set(every - p["excluded"] - set().union(*(lo[j] for j in others)))
                    if others
                    else set(every - p["excluded"])
                )
    for k, (r, p) in enumerate(zip(rows, parses, strict=False)):
        resolved = lo[k] == hi[k] and bool(lo[k] or p["all"] or p["rest"])
        if resolved:
            status = "resolved"
        elif lo[k]:
            status = "partial"
        elif p["contextual"] and k not in madd_rows:
            status = "contextual"
        elif k in madd_rows or rel[k] or rel[k]:
            status = "partial"
        else:
            status = "unresolved"
        r["ruwat"] = sorted(lo[k]) if resolved else None
        r["ruwat_min"] = sorted(lo[k])
        r["ruwat_max"] = sorted(hi[k])
        rp = {"status": status, "forms": p["forms"]}
        for key in ("named", "included", "excluded", "khulf", "waqf_only"):
            if p[key]:
                rp[key] = sorted(p[key])
        for key in ("all", "except_following", "rest", "include_previous"):
            if p[key]:
                rp[key] = True
        if p["contextual"]:
            rp["contextual"] = p["contextual"]
        if p["contextual_excluded"]:
            rp["contextual_excluded"] = p["contextual_excluded"]
        if k in madd_rows:
            rp["madd"] = madd_rows[k]
            rp["madd_cite"] = lex.madd_cite[madd_rows[k]]
        if p["unparsed"] and not p["forms"]:
            rp["unparsed"] = " ".join(p["unparsed"])
        r["readers_parse"] = rp
    # what each segment lists (informational: some segments show only the differing ruwat)
    for _g, ks in groups.items():
        first = rows[ks[0]]
        certain = set().union(*(lo[k] for k in ks))
        possible = set().union(*(hi[k] for k in ks))
        first["coverage"] = {
            "all_certain": certain == every,
            "not_listed": sorted(every - possible),
            "uncertain": sorted(possible - certain),
        }


def process(paths: list[Path]) -> dict:
    from .quran import canon

    lex = Lexicon()
    ref = canon()
    stats: dict = {
        "rows": 0,
        "resolved": 0,
        "partial": 0,
        "contextual": 0,
        "unresolved": 0,
        "segments": 0,
        "all_certain": 0,
        "some_not_listed": 0,
        "some_uncertain": 0,
    }
    for path in paths:
        recs = [json.loads(ln) for ln in path.read_text().splitlines()]
        rows = [r for r in recs if r["type"] == "row"]
        resolve_rows(lex, rows, ref)
        for r in rows:
            stats["rows"] += 1
            stats[r["readers_parse"]["status"]] += 1
            if "coverage" in r:
                c = r["coverage"]
                stats["segments"] += 1
                stats["all_certain"] += c["all_certain"]
                stats["some_not_listed"] += bool(c["not_listed"])
                stats["some_uncertain"] += bool(c["uncertain"])
        path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in recs))
    return stats


if __name__ == "__main__":
    print(json.dumps(process([Path(p) for p in sys.argv[1:]]), indent=1))
