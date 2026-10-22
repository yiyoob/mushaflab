"""The one documented normalization function (CLAUDE.md: no normalization elsewhere).

skeleton() reduces text to its base letters, so texts in different encodings or printings
can be aligned and compared. It never changes stored data; it is only used to compare.

Rules (each needed because the pinned sources encode these differently, see S1/S2):
  * keep Arabic letters U+0622..U+064A, except tatweel U+0640;
  * drop everything else: marks, signs, digits, ayah numbers, spaces, controls, and the
    lone hamza U+0621 (not part of the letter skeleton);
  * alef with hamza above/below, alef with madda and alef wasla -> alef U+0627;
  * waw with hamza -> waw U+0648;
  * yeh with hamza, yeh and Farsi yeh -> alef maqsura U+0649 (dotless form).
"""

SKELETON_MAP = str.maketrans(
    {
        "\u0623": "\u0627",
        "\u0625": "\u0627",
        "\u0622": "\u0627",
        "\u0671": "\u0627",
        "\u0624": "\u0648",
        "\u0626": "\u0649",
        "\u064a": "\u0649",
        "\u06cc": "\u0649",
    }
)
FIRST_LETTER, LAST_LETTER, TATWEEL = "\u0622", "\u064a", "\u0640"


def skeleton(text: str) -> str:
    mapped = text.translate(SKELETON_MAP)
    return "".join(c for c in mapped if FIRST_LETTER <= c <= LAST_LETTER and c != TATWEEL)
