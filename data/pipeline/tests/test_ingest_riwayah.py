"""T4.1: ingest of the 8 KFGQPC riwayah texts into items with their own ayah numbering.

Synthetic cases use arbitrary letters (U+0628, U+062A) to exercise structure only; they are
not Quranic text. Integration tests read the pinned sources (skipped locally when they are
not fetched; CI always fetches them). Expected values in integration tests were computed from
the pinned sources named in data/sources/MANIFEST.json (kfgqpc-official-* v2.x zips, and
kfgqpc-mirror-bazzi-data v7 for Bazzi) during the Phase 4 probe; each cites its source.
"""

import pytest

from pipeline import ingest_kfgqpc, ingest_riwayah, kfgqpc
from pipeline.ingest_riwayah import (
    IngestError,
    ayah_free_lines,
    decode_ayah_number,
    ingest,
    parse_ayah,
    parse_page,
    rebuild,
)
from pipeline.tests.markers import needs_sources

BEH, TEH, FATHA = "\u0628", "\u062a", "\u064e"
NBSP, HIZB, RLM = "\u00a0", "\u06de", "\u200f"


def sign(n: int) -> str:
    return chr(0xFC00 + n - 1)


def digits(n: int) -> str:
    return "".join(chr(0x0660 + int(d)) for d in str(n))


# --- decode_ayah_number -------------------------------------------------------------------


def test_decode_sign_and_digits() -> None:
    assert decode_ayah_number(sign(1)) == 1
    assert decode_ayah_number(sign(286)) == 286
    assert decode_ayah_number(digits(7)) == 7
    assert decode_ayah_number(digits(218)) == 218


@pytest.mark.parametrize("bad", ["", sign(1) + sign(2), "a", "\ufbff", digits(1) + "a"])
def test_decode_rejects_non_numbers(bad: str) -> None:
    with pytest.raises(IngestError):
        decode_ayah_number(bad)


# --- parse_ayah (synthetic) ---------------------------------------------------------------


def test_plain_ayah_with_sign() -> None:
    text = f"{HIZB}{NBSP}{BEH}{FATHA} {TEH}{NBSP}{sign(3)}"
    items, found = parse_ayah(2, 3, text, "sign", frozenset())
    assert [(i.ref, i.kind, i.sep_after) for i in items] == [
        ("2:3:0", "hizb", NBSP),
        ("2:3:1", "word", " "),
        ("2:3:2", "word", NBSP),
        ("2:3:3", "ayah_end", ""),
    ]
    assert found == set()
    assert rebuild(items) == text


def test_digit_convention() -> None:
    text = f"{BEH} {TEH}{NBSP}{digits(12)}"
    items, _ = parse_ayah(1, 12, text, "digits", frozenset())
    assert items[-1].kind == "ayah_end"
    assert items[-1].text == digits(12)
    with pytest.raises(IngestError, match="ayah number"):
        parse_ayah(1, 12, f"{BEH}{NBSP}{sign(12)}", "digits", frozenset())


def test_digits_under_sign_convention_only_when_listed() -> None:
    text = f"{BEH}{NBSP}{digits(5)}"
    with pytest.raises(IngestError, match="digit_ayah_number"):
        parse_ayah(1, 5, text, "sign", frozenset())
    items, found = parse_ayah(1, 5, text, "sign", frozenset({"digit_ayah_number"}))
    assert found == {"digit_ayah_number"}
    assert rebuild(items) == text


def test_attached_ayah_number_is_split_with_an_empty_separator() -> None:
    text = f"{BEH} {TEH}{sign(4)}"
    with pytest.raises(IngestError, match="attached_ayah_number"):
        parse_ayah(1, 4, text, "sign", frozenset())
    items, found = parse_ayah(1, 4, text, "sign", frozenset({"attached_ayah_number"}))
    assert [(i.kind, i.text, i.sep_after) for i in items] == [
        ("word", BEH, " "),
        ("word", TEH, ""),
        ("ayah_end", sign(4), ""),
    ]
    assert found == {"attached_ayah_number"}
    assert rebuild(items) == text


def test_hizb_inside_ayah_only_when_listed() -> None:
    text = f"{BEH} {HIZB}{NBSP}{TEH}{NBSP}{sign(2)}"
    with pytest.raises(IngestError, match="hizb_inside_ayah"):
        parse_ayah(1, 2, text, "sign", frozenset())
    items, _ = parse_ayah(1, 2, text, "sign", frozenset({"hizb_inside_ayah"}))
    assert [(i.ref, i.kind) for i in items] == [
        ("1:2:1", "word"),
        ("1:2:1.hizb", "hizb"),
        ("1:2:2", "word"),
        ("1:2:3", "ayah_end"),
    ]
    assert rebuild(items) == text


@pytest.mark.parametrize(
    ("text", "kind"),
    [
        (f"{BEH}  {TEH}{NBSP}{sign(1)}", "multi_separator"),
        (f"{BEH} {NBSP}{TEH}{NBSP}{sign(1)}", "multi_separator"),
        (f" {BEH}{NBSP}{sign(1)}", "leading_separator"),
    ],
)
def test_separator_anomalies_only_when_listed(text: str, kind: str) -> None:
    with pytest.raises(IngestError, match=kind):
        parse_ayah(1, 1, text, "sign", frozenset())
    items, found = parse_ayah(1, 1, text, "sign", frozenset({kind}))
    assert found == {kind}
    assert rebuild(items) == text


@pytest.mark.parametrize(
    ("text", "why"),
    [
        (f"{BEH}{NBSP}{sign(2)}", "does not match"),
        (f"{sign(1)} {BEH}", "last"),
        (f"{BEH} {TEH}", "ayah number"),
        (f"{BEH} {TEH} ", "trailing"),
    ],
)
def test_rejects_unexpected_structure(text: str, why: str) -> None:
    with pytest.raises(IngestError, match=why):
        parse_ayah(1, 1, text, "sign", frozenset())


def test_controls_stay_inside_their_word() -> None:
    text = f"{BEH}{FATHA}{RLM} {TEH}{NBSP}{sign(1)}"
    items, _ = parse_ayah(1, 1, text, "sign", frozenset())
    assert items[0].text == BEH + FATHA + RLM


def test_ingest_requires_the_listed_anomalies_exactly() -> None:
    rows = [
        {"sura_no": 1, "aya_no": 1, "aya_text": f"{BEH}{NBSP}{sign(1)}"},
        {"sura_no": 1, "aya_no": 2, "aya_text": f" {TEH}{NBSP}{sign(2)}"},
    ]
    listed = {"leading_separator": frozenset({(1, 2)})}
    text = ingest("x", rows, "sign", listed)
    assert text.anomalies == {"leading_separator": {(1, 2)}}
    with pytest.raises(IngestError, match="listed but not found"):
        ingest("x", rows, "sign", {**listed, "multi_separator": frozenset({(1, 1)})})
    with pytest.raises(IngestError, match="leading_separator"):
        ingest("x", rows, "sign", {})


def test_ingest_rejects_gaps_in_numbering() -> None:
    rows = [
        {"sura_no": 1, "aya_no": 1, "aya_text": f"{BEH}{NBSP}{sign(1)}"},
        {"sura_no": 1, "aya_no": 3, "aya_text": f"{TEH}{NBSP}{sign(3)}"},
    ]
    with pytest.raises(IngestError, match="order"):
        ingest("x", rows, "sign", {})


# --- pages --------------------------------------------------------------------------------


def test_parse_page() -> None:
    assert parse_page(5) == (5, 5)
    assert parse_page("12") == (12, 12)
    assert parse_page("85-86") == (85, 86)
    for bad in ["85-87", "a", "86-85", "0"]:
        with pytest.raises(IngestError):
            parse_page(bad)


def test_ayah_free_lines_cover_page_crossings() -> None:
    rows = [
        {"page": "1", "line_start": 2, "line_end": 15},
        {"page": "1-2", "line_start": 15, "line_end": 1},
        {"page": 2, "line_start": 3, "line_end": 15},
    ]
    assert ayah_free_lines(rows) == {(1, 1), (2, 2)}


# --- integration with the pinned sources --------------------------------------------------

# Ayah totals per riwayah, counted from the pinned KFGQPC files (rows per file); they agree
# with docs/spikes/s4-layouts.md.
EXPECTED_AYAHS = {
    "hafs": 6236,
    "shuba": 6236,
    "warsh": 6214,
    "qalun": 6214,
    "bazzi": 6220,
    "qunbul": 6220,
    "duri_abuamr": 6217,
    "susi": 6217,
}


@needs_sources
@pytest.mark.parametrize("riwayah", kfgqpc.RIWAYAT)
def test_every_ayah_rebuilds_exactly(riwayah: str) -> None:
    text = ingest_riwayah.load(riwayah)
    rows = kfgqpc.rows(riwayah)
    assert len(rows) == EXPECTED_AYAHS[riwayah]
    rebuilt = text.ayah_texts()
    assert list(rebuilt) == [(int(r["sura_no"]), int(r["aya_no"])) for r in rows]
    for row in rows:
        key = (int(row["sura_no"]), int(row["aya_no"]))
        assert rebuilt[key] == row["aya_text"], key
    assert sum(1 for i in text.items if i.kind == "ayah_end") == len(rows)


@needs_sources
def test_hafs_items_equal_the_spine_ingest() -> None:
    mine = ingest_riwayah.load("hafs").items
    spine = ingest_kfgqpc.ingest(ingest_kfgqpc.load_rows())
    assert [(i.ref, i.kind, i.text, i.sep_before, i.sep_after) for i in mine] == [
        (i.id, i.kind, i.text, i.sep_before, i.sep_after) for i in spine
    ]


@needs_sources
def test_controls_are_located() -> None:
    # G-017 / G-029: U+200F (RLM) x14 in Warsh and Qalun at word ends, U+200D (ZWJ) x1 in
    # Duri and Susi 17:7 (counts from docs/font-coverage.md section 1).
    found = {r: ingest_riwayah.controls(ingest_riwayah.load(r)) for r in kfgqpc.RIWAYAT}
    assert {r: len(v) for r, v in found.items() if v} == {
        "warsh": 14,
        "qalun": 14,
        "duri_abuamr": 1,
        "susi": 1,
    }
    assert {c.cp for r in ("warsh", "qalun") for c in found[r]} == {0x200F}
    assert [(c.ref, c.cp) for c in found["susi"]] == [(c.ref, c.cp) for c in found["duri_abuamr"]]
    assert found["susi"][0].ref.startswith("17:7:")
    assert found["susi"][0].cp == 0x200D


@needs_sources
def test_g023_extra_ayah_free_line_is_page_1_line_2() -> None:
    # G-023: four riwayat have one more line without an ayah than Hafs; the data puts it on
    # page 1, line 2 (their Fatihah ayah 1 starts on line 3).
    hafs = ayah_free_lines(kfgqpc.rows("hafs"))
    extra = {r: ayah_free_lines(kfgqpc.rows(r)) ^ hafs for r in kfgqpc.RIWAYAT}
    assert {r: v for r, v in extra.items() if v} == {
        r: {(1, 2)} for r in ("warsh", "qalun", "duri_abuamr", "susi")
    }
