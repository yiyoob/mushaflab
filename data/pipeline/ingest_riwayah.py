"""T4.1: parse each KFGQPC riwayah text into items, preserving its own ayah numbering.

    python -m pipeline.ingest_riwayah report          # print the ingest report (markdown)
    python -m pipeline.ingest_riwayah write           # write docs/review/phase4/ingest.md

Generalizes `ingest_kfgqpc` (which stays the spine's parser) to all 8 texts. An ayah's
`aya_text` is tokens separated by separators (U+0020, U+00A0):
  * a rub el hizb sign (U+06DE) -> item kind `hizb`; normally the first token;
  * words -> kind `word`, numbered 1..n;
  * the ayah number as the last token -> kind `ayah_end`: a U+FC00+n-1 sign, or (Bazzi v7,
    `AYAH_NUMBER_STYLE`) a run of Arabic-Indic digits.
Every structural departure from that is a source anomaly, accepted only where `ANOMALIES`
lists it for that riwayah and ayah, and the list must be exact: an anomaly found but not
listed, or listed but not found, is an IngestError. Text is never repaired (rule 1).
"""

import re
import sys
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path

from pipeline import kfgqpc
from pipeline.clusterize import clusterize
from pipeline.codepoints import load_table

SEPARATORS = frozenset(" \u00a0")
HIZB = "\u06de"
SIGN_FIRST, SIGN_LAST = 0xFC00, 0xFDFF
DIGIT_ZERO, DIGIT_NINE = 0x0660, 0x0669
TOKEN_RE = re.compile("[ \u00a0]+|[^ \u00a0]+")
REPORT_PATH = Path(__file__).resolve().parents[2] / "docs" / "review" / "phase4" / "ingest.md"

# How each text writes ayah numbers (from the pinned files: Bazzi v7 uses digits throughout).
AYAH_NUMBER_STYLE = {r: "sign" for r in kfgqpc.RIWAYAT} | {"bazzi": "digits"}

ANOMALY_KINDS = (
    "leading_separator",  # aya_text starts with a separator
    "multi_separator",  # more than one separator character between two tokens
    "hizb_inside_ayah",  # a hizb sign that is not the ayah's first token
    "attached_ayah_number",  # the ayah sign is the last codepoint of the last word
    "digit_ayah_number",  # digits where the text otherwise uses U+FC00 signs
)

# Listed source anomalies, located with `report` from the pinned files (Phase 4 probe,
# 2026-10-22). Kept exactly as published; see docs/review/phase4/ingest.md and known-gaps.
_DURI_SUSI = {
    "leading_separator": {(2, 5)},
    "hizb_inside_ayah": {(2, 195), (3, 7), (18, 22)},
}
ANOMALIES: dict[str, dict[str, set[tuple[int, int]]]] = {
    "hafs": {"leading_separator": {(2, 6)}},  # G-026
    "shuba": {"leading_separator": {(2, 6)}, "digit_ayah_number": {(2, 286)}},  # G-016
    "warsh": {
        "multi_separator": {(4, 44), (20, 86), (24, 36), (24, 42)},
        "hizb_inside_ayah": {
            (2, 84),
            (2, 195),
            (2, 233),
            (3, 80),
            (4, 19),
            (4, 91),
            (4, 156),
            (5, 34),
            (33, 37),
            (59, 2),
            (65, 6),
            (72, 23),
        },
        "attached_ayah_number": {(16, 123)},
    },
    "qalun": {"leading_separator": {(2, 5), (17, 67)}, "hizb_inside_ayah": {(2, 84)}},
    "bazzi": {
        "multi_separator": {(2, 105), (2, 218), (3, 48), (4, 44), (18, 49), (24, 36), (24, 42)}
    },
    "qunbul": {},
    "duri_abuamr": _DURI_SUSI,
    "susi": _DURI_SUSI,
}


class IngestError(ValueError):
    pass


@dataclass(frozen=True)
class Item:
    surah: int
    ayah: int
    pos: int  # words 1..n; ayah_end n+1; hizb: the number of words before it
    kind: str  # word | ayah_end | hizb
    text: str
    sep_after: str
    sep_before: str = ""

    @property
    def ref(self) -> str:
        """Riwayah-local reference; equals the spine id for Hafs."""
        base = f"{self.surah}:{self.ayah}:{self.pos}"
        return f"{base}.hizb" if self.kind == "hizb" and self.pos else base


@dataclass
class Text:
    riwayah: str
    items: list[Item]
    anomalies: dict[str, set[tuple[int, int]]] = field(default_factory=dict)

    def ayah_texts(self) -> dict[tuple[int, int], str]:
        out: dict[tuple[int, int], str] = {}
        for i in self.items:
            key = (i.surah, i.ayah)
            out[key] = out.get(key, "") + i.sep_before + i.text + i.sep_after
        return out


@dataclass(frozen=True)
class Control:
    ref: str
    cluster: int  # 1-based cluster index inside the item
    cp: int


def _is_sign(token: str) -> bool:
    return len(token) == 1 and SIGN_FIRST <= ord(token) <= SIGN_LAST


def _is_digits(token: str) -> bool:
    return bool(token) and all(DIGIT_ZERO <= ord(c) <= DIGIT_NINE for c in token)


def is_ayah_number(token: str) -> bool:
    return _is_sign(token) or _is_digits(token)


def decode_ayah_number(token: str) -> int:
    """A U+FC00+n-1 sign (G-013) or a run of Arabic-Indic digits, as the number n."""
    if _is_sign(token):
        return ord(token) - SIGN_FIRST + 1
    if _is_digits(token):
        return int("".join(str(ord(c) - DIGIT_ZERO) for c in token))
    raise IngestError(f"not an ayah number: {token.encode('unicode_escape').decode()}")


def _tokens(where: str, text: str) -> tuple[str, list[str], list[str], set[str]]:
    """Leading separator, tokens, the separators after each token but the last, anomalies."""
    parts = TOKEN_RE.findall(text)
    if "".join(parts) != text:
        raise IngestError(f"{where}: tokenization lost characters")
    found: set[str] = set()
    leading = ""
    if parts and parts[0][0] in SEPARATORS:
        leading = parts.pop(0)
        found.add("leading_separator")
    if parts and parts[-1][0] in SEPARATORS:
        raise IngestError(f"{where}: trailing separator")
    tokens, seps = parts[0::2], parts[1::2]
    if any(s[0] not in SEPARATORS for s in seps) or any(t[0] in SEPARATORS for t in tokens):
        raise IngestError(f"{where}: unexpected separator layout")
    if len(leading) > 1 or any(len(s) > 1 for s in seps):
        found.add("multi_separator")
    return leading, tokens, seps, found


def _check_number(
    where: str, ayah: int, tokens: list[str], seps: list[str], style: str
) -> set[str]:
    """Splits an attached ayah sign off the last word; the last token must number the ayah.
    Returns the anomaly kinds seen."""
    found: set[str] = set()
    if not tokens:
        raise IngestError(f"{where}: empty ayah")
    last = tokens[-1]
    if style == "sign" and len(last) > 1 and _is_sign(last[-1]) and not _is_digits(last):
        tokens[-1:] = [last[:-1], last[-1]]
        seps.append("")
        found.add("attached_ayah_number")
    number = tokens[-1]
    if style == "sign" and _is_digits(number):
        found.add("digit_ayah_number")
    elif not (_is_sign(number) if style == "sign" else _is_digits(number)):
        raise IngestError(f"{where}: no ayah number ({style}) as the last token")
    if decode_ayah_number(number) != ayah:
        raise IngestError(f"{where}: ayah number does not match the ayah")
    return found


def parse_ayah(
    surah: int, ayah: int, text: str, style: str, allowed: frozenset[str]
) -> tuple[list[Item], set[str]]:
    """Items of one ayah and the anomaly kinds it shows; each must be in `allowed`."""
    where = f"{surah}:{ayah}"
    leading, tokens, seps, found = _tokens(where, text)
    found |= _check_number(where, ayah, tokens, seps, style)
    items: list[Item] = []
    words = 0
    for i, token in enumerate(tokens):
        sep = seps[i] if i < len(seps) else ""
        if i == len(tokens) - 1:
            items.append(Item(surah, ayah, words + 1, "ayah_end", token, sep))
        elif token == HIZB:
            if i != 0:
                found.add("hizb_inside_ayah")
            items.append(Item(surah, ayah, words, "hizb", token, sep))
        elif _is_sign(token) or _is_digits(token):
            raise IngestError(f"{where}: ayah number is not the last token")
        else:
            words += 1
            items.append(Item(surah, ayah, words, "word", token, sep))
    if leading:
        f = items[0]
        items[0] = Item(f.surah, f.ayah, f.pos, f.kind, f.text, f.sep_after, leading)
    if unlisted := sorted(found - allowed):
        raise IngestError(f"{where}: unlisted source anomaly {unlisted}")
    return items, found


def rebuild(items: Iterable[Item]) -> str:
    return "".join(i.sep_before + i.text + i.sep_after for i in items)


def ingest(
    riwayah: str,
    rows: list[dict],
    style: str,
    listed: Mapping[str, Iterable[tuple[int, int]]],
) -> Text:
    """All ayahs in source order; numbering must run 1..n per surah, surahs 1..m."""
    listed_sets = {k: set(v) for k, v in listed.items() if v}
    if unknown := set(listed_sets) - set(ANOMALY_KINDS):
        raise IngestError(f"{riwayah}: unknown anomaly kinds {sorted(unknown)}")
    text = Text(riwayah, [])
    prev: tuple[int, int] | None = None
    for row in rows:
        key = (int(row["sura_no"]), int(row["aya_no"]))
        ok = [(1, 1)] if prev is None else [(prev[0], prev[1] + 1), (prev[0] + 1, 1)]
        if key not in ok:
            raise IngestError(f"{riwayah} {key}: ayahs out of order (after {prev})")
        allowed = frozenset(k for k, v in listed_sets.items() if key in v)
        items, found = parse_ayah(*key, row["aya_text"], style, allowed)
        text.items.extend(items)
        for kind in found:
            text.anomalies.setdefault(kind, set()).add(key)
        prev = key
    if missing := {k: v - text.anomalies.get(k, set()) for k, v in listed_sets.items()}:
        missing = {k: sorted(v) for k, v in missing.items() if v}
        if missing:
            raise IngestError(f"{riwayah}: anomalies listed but not found: {missing}")
    return text


def load(riwayah: str) -> Text:
    return ingest(riwayah, kfgqpc.rows(riwayah), AYAH_NUMBER_STYLE[riwayah], ANOMALIES[riwayah])


def controls(text: Text) -> list[Control]:
    """Where the text carries control codepoints (class `control` or `joiner`, G-017/G-029)."""
    table = load_table()
    out = []
    for item in text.items:
        for n, cluster in enumerate(clusterize(item.text, table), start=1):
            out.extend(
                Control(item.ref, n, ord(c))
                for c in cluster.text
                if table[ord(c)].cls in ("control", "joiner")
            )
    return out


def parse_page(value: object) -> tuple[int, int]:
    """KFGQPC `page`: an int, "n", or "a-b" for an ayah crossing from page a to page a+1."""
    s = str(value)
    m = re.fullmatch(r"([1-9][0-9]*)(?:-([1-9][0-9]*))?", s)
    if not m:
        raise IngestError(f"bad page value {s!r}")
    first = int(m.group(1))
    last = int(m.group(2)) if m.group(2) else first
    if last not in (first, first + 1):
        raise IngestError(f"bad page range {s!r}")
    return first, last


LINES_PER_PAGE = 15


def ayah_free_lines(rows: list[dict]) -> set[tuple[int, int]]:
    """(page, line) pairs on pages that hold ayahs, with no ayah on them."""
    covered: set[tuple[int, int]] = set()
    for row in rows:
        first, last = parse_page(row["page"])
        start, end = int(row["line_start"]), int(row["line_end"])
        if first == last:
            covered |= {(first, n) for n in range(start, end + 1)}
        else:
            covered |= {(first, n) for n in range(start, LINES_PER_PAGE + 1)}
            covered |= {(last, n) for n in range(1, end + 1)}
    pages = {p for p, _ in covered}
    return {(p, n) for p in pages for n in range(1, LINES_PER_PAGE + 1)} - covered


def _fmt(keys: Iterable[tuple[int, int]]) -> str:
    return ", ".join(f"{s}:{a}" for s, a in sorted(keys))


def report() -> str:
    hafs_free = ayah_free_lines(kfgqpc.rows("hafs"))
    lines = [
        "# Phase 4 ingest report (T4.1)",
        "",
        "Generated by `python -m pipeline.ingest_riwayah write` from the pinned KFGQPC files"
        " (official v2.x zips; Bazzi: 2022 mirror v7, D13). Every ayah of every text rebuilds"
        " byte for byte from its items. Anomalies are kept exactly as published.",
        "",
        "| Riwayah | Ayahs | Words | Hizb signs | Ayah numbers |",
        "|---|---|---|---|---|",
    ]
    texts = {r: load(r) for r in kfgqpc.RIWAYAT}
    for r, t in texts.items():
        kinds = [i.kind for i in t.items]
        lines.append(
            f"| {r} | {kinds.count('ayah_end')} | {kinds.count('word')} | "
            f"{kinds.count('hizb')} | {AYAH_NUMBER_STYLE[r]} |"
        )
    lines += ["", "## Source anomalies (listed in `ANOMALIES`, kept as published)", ""]
    lines += ["| Riwayah | Kind | Ayahs |", "|---|---|---|"]
    for r, t in texts.items():
        lines += [f"| {r} | {k} | {_fmt(v)} |" for k, v in sorted(t.anomalies.items())]
    lines += ["", "## Control codepoints (G-017, G-029)", ""]
    lines += ["| Riwayah | Item | Cluster | Codepoint |", "|---|---|---|---|"]
    for r, t in texts.items():
        lines += [f"| {r} | {c.ref} | {c.cluster} | U+{c.cp:04X} |" for c in controls(t)]
    lines += ["", '## Ayahs crossing a page (`page` = "a-b")', ""]
    for r in kfgqpc.RIWAYAT:
        crossing = [
            f"{row['sura_no']}:{row['aya_no']} ({row['page']})"
            for row in kfgqpc.rows(r)
            if parse_page(row["page"])[0] != parse_page(row["page"])[1]
        ]
        lines.append(f"- {r}: {', '.join(crossing) or 'none'}")
    lines += ["", "## Lines without an ayah, compared with Hafs (G-023)", ""]
    for r in kfgqpc.RIWAYAT[1:]:
        diff = ayah_free_lines(kfgqpc.rows(r)) ^ hafs_free
        shown = ", ".join(f"page {p} line {n}" for p, n in sorted(diff))
        lines.append(f"- {r}: {shown or 'same as Hafs'}")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    text = report()
    if argv[1:] == ["write"]:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(text, encoding="utf-8")
        print(f"ingest report written to {REPORT_PATH}")
    else:
        print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
