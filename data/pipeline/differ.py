"""T4.4: turn an alignment (T4.3) into patch ops on the Hafs spine (D12 semantics, D20).

* Paired items: separators that differ -> one item `replace`; clusters aligned by base
  codepoint (difflib, then position or a small cost-based pairing). A paired cluster with the
  same base and kind -> one `set_marks` (checked by simulating the resolver's rule, else a
  `replace`); a different base or kind -> cluster `replace`; an unpaired Hafs cluster ->
  cluster `delete`; an unpaired riwayah cluster -> cluster insert after the previous Hafs
  cluster of the item (or before its first cluster).
* Unpaired Hafs item -> item `delete`; unpaired riwayah item -> item insert after the previous
  Hafs item of the surah, deleted or not (the D12 replacement), or before the surah's first
  item. `seq` counts 1, 2, ... per anchor, so every key `<anchor>+ins.<seq>` is unique.
No op ever targets a cluster of an item the same file deletes (D12 `Overlap`).
"""

import difflib
import functools
from collections import defaultdict
from collections.abc import Callable
from dataclasses import dataclass

from pipeline.align import Pair
from pipeline.clusterize import Cluster
from pipeline.ingest_riwayah import Item
from pipeline.spine import from_cps, to_cps

OP_ABBR = {
    "delete": "del",
    "replace": "rep",
    "insert_before": "ins",
    "insert_after": "ins",
    "set_marks": "marks",
}
CLUSTER_GAP, BASE_MISMATCH = 0.6, 0.5

Clusterizer = Callable[[str], list[Cluster]]


@dataclass(frozen=True)
class Change:
    target: str
    op: str
    payload: dict | None
    level: str  # item | cluster
    kind: str  # separator | marks | letter | cluster-delete | cluster-insert | item-delete | ...
    item_kind: str  # word | ayah_end | hizb
    old: str  # the Hafs text concerned ("" for inserts)
    new: str  # the riwayah text ("" for deletes)
    ref: str  # riwayah item ref ("" when only Hafs is involved)
    surah: int

    @property
    def key(self) -> str:
        """The id this change claims in its layer (D9): the new id for an insert."""
        if self.op.startswith("insert"):
            return f"{self.target}+ins.{self.payload['seq']}"  # type: ignore[index]
        return self.target


def patch_id(riwayah: str, key: str, op: str) -> str:
    return f"{riwayah}.{key.replace(':', '.')}.{OP_ABBR[op]}"


# --- set_marks ----------------------------------------------------------------------------


def _cp(c: str) -> str:
    return f"U+{ord(c):04X}"


def simulate_set_marks(text: str, payload: dict) -> str:
    """The resolver's `set_marks` (D12): remove first occurrences, then add in order."""
    base, marks = text[0], list(text[1:])
    for cp in payload.get("remove", []):
        marks.remove(from_cps(cp))
    for add in payload.get("add", []):
        if isinstance(add, str):
            marks.append(from_cps(add))
        else:
            marks.insert(add["at"], from_cps(add["cp"]))
    return base + "".join(marks)


def set_marks_payload(old: str, new: str) -> dict | None:
    """A `set_marks` payload turning `old` into `new` (same base), or None if equal."""
    if old == new:
        return None
    src, dst = old[1:], new[1:]
    matcher = difflib.SequenceMatcher(None, src, dst, autojunk=False)
    kept = {i for a, _b, n in matcher.get_matching_blocks() for i in range(a, a + n)}
    remove = [c for i, c in enumerate(src) if i not in kept]
    left = list(src)
    for c in remove:
        left.remove(c)
    # Adds go in ascending final position: when dst[k] is added, dst[:k] is already in place
    # and only kept marks follow it, so `at = k`; with none following, it is an append.
    adds: list = []
    j = 0
    for k, c in enumerate(dst):
        if j < len(left) and left[j] == c:
            j += 1
        else:
            adds.append(_cp(c) if j == len(left) else {"cp": _cp(c), "at": k})
    payload = {
        **({"remove": [_cp(c) for c in remove]} if remove else {}),
        **({"add": adds} if adds else {}),
    }
    if simulate_set_marks(old, payload) == new:
        return payload
    payload = {
        **({"remove": [_cp(c) for c in src]} if src else {}),
        **({"add": [_cp(c) for c in dst]} if dst else {}),
    }
    if simulate_set_marks(old, payload) != new:  # pragma: no cover - cannot happen
        raise AssertionError("set_marks fallback does not reproduce the target")
    return payload


# --- cluster alignment --------------------------------------------------------------------


def pair_clusters(
    hs: list[tuple[str, str, str]], rs: list[Cluster]
) -> list[tuple[tuple[str, str, str] | None, Cluster | None]]:
    """(id, kind, text) Hafs clusters against riwayah clusters, by base codepoint."""
    matcher = difflib.SequenceMatcher(
        None, [h[2][0] for h in hs], [r.text[0] for r in rs], autojunk=False
    )
    out: list[tuple[tuple[str, str, str] | None, Cluster | None]] = []
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        a, b = hs[i1:i2], rs[j1:j2]
        if tag == "equal" or len(a) == len(b):
            out += list(zip(a, b, strict=True))
        else:
            out += _cluster_dp(a, b)
    return out


def _cluster_dp(
    a: list[tuple[str, str, str]], b: list[Cluster]
) -> list[tuple[tuple[str, str, str] | None, Cluster | None]]:
    n, m = len(a), len(b)
    cost = [[0.0] * (m + 1) for _ in range(n + 1)]
    move = [[""] * (m + 1) for _ in range(n + 1)]
    for i in range(n + 1):
        for j in range(m + 1):
            if i == j == 0:
                continue
            options = []
            if i and j:
                c = 0.0 if a[i - 1][2][0] == b[j - 1].text[0] else BASE_MISMATCH
                options.append((cost[i - 1][j - 1] + c, 0, "pair"))
            if i:
                options.append((cost[i - 1][j] + CLUSTER_GAP, 1, "del"))
            if j:
                options.append((cost[i][j - 1] + CLUSTER_GAP, 2, "ins"))
            best = min(options)
            cost[i][j], move[i][j] = best[0], best[2]
    out: list[tuple[tuple[str, str, str] | None, Cluster | None]] = []
    i, j = n, m
    while i or j:
        mv = move[i][j]
        if mv == "pair":
            out.append((a[i - 1], b[j - 1]))
            i, j = i - 1, j - 1
        elif mv == "del":
            out.append((a[i - 1], None))
            i -= 1
        else:
            out.append((None, b[j - 1]))
            j -= 1
    return out[::-1]


# --- diff ---------------------------------------------------------------------------------


class _Seqs:
    """Next free `seq` per anchor, shared by both sides (ids are `<anchor>+ins.<seq>`)."""

    def __init__(self) -> None:
        self._next: dict[str, int] = defaultdict(lambda: 1)

    def take(self, anchor: str) -> int:
        n = self._next[anchor]
        self._next[anchor] = n + 1
        return n


def _cluster_changes(rec: dict, r: Item, clusters_of: Clusterizer, seqs: _Seqs) -> list[Change]:
    hs = [(c["id"], c["kind"], from_cps(c["cps"])) for c in rec["clusters"]]
    rs = clusters_of(r.text)
    out: list[Change] = []
    prev: str | None = None

    mk = functools.partial(Change, level="cluster", item_kind=r.kind, ref=r.ref, surah=r.surah)

    for h, c in pair_clusters(hs, rs):
        if h is None:
            if c is None:
                raise ValueError("empty cluster pair")
            side, anchor = ("insert_after", prev) if prev else ("insert_before", hs[0][0])
            payload = {"seq": seqs.take(anchor), "cluster": {"kind": c.kind, "cps": to_cps(c.text)}}
            out.append(mk(anchor, side, payload, kind="cluster-insert", old="", new=c.text))
            continue
        prev = h[0]
        if c is None:
            out.append(mk(h[0], "delete", None, kind="cluster-delete", old=h[2], new=""))
        elif h[2] == c.text and h[1] == c.kind:
            continue
        elif h[2][0] == c.text[0] and h[1] == c.kind:
            payload = set_marks_payload(h[2], c.text)
            out.append(mk(h[0], "set_marks", payload, kind="marks", old=h[2], new=c.text))
        else:
            payload = {"kind": c.kind, "cps": to_cps(c.text)}
            out.append(mk(h[0], "replace", payload, kind="letter", old=h[2], new=c.text))
    return out


def _new_item(r: Item, clusters_of: Clusterizer, seq: int) -> dict:
    item: dict = {"kind": r.kind}
    if r.sep_before:
        item["sep_before"] = to_cps(r.sep_before)
    item["sep_after"] = to_cps(r.sep_after)
    item["clusters"] = [{"kind": c.kind, "cps": to_cps(c.text)} for c in clusters_of(r.text)]
    return {"seq": seq, "item": item}


def _item_text(rec: dict) -> str:
    return "".join(from_cps(c["cps"]) for c in rec["clusters"])


def diff(pairs: list[Pair], hafs: dict[str, dict], clusters_of: Clusterizer) -> list[Change]:
    """Changes in reading order. `hafs` maps spine item ids to spine records (D11)."""
    seqs = _Seqs()
    out: list[Change] = []
    prev: str | None = None
    surah = None
    for n, p in enumerate(pairs):
        s = (p.h or p.r).surah  # type: ignore[union-attr]
        if s != surah:
            surah, prev = s, None
        if p.h is None:
            r = p.r
            if r is None:
                raise ValueError("empty item pair")
            if prev:
                side, anchor = "insert_after", prev
            else:
                side = "insert_before"
                anchor = next(q.h.ref for q in pairs[n:] if q.h and q.h.surah == s)
            payload = _new_item(r, clusters_of, seqs.take(anchor))
            out.append(
                Change(anchor, side, payload, "item", "item-insert", r.kind, "", r.text, r.ref, s)
            )
            continue
        rec = hafs[p.h.ref]
        prev = p.h.ref
        if p.r is None:
            out.append(
                Change(
                    p.h.ref, "delete", None, "item", "item-delete", p.h.kind, _item_text(rec), "",
                    "", s,
                )
            )  # fmt: skip
            continue
        seps = {}
        for f in ("sep_before", "sep_after"):
            if from_cps(rec.get(f, "")) != getattr(p.r, f):
                seps[f] = to_cps(getattr(p.r, f))
        if seps:
            old = from_cps(rec.get("sep_before", "")) + "|" + from_cps(rec["sep_after"])
            new = p.r.sep_before + "|" + p.r.sep_after
            out.append(
                Change(
                    p.h.ref, "replace", seps, "item", "separator", p.h.kind, old, new, p.r.ref, s
                )
            )
        out += _cluster_changes(rec, p.r, clusters_of, seqs)
    return out
