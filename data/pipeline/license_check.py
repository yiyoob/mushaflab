"""Fail if third-party data is tracked in git before its license allows it.

CLAUDE.md rules 2 and 10: raw sources are never tracked, and fonts are not tracked
until their row in data/sources/LICENSES.md allows redistribution. Until T0.7 adds
that lookup, every tracked file under these paths is a violation.
"""

import subprocess
import sys
from pathlib import Path

PROTECTED_PREFIXES = ("data/sources/raw/", "fonts/")
ALLOWED_NAMES = (".gitkeep",)


def tracked_files(repo: Path) -> list[str]:
    out = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z", "--cached"],
        check=True,
        capture_output=True,
    ).stdout
    return [p for p in out.decode("utf-8").split("\0") if p]


def find_violations(repo: Path) -> list[str]:
    return sorted(
        path
        for path in tracked_files(repo)
        if path.startswith(PROTECTED_PREFIXES) and Path(path).name not in ALLOWED_NAMES
    )


def main() -> int:
    repo = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    violations = find_violations(repo)
    for path in violations:
        print(f"license-check: tracked without license clearance: {path}", file=sys.stderr)
    if violations:
        return 1
    print("license-check: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
