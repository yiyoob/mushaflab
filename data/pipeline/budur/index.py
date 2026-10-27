"""The book's rule index (TB.3, D52), built from every block of the book.

A *rule* is a clause that carries a scope phrase (features.yaml `scopes`), or any clause of
the general chapters (introduction, isti'adhah, basmalah). Each rule records where it is
stated (block, EPUB page id and printed page), the first place it is attached to (the
block's headwords), its readers, feature, waqf/wasl, khulf and exceptions.

Scopes and what is applied mechanically (TB.5):
  * `whole_quran_word`: the block's headwords wherever they occur (letter match, match.py);
  * `surah`: the block's headwords within the block's surah;
  * `class` and `general`: a condition in prose. Never applied mechanically; listed as
    `scope: unclear` for the owner to map (to a Phase 6 rule, or by hand).
An exception marker without a quoted word cannot be applied: it is kept and reported.

Corrections: the appended "tasweebat" (page_2_*, by Abu Khalid al-Sulami, which
page_2_0354 says are not part of the book) are numbered items "N - <page> ...". Each is
linked to the printed page it names and shown next to that page's passages; it never
changes a status. The "mudakhalat" items (questions and answers) carry no page and are kept
as notes.
"""

import re
from dataclasses import dataclass, field

from pipeline.budur.statements import Block, Clause

ARABIC_DIGITS = str.maketrans(
    "\u0660\u0661\u0662\u0663\u0664\u0665\u0666\u0667\u0668\u0669", "0123456789"
)
ITEM_RE = re.compile("^\\s*([0-9]+)\\s*-\\s*(?:\u0635\\s*([0-9]+))?")  # "N - [sad] P"
APPLIED = ("whole_quran_word", "surah")


@dataclass
class Rule:
    id: str
    block: str
    cite: str
    anchor: str
    surah: int | None
    section: str
    headwords: list[str]
    scope: str  # whole_quran_word | surah | class | general
    clause: int  # index of the clause in its block
    readers: set[str]  # in-scope ruwat named
    khulf: set[str]
    all_readers: bool
    complement: bool
    complement_in_scope: set[str] | None
    feature: str | None
    wq: str | None
    exceptions: list[str]
    exception_unparsed: bool
    unresolved: list[str]
    text: str = ""  # book text: build/ only

    @property
    def applied(self) -> bool:
        return self.scope in APPLIED and bool(self.headwords)


@dataclass
class Correction:
    id: str
    cite: str
    anchor: str
    number: int | None
    page: int | None  # printed page it corrects; None for the mudakhalat
    text: str = ""  # book text: build/ only
    blocks: list[str] = field(default_factory=list)


def rules(blocks: list[Block], in_scope: set[str]) -> list[Rule]:
    out = []
    for b in blocks:
        for i, c in enumerate(b.clauses):
            scope = c.scope or ("general" if b.zone == "general" else None)
            if scope is None:
                continue
            out.append(_rule(f"R{len(out) + 1:04d}", b, (i, c), scope, in_scope))
    return out


def _rule(rid: str, b: Block, at: tuple[int, Clause], scope: str, in_scope: set[str]) -> Rule:
    i, c = at
    return Rule(
        id=rid,
        block=b.id,
        cite=b.cite,
        anchor=b.anchors[0],
        surah=b.surah,
        section=b.section,
        headwords=list(b.headwords),
        scope=scope,
        clause=i,
        readers=c.readers.named & in_scope,
        khulf=c.readers.khulf & in_scope,
        all_readers=c.readers.all_readers,
        complement=c.readers.complement,
        complement_in_scope=c.complement_in_scope,
        feature=c.feature,
        wq=c.wq,
        exceptions=list(c.exception_quotes),
        exception_unparsed=c.exception and not c.exception_quotes,
        unresolved=list(c.readers.unresolved),
        text=c.text,
    )


def corrections(blocks: list[Block]) -> list[Correction]:
    out: list[Correction] = []
    for b in blocks:
        if b.section != "corrections":
            continue
        m = ITEM_RE.match(b.text.translate(ARABIC_DIGITS))
        if m:
            page = int(m.group(2)) if m.group(2) else None
            out.append(
                Correction(
                    f"C{len(out) + 1:03d}", b.cite, b.anchors[0], int(m.group(1)), page, b.text
                )
            )
        elif out:
            out[-1].text += " " + b.text  # "al-sawab: ..." continues the item
    by_page: dict[int, list[str]] = {}
    for b in blocks:
        if (p := _printed(b.cite)) is not None and b.zone != "corrections":
            by_page.setdefault(p, []).append(b.id)
    for c in out:
        if c.page is not None:
            c.blocks = by_page.get(c.page, [])
    return out


def _printed(cite: str) -> int | None:
    m = re.search(r" p\. (\d+)$", cite)
    return int(m.group(1)) if m else None
