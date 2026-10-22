"""The one documented normalization function (CLAUDE.md conventions)."""

from pipeline.normalize import skeleton

# Codepoints written as escapes; these are single letters and marks, not Quranic text.
ALEF, ALEF_HAMZA, ALEF_WASLA, ALEF_MADDA = "\u0627", "\u0623", "\u0671", "\u0622"
WAW, WAW_HAMZA, YEH, YEH_HAMZA, ALEF_MAQSURA = "\u0648", "\u0624", "\u064a", "\u0626", "\u0649"
BEH, FATHA, SUKUN_HIGH, TATWEEL, HAMZA = "\u0628", "\u064e", "\u06e1", "\u0640", "\u0621"


def test_marks_and_tatweel_are_removed() -> None:
    assert skeleton(BEH + FATHA + TATWEEL + SUKUN_HIGH + BEH) == BEH + BEH


def test_alef_variants_map_to_alef() -> None:
    for c in (ALEF_HAMZA, ALEF_WASLA, ALEF_MADDA, "\u0625"):
        assert skeleton(c) == ALEF


def test_hamza_seats_map_to_their_carrier_and_lone_hamza_is_dropped() -> None:
    assert skeleton(WAW_HAMZA) == WAW
    assert skeleton(YEH_HAMZA) == ALEF_MAQSURA
    assert skeleton(HAMZA) == ""


def test_yeh_and_alef_maqsura_are_unified() -> None:
    assert skeleton(YEH) == skeleton(ALEF_MAQSURA) == ALEF_MAQSURA


def test_yeh_barree_joins_the_yeh_family() -> None:
    # D20: KFGQPC Warsh/Qalun write U+06D2; comparison only, stored text is unchanged.
    assert skeleton(BEH + "\u06d2") == BEH + ALEF_MAQSURA


def test_spaces_signs_and_digits_are_dropped() -> None:
    assert skeleton(BEH + " " + "\u06de" + "\u06e9" + "\u0661" + "\ufc00" + BEH) == BEH + BEH
