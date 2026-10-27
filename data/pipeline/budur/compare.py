"""Statuses and the two-way comparison (TB.5, D52).

Status per word x riwayah, exactly one:
  * `mentioned`: a placed book entry at this word covers the riwayah (named, by a group
    term, by a complement, or by an all-readers phrase), or the entry defers to an earlier
    one (back-reference) or calls the place obvious (readers unstated, noted);
  * `rule-covered`: a mechanically applied rule (index.py) covers the word and the riwayah;
  * `not-mentioned`: neither. Never "confirmed".
An entry that covers none of the 8 (e.g. only Hamzah's waqf) leaves them `not-mentioned`,
with a note. An `ambiguous` placement leaves its words `not-mentioned` and is flagged.

Book -> ours, for each placed entry (and each rule application) with written-text clauses:
every in-scope `textual` clause that is not waqf-only and not conditional splits the 8 into
its readers and the rest (a khulf reader may sit on either side). Two riwayat the book
separates must have different `loose` forms; two it never separates must have the same.
Our forms are compared over all the entry's words. A pair the data separates and the book
does not is explained, not flagged, when one of the two is named in a phonetic clause of the
entry (KFGQPC writes that delivery; recorded per term) or when it is a Maghribi-family
pattern (warsh and qalun alike, against the six others alike) that the book does not draw
(a convention candidate). Remaining disagreements are attributed by a minimum vertex cover
of the disagreeing pairs: a riwayah in every minimum cover is `flagged`; one in some
minimum covers only is `possible`.

Ours -> book: on a `not-mentioned` word every loose difference between riwayat is a
disagreement with the book's silence, attributed the same way (`patch-without-statement`).
"""

import itertools
from dataclasses import dataclass, field

from pipeline.budur.statements import Block, Clause

MAGHRIBI = frozenset({"warsh", "qalun"})


@dataclass(frozen=True)
class Split:
    group: frozenset[str]  # in-scope readers of the clause
    khulf: frozenset[str]
    source: str  # "<block id>/c<clause>" or "<rule id>"


@dataclass
class Unit:
    """One place the book speaks about (an entry's words, or one rule application)."""

    id: str
    kind: str  # entry | rule | silence
    words: list[int]  # spine word positions
    covered: set[str]  # in-scope riwayat the statement covers
    splits: list[Split] = field(default_factory=list)
    explicit: set[str] = field(default_factory=set)  # covered by name, group or phrase
    phonetic_named: set[str] = field(default_factory=set)
    phonetic_terms: list[str] = field(default_factory=list)
    cite: str = ""
    note: str = ""


@dataclass
class Finding:
    unit: str
    kind: str  # missing-difference | unstated-difference | patch-without-statement
    pair: tuple[str, str]
    words: list[int]


@dataclass
class Verdict:
    unit: str
    words: list[int]
    findings: list[Finding]
    explained: list[tuple[tuple[str, str], str]]  # (pair, phonetic | convention-family)
    flagged: set[str]  # in every minimum cover
    possible: set[str]  # in some minimum cover only
    undecidable: str | None = None


def clause_split(
    c: Clause, in_scope: set[str], source: str, complement: set[str] | None = None
) -> Split | None:
    """The split a written-text clause draws among the 8, or None.

    `complement` is the complement's group under the reading chosen by the caller (for an
    ambiguous complement); otherwise the clause's own unambiguous group is used.
    """
    if c.feature != "textual" or c.wq == "waqf" or c.conditional:
        return None
    if c.readers.complement:
        group = complement if complement is not None else c.complement_in_scope
        if group is None:
            return None
    elif c.readers.all_readers:
        return None
    else:
        group = c.readers.named & in_scope
    return Split(frozenset(group), frozenset(c.readers.khulf & in_scope), source)


def entry_unit(b: Block, words: list[int], in_scope: set[str], reading: str = "prev") -> Unit:
    """Unit for a placed entry. An ambiguous complement is read as `reading`: the
    complement of the clause just before it ("prev") or of every reader named before it
    ("all"); the caller compares both and decides only when they agree.

    Coverage: the readers a clause names, or a complement or all-readers phrase covers;
    when any of the 8 is covered, the others are covered implicitly, since the book states
    each word's khilaf among the ten where it discusses the word (page_1_0001 p. 5).
    """
    explicit: set[str] = set()
    u = Unit(b.id, "entry", words, set(), cite=b.cite)
    prev: set[str] = set()
    named: set[str] = set()
    for i, c in enumerate(b.clauses):
        r = c.readers
        comp = None
        if r.complement:
            comp = (in_scope - prev) if reading == "prev" else (in_scope - named)
            explicit |= comp
        if r.all_readers:
            explicit |= in_scope
        explicit |= r.named & in_scope
        if c.feature == "phonetic":
            u.phonetic_named |= (r.named & in_scope) | (comp or set())
            u.phonetic_terms += c.terms
        s = clause_split(c, in_scope, f"{b.id}/c{i}", comp if c.complement_ambiguous else None)
        if s is not None:
            u.splits.append(s)
        named |= r.named
        if r.named:
            prev = r.named
    u.explicit = explicit
    if explicit:
        u.covered = set(in_scope)
    elif b.kind in ("backref", "obvious"):
        u.covered, u.note = set(in_scope), b.kind
    return u


def ambiguous_complement(b: Block) -> bool:
    return any(c.complement_ambiguous and c.feature == "textual" for c in b.clauses)


def book_separates(splits: list[Split], a: str, b: str) -> bool | None:
    """True: some split puts a and b apart for sure; False: the book never parts them;
    None: only a khulf reader's two wajhs decide (either is fine)."""
    maybe = False
    for s in splits:
        khulf = a in s.khulf or b in s.khulf
        if (a in s.group) != (b in s.group):
            if not khulf:
                return True
            maybe = True
        elif khulf:
            maybe = True
    return None if maybe else False


def min_covers(edges: list[tuple[str, str]], nodes: list[str]) -> list[frozenset[str]]:
    if not edges:
        return [frozenset()]
    for k in range(1, len(nodes) + 1):
        found = [
            frozenset(c)
            for c in itertools.combinations(nodes, k)
            if all(x in c or y in c for x, y in edges)
        ]
        if found:
            return found
    return [frozenset(nodes)]


def judge(
    u: Unit,
    forms: dict[str, str | None],
    diffs: dict[str, str],
    riwayat: list[str],
    letters_changed: frozenset[str] = frozenset(),
) -> Verdict:
    """`forms`: loose form of the unit per riwayah; `diffs`: loose diff pattern vs hafs;
    `letters_changed`: riwayat whose difference from hafs changes a base letter.

    For an `entry` or `rule` unit only pairs the statement covers are compared; for a
    `silence` unit (a not-mentioned word) every pair is, against "no difference".
    """
    findings: list[Finding] = []
    explained: list[tuple[tuple[str, str], str]] = []
    silence = u.kind == "silence"
    for a, b in itertools.combinations(riwayat, 2):
        if not silence and not (a in u.covered and b in u.covered):
            continue
        apart = False if silence else book_separates(u.splits, a, b)
        if apart is None:
            continue  # only through a khulf reader: either is fine
        differ = forms[a] != forms[b]
        if apart and not differ:
            findings.append(Finding(u.id, "missing-difference", (a, b), u.words))
        elif not apart and differ:
            if why := _explain(u, (a, b), diffs, letters_changed):
                explained.append(((a, b), why))
            else:
                kind = "patch-without-statement" if silence else "unstated-difference"
                findings.append(Finding(u.id, kind, (a, b), u.words))
    covers = min_covers([f.pair for f in findings], riwayat)
    always = frozenset.intersection(*covers)
    some = frozenset.union(*covers) - always
    return Verdict(u.id, u.words, findings, explained, set(always), set(some))


def _explain(
    u: Unit, pair: tuple[str, str], diffs: dict[str, str], letters_changed: frozenset[str]
) -> str | None:
    """Why a data difference the book does not draw is not a flag, if it is not."""
    a, b = pair
    if a in u.phonetic_named or b in u.phonetic_named:
        return "phonetic"
    if len(set(pair) & MAGHRIBI) == 1:
        drawn = any(MAGHRIBI & s.group for s in u.splits)
        family = diffs.get("warsh", "") == diffs.get("qalun", "") != ""
        if family and not drawn and not (MAGHRIBI & letters_changed):
            return "convention-family"
    return None
