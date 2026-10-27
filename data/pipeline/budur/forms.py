"""Spine words, our word forms per riwayah, and the convention projections (TB.5, D50).

Our form of a spine word for a riwayah is the word with that riwayah's raw patches applied
(the same operations as the resolver; Phase 4 patches reproduce KFGQPC exactly). Inserted
clusters join their word; inserted *words* join the word they follow. Owner-confirmed
`encoding` rows (data/approvals/difference_kinds.yaml, D21, read through pipeline.classify)
are mapped back to the Hafs codepoint; that is the `strict` form.

The `loose` form additionally drops or merges the codepoints listed in
`data/books/al-budur-al-zahirah/conventions.yaml`: candidate spelling/mark conventions of the
KFGQPC encodings, each with its evidence. They are OUR hypothesis (D50, proposed): the owner
confirms or removes each row. Book-vs-data comparisons of written readings use `loose`; a
difference seen only in `strict` is reported as a convention candidate, never as a reading.
"""

from dataclasses import dataclass, field
from pathlib import Path

import yaml

from pipeline import classify, spine
from pipeline.codepoints import Row
from pipeline.differ import simulate_set_marks
from pipeline.spine import from_cps

DATA = Path(__file__).resolve().parents[2]
PATCH_DIR = DATA / "patches" / "raw"
CONVENTIONS = DATA / "books" / "al-budur-al-zahirah" / "conventions.yaml"
_Loader = getattr(yaml, "CSafeLoader", yaml.SafeLoader)


@dataclass(frozen=True)
class Word:
    id: str  # "s:a:w"
    surah: int
    ayah: int
    pos: int  # index among all spine words
    clusters: tuple[tuple[str, str], ...]  # (cluster id, text)
    after_rub_sign: bool = False  # a hizb/rub' sign item precedes it (spine kind "hizb")

    @property
    def text(self) -> str:
        return "".join(t for _, t in self.clusters)


def load_words(recs: list[dict] | None = None) -> list[Word]:
    recs = spine.load_records() if recs is None else recs
    out: list[Word] = []
    sign = False
    for r in recs:
        if r["kind"] == "hizb":
            sign = True
            continue
        if r["kind"] != "word":
            continue
        s, a, _ = (int(x) for x in r["id"].split(":"))
        cl = tuple((c["id"], from_cps(c["cps"])) for c in r["clusters"])
        out.append(Word(r["id"], s, a, len(out), cl, sign))
        sign = False
    return out


@dataclass
class Patches:
    riwayah: str
    by_word: dict[str, list[dict]] = field(default_factory=dict)
    after_word: dict[str, list[dict]] = field(default_factory=dict)
    to_hafs: dict[str, str] = field(default_factory=dict)  # confirmed encoding rows, reversed
    ids_by_word: dict[str, list[str]] = field(default_factory=dict)


def load_patches(riwayah: str, path: Path | None = None) -> Patches:
    out = Patches(riwayah)
    if riwayah == "hafs":
        return out  # the spine is Hafs (corrections are not raw patches)
    path = path or PATCH_DIR / f"{riwayah}.yaml"
    doc = yaml.load(path.read_text(encoding="utf-8"), Loader=_Loader)  # noqa: S506 (SafeLoader)
    for p in doc["patches"]:
        word_id = ":".join(p["target"].split("+")[0].split(":")[:3])
        out.ids_by_word.setdefault(word_id, []).append(p["id"])
        if p["op"] in ("insert_after", "insert_before") and "item" in p.get("payload", {}):
            out.after_word.setdefault(word_id, []).append(p)
        else:
            out.by_word.setdefault(word_id, []).append(p)
    for (frm, to), kind in classify.load_confirmed().get(riwayah, {}).items():
        if kind == "encoding":
            out.to_hafs[to] = frm
    return out


def our_text(w: Word, patches: Patches) -> str | None:
    """Our strict text of `w` (plus words inserted after it); None if the word is deleted."""
    ps = patches.by_word.get(w.id, [])
    if any(p["op"] == "delete" and p["target"] == w.id for p in ps):
        return None
    cl = list(w.clusters)
    for p in ps:
        op, target, payload = p["op"], p["target"], p.get("payload", {})
        at = next((i for i, (cid, _) in enumerate(cl) if cid == target), None)
        if op == "replace" and target == w.id and set(payload) <= {"sep_after", "sep_before"}:
            continue  # spacing only
        if op == "replace" and at is not None:
            cl[at] = (target, from_cps(payload["cps"]))
        elif op == "set_marks" and at is not None:
            cl[at] = (target, simulate_set_marks(cl[at][1], payload))
        elif op == "delete" and at is not None:
            del cl[at]
        elif op in ("insert_after", "insert_before") and at is not None and "cluster" in payload:
            new = (f"{target}+ins.{payload['seq']}", from_cps(payload["cluster"]["cps"]))
            cl.insert(at + 1 if op == "insert_after" else at, new)
        else:
            raise ValueError(f"patch {p['id']}: cannot apply {op} to {target}")
    text = "".join(t for _, t in cl)
    for p in sorted(patches.after_word.get(w.id, []), key=lambda p: p["payload"]["seq"]):
        item = p["payload"]["item"]
        if item.get("kind") == "word":
            text += " " + "".join(from_cps(c["cps"]) for c in item["clusters"])
    return "".join(patches.to_hafs.get(c, c) for c in text)


@dataclass(frozen=True)
class Conventions:
    drop: frozenset[str]
    merge: dict[str, str]
    cluster_rules: dict[str, frozenset[str]]  # base -> marks dropped on that base
    word_level: frozenset[str]  # marks compared per word, not per letter
    sort_marks: bool
    rows: list[dict]


def load_conventions(path: Path = CONVENTIONS) -> Conventions:
    d = yaml.safe_load(path.read_text(encoding="utf-8"))
    drop, merge = set(), {}
    for r in d["rows"]:
        cp = from_cps(r["cp"])
        if r["action"] == "drop":
            drop.add(cp)
        else:
            merge[cp] = from_cps(r["to"])
    rules = {
        from_cps(r["base"]): frozenset(from_cps(m) for m in r["drop_marks"])
        for r in d.get("cluster_rules", [])
    }
    word_level = frozenset(from_cps(m) for m in d.get("word_level_marks", []))
    return Conventions(
        frozenset(drop), merge, rules, word_level, bool(d.get("sort_marks")), d["rows"]
    )


def letters(text: str, table: dict[int, Row]) -> list[tuple[str, list[str]]]:
    """(base, marks) groups; a mark before any base starts its own group."""
    out: list[tuple[str, list[str]]] = []
    for c in text:
        if table[ord(c)].cls == "mark" and out:
            out[-1][1].append(c)
        else:
            out.append((c, []))
    return out


def loose(text: str | None, conv: Conventions, table: dict[int, Row]) -> str | None:
    if text is None:
        return None
    mapped = "".join(conv.merge.get(c, c) for c in text if c not in conv.drop)
    out, floating = [], []
    for base, marks in letters(mapped, table):
        gone = conv.cluster_rules.get(base, frozenset())
        kept = [m for m in marks if m not in gone and m not in conv.word_level]
        floating += [m for m in marks if m in conv.word_level]
        out.append(base + "".join(sorted(kept) if conv.sort_marks else kept))
    return "".join(out) + "".join(sorted(floating))
