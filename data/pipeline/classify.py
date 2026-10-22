"""D21: difference-kind tags for raw patches (D13). Review and stats only; never applied.

Rules, in order (the rule id is reported in stats and review sheets):
  * `ayah-number`: any change to an ayah-end item (renumbering, merged or split ayahs)
    -> `unclassified`;
  * `confirmed`: a cluster change whose codepoint differences are all one-for-one
    substitutions confirmed for its riwayah -> that row's kind;
  * `default` -> `unclassified`.
No other rule exists: the pinned data cannot tell an encoding difference from a spelling or
reading difference without the owner's judgment (D21, with the evidence in the stats).

Confirmed rows live in `data/approvals/difference_kinds.yaml` (owner decision, D21): only
owner-signed commits may change that directory (CI, rule 4), and Claude never writes it.
A missing file means no confirmed rows. Its form:

    confirmed:
      - {riwayah: "warsh", from: "U+06E1", to: "U+0652", kind: "encoding",
         by: "<owner>", date: "YYYY-MM-DD", evidence: "<where the evidence is>"}

A row says: in that riwayah, the Hafs codepoint `from`, written as `to` at the same position,
is a difference of `kind`. A patch is tagged only if every codepoint difference it carries is
a confirmed row of one kind; otherwise it stays `unclassified`.
"""

import difflib
from dataclasses import dataclass
from pathlib import Path

from pipeline import kfgqpc
from pipeline.compile_db import load_yaml
from pipeline.differ import Change
from pipeline.spine import from_cps

RULES_PATH = Path(__file__).resolve().parents[1] / "approvals" / "difference_kinds.yaml"
KINDS = ("encoding", "spelling", "reading", "unclassified")
ROW_KEYS = frozenset({"riwayah", "from", "to", "kind", "by", "date", "evidence"})

Confirmed = dict[tuple[str, str], str]


class RuleError(ValueError):
    pass


@dataclass(frozen=True)
class Tag:
    kind: str
    rule: str


def _one_cp(value: object, where: str) -> str:
    text = from_cps(value) if isinstance(value, str) and value.startswith("U+") else ""
    if len(text) != 1:
        raise RuleError(f'{where}: expected one "U+XXXX" codepoint, got {value!r}')
    return text


def load_confirmed(path: Path = RULES_PATH) -> dict[str, Confirmed]:
    if not path.exists():
        return {r: {} for r in kfgqpc.RIWAYAT[1:]}
    data = load_yaml(path)
    if not isinstance(data, dict) or set(data) != {"confirmed"}:
        raise RuleError(f"{path}: expected a mapping with only `confirmed`")
    out: dict[str, Confirmed] = {r: {} for r in kfgqpc.RIWAYAT[1:]}
    for n, row in enumerate(data["confirmed"] or [], start=1):
        where = f"{path.name} row {n}"
        if not isinstance(row, dict) or set(row) != ROW_KEYS:
            raise RuleError(f"{where}: keys must be {sorted(ROW_KEYS)}")
        if row["riwayah"] not in out:
            raise RuleError(f"{where}: unknown riwayah {row['riwayah']!r}")
        if row["kind"] not in KINDS[:-1]:
            raise RuleError(f"{where}: kind must be one of {KINDS[:-1]}")
        if not all(row[k] for k in ("by", "date", "evidence")):
            raise RuleError(f"{where}: by, date and evidence are required")
        key = (_one_cp(row["from"], where), _one_cp(row["to"], where))
        if key in out[row["riwayah"]]:
            raise RuleError(f"{where}: duplicate row")
        out[row["riwayah"]][key] = row["kind"]
    return out


def components(old: str, new: str) -> list[tuple[str, str, str]]:
    """Codepoint differences: ("sub", x, y) for one-for-one substitutions, else
    (tag, removed, added) for a span difflib cannot pair one for one."""
    out: list[tuple[str, str, str]] = []
    matcher = difflib.SequenceMatcher(None, old, new, autojunk=False)
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            continue
        if tag == "replace" and i2 - i1 == j2 - j1:
            out += [("sub", x, y) for x, y in zip(old[i1:i2], new[j1:j2], strict=True)]
        else:
            out.append((tag, old[i1:i2], new[j1:j2]))
    return out


def classify(change: Change, confirmed: Confirmed) -> Tag:
    if change.item_kind == "ayah_end":
        return Tag("unclassified", "ayah-number")
    if change.level == "cluster" and change.kind in ("marks", "letter"):
        comps = components(change.old, change.new)
        kinds = {confirmed.get((x, y)) if t == "sub" else None for t, x, y in comps}
        if len(kinds) == 1 and None not in kinds:
            return Tag(kinds.pop(), "confirmed")  # type: ignore[arg-type]
    return Tag("unclassified", "default")
