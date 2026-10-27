"""Reader names in al-Budur, resolved with the book's own terms (TB.2, D51).

The lexicon is `data/books/al-budur-al-zahirah/readers.yaml` (names and phrases only, each
cited); this module holds no Arabic. `name_key` is the one normalization used for matching
book prose: alif forms unified, ya and alif maqsura merged, harakat and tatweel removed.

A clause's tokens are scanned left to right, longest form first. Phrases (khulf, no khulf,
complement, all readers) are tried before names, so the khulf phrase "bi-khulf" is never read
as the name Khalaf. Names and phrases may carry the prefixes listed in the lexicon.

A khulf phrase attaches to the last name (`last`), the last two (`last_two`) or every name so
far (`all`), as its pronoun says; "bi-khulf 'an <name>" attaches to the name that follows.

Contextual forms follow the book's rules on page_1_0007 (p. 11):
  * `duri` (unqualified al-Duri): Abu 'Amr's Duri right after "bi-khulf 'an"; al-Kisa'i's
    Duri when an Abu 'Amr form comes earlier in the same clause; otherwise unresolved.
  * `khalaf` (unqualified Khalaf): Khalaf al-'Ashir, outside the 8 in scope either way.
  * `none`: always unresolved.
"""

import re
from dataclasses import dataclass, field
from pathlib import Path

import yaml

LEXICON = Path(__file__).resolve().parents[2] / "books" / "al-budur-al-zahirah" / "readers.yaml"
PHRASE_KINDS = ("khulf", "no_khulf", "complement", "all_readers")

_HARAKAT = re.compile("[\u064b-\u065f\u0670\u0640\u06d6-\u06ed]")
_UNIFY = str.maketrans(
    {
        "\u0623": "\u0627",
        "\u0625": "\u0627",
        "\u0622": "\u0627",
        "\u0671": "\u0627",
        "\u0649": "\u064a",
    }
)  # alif with hamza above/below, alif madda, alif wasla -> alif; alif maqsura -> ya
_SPLIT = re.compile("[\\s\u060c\u061b\u061f.,:;()\\[\\]\"'\u00ab\u00bb\\-]+")


def name_key(text: str) -> str:
    return " ".join(_HARAKAT.sub("", text).translate(_UNIFY).split())


def tokens(text: str) -> list[str]:
    return [t for t in _SPLIT.split(name_key(text)) if t]


Key = tuple[str, ...]


@dataclass(frozen=True)
class Form:
    form: str
    key: Key
    ruwat: frozenset[str]
    basis: str
    rule: str | None
    cite: str


@dataclass
class ClauseReaders:
    named: set[str] = field(default_factory=set)
    khulf: set[str] = field(default_factory=set)
    complement: bool = False
    all_readers: bool = False
    no_khulf: bool = False
    unresolved: list[str] = field(default_factory=list)
    forms: list[str] = field(default_factory=list)  # every form matched, for the report


@dataclass
class _Scan:
    toks: list[str]
    out: ClauseReaders = field(default_factory=ClauseReaders)
    hits: list[set[str]] = field(default_factory=list)  # ruwat of each resolved name


@dataclass
class Lexicon:
    ruwat: dict[str, str]  # rawi -> qari
    in_scope: list[str]
    forms: list[Form]
    phrases: dict[str, list[tuple[Key, str | None]]]  # kind -> (key, khulf attach)
    name_prefixes: tuple[str, ...]
    phrase_prefixes: tuple[str, ...]
    khulf_tail: str

    def matches(self, toks: list[str], i: int, key: Key, prefixes: tuple[str, ...]) -> bool:
        if i + len(key) > len(toks) or toks[i + 1 : i + len(key)] != list(key[1:]):
            return False
        return toks[i] == key[0] or any(toks[i] == p + key[0] for p in prefixes)

    def resolve(self, toks: list[str]) -> ClauseReaders:
        s = _Scan(toks)
        i = 0
        while i < len(toks):
            j = self._phrase_at(s, i)
            if j == i:
                j = self._name_at(s, i, after_khulf=False)
            i = j if j > i else i + 1
        return s.out

    def _phrase_at(self, s: _Scan, i: int) -> int:
        for kind in PHRASE_KINDS:
            for key, attach in self.phrases[kind]:
                if not self.matches(s.toks, i, key, self.phrase_prefixes):
                    continue
                i += len(key)
                if kind == "khulf":
                    return self._khulf(s, i, attach)
                setattr(s.out, kind, True)
                return i
        return i

    def _khulf(self, s: _Scan, i: int, attach: str | None) -> int:
        if s.toks[i : i + 1] == [self.khulf_tail]:
            before = len(s.hits)
            j = self._name_at(s, i + 1, after_khulf=True)
            for h in s.hits[before:]:
                s.out.khulf |= h
            return j
        if attach == "all":
            for h in s.hits:
                s.out.khulf |= h
        elif attach == "last_two":
            for h in s.hits[-2:]:
                s.out.khulf |= h
        elif s.hits:
            s.out.khulf |= s.hits[-1]
        return i

    def _name_at(self, s: _Scan, i: int, after_khulf: bool) -> int:
        for f in self.forms:
            if not self.matches(s.toks, i, f.key, self.name_prefixes):
                continue
            s.out.forms.append(f.form)
            got = self._contextual(f, s.hits, after_khulf)
            if got is None:
                s.out.unresolved.append(f.form)
            else:
                s.out.named |= got
                s.hits.append(got)
            return i + len(f.key)
        return i

    def _contextual(self, f: Form, hits: list[set[str]], after_khulf: bool) -> set[str] | None:
        if f.rule is None or f.rule == "khalaf":
            return set(f.ruwat)
        if f.rule == "duri":
            abu_amr = {r for r, q in self.ruwat.items() if q == "abu_amr"}
            if after_khulf:
                return {"duri_abuamr"}
            if any(h & abu_amr for h in hits):
                return {"duri_kisai"}
        return None


def _longest_first(keys: list) -> list:
    return sorted(keys, key=lambda k: (-len(k[0]), -len("".join(k[0]))))


def load(path: Path = LEXICON) -> Lexicon:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    ruwat = {r["id"]: r["qari"] for r in data["ruwat"]}
    forms = []
    for f in data["forms"]:
        rw = set(f.get("ruwat", []))
        for q in f.get("qurra", []):
            got = {r for r, qq in ruwat.items() if qq == q}
            if not got:
                raise ValueError(f"readers.yaml: unknown qari {q!r}")
            rw |= got
        if not rw <= set(ruwat):
            raise ValueError(f"readers.yaml: unknown rawi in {f['form']!r}")
        key = tuple(tokens(f["form"]))
        forms.append(Form(f["form"], key, frozenset(rw), f["basis"], f.get("rule"), f["cite"]))
    if len({f.key for f in forms}) != len(forms):
        raise ValueError("readers.yaml: two forms have the same key")
    forms = [f for _, f in _longest_first([(f.key, f) for f in forms])]
    phrases = {
        kind: _longest_first([(tuple(tokens(p["form"])), p.get("attach")) for p in data[kind]])
        for kind in PHRASE_KINDS
    }
    return Lexicon(
        ruwat,
        list(data["in_scope"]),
        forms,
        phrases,
        tuple(name_key(p) for p in data["name_prefixes"]),
        tuple(name_key(p) for p in data["phrase_prefixes"]),
        name_key(data["khulf_tail"]),
    )


def cited_pages(path: Path = LEXICON) -> list[tuple[str, str]]:
    """(form, first page id cited) for every name and phrase, for the run's preflight check."""
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    out = []
    for kind in ("forms", *PHRASE_KINDS):
        for f in data[kind]:
            if m := re.search(r"page_\d+_\d+", f["cite"]):
                out.append((f["form"], m.group(0)))
    return out
