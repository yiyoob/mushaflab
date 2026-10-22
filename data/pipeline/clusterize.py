"""T1.2: split an item's text into clusters using the codepoint class table.

A cluster is one base codepoint plus the marks (and joiners, UAX #29) that follow it, or a
standalone sign, ayah number, digit or control. A mark with no base before it, a space
inside an item, or a codepoint missing from the table is an error: the build stops.
"""

from dataclasses import dataclass

from pipeline.codepoints import Row

KIND_OF_CLASS = {
    "base": "letter",
    "sign": "sign",
    "ayah_number": "ayah_number",
    "digit": "digit",
    "control": "control",
}
STANDALONE = {"sign", "ayah_number", "digit", "control"}


class ClusterError(ValueError):
    pass


@dataclass(frozen=True)
class Cluster:
    text: str
    kind: str


def clusterize(text: str, table: dict[int, Row]) -> list[Cluster]:
    clusters: list[Cluster] = []
    open_base = False  # whether the last cluster can take marks
    for c in text:
        row = table.get(ord(c))
        if row is None:
            raise ClusterError(f"U+{ord(c):04X} is not in the codepoint table")
        if row.cls == "space":
            raise ClusterError(f"space U+{ord(c):04X} inside an item")
        if row.cls in ("mark", "joiner"):
            if not open_base:
                raise ClusterError(f"{row.cls} U+{ord(c):04X} has no base")
            last = clusters[-1]
            clusters[-1] = Cluster(last.text + c, last.kind)
            continue
        clusters.append(Cluster(c, KIND_OF_CLASS[row.cls]))
        open_base = row.cls not in STANDALONE
    return clusters
