"""T6.1: the rule register agrees with the rule files (ADR D38).

    python -m pipeline.rules_register [ROOT]

The same checks as `Register::check` in the core (`mushaf-model/src/rules.rs`), so that a
disagreement fails `just data-verify` and `just data-test` as well as the Rust tests: every
rule has exactly one entry and every entry a rule; owner and option agree; an entry without
a cited source is a `hypothesis`; descriptions are not empty.
"""

import sys
from pathlib import Path

from pipeline.compile_db import DATA_ROOT, REGISTER, load_yaml, yaml_files

STATUSES = ("rule-derived", "hypothesis")


def _option(rule: dict) -> str | None:
    when = rule.get("when")
    return when["option"] if isinstance(when, dict) else None


def _compare(rid: str, rule: dict, e: dict) -> list[str]:
    fields = {
        "owner": (e["owner"], rule["owner"]),
        "option": (e.get("option"), _option(rule)),
        "single_source": (bool(e.get("single_source")), bool(rule.get("single_source"))),
    }
    return [f"rule {rid}: {k} differs from the register" for k, (a, b) in fields.items() if a != b]


def check(root: Path = DATA_ROOT) -> list[str]:
    problems: list[str] = []
    reg_path = root / REGISTER
    register = load_yaml(reg_path, ascii_only=False) if reg_path.exists() else {"rules": []}
    entries: dict[str, dict] = {}
    for e in register["rules"]:
        if e["id"] in entries:
            problems.append(f"register: {e['id']} is listed twice")
        entries[e["id"]] = e
        if not str(e.get("description", "")).strip():
            problems.append(f"register: {e['id']} has no description")
        if e.get("status") not in STATUSES:
            problems.append(f"register: {e['id']} has status {e.get('status')!r}")
        if e.get("source") is None and e.get("status") == "rule-derived":
            problems.append(f"register: {e['id']} is rule-derived without a source")
    found: set[str] = set()
    for path in yaml_files(root, "rules"):
        if path == reg_path:
            continue
        for rule in load_yaml(path)["rules"]:
            rid = rule["id"]
            if rid in found:
                problems.append(f"{path.name}: rule {rid} is defined twice")
                continue
            found.add(rid)
            e = entries.get(rid)
            if e is None:
                problems.append(f"{path.name}: rule {rid} has no register entry")
                continue
            problems += _compare(rid, rule, e)
    problems += [f"register: {rid} has no rule" for rid in sorted(set(entries) - found)]
    return problems


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else DATA_ROOT
    problems = check(root)
    for p in problems:
        print(p, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
