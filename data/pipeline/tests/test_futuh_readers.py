"""al-Futuh reader-label resolution (pipeline/futuh/readers.py). Cases: futuh_cases.yaml."""

from pathlib import Path

import pytest
import yaml

from pipeline.futuh.quran import canon
from pipeline.futuh.readers import Lexicon, parse_head, resolve_rows

CASES = yaml.safe_load((Path(__file__).with_name("futuh_cases.yaml")).read_text(encoding="utf-8"))
LEX = Lexicon()
HAMZAH = {"khalaf_an_hamzah", "khallad"}


def rows(labels: list[str], spine: list[str] | None = None) -> list[dict]:
    rs = [
        {"id": f"r{i}", "group": "g", "readers": lab, "words": [{"spine": s} for s in spine or []]}
        for i, lab in enumerate(labels)
    ]
    resolve_rows(LEX, rs, canon() if spine else None)
    return rs


def test_terms_table() -> None:
    c = CASES["terms_table"]
    assert set(rows([c["label"]])[0]["ruwat"]) == set(c["ruwat"])


def test_except_following_takes_later_rows() -> None:
    rs = rows(CASES["except_following"]["labels"])
    assert rs[1]["ruwat"] == ["qunbul", "ruways"]
    assert set(rs[2]["ruwat"]) == HAMZAH
    assert len(rs[0]["ruwat"]) == 16
    assert rs[0]["coverage"]["all_certain"]


def test_include_previous_is_bounded_not_guessed() -> None:
    r = rows([CASES["include_previous"]])[0]
    assert r["ruwat"] is None
    assert r["ruwat_min"] == ["qalun"]
    assert len(r["ruwat_max"]) == 20


def test_madd_group_without_madd_is_contextual() -> None:
    r = rows([CASES["madd_group"]])[0]
    assert r["readers_parse"]["status"] == "contextual"


def test_waqf_only_qualifier() -> None:
    p = parse_head(LEX, CASES["waqf_only"])
    assert p["waqf_only"] == HAMZAH
    assert p["named"] == {"warsh"}


@pytest.mark.skipif(not canon().ayahs, reason="spine not available")
def test_munfasil_madd_group_bounds() -> None:
    c = CASES["munfasil_segment"]
    r = rows(c["labels"], c["spine"])[1]
    assert r["readers_parse"]["madd"] == "munfasil"
    assert set(r["ruwat_max"]) == set(c["tawassut_max"])
    assert not set(c["not_certain"]) & set(r["ruwat_min"])


def test_all_except_madd_group() -> None:
    c = CASES["all_except_madd_group"]
    r = rows([c["label"]], c["spine"])[0]
    assert r["readers_parse"]["madd"] == "muttasil"
    assert len(r["ruwat"]) == 17
    assert not set(c["excluded"]) & set(r["ruwat"])
