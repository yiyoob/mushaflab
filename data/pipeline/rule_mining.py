"""T6.4: data-derived rules for the seven riwayat (ADR D42).

    python -m pipeline.rule_mining write [RIWAYAH...]   # rules, registers, residual patches
    python -m pipeline.rule_mining check                # exit 1 if a generated file is stale

Every rule is induced from the raw patches (T4.5), which reproduce the riwayah's KFGQPC text
exactly. No rule says anything the KFGQPC data does not: a rule is a cluster form with at
most one context feature (its place in the word, the letter before or after it, or the first
letter of the next word) under which the raw patches change that form in one way. A rule is
kept only if it has at least MIN_HITS such places, and the places where its pattern matches
but the raw patches do something else are few (MAX_EXCEPTIONS); those are listed as
exceptions with their reason. Every rule is a `hypothesis` (no citation, CLAUDE.md rule 1)
and `single_source`; the owner or a muqri' adds a source or rejects it at GATE G4.

Outputs per riwayah:
- `rules/<r>.yaml`: the rules (ASCII);
- `rules/<r>.register.yaml`: their register entries;
- `patches/curated/<r>.yaml`: the raw patches no rule explains (the residual), unchanged.
The riwayah's profile then resolves its rules and its residual instead of the raw file, and
`just roundtrip` checks that the result is still the KFGQPC text, codepoint for codepoint.

The matcher below mirrors `mushaf_rules::matches` (leftmost, non-overlapping, in spine order)
so that each rule's match set, and therefore its exceptions, are exactly what the core sees.
"""

import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

import yaml

from pipeline import emit_patches, spine
from pipeline.differ import simulate_set_marks
from pipeline.spine import from_cps, to_cps

DATA = Path(__file__).resolve().parents[1]
RULES_DIR = DATA / "rules"
RESIDUAL_DIR = DATA / "patches" / "curated"
RIWAYAT = ("shuba", "warsh", "qalun", "bazzi", "qunbul", "duri_abuamr", "susi")
MIN_HITS = 3
MAX_EXCEPTIONS = 2  # or MAX_EXCEPTION_SHARE of the matches, whichever is larger
MAX_EXCEPTION_SHARE = 0.05
RULES_PER_TRANSFORM = 4
CANDIDATES = 40
_Loader = getattr(yaml, "CSafeLoader", yaml.SafeLoader)


# --- spine view and matcher ------------------------------------------------------------


@dataclass
class View:
    items: list[dict]  # spine records
    texts: list[list[str]]  # cluster texts per item
    pos: dict[str, tuple[int, int]]  # cluster id -> (item, cluster)
    by_first: dict[str, list[tuple[int, int]]] = field(default_factory=dict)

    @staticmethod
    def load() -> "View":
        recs = spine.load_records()
        texts = [[from_cps(c["cps"]) for c in r["clusters"]] for r in recs]
        pos = {c["id"]: (i, k) for i, r in enumerate(recs) for k, c in enumerate(r["clusters"])}
        by_first: dict[str, list[tuple[int, int]]] = defaultdict(list)
        for i, cs in enumerate(texts):
            for k, t in enumerate(cs):
                by_first[t[0]].append((i, k))
        return View(recs, texts, pos, dict(by_first))

    def cid(self, i: int, k: int) -> str:
        return self.items[i]["clusters"][k]["id"]

    def next_word(self, i: int, boundary: str = "any") -> int | None:
        """The next word item after `i` across a boundary (`mushaf_rules::cross`)."""
        ayah_end = False
        for j in range(i + 1, len(self.items)):
            kind = self.items[j]["kind"]
            if kind == "word":
                if (boundary == "word" and ayah_end) or (boundary == "ayah_end" and not ayah_end):
                    return None
                return j
            if kind == "ayah_end":
                ayah_end = True
        return None


def _at_ok(at: str | None, k: int, n: int) -> bool:
    return (
        at is None
        or (at == "first" and k == 0)
        or (at == "last" and k == n - 1)
        or (at == "only" and n == 1)
    )


def _cluster_ok(m: dict, v: View, i: int, k: int) -> bool:
    t = v.texts[i][k]
    marks = t[1:]
    has = [from_cps(x) in marks for x in m.get("marks_any", [])]
    return (
        ("cps" not in m or from_cps(m["cps"]) == t)
        and ("base" not in m or t[0] in {from_cps(b) for b in m["base"]})
        and ("kind" not in m or v.items[i]["clusters"][k]["kind"] == m["kind"])
        and all(from_cps(x) in marks for x in m.get("marks_all", []))
        and (not has or any(has))
        and not any(from_cps(x) in marks for x in m.get("marks_none", []))
        and (not m.get("bare") or not marks)
        and _at_ok(m.get("at"), k, len(v.texts[i]))
    )


def _match_at(pattern: list[dict], v: View, i: int, k: int) -> tuple[str, tuple[int, int]] | None:
    """The captured cluster id and the last matched position, or None."""
    it, c, first, pending, cap = i, k, True, None, None
    for step in pattern:
        if "boundary" in step:
            pending = step["boundary"]
            continue
        if not first:
            if pending is not None:
                if c + 1 != len(v.texts[it]):
                    return None
                nxt = v.next_word(it, pending)
                if nxt is None:
                    return None
                it, c, pending = nxt, 0, None
            else:
                c += 1
                if c >= len(v.texts[it]):
                    return None
        first = False
        if not _cluster_ok(step, v, it, c):
            return None
        if "capture" in step:
            cap = v.cid(it, c)
    return (cap, (it, c)) if cap else None


def _in_scope(scope: list[dict], item: dict) -> bool:
    s_, a_ = (int(x) for x in item["id"].split(":")[:2])
    return not scope or any(
        s_ == sc["surah"] and a_ >= sc.get("from", 0) and a_ <= sc.get("to", 10**6) for sc in scope
    )


def match_set(pattern: list[dict], v: View, scope: list[dict] = ()) -> list[str]:  # type: ignore[assignment]
    """Captured cluster ids of all matches, as `mushaf_rules::matches` finds them."""
    m0 = pattern[0]
    firsts = (
        {from_cps(m0["cps"])[0]}
        if "cps" in m0
        else ({from_cps(b) for b in m0["base"]} if "base" in m0 else None)
    )
    starts = (
        sorted(p for f in firsts for p in v.by_first.get(f, []))
        if firsts is not None
        else [(i, k) for i, cs in enumerate(v.texts) for k in range(len(cs))]
    )
    out, resume = [], (0, 0)
    for i, k in starts:
        if (i, k) < resume or not _in_scope(list(scope), v.items[i]):
            continue
        r = _match_at(pattern, v, i, k)
        if r is not None:
            out.append(r[0])
            resume = (r[1][0], r[1][1] + 1)
    return out


# --- induction --------------------------------------------------------------------------


@dataclass(frozen=True)
class Change:
    kind: str  # "mod" | "insert_after" | "insert_before"
    old: str  # spine cluster text
    new: str  # new cluster text (mod) or inserted cluster text
    new_kind: str  # cluster kind of the result


def changes(raw: list[dict], v: View) -> dict[tuple[str, str], tuple[Change, dict]]:
    """(target cluster id, kind) -> its change, for single-cluster raw patches."""
    out = {}
    for p in raw:
        t = p["target"]
        if t not in v.pos:  # item targets and inserted ids are left to the residual
            continue
        i, k = v.pos[t]
        old = v.texts[i][k]
        kind0 = v.items[i]["clusters"][k]["kind"]
        pay = p.get("payload", {})
        if p["op"] == "replace":
            ch = Change("mod", old, from_cps(pay["cps"]), pay["kind"])
        elif p["op"] == "set_marks":
            ch = Change("mod", old, simulate_set_marks(old, pay), kind0)
        elif p["op"] in ("insert_after", "insert_before") and "cluster" in pay and pay["seq"] == 1:
            ch = Change(p["op"], old, from_cps(pay["cluster"]["cps"]), pay["cluster"]["kind"])
        else:
            continue
        out[(t, ch.kind)] = (ch, p)
    return out


def _features(v: View, cid: str) -> list[tuple[str, str | None]]:
    i, k = v.pos[cid]
    cs, n = v.texts[i], len(v.texts[i])
    f: list[tuple[str, str | None]] = [("any", None)]
    if n == 1:
        f.append(("at", "only"))
    elif k == 0:
        f.append(("at", "first"))
    elif k == n - 1:
        f.append(("at", "last"))
    if k > 0:
        f.append(("prev", cs[k - 1][0]))
    if k < n - 1:
        f.append(("next", cs[k + 1][0]))
    if k == n - 1 and v.items[i]["kind"] == "word":
        j = v.next_word(i)
        if j is not None:
            f.append(("nextword", v.texts[j][0][0]))
    return f


def pattern_for(old: str, feat: tuple[str, str | None]) -> list[dict]:
    t: dict = {"cps": to_cps(old), "capture": "t"}
    kind, val = feat
    if kind == "at":
        return [{**t, "at": val}]
    if kind == "prev":
        return [{"base": [to_cps(val)]}, t]
    if kind == "next":
        return [t, {"base": [to_cps(val)]}]
    if kind == "nextword":
        return [{**t, "at": "last"}, {"boundary": "any"}, {"base": [to_cps(val)], "at": "first"}]
    return [t]


@dataclass
class Mined:
    id: str
    change: Change
    feature: tuple[str, str | None]
    pattern: list[dict]
    hits: list[str]
    exceptions: list[tuple[str, str]]  # (cluster id, reason)


def _allowed(n_matches: int, n_exc: int) -> bool:
    return n_exc <= max(MAX_EXCEPTIONS, MAX_EXCEPTION_SHARE * n_matches)


def mine(riwayah: str, raw: list[dict], v: View) -> tuple[list[Mined], set[str]]:
    """The rules for one riwayah and the ids of the raw patches they explain."""
    ch = changes(raw, v)
    groups: dict[Change, set[str]] = defaultdict(set)
    for (t, _kind), (c, _p) in ch.items():
        groups[c].add(t)
    claimed: dict[str, set[str]] = defaultdict(set)  # change kind -> claimed targets
    rules: list[Mined] = []
    explained: set[str] = set()
    for c, targets in sorted(groups.items(), key=lambda kv: (-len(kv[1]), repr(kv[0]))):
        remaining = set(targets)
        for _ in range(RULES_PER_TRANSFORM):
            if len(remaining) < MIN_HITS:
                break
            counts = Counter(f for t in remaining for f in _features(v, t))
            best = None
            for feat, n in sorted(counts.items(), key=lambda kv: (-kv[1], repr(kv[0])))[
                :CANDIDATES
            ]:
                if n < MIN_HITS or (best and n < len(best[2])):
                    break
                pat = pattern_for(c.old, feat)
                matched = match_set(pat, v)
                hits = [
                    t
                    for t in matched
                    if t in remaining
                    and t not in claimed[c.kind]
                    and ch.get((t, c.kind), (None,))[0] == c
                ]
                exc = [t for t in matched if t not in hits]
                good = len(hits) >= MIN_HITS and _allowed(len(matched), len(exc))
                # More places first; between equals, fewer exceptions.
                if good and (not best or (len(hits), -len(exc)) > (len(best[2]), -len(best[3]))):
                    best = (feat, pat, hits, exc)
            if best is None:
                break
            feat, pat, hits, exc = best
            reasons = []
            for t in exc:
                other = ch.get((t, c.kind))
                why = (
                    f"KFGQPC {riwayah} text: other change here ({other[1]['id']})"
                    if other
                    else (
                        "an earlier rule covers this place"
                        if t in claimed[c.kind]
                        else f"KFGQPC {riwayah} text: no such change here"
                    )
                )
                reasons.append((t, why))
            rid = f"{riwayah}.d{len(rules) + 1:04d}"
            rules.append(Mined(rid, c, feat, pat, hits, reasons))
            claimed[c.kind].update(hits)
            claimed[c.kind].update(exc)
            for t in hits:
                explained.add(ch[(t, c.kind)][1]["id"])
            remaining -= set(hits)
    return rules, explained


# --- output -----------------------------------------------------------------------------


def _describe(m: Mined) -> str:
    c, (kind, val) = m.change, m.feature
    what = {
        "mod": f"{to_cps(c.old)} becomes {to_cps(c.new)}",
        "insert_after": f"{to_cps(c.new)} is inserted after {to_cps(c.old)}",
        "insert_before": f"{to_cps(c.new)} is inserted before {to_cps(c.old)}",
    }[c.kind]
    where = {
        "any": "everywhere",
        "at": f"when it is the {val} cluster of its word",
        "prev": f"after {to_cps(val) if val else ''}",
        "next": f"before {to_cps(val) if val else ''}",
        "nextword": f"at a word end before a word starting with {to_cps(val) if val else ''}",
    }[kind]
    return f"data-derived: {what} {where} ({len(m.hits)} places, {len(m.exceptions)} exceptions)"


def _action(m: Mined) -> dict:
    c = m.change
    if c.kind == "mod":
        return {
            "op": "replace",
            "target": "t",
            "payload": {"kind": c.new_kind, "cps": to_cps(c.new)},
        }
    return {
        "op": c.kind,
        "target": "t",
        "payload": {"seq": 1, "cluster": {"kind": c.new_kind, "cps": to_cps(c.new)}},
    }


def rule_file(riwayah: str, rules: list[Mined]) -> str:
    out = [
        f"# Data-derived rules for {riwayah} (T6.4, ADR D42): every rule is a `hypothesis` induced",
        f"# from the raw patches (KFGQPC {riwayah}); see {riwayah}.register.yaml. Generated by",
        "# `python -m pipeline.rule_mining write`. Do not edit.",
        f'spine: "{emit_patches.SPINE_ID}"',
        "rules:",
    ]
    for m in rules:
        rule = {
            "id": m.id,
            "owner": {"level": "riwayah", "id": riwayah},
            "single_source": True,
            "pattern": m.pattern,
            "except": [{"cluster": t, "reason": why} for t, why in m.exceptions],
            "actions": [_action(m)],
        }
        if not rule["except"]:
            del rule["except"]
        out.append("  - " + emit_patches.flow(rule))
    return "\n".join(out) + "\n"


def register_file(riwayah: str, rules: list[Mined]) -> str:
    out = [
        f"# Register for rules/{riwayah}.yaml (T6.4, ADR D42). Every entry is a hypothesis: no",
        "# citation yet (CLAUDE.md rule 1). Generated by `python -m pipeline.rule_mining write`.",
        "rules:",
    ]
    for m in rules:
        entry = {
            "id": m.id,
            "owner": {"level": "riwayah", "id": riwayah},
            "description": _describe(m),
            "option": None,
            "source": None,
            "status": "hypothesis",
            "single_source": True,
        }
        out.append("  - " + emit_patches.flow(entry))
    return "\n".join(out) + "\n"


def residual_file(riwayah: str, raw_text: str, explained: set[str]) -> str:
    """The raw file without the explained patches (lines kept byte for byte)."""
    out = [
        f"# Residual patches for {riwayah} (T6.4, ADR D42): the raw patches",
        f"# (patches/raw/{riwayah}.yaml) that no data-derived rule explains, unchanged. Curated",
        "# farsh gets citations here as they are found. Generated by",
        "# `python -m pipeline.rule_mining write`. Do not edit.",
    ]
    for line in raw_text.splitlines():
        if line.startswith("#"):
            continue
        if line.startswith("  - {id: "):
            pid = line.split('"', 2)[1]
            if pid in explained:
                continue
        out.append(line)
    return "\n".join(out) + "\n"


def build(riwayah: str, v: View) -> dict[Path, str]:
    raw_path = DATA / "patches" / "raw" / f"{riwayah}.yaml"
    raw_text = raw_path.read_text(encoding="ascii")
    raw = yaml.load(raw_text, Loader=_Loader)["patches"]  # noqa: S506 (a SafeLoader)
    rules, explained = mine(riwayah, raw, v)
    return {
        RULES_DIR / f"{riwayah}.yaml": rule_file(riwayah, rules),
        RULES_DIR / f"{riwayah}.register.yaml": register_file(riwayah, rules),
        RESIDUAL_DIR / f"{riwayah}.yaml": residual_file(riwayah, raw_text, explained),
    }


def main(argv: list[str]) -> int:
    if argv[1:2] not in (["write"], ["check"]):
        print(__doc__, file=sys.stderr)
        return 2
    riwayat = argv[2:] or list(RIWAYAT)
    v = View.load()
    stale = []
    for r in riwayat:
        for path, text in build(r, v).items():
            if argv[1] == "write":
                path.write_text(text, encoding="ascii")
            elif not path.exists() or path.read_text(encoding="ascii") != text:
                stale.append(str(path.relative_to(DATA)))
        print(f"rule_mining: {r} done", file=sys.stderr)
    for s in stale:
        print(f"stale: {s}", file=sys.stderr)
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
