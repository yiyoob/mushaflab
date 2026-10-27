"""`just budur-verify`: the al-Budur pilot (TB.6).

    BOOKS_DIR=<main checkout>/data/sources/raw/books just budur-verify

Range: al-Fatihah and al-Baqarah up to the book's first rub' end (the entry the book marks
"akhir/muntaha al-rub'"), all 8 riwayat, with the rule index built from the whole book.
Outputs (untracked: they hold book text, LICENSES.md books row):
  build/budur/index.json            the full-book rule index and corrections
  build/budur/pilot/report.json     every unit, status, verdict and finding
  build/budur/pilot/report.html     the owner's coverage report
Preflight, before anything is judged: the EPUB matches MANIFEST; every cited lexicon form
occurs on its cited page; the book's rub' end is placed uniquely.
"""

import json
import sys
from collections import Counter, defaultdict
from dataclasses import asdict
from pathlib import Path

import yaml

from pipeline.budur import compare, epub, forms, index, match, readers, report, statements
from pipeline.codepoints import load_table
from pipeline.rule_mining import View, match_set
from pipeline.spine import to_cps

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "build" / "budur"
RULES_DIR = Path(__file__).resolve().parents[2] / "rules"
PILOT_SURAHS = (1, 2)
FARSH = ("farsh", "rub_end")
LISTS = ("mumal", "mudgham_saghir", "mudgham_kabir")
IMPLICIT_CITE = "page_1_0001 p. 5"  # each word's khilaf among the ten is stated where discussed


class PilotError(RuntimeError):
    pass


def preflight(paras: list[epub.Para]) -> list[str]:
    by_page: dict[str, list[str]] = defaultdict(list)
    for p in paras:
        by_page[p.page_id] += readers.tokens(p.text)
    lex = readers.load()
    problems = []
    for form, page in readers.cited_pages():
        key, toks = readers.tokens(form), by_page.get(page, [])
        prefixes = ("", *lex.name_prefixes, *lex.phrase_prefixes)
        ok = any(
            all(toks[i + j] == key[j] for j in range(1, len(key)))
            and any(toks[i] == p + key[0] for p in prefixes)
            for i in range(len(toks) - len(key) + 1)
        )
        if not ok:
            problems.append(f"readers.yaml: {form!r} not found on its cited page {page}")
    return problems


def _entries(
    blocks: list[statements.Block], idx: match.Index
) -> list[tuple[statements.Block, int, list[match.Candidate]]]:
    out = []
    for b in blocks:
        if b.section not in FARSH or b.surah not in PILOT_SURAHS or not b.headwords:
            continue
        lo, hi = idx.surah_range[b.surah]
        for h, head in enumerate(b.headwords):
            cands = idx.candidates(head, lo, hi)
            out.append((b, h, match.combine(cands, b.count) if b.count else cands))
    return out


def _spans(c: match.Candidate) -> list[int]:
    return [p for a, z in c.spans for p in range(a, z + 1)]


def place(blocks: list[statements.Block], idx: match.Index, n_words: int) -> dict:
    ents = _entries(blocks, idx)
    placed = match.align([c for _, _, c in ents], n_words)
    by_block: dict[str, list[dict]] = defaultdict(list)
    for (b, h, _), p in zip(ents, placed, strict=True):
        by_block[b.id].append(
            {
                "headword": b.headwords[h],
                "status": p.status,
                "candidates": p.candidates,
                "out_of_order": p.out_of_order,
                "places": [_spans(c) for c in p.places],
                "tiers": [c.tier for c in p.places],
            }
        )
    rub_end = next((b for b in blocks if b.section == "rub_end" and b.rub == 0), None)
    if rub_end is None or by_block[rub_end.id][0]["status"] != "matched":
        raise PilotError("the book's first rub' end could not be placed uniquely")
    end = by_block[rub_end.id][0]["places"][0][-1]
    lists: dict[str, dict] = {}
    for b in blocks:
        if b.section in LISTS and b.rub == 0:
            cands = [c for head in b.headwords for c in idx.candidates(head, 0, end)]
            p = match.place_list(sorted(cands, key=lambda c: c.start), b.count)
            lists[b.id] = {
                "headword": " + ".join(b.headwords),
                "status": p.status,
                "candidates": p.candidates,
                "out_of_order": False,
                "places": [_spans(c) for c in p.places],
                "tiers": [c.tier for c in p.places],
            }
    for k, v in lists.items():
        by_block[k] = [v]
    return {"end": end, "rub_end_block": rub_end.id, "by_block": dict(by_block)}


def _backref_source(
    b: statements.Block, blocks_before: list[statements.Block]
) -> statements.Block | None:
    keys = {match.book_key(h) for h in b.headwords}
    for prev in reversed(blocks_before):
        if prev.kind == "statement" and keys & {match.book_key(h) for h in prev.headwords}:
            return prev
    return None


def entry_units(
    blocks: list[statements.Block], placement: dict, in_scope: set[str]
) -> tuple[list[compare.Unit], list[dict]]:
    """Units for the placed entries in range (two per entry with an ambiguous complement)."""
    end = placement["end"]
    units: list[compare.Unit] = []
    not_placed: list[dict] = []
    seen: list[statements.Block] = []
    for b in blocks:
        pls = placement["by_block"].get(b.id, [])
        not_placed += [{"block": b.id, **p} for p in pls if p["status"] != "matched"]
        good = [p for p in pls if p["status"] == "matched" and p["places"][0][0] <= end]
        if good and b.section != "rub_end":
            words = sorted({w for p in good for pl in p["places"] for w in pl if w <= end})
            src = (_backref_source(b, seen) or b) if b.kind == "backref" else b
            for reading in ("prev", "all") if compare.ambiguous_complement(src) else ("prev",):
                u = compare.entry_unit(src, words, in_scope, reading)
                u.id, u.cite = (b.id if reading == "prev" else f"{b.id}~all"), b.cite
                if src is not b:
                    u.note = f"back-reference to {src.id}"
                units.append(u)
        seen.append(b)
    return units, not_placed


def rule_applications(
    rules: list[index.Rule], idx: match.Index, end: int, mentioned: set[int]
) -> list[dict]:
    """Every place in range where an applied rule's headwords occur (exceptions removed)."""
    out = []
    for r in rules:
        if not r.applied:
            continue
        lo, hi = (0, end) if r.scope == "whole_quran_word" else _surah_window(idx, r.surah, end)
        excluded = {match.book_key(e) for e in r.exceptions}
        for head in r.headwords:
            if lo > hi or match.book_key(head) in excluded:
                continue
            for c in idx.candidates(head, lo, hi):
                if c.tier == "no-hamza":
                    continue  # too weak to apply a rule
                ws = _spans(c)
                out.append({
                    "rule": r.id, "headword": head, "words": ws, "tier": c.tier,
                    "stated_after": idx.words[ws[0]].surah < (r.surah or 0),
                    "own_place": bool(set(ws) & mentioned),
                })  # fmt: skip
    return out


def rule_unit(r: index.Rule, a: dict, in_scope: set[str]) -> compare.Unit:
    """A rule application as a unit; any of the 8 covered covers all of them (p. 5)."""
    covered = set(in_scope) if (r.readers or r.all_readers or r.complement) else set()
    u = compare.Unit(f"{r.id}@{a['words'][0]}", "rule", a["words"], covered, cite=r.cite)
    if r.feature == "textual" and r.wq != "waqf":
        group = r.complement_in_scope if r.complement else r.readers
        if group is not None:
            u.splits.append(compare.Split(frozenset(group), frozenset(r.khulf), r.id))
    elif r.feature == "phonetic":
        u.phonetic_named |= r.readers
    return u


def build_units(
    blocks: list[statements.Block],
    placement: dict,
    rules: list[index.Rule],
    idx: match.Index,
    in_scope: set[str],
) -> dict:
    units, not_placed = entry_units(blocks, placement, in_scope)
    mentioned = {w for u in units for w in u.words if u.covered}
    applications = rule_applications(rules, idx, placement["end"], mentioned)
    by_id = {r.id: r for r in rules}
    units += [rule_unit(by_id[a["rule"]], a, in_scope) for a in applications if not a["own_place"]]
    return {"units": units, "not_placed": not_placed, "applications": applications}


def _surah_window(idx: match.Index, surah: int | None, end: int) -> tuple[int, int]:
    if surah is None:
        return (1, 0)
    lo, hi = idx.surah_range[surah]
    return lo, min(hi, end)


def our_forms(words: list[forms.Word], end: int, riwayat: list[str]) -> dict[str, dict[int, tuple]]:
    table = load_table()
    conv = forms.load_conventions()
    out: dict[str, dict[int, tuple]] = {}
    for r in riwayat:
        pt = forms.load_patches(r)
        out[r] = {}
        for w in words[: end + 1]:
            strict = forms.our_text(w, pt)
            out[r][w.pos] = (strict, forms.loose(strict, conv, table), pt.ids_by_word.get(w.id, []))
    return out


def _diff(a: str | None, b: str | None) -> str:
    ca, cb = Counter(a or ""), Counter(b or "")
    minus = " ".join("-" + to_cps(c) for c in sorted((ca - cb).elements()))
    plus = " ".join("+" + to_cps(c) for c in sorted((cb - ca).elements()))
    return f"{minus} {plus}".strip() if a is not None and b is not None else "deleted"


def _bases(text: str | None, table: dict) -> str:
    return "".join(b for b, _ in forms.letters(text or "", table) if table[ord(b)].cls == "base")


def merge_overlaps(units: list[compare.Unit]) -> list[compare.Unit]:
    """A word's book partition is everything the book says there: each unit also carries the
    splits and phonetic names of the other units that share a word with it (the two
    readings of an ambiguous complement stay apart)."""
    out = []
    for u in units:
        base = u.id.removesuffix("~all")
        extra = [o for o in units
                 if o.id.removesuffix("~all") != base and not o.id.endswith("~all")
                 and set(o.words) & set(u.words) and o.covered]  # fmt: skip
        m = compare.Unit(u.id, u.kind, u.words, set(u.covered), list(u.splits),
                         set(u.explicit), set(u.phonetic_named), list(u.phonetic_terms),
                         u.cite, u.note)  # fmt: skip
        for o in extra:
            m.splits += o.splits
            m.phonetic_named |= o.phonetic_named
        out.append(m)
    return out


def judge_all(units: list[compare.Unit], ours: dict, end: int, riwayat: list[str]) -> dict:
    table = load_table()

    def changed(loose: dict[str, str]) -> frozenset[str]:
        h = _bases(loose["hafs"], table)
        return frozenset(r for r in riwayat if _bases(loose[r], table) != h)

    verdicts = []
    in_unit: dict[int, list[str]] = defaultdict(list)
    by_id: dict[str, compare.Verdict] = {}
    units = merge_overlaps(units)
    for u in units:
        if not u.covered:
            continue
        loose = {r: " ".join(ours[r][w][1] or "" for w in u.words) for r in riwayat}
        diffs = {
            r: _diff(loose["hafs"], loose[r]) if loose[r] != loose["hafs"] else "" for r in riwayat
        }
        v = compare.judge(u, loose, diffs, riwayat, changed(loose))
        if u.id.endswith("~all"):
            first = by_id[u.id.removesuffix("~all")]
            same = (first.flagged, first.possible, [f.pair for f in first.findings]) == (
                v.flagged,
                v.possible,
                [f.pair for f in v.findings],
            )
            if not same:  # the two readings of the complement disagree: decide nothing
                first.undecidable = "complement-scope"
                first.flagged, first.possible = set(), set()
            continue
        by_id[u.id] = v
        verdicts.append(v)
        for w in u.words:
            in_unit[w].append(u.id)
    for w in range(end + 1):
        if w in in_unit:
            continue
        loose = {r: ours[r][w][1] or "" for r in riwayat}
        if len(set(loose.values())) == 1:
            continue
        diffs = {
            r: _diff(loose["hafs"], loose[r]) if loose[r] != loose["hafs"] else "" for r in riwayat
        }
        u = compare.Unit(f"w{w}", "silence", [w], set())
        verdicts.append(compare.judge(u, loose, diffs, riwayat, changed(loose)))
    return {"verdicts": verdicts, "in_unit": dict(in_unit)}


def statuses(units: list[compare.Unit], end: int, riwayat: list[str]) -> dict[int, dict[str, dict]]:
    out: dict[int, dict[str, dict]] = {
        w: {r: {"status": "not-mentioned"} for r in riwayat} for w in range(end + 1)
    }
    for u in units:
        if u.id.endswith("~all"):
            continue
        for w in u.words:
            if w > end:
                continue
            for r in riwayat:
                cur = out[w][r]
                if r in u.covered and u.kind == "entry":
                    cur.update(status="mentioned", unit=u.id, cite=u.cite, note=u.note)
                elif r in u.covered and u.kind == "rule" and cur["status"] != "mentioned":
                    cur.update(status="rule-covered", unit=u.id, cite=u.cite)
                elif u.kind == "entry" and not u.covered and cur["status"] == "not-mentioned":
                    cur["note"] = f"{u.id}: discussed for other readers only"
    return out


def phase6_support(words: list[forms.Word], end: int, stat: dict, riwayat: list[str]) -> list[dict]:
    view = View.load()
    last = words[end]
    scope = [
        {"surah": s, "from": 1, "to": (last.ayah if s == last.surah else 10**6)}
        for s in range(1, last.surah + 1)
    ]
    pos_of = {w.id: w.pos for w in words[: end + 1]}
    out = []
    for r in riwayat:
        path = RULES_DIR / f"{r}.yaml"
        reg_path = RULES_DIR / f"{r}.register.yaml"
        if not path.exists():
            continue
        rules = yaml.safe_load(path.read_text(encoding="utf-8"))["rules"]
        reg = (
            {e["id"]: e for e in yaml.safe_load(reg_path.read_text(encoding="utf-8"))["rules"]}
            if reg_path.exists()
            else {}
        )
        for rule in rules:
            exc = {e["cluster"] for e in rule.get("except", [])}
            cids = [c for c in match_set(rule["pattern"], view, scope) if c not in exc]
            ws = sorted(
                {
                    pos_of[":".join(c.split(":")[:3])]
                    for c in cids
                    if ":".join(c.split(":")[:3]) in pos_of
                }
            )
            if not ws:
                continue
            st = Counter(stat[w][r]["status"] for w in ws)
            kind = reg.get(rule["id"], {}).get("difference_kind", "")
            support = (
                "encoding-only"
                if kind == "encoding"
                else "supported"
                if st["not-mentioned"] == 0
                else "unsupported"
                if st["not-mentioned"] == len(ws)
                else "partly"
            )
            out.append(
                {
                    "rule": rule["id"],
                    "riwayah": r,
                    "places": len(ws),
                    "words": ws,
                    "statuses": dict(st),
                    "support": support,
                    "difference_kind": kind,
                    "description": reg.get(rule["id"], {}).get("description", ""),
                }
            )
    return out


def awjuh(
    blocks: list[statements.Block], units: list[compare.Unit], in_scope: set[str]
) -> list[dict]:
    placed = {u.id for u in units if u.kind == "entry"}
    out = []
    for b in blocks:
        if b.id not in placed:
            continue
        for i, c in enumerate(b.clauses):
            k = c.readers.khulf & in_scope
            if not (k or (c.awjuh and (c.readers.named & in_scope or c.readers.all_readers))):
                continue
            out.append(
                {
                    "block": b.id,
                    "clause": i,
                    "cite": b.cite,
                    "readers": sorted(k or (c.readers.named & in_scope)),
                    "khulf": bool(k),
                    "awjuh": c.awjuh,
                    "wq": c.wq,
                    "feature": c.feature,
                    "kind": "usul-option" if c.feature == "phonetic" else "farsh-margin",
                    "restriction": c.restriction,
                    "default": "first in sentence order (owner to confirm)",
                    "text": c.text,
                }
            )
    return out


def run(env: dict[str, str] | None = None) -> dict:
    path = epub.book_path(env)
    epub.verify(path)
    paras = epub.read(path)
    problems = preflight(paras)
    if problems:
        raise PilotError("; ".join(problems))
    lex, feats = readers.load(), statements.load_features()
    in_scope = set(lex.in_scope)
    riwayat = list(lex.in_scope)
    blocks = statements.blocks(paras, lex, feats)
    rules = index.rules(blocks, in_scope)
    corrs = index.corrections(blocks)
    words = forms.load_words()
    idx = match.Index(words, load_table())
    placement = place(blocks, idx, len(words))
    end = placement["end"]
    built = build_units(blocks, placement, rules, idx, in_scope)
    units = built["units"]
    ours = our_forms(words, end, riwayat)
    judged = judge_all(units, ours, end, riwayat)
    stat = statuses(units, end, riwayat)
    support = phase6_support(words, end, stat, riwayat)
    result = {
        "range": {
            "first": words[0].id,
            "last": words[end].id,
            "words": end + 1,
            "rub_end_block": placement["rub_end_block"],
            "spine_rub_sign_after": end + 1 < len(words) and words[end + 1].after_rub_sign,
        },
        "blocks": {b.id: asdict(b) for b in blocks if b.surah in PILOT_SURAHS and b.rub == 0},
        "placement": placement["by_block"],
        "units": [asdict(u) for u in units],
        "not_placed": built["not_placed"],
        "applications": built["applications"],
        "verdicts": [asdict(v) for v in judged["verdicts"]],
        "statuses": {str(w): s for w, s in stat.items()},
        "words": [
            {
                "pos": w.pos,
                "id": w.id,
                "forms": {
                    r: [
                        to_cps(ours[r][w.pos][0] or ""),
                        to_cps(ours[r][w.pos][1] or ""),
                        ours[r][w.pos][2],
                    ]
                    for r in riwayat
                },
            }
            for w in words[: end + 1]
        ],
        "phase6": support,
        "awjuh": awjuh(blocks, units, in_scope),
        "conventions": forms.load_conventions().rows,
        "riwayat": riwayat,
    }
    full_index = {
        "rules": [asdict(r) for r in rules],
        "corrections": [asdict(c) for c in corrs],
        "blocks": len(blocks),
    }
    return {"pilot": result, "index": full_index}


def main(argv: list[str]) -> int:
    del argv
    out = run()
    (OUT / "pilot").mkdir(parents=True, exist_ok=True)
    (OUT / "index.json").write_text(
        json.dumps(out["index"], ensure_ascii=False, indent=1, default=sorted), "utf-8"
    )
    (OUT / "pilot" / "report.json").write_text(
        json.dumps(out["pilot"], ensure_ascii=False, indent=1, default=sorted), "utf-8"
    )
    summary = report.summary(out["pilot"], out["index"])
    (OUT / "pilot" / "report.html").write_text(
        report.render(out["pilot"], out["index"], summary), "utf-8"
    )
    for line in report.summary_lines(summary):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
