"""T4.4: the differ turns an alignment into patch ops (D12 semantics, D20).

Synthetic cases use Latin letters with combining marks (U+0300..U+036F), clusterized by a
test clusterizer; they test the differ's logic only and are not Quranic text.

`apply` below is a test oracle that mirrors the resolver's documented op semantics (D12) for
the ops the differ emits. The authoritative check is the Rust round-trip (T4.6).
"""

import random

import pytest

from pipeline import align, differ, ingest_riwayah, spine
from pipeline.align import align_surah
from pipeline.clusterize import Cluster, clusterize
from pipeline.codepoints import load_table
from pipeline.differ import Change, diff, set_marks_payload, simulate_set_marks
from pipeline.ingest_riwayah import Item
from pipeline.spine import from_cps, to_cps
from pipeline.tests.markers import needs_sources

MARKS = "\u0300\u0301\u0302\u0303\u0308"


def latin_clusters(text: str) -> list[Cluster]:
    out: list[Cluster] = []
    for c in text:
        if c in MARKS:
            out[-1] = Cluster(out[-1].text + c, out[-1].kind)
        elif c == "@":
            out.append(Cluster(c, "ayah_number"))
        else:
            out.append(Cluster(c, "letter"))
    return out


def items(spec: list[str], surah: int = 1) -> list[Item]:
    """Words separated by single spaces; '@' ends an ayah."""
    out, ayah, pos = [], 1, 0
    for i, tok in enumerate(spec):
        last = i == len(spec) - 1
        if tok == "@":
            out.append(Item(surah, ayah, pos + 1, "ayah_end", tok, ""))
            ayah, pos = ayah + 1, 0
        else:
            pos += 1
            out.append(Item(surah, ayah, pos, "word", tok, "" if last else " "))
    return out


def records(hs: list[Item]) -> dict[str, dict]:
    return {
        h.ref: {
            "id": h.ref,
            "kind": h.kind,
            **({"sep_before": to_cps(h.sep_before)} if h.sep_before else {}),
            "sep_after": to_cps(h.sep_after),
            "clusters": [
                {"id": f"{h.ref}:{n}", "kind": c.kind, "cps": to_cps(c.text)}
                for n, c in enumerate(latin_clusters(h.text), start=1)
            ],
        }
        for h in hs
    }


def strip(item: Item) -> str:
    return "".join(c for c in item.text if c not in MARKS)


def key(item: Item) -> str:
    return strip(item) if item.kind == "word" else item.kind


# --- test oracle: the resolver's semantics for the emitted ops ----------------------------


def apply(recs: dict[str, dict], changes: list[Change]) -> str:
    """A tree with tombstones, like the engine: inserts first, then modifications."""
    nodes: dict[str, dict] = {}

    def node(text: str, **extra: object) -> dict:
        return {"text": text, "deleted": False, "before": [], "after": [], **extra}

    for r in recs.values():
        roots = [c["id"] for c in r["clusters"]]
        for c in r["clusters"]:
            nodes[c["id"]] = node(from_cps(c["cps"]))
        nodes[r["id"]] = node(
            "",
            roots=roots,
            sep_before=from_cps(r.get("sep_before", "")),
            sep_after=from_cps(r["sep_after"]),
        )
    for ch in changes:
        if not ch.op.startswith("insert"):
            continue
        p = ch.payload
        new_id = f"{ch.target}+ins.{p['seq']}"
        if "item" in p:
            it = p["item"]
            roots = []
            for k, c in enumerate(it["clusters"], start=1):
                nodes[f"{new_id}:{k}"] = node(from_cps(c["cps"]))
                roots.append(f"{new_id}:{k}")
            nodes[new_id] = node(
                "",
                roots=roots,
                sep_before=from_cps(it.get("sep_before", "")),
                sep_after=from_cps(it["sep_after"]),
            )
        else:
            nodes[new_id] = node(from_cps(p["cluster"]["cps"]))
        side = "before" if ch.op == "insert_before" else "after"
        nodes[ch.target][side].append((p["seq"], new_id))
    _modify(nodes, changes)
    return "".join(_walk(nodes, i, True) for i in recs)


def _modify(nodes: dict[str, dict], changes: list[Change]) -> None:
    for ch in changes:
        n = nodes[ch.target]
        if ch.op == "delete":
            n["deleted"] = True
        elif ch.op == "replace" and "cps" in ch.payload:
            n["text"] = from_cps(ch.payload["cps"])
        elif ch.op == "replace":
            for f in ("sep_before", "sep_after"):
                if f in ch.payload:
                    n[f] = from_cps(ch.payload[f])
        elif ch.op == "set_marks":
            n["text"] = simulate_set_marks(n["text"], ch.payload)


def _walk(nodes: dict[str, dict], node_id: str, is_item: bool) -> str:
    n = nodes[node_id]
    out = "".join(_walk(nodes, i, is_item) for _, i in sorted(n["before"]))
    if not n["deleted"]:
        if is_item:
            body = "".join(_walk(nodes, c, False) for c in n["roots"])
            out += n["sep_before"] + body + n["sep_after"]
        else:
            out += n["text"]
    return out + "".join(_walk(nodes, i, is_item) for _, i in sorted(n["after"]))


def run(h_spec: list[str], r_spec: list[str]) -> tuple[list[Change], str, str]:
    hs, rs = items(h_spec), items(r_spec)
    pairs = align_surah(hs, rs, key)
    changes = diff(pairs, records(hs), latin_clusters)
    want = "".join(i.sep_before + i.text + i.sep_after for i in rs)
    return changes, apply(records(hs), changes), want


# --- set_marks ----------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ("a\u0301", "a\u0300"),
        ("a\u0301\u0302", "a\u0302\u0301"),
        ("a\u0301\u0301", "a\u0301"),
        ("a\u0301\u0302\u0301", "a\u0302\u0301"),
        ("a\u0301\u0302\u0301", "a\u0301\u0302"),
        ("a", "a\u0300\u0301"),
        ("a\u0300\u0301", "a"),
        ("a\u0302", "a\u0300\u0302\u0303"),
    ],
)
def test_set_marks_payload_reproduces_the_target(old: str, new: str) -> None:
    payload = set_marks_payload(old, new)
    assert payload
    assert simulate_set_marks(old, payload) == new


def test_simulate_follows_the_resolver_rules() -> None:
    # remove takes the first occurrence; `at` is a mark index (0 = after the base)
    assert simulate_set_marks("a\u0301\u0302\u0301", {"remove": ["U+0301"]}) == "a\u0302\u0301"
    assert simulate_set_marks("a\u0301", {"add": [{"cp": "U+0300", "at": 0}]}) == "a\u0300\u0301"
    assert simulate_set_marks("a\u0301", {"add": ["U+0300"]}) == "a\u0301\u0300"


# --- diff ---------------------------------------------------------------------------------


def test_identical_texts_give_no_changes() -> None:
    changes, got, want = run(["ab", "c", "@"], ["ab", "c", "@"])
    assert changes == []
    assert got == want


def test_mark_change_is_one_set_marks_per_cluster() -> None:
    changes, got, want = run(["a\u0301b", "@"], ["a\u0300b\u0302", "@"])
    assert [(c.op, c.target) for c in changes] == [
        ("set_marks", "1:1:1:1"),
        ("set_marks", "1:1:1:2"),
    ]
    assert got == want


def test_letter_change_is_a_cluster_replace() -> None:
    changes, got, want = run(["abc", "@"], ["axc", "@"])
    assert [(c.op, c.target, c.payload) for c in changes] == [
        ("replace", "1:1:1:2", {"kind": "letter", "cps": "U+0078"})
    ]
    assert got == want


def test_extra_and_missing_letters() -> None:
    changes, got, want = run(["abc", "@"], ["xabdc", "@"])
    assert got == want
    assert {c.op for c in changes} <= {"insert_before", "insert_after", "replace", "delete"}
    changes, got, want = run(["abcd", "@"], ["ad", "@"])
    assert [(c.op, c.target) for c in changes] == [("delete", "1:1:1:2"), ("delete", "1:1:1:3")]
    assert got == want


def test_word_insert_and_delete_and_seq() -> None:
    changes, got, want = run(["ab", "ef", "gh", "@"], ["ab", "xx", "yy", "ef", "@"])
    assert got == want
    assert [(c.op, c.target, (c.payload or {}).get("seq")) for c in changes] == [
        ("insert_after", "1:1:1", 1),
        ("insert_after", "1:1:1", 2),
        ("delete", "1:1:3", None),
    ]


def test_insert_at_surah_start_goes_before_the_first_item() -> None:
    changes, got, want = run(["ab", "@"], ["xx", "ab", "@"])
    assert [(c.op, c.target) for c in changes] == [("insert_before", "1:1:1")]
    assert got == want


def test_separator_change_is_an_item_replace() -> None:
    hs = items(["ab", "cd", "@"])
    rs = [Item(1, 1, 1, "word", "ab", "  "), *items(["ab", "cd", "@"])[1:]]
    changes = diff(align_surah(hs, rs, key), records(hs), latin_clusters)
    assert [(c.op, c.target, c.payload) for c in changes] == [
        ("replace", "1:1:1", {"sep_after": "U+0020 U+0020"})
    ]


def test_ayah_end_moves_are_item_delete_and_insert() -> None:
    changes, got, want = run(["ab", "@", "cd", "ef", "@"], ["ab", "cd", "@", "ef", "@"])
    assert got == want
    assert {c.kind for c in changes} >= {"item-delete", "item-insert"}


def test_change_keys_are_unique() -> None:
    changes, _, _ = run(["ab", "cd", "ef", "@"], ["xb", "q", "cd", "ef", "r", "s", "@"])
    keys = [c.key for c in changes]
    assert len(keys) == len(set(keys))


def test_random_texts_round_trip_through_the_oracle() -> None:
    rng = random.Random(20261022)  # noqa: S311 - fixed seed, reproducible test data
    letters = "abcdefg"

    def word() -> str:
        return "".join(
            rng.choice(letters) + "".join(rng.sample(MARKS, rng.randint(0, 2)))
            for _ in range(rng.randint(1, 4))
        )

    def text() -> list[str]:
        out = []
        for _ in range(rng.randint(1, 4)):
            out += [word() for _ in range(rng.randint(1, 5))] + ["@"]
        return out

    for _ in range(300):
        h = text()
        r = [w if rng.random() < 0.6 else word() for w in h if w == "@" or rng.random() < 0.9]
        if r[-1] != "@":
            r.append("@")
        changes, got, want = run(h, r)
        assert got == want, (h, r)
        assert len({c.key for c in changes}) == len(changes)


def test_differ_never_touches_clusters_of_a_deleted_item() -> None:
    changes, _, _ = run(["ab", "cd", "@"], ["ab", "@"])
    deleted = {c.target for c in changes if c.op == "delete" and c.target.count(":") == 2}
    assert all(c.target.rsplit(":", 1)[0] not in deleted for c in changes if c.level == "cluster")


def test_patch_id_from_key() -> None:
    assert differ.patch_id("warsh", "3:133:1:1", "delete") == "warsh.3.133.1.1.del"
    assert differ.patch_id("qalun", "1:1:4+ins.2", "insert_after") == "qalun.1.1.4+ins.2.ins"


@needs_sources
def test_shuba_is_reproduced_by_the_oracle() -> None:
    table = load_table()
    recs = {r["id"]: r for r in spine.load_records()}
    text = ingest_riwayah.load("shuba")
    pairs = align.align_text(ingest_riwayah.load("hafs").items, text.items)
    changes = diff(pairs, recs, lambda t: clusterize(t, table))
    assert apply(recs, changes) == "".join(text.ayah_texts().values())
    assert len({c.key for c in changes}) == len(changes)
