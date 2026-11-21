"""Skip `just data-build` when nothing it reads or writes has changed since its last success.

The stamp (`build/data-build.stamp`, untracked) holds one SHA-256 over the contents of every
file under the build's input and output trees that git tracks or would track (ignored files
such as `build/`, `core/target/` and the raw sources are excluded; the raw sources are pinned
by `data/sources/MANIFEST.json`, which is included). Outputs are part of the hash, so a
hand-edited or stale generated file also triggers a rebuild. Rust test files do not affect the
build and are left out.

    python -m pipeline.build_stamp check   # exit 0: current, skip; exit 1: build needed
    python -m pipeline.build_stamp write   # after a successful build

`FORCE=1 just data-build` ignores the stamp.
"""

import hashlib
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
STAMP = REPO / "build" / "data-build.stamp"
TREES = ("data", "core", "fonts", "docs/review", "justfile")


def _included(path: str) -> bool:
    parts = path.split("/")
    return not (parts[0] == "core" and "tests" in parts)


def digest() -> str:
    out = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *TREES],
        cwd=REPO,
        check=True,
        capture_output=True,
    ).stdout
    paths = sorted({p for p in out.decode().split("\0") if p and _included(p)})
    h = hashlib.sha256()
    for p in paths:
        f = REPO / p
        h.update(p.encode() + b"\0")
        h.update(f.read_bytes() if f.is_file() else b"<missing>")
        h.update(b"\0")
    return h.hexdigest()


def main(argv: list[str]) -> int:
    cmd = argv[0] if argv else ""
    if cmd == "check":
        current = STAMP.is_file() and STAMP.read_text().strip() == digest()
        current = current and (REPO / "build" / "mushaf.db").is_file()
        if current:
            print("data-build: inputs and outputs unchanged since the last build, skipping")
            print("            (FORCE=1 just data-build to rebuild anyway)")
        return 0 if current else 1
    if cmd == "write":
        STAMP.parent.mkdir(parents=True, exist_ok=True)
        STAMP.write_text(digest() + "\n")
        return 0
    print("usage: python -m pipeline.build_stamp check|write", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
