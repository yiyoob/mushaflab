"""Blocks and clauses of al-Budur's prose (TB.3, D51). No Arabic here: terms are in
`data/books/al-budur-al-zahirah/features.yaml`.

A *block* is what the book says at one place: a run of headword quotes and the prose after
it, up to the next headword. Paragraphs without a leading quote continue the block. A
headword may also start inside a paragraph, after a full stop, or right after a waqf/wasl
term when the quote is followed by a verb, a back-reference or an "obvious" term (the EPUB
sometimes runs two entries together). Quotes soon after an example marker ("nahw", "mithl",
...) are examples, not headwords.

Sections follow the order the author gives for every rub' (page_1_0001-0002, pp. 5-6):
farsh entries, the rub' end ("<word> akhir al-rub'"), then al-mumal, al-mudgham (al-saghir,
al-kabir) and fawa'id. In al-mumal and al-mudgham the words are unquoted lists. Each segment
(split at the Arabic comma and full stops) is "<words> <verb> <readers>". A segment with no
words continues the previous item. In al-mudgham al-kabir a segment is the pair of words, and
its reader is al-Susi, by the book's own statement (page_1_0001 p. 5). A block's `rub`
counts the rub' ends that come before its rub''s first farsh entry.

A *clause* starts at a clause verb, a complement phrase or a full stop. Each clause records
its readers (readers.py), feature class, waqf/wasl, scope, exceptions, examples, awjuh and
restrictions, all from features.yaml (our interpretation; ADR D51).

A complement ("wa-l-baqun") is read two ways: as the complement of the clause just before it,
and as the complement of every reader named earlier in the block. When the two readings
differ for the 8 riwayat in scope, the complement is `ambiguous` and no partition is drawn
from it.
"""

import itertools
import re
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

import yaml

from pipeline.budur.epub import Para
from pipeline.budur.readers import ClauseReaders, Key, Lexicon, name_key, tokens

FEATURES = Path(__file__).resolve().parents[2] / "books" / "al-budur-al-zahirah" / "features.yaml"

QUOTE_RE = re.compile('"\\s*([^"]*?)\\s*"')
LEADING_QUOTE_RE = re.compile('(?:\u0648\\s*)?"\\s*([^"]*?)\\s*"\\s*\u060c?\\s*')  # [waw] "..." [,]
SEGMENT_RE = re.compile("[\u060c.]")  # Arabic comma, full stop
ARABIC_COMMA = "\u060c"
WAW = "\u0648"
SUSI_DEFAULT_CITE = "page_1_0001 p. 5"
SCOPE_ORDER = ("whole_quran_word", "surah", "class")  # listed_places is a headword count
EXAMPLE_REACH = 3  # a quote this many tokens after an example marker is an example
HEAD_WINDOW = 4  # tokens after the headwords searched for obvious/back-reference terms


@dataclass(frozen=True)
class Features:
    textual: list[tuple[str, bool]]  # (key, prefix)
    phonetic: list[tuple[str, bool]]
    verbs: set[str]
    verb_prefixes: list[str]
    scopes: dict[str, list[Key]]
    exceptions: list[Key]
    conditions: list[Key]
    examples: list[Key]
    waqf: list[Key]
    wasl: list[Key]
    both: list[Key]
    awjuh: list[Key]
    restrictions: list[Key]
    backrefs: list[Key]
    obvious: list[Key]
    sections: dict[str, list[Key]]
    ordinals: set[str]
    saghir: str
    kabir: str
    waqf_pronouns: set[str]


def _keys(words: list[str]) -> list[Key]:
    return sorted((tuple(tokens(w)) for w in words), key=lambda k: (-len(k), -len("".join(k))))


def load_features(path: Path = FEATURES) -> Features:
    d = yaml.safe_load(path.read_text(encoding="utf-8"))

    def terms(kind: str) -> list[tuple[str, bool]]:
        return [(name_key(t["term"]), bool(t["prefix"])) for t in d["features"][kind]]

    return Features(
        textual=terms("textual"),
        phonetic=terms("phonetic"),
        verbs={name_key(v) for v in d["clause_verbs"]},
        verb_prefixes=[name_key(v) for v in d["clause_verb_prefixes"]],
        scopes={k: _keys(v) for k, v in d["scopes"].items()},
        exceptions=_keys(d["exceptions"]),
        conditions=_keys(d["conditions"]),
        examples=_keys(d["examples"]),
        waqf=_keys(d["waqf"]),
        wasl=_keys(d["wasl"]),
        both=_keys(d["both"]),
        awjuh=_keys(d["awjuh"]),
        restrictions=_keys(d["restrictions"]),
        backrefs=_keys(d["backrefs"]),
        obvious=_keys(d["obvious"]),
        sections={k: _keys(v) for k, v in d["sections"].items()},
        ordinals={name_key(w) for w in d["ordinals"]},
        saghir=name_key(d["mudgham_labels"]["saghir"]),
        kabir=name_key(d["mudgham_labels"]["kabir"]),
        waqf_pronouns={name_key(w) for w in d["waqf_pronouns"]},
    )


def find(toks: list[str], keys: list[Key], prefix_waw: bool = True) -> int | None:
    """Index of the first token where one of `keys` starts (optionally after a waw prefix)."""
    for i in range(len(toks)):
        if _at(toks, i, keys, prefix_waw):
            return i
    return None


def _at(toks: list[str], i: int, keys: list[Key], prefix_waw: bool = True) -> bool:
    return any(
        i + len(k) <= len(toks)
        and toks[i + 1 : i + len(k)] == list(k[1:])
        and (toks[i] == k[0] or (prefix_waw and toks[i] == WAW + k[0]))
        for k in keys
    )


def _last(toks: list[str], keys: list[Key]) -> int | None:
    hits = [i for i in range(len(toks)) if _at(toks, i, keys)]
    return hits[-1] if hits else None


@dataclass
class Clause:
    text: str  # keyed tokens joined (book text: build/ only)
    readers: ClauseReaders
    feature: str | None  # textual | phonetic | None
    terms: list[str]
    wq: str | None  # waqf | wasl | both | None
    scope: str | None  # whole_quran_word | surah | class | None
    exception: bool
    exception_quotes: list[str]
    examples: list[str]
    awjuh: bool
    restriction: bool
    conditional: bool = False
    complement_in_scope: set[str] | None = None  # complement clauses, when unambiguous
    complement_ambiguous: bool = False
    default_reader_cite: str | None = None  # readers given by a section default


@dataclass
class Block:
    id: str
    anchors: list[str]
    cite: str  # first paragraph's page id and printed page
    surah: int | None
    zone: str
    section: str  # farsh rub_end mumal mudgham_saghir mudgham_kabir fawaid general corrections
    rub: int
    headwords: list[str]
    kind: str = "statement"  # statement | backref | obvious | rub_end | label | none
    count: int | None = None  # 2 for "ma'an" / "fi l-mawdi'ayn"
    clauses: list[Clause] = field(default_factory=list)
    text: str = ""  # the block's book text (build/ only)


def _terms(toks: list[str], feats: Features) -> tuple[str | None, list[str]]:
    found: dict[str, list[str]] = {"textual": [], "phonetic": []}
    for t in toks:
        bare = t[1:] if t.startswith(WAW) and len(t) > 1 else t
        for kind in ("textual", "phonetic"):
            hit = any(
                key in (t, bare) or (prefix and (t.startswith(key) or bare.startswith(key)))
                for key, prefix in getattr(feats, kind)
            )
            if hit:
                found[kind].append(t)
                break
    if found["textual"]:
        return "textual", found["textual"] + found["phonetic"]
    if found["phonetic"]:
        return "phonetic", found["phonetic"]
    return None, []


def _wq(toks: list[str], feats: Features) -> str | None:
    waqf, wasl = find(toks, feats.waqf) is not None, find(toks, feats.wasl) is not None
    if find(toks, feats.both) is not None or (waqf and wasl):
        return "both"
    return "waqf" if waqf else "wasl" if wasl else None


def make_clause(text: str, lex: Lexicon, feats: Features) -> Clause:
    toks = tokens(QUOTE_RE.sub(" ", text))
    feature, terms = _terms(toks, feats)
    exc_quotes, examples = [], []
    for m in QUOTE_RE.finditer(text):
        before = tokens(QUOTE_RE.sub(" ", text[: m.start()]))
        last_exc, last_ex = _last(before, feats.exceptions), _last(before, feats.examples)
        if last_exc is not None and (last_ex is None or last_exc > last_ex):
            exc_quotes.append(m.group(1))
        elif last_ex is not None:
            examples.append(m.group(1))
    scope = next((s for s in SCOPE_ORDER if find(toks, feats.scopes[s]) is not None), None)
    return Clause(
        text=" ".join(toks),
        readers=lex.resolve(toks),
        feature=feature,
        terms=terms,
        wq=_wq(toks, feats),
        scope=scope,
        exception=find(toks, feats.exceptions) is not None,
        exception_quotes=exc_quotes,
        examples=examples,
        awjuh=find(toks, feats.awjuh) is not None,
        restriction=find(toks, feats.restrictions) is not None,
        conditional=find(toks, feats.conditions) is not None,
    )


def _starts_clause(word: str, lex: Lexicon, feats: Features) -> bool:
    k = name_key(word).strip("\u060c:;()")
    if k in feats.verbs or any(k.startswith(p) for p in feats.verb_prefixes):
        return True
    return any(k in (c[0], WAW + c[0]) for c, _ in lex.phrases["complement"])


def split_clauses(text: str, lex: Lexicon, feats: Features) -> list[Clause]:
    """Split at full stops, clause verbs and complement phrases (never inside quotes).

    A complement right after a verb ("wa-qara'a l-baqun") stays in the verb's clause.
    """
    pieces: list[str] = []
    cur: list[str] = []
    for part in re.split('("[^"]*")', text):
        if part.startswith('"'):
            cur.append(part)
            continue
        for word in re.split(r"(\s+|\.)", part):
            if word == ".":
                pieces.append("".join([*cur, word]))
                cur = []
                continue
            words = [w for w in cur if w.strip()]
            after_verb = bool(words) and name_key(words[-1]) in feats.verbs
            if _starts_clause(word, lex, feats) and words and not after_verb:
                pieces.append("".join(cur))
                cur = []
            cur.append(word)
    pieces.append("".join(cur))
    return [make_clause(p, lex, feats) for p in pieces if tokens(QUOTE_RE.sub(" ", p))]


def resolve_complements(clauses: list[Clause], in_scope: set[str]) -> None:
    named_all: set[str] = set()
    prev: set[str] = set()
    for c in clauses:
        if c.readers.complement:
            a, b = in_scope - prev, in_scope - named_all
            c.complement_in_scope = a if a == b else None
            c.complement_ambiguous = a != b
        named_all |= c.readers.named
        if c.readers.named:
            prev = c.readers.named


def leading_quotes(text: str) -> tuple[list[str], str]:
    """Headword quotes at the start of `text` (joined by waw, comma or space) and the rest."""
    heads = []
    rest = text.lstrip()
    while m := LEADING_QUOTE_RE.match(rest):
        # One quote may list several words, separated by commas.
        heads += [h.strip() for h in m.group(1).split(ARABIC_COMMA) if h.strip()]
        rest = rest[m.end() :]
    return heads, rest


def split_inline(text: str, feats: Features) -> list[str]:
    """Cut a paragraph before each quote that starts a new entry inside it."""
    cuts = [0]
    for m in QUOTE_RE.finditer(text):
        if m.start() == 0:
            continue
        before = text[: m.start()].rstrip()
        btoks = tokens(QUOTE_RE.sub(" ", before))
        last_ex = _last(btoks, feats.examples)
        if not btoks or (last_ex is not None and len(btoks) - last_ex <= EXAMPLE_REACH):
            continue
        after = tokens(QUOTE_RE.sub(" ", text[m.end() :]))[:3]
        after_wq = find(btoks[-2:], feats.waqf + feats.wasl + feats.both) is not None
        starts_entry = bool(after) and (
            after[0] in feats.verbs
            or find(after, feats.backrefs) == 0
            or find(after, feats.obvious) == 0
        )
        if before.endswith(".") or (after_wq and starts_entry):
            cuts.append(m.start())
    cuts.append(len(text))
    return [text[a:b].strip() for a, b in itertools.pairwise(cuts) if text[a:b].strip()]


def section_marker(text: str, feats: Features) -> str | None:
    toks = tokens(QUOTE_RE.sub(lambda m: f" {m.group(1)} ", text))
    for name in ("mumal", "mudgham", "fawaid"):
        if any(toks == list(k) for k in feats.sections[name]):
            return name
    return None


def _qualifiers(feats: Features) -> set[str]:
    words = set(feats.waqf_pronouns)
    for keys in (feats.scopes["listed_places"], feats.waqf, feats.wasl, feats.both):
        for k in keys:
            words |= set(k)
    return words


def list_items(
    text: str, section: str, lex: Lexicon, feats: Features
) -> list[tuple[list[str], str]]:
    """Segments of a mumal/mudgham list: (headwords, statement). No headwords: continuation."""
    out = []
    quals = _qualifiers(feats)
    for seg in SEGMENT_RE.split(QUOTE_RE.sub(lambda m: f" {m.group(1)} ", text)):
        raw = seg.split()
        if not raw:
            continue
        keyed = [name_key(w) for w in raw]
        if section == "mudgham_kabir" and not lex.resolve(keyed).named:
            out.append(
                ([" ".join(w for w, k in zip(raw, keyed, strict=True) if k not in quals)], "")
            )
            continue
        cut = next(
            (
                i
                for i, k in enumerate(keyed)
                if k in feats.verbs
                or any(k.startswith(p) for p in feats.verb_prefixes)
                or lex.resolve([k]).named
                or lex.resolve([k]).all_readers
            ),
            len(raw),
        )
        heads = [w for w, k in zip(raw[:cut], keyed[:cut], strict=True) if k not in quals]
        out.append((heads, " ".join(raw[cut:])))
    return out


NewBlock = Callable[[Para, str, list[str], str], Block]


@dataclass
class _State:
    lex: Lexicon
    feats: Features
    out: list[Block] = field(default_factory=list)
    section: str = "general"
    rub: int = 0
    after_rub_end: bool = False
    n: int = 0

    def new(self, p: Para, section: str, heads: list[str], body: str) -> Block:
        self.n += 1
        b = Block(f"b{self.n:05d}", [p.anchor], p.cite, p.surah, p.zone, section, self.rub, heads)
        b.text = body
        self.out.append(b)
        return b

    def extend(self, b: Block, p: Para, text: str) -> None:
        if p.anchor not in b.anchors:
            b.anchors.append(p.anchor)
        b.text += " " + text
        b.clauses += split_clauses(text, self.lex, self.feats)
        resolve_complements(b.clauses, set(self.lex.in_scope))

    def list_item(self, p: Para, words: list[str], stmt: str) -> None:
        sect = self.section
        if not words:
            last = self.out[-1] if self.out else None
            if last is not None and last.section == sect and stmt:
                last.clauses += split_clauses(stmt, self.lex, self.feats)
                last.text += f" {ARABIC_COMMA} {stmt}"
            return
        heads = [" ".join(words)] if sect == "mudgham_kabir" else words
        b = self.new(p, sect, heads, stmt)
        b.clauses = split_clauses(stmt, self.lex, self.feats) if stmt else []
        if sect == "mudgham_kabir" and not b.clauses:
            c = make_clause("", self.lex, self.feats)
            c.readers.named = {"susi"}
            c.feature, c.terms = "phonetic", ["idgham-kabir"]
            c.default_reader_cite = SUSI_DEFAULT_CITE
            b.clauses = [c]

    def farsh(self, p: Para) -> None:
        if self.after_rub_end:
            self.rub += 1
            self.after_rub_end = False
        self.section = "farsh"
        for piece in split_inline(p.text, self.feats):
            heads, body = leading_quotes(piece)
            if not heads:
                if self.out and self.out[-1].section == "farsh":
                    self.extend(self.out[-1], p, piece)
                continue
            self.entry(p, heads, body, piece)

    def entry(self, p: Para, heads: list[str], body: str, piece: str) -> None:
        feats = self.feats
        btoks = tokens(QUOTE_RE.sub(" ", body))
        b = self.new(p, "farsh", heads, piece)
        if find(btoks, feats.sections["rub_end"]) is not None:
            b.section = b.kind = "rub_end"
            self.after_rub_end = True
            return
        if find(btoks[:3], feats.scopes["listed_places"], prefix_waw=False) == 0:
            b.count = 2
        b.clauses = split_clauses(body, self.lex, feats)
        resolve_complements(b.clauses, set(self.lex.in_scope))
        named = any(
            c.readers.named or c.readers.complement or c.readers.all_readers for c in b.clauses
        )
        if not named and find(btoks[:HEAD_WINDOW], feats.obvious) is not None:
            b.kind = "obvious"
        elif not named and find(btoks[:HEAD_WINDOW], feats.backrefs) is not None:
            b.kind = "backref"

    def listed(self, p: Para, heads: list[str], rest: str) -> bool:
        """Mudgham/mumal list paragraphs and fawa'id labels; False for anything else."""
        feats = self.feats
        hk = name_key(heads[0]) if heads else None
        if self.section.startswith("mudgham") and hk in (feats.saghir, feats.kabir):
            self.section = "mudgham_saghir" if hk == feats.saghir else "mudgham_kabir"
            for words, stmt in list_items(rest, self.section, self.lex, feats):
                self.list_item(p, words, stmt)
            return True
        if hk in feats.ordinals:
            self.section = "fawaid"
            self.new(p, "fawaid", heads, p.text).kind = "label"
            return True
        if self.section in ("mumal", "mudgham_saghir", "mudgham_kabir") and not heads:
            for words, stmt in list_items(p.text, self.section, self.lex, feats):
                self.list_item(p, words, stmt)
            return True
        return False

    def para(self, p: Para) -> None:
        feats = self.feats
        if p.zone in ("corrections", "general"):
            b = self.new(p, p.zone, [], p.text)
            if p.zone == "general":
                b.clauses = split_clauses(p.text, self.lex, feats)
            b.kind = "statement" if b.clauses else "none"
            return
        marker = section_marker(p.text, feats)
        if marker is not None:
            self.section = {"mudgham": "mudgham_saghir"}.get(marker, marker)
            if marker == "fawaid":
                self.new(p, "fawaid", [], p.text).kind = "label"
            return
        heads, rest = leading_quotes(p.text)
        if self.listed(p, heads, rest):
            return
        if not heads:
            last = self.out[-1] if self.out else None
            if last is not None and last.section in ("farsh", "rub_end"):
                self.extend(last, p, p.text)
            elif last is not None and last.section == "fawaid":
                last.anchors.append(p.anchor)
                last.text += " " + p.text
            else:
                self.new(p, "farsh", [], p.text).kind = "none"
            return
        self.farsh(p)


def blocks(paras: list[Para], lex: Lexicon, feats: Features) -> list[Block]:
    st = _State(lex, feats)
    for p in paras:
        if not p.is_heading:
            st.para(p)
    return st.out
