"""T1.1: parse the KFGQPC official v2.0 Hafs text into spine items.

An ayah's `aya_text` is a sequence of tokens separated by single U+0020 or U+00A0
characters. Tokens are classified by structure only:
  * an optional rub el hizb sign (U+06DE) as the first token -> item kind `hizb`, pos 0;
  * words -> kind `word`, pos 1..n (a sajdah sign U+06E9 stays inside its word);
  * one ayah-number sign as the last token -> kind `ayah_end`, pos n+1.
Separators are kept exactly (`sep_after`; a leading one as `sep_before` on the first item,
allowed only for listed source anomalies), so items rebuild the source byte for byte.
Anything else is an IngestError: the source is reported, never repaired (rule 1).
"""

import re
from dataclasses import dataclass

from pipeline import sources

SOURCE_ID = "kfgqpc-official-hafs"
SOURCE_MEMBER = "hafsData_v2-0.json"
SEPARATORS = (" ", "\u00a0")
HIZB = "\u06de"
AYAH_SIGN_FIRST = 0xFC00
AYAH_SIGN_LAST = 0xFDFF
TOKEN_RE = re.compile("[ \u00a0]|[^ \u00a0]+")
# Source anomalies kept exactly as published (never repaired, rule 1); see known-gaps.
LEADING_SEPARATOR_ANOMALIES = {(2, 6)}  # G-026: aya_text starts with U+0020


class IngestError(ValueError):
    pass


@dataclass(frozen=True)
class Item:
    surah: int
    ayah: int
    pos: int
    kind: str  # word | ayah_end | hizb
    text: str
    sep_after: str
    sep_before: str = ""

    @property
    def id(self) -> str:
        return f"{self.surah}:{self.ayah}:{self.pos}"


def decode_ayah_sign(token: str) -> int:
    """Decode a KFGQPC v2 ayah-number sign: the single codepoint U+FC00 + n - 1 means n.

    KFGQPC repurposes Arabic presentation-form codepoints for these signs; only its own font
    draws them as ayah numbers (G-013). This is the only place that knows the convention.
    """
    if len(token) != 1 or not AYAH_SIGN_FIRST <= ord(token) <= AYAH_SIGN_LAST:
        raise IngestError(f"not an ayah sign: {token.encode('unicode_escape').decode()}")
    return ord(token) - AYAH_SIGN_FIRST + 1


def _is_ayah_sign(token: str) -> bool:
    return len(token) == 1 and AYAH_SIGN_FIRST <= ord(token) <= AYAH_SIGN_LAST


def _split(where: str, text: str) -> tuple[list[str], list[str]]:
    """Tokens and the single separators between them; anything else is an error."""
    parts = TOKEN_RE.findall(text)
    if "".join(parts) != text:
        raise IngestError(f"{where}: tokenization lost characters")
    tokens: list[str] = []
    seps: list[str] = []
    for i, part in enumerate(parts):
        is_sep = part in SEPARATORS
        if is_sep != (i % 2 == 1):
            raise IngestError(f"{where}: unexpected separator layout at part {i}")
        (seps if is_sep else tokens).append(part)
    if len(seps) != len(tokens) - 1:
        raise IngestError(f"{where}: separator count does not fit the tokens")
    if not tokens or not _is_ayah_sign(tokens[-1]):
        raise IngestError(f"{where}: no ayah sign as the last token")
    return tokens, seps


def parse_ayah(surah: int, ayah: int, text: str) -> list[Item]:
    where = f"{surah}:{ayah}"
    leading = ""
    if text[:1] in SEPARATORS and (surah, ayah) in LEADING_SEPARATOR_ANOMALIES:
        leading, text = text[0], text[1:]
    tokens, seps = _split(where, text)
    items: list[Item] = []
    pos = 0
    for i, token in enumerate(tokens):
        sep = seps[i] if i < len(seps) else ""
        if token == HIZB:
            if i != 0:
                raise IngestError(f"{where}: hizb sign is not the first token")
            items.append(Item(surah, ayah, 0, "hizb", token, sep))
        elif _is_ayah_sign(token):
            if i != len(tokens) - 1:
                raise IngestError(f"{where}: ayah sign is not the last token")
            if decode_ayah_sign(token) != ayah:
                raise IngestError(f"{where}: ayah sign does not match the ayah number")
            items.append(Item(surah, ayah, pos + 1, "ayah_end", token, sep))
        else:
            pos += 1
            items.append(Item(surah, ayah, pos, "word", token, sep))
    if leading:
        first = items[0]
        items[0] = Item(
            first.surah, first.ayah, first.pos, first.kind, first.text, first.sep_after, leading
        )
    return items


def rebuild_ayah(items: list[Item]) -> str:
    return "".join(i.sep_before + i.text + i.sep_after for i in items)


def load_rows() -> list[dict]:
    return sources.zip_json(SOURCE_ID, SOURCE_MEMBER)  # type: ignore[return-value]


def ingest(rows: list[dict]) -> list[Item]:
    items: list[Item] = []
    for row in rows:
        items.extend(parse_ayah(int(row["sura_no"]), int(row["aya_no"]), row["aya_text"]))
    return items
