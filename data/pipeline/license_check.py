"""Fail if third-party data is tracked in git before its license allows it.

CLAUDE.md rules 2 and 10:
  * raw sources (data/sources/raw/) are never tracked;
  * a file under fonts/ may be tracked only if fonts/FONTS.json lists it with the id of the
    pinned source it is a copy of, its bytes have that source's sha256 (MANIFEST.json), and
    that source's group row in data/sources/LICENSES.md says redistribution is allowed.
  * a font taken from a member of a pinned zip (T5.3: the KFGQPC Warsh and Qalun fonts) is
    listed with that `member` and its own `sha256`: its bytes must have that sha256 and, when
    the pinned zip is present (it is untracked), equal the member's bytes.
"""

import hashlib
import json
import re
import subprocess
import sys
import zipfile
from pathlib import Path

from pipeline.manifest import Source, load_manifest

RAW_PREFIX = "data/sources/raw/"
FONTS_PREFIX = "fonts/"
FONTS_INDEX = "fonts/FONTS.json"
ALLOWED_NAMES = (".gitkeep",)
LICENSE_ROW = re.compile(r"^\|\s*`([^`]+)`\s*\|\s*([^|]*?)\s*\|")
ALLOWED = {"yes": True, "no": False}


class LicenseError(ValueError):
    pass


def tracked_files(repo: Path) -> list[str]:
    out = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z", "--cached"],
        check=True,
        capture_output=True,
    ).stdout
    return [p for p in out.decode("utf-8").split("\0") if p]


def redistributable_groups(licenses: Path) -> dict[str, bool]:
    """Group -> redistribution allowed, from the LICENSES.md table (only `yes` or `no`)."""
    groups: dict[str, bool] = {}
    for line in licenses.read_text(encoding="utf-8").splitlines():
        m = LICENSE_ROW.match(line)
        if m is None:
            continue
        group, allowed = m.groups()
        if allowed not in ALLOWED:
            raise LicenseError(f"LICENSES.md: group {group!r}: unreadable value {allowed!r}")
        if group in groups:
            raise LicenseError(f"LICENSES.md: group {group!r} listed twice")
        groups[group] = ALLOWED[allowed]
    return groups


def font_violations(repo: Path, fonts: list[str]) -> list[str]:
    index = repo / FONTS_INDEX
    listed = json.loads(index.read_text(encoding="utf-8"))["fonts"] if index.exists() else []
    by_path = {f["path"]: f["source"] for f in listed}
    members = {f["path"]: (f["member"], f["sha256"]) for f in listed if "member" in f}
    if len(by_path) != len(listed):
        return [f"{FONTS_INDEX}: a path is listed twice"]
    groups = redistributable_groups(repo / "data/sources/LICENSES.md")
    sources = {s.id: s for s in load_manifest(repo / "data/sources/MANIFEST.json").sources}
    out = []
    for path in fonts:
        sid = by_path.get(path)
        if sid is None:
            out.append(f"{path}: not listed in {FONTS_INDEX}")
            continue
        src = sources.get(sid)
        if src is None:
            out.append(f"{path}: unknown source {sid!r}")
        elif not groups.get(src.group, False):
            out.append(f"{path}: group {src.group!r} does not allow redistribution (LICENSES.md)")
        elif path in members:
            out.extend(_member_violations(repo, path, src, members[path]))
        elif hashlib.sha256((repo / path).read_bytes()).hexdigest() != src.sha256:
            out.append(f"{path}: sha256 differs from pinned source {sid!r}")
    out.extend(
        f"{path}: listed in {FONTS_INDEX}, not tracked" for path in by_path if path not in fonts
    )
    return out


def _member_violations(repo: Path, path: str, src: Source, entry: tuple[str, str]) -> list[str]:
    member, sha = entry
    sid, raw_file = src.id, src.file
    data = (repo / path).read_bytes()
    if hashlib.sha256(data).hexdigest() != sha:
        return [f"{path}: sha256 differs from FONTS.json"]
    raw = repo / RAW_PREFIX / raw_file
    if raw.exists():
        with zipfile.ZipFile(raw) as zf:
            if member not in zf.namelist() or zf.read(member) != data:
                return [f"{path}: differs from member {member} of {sid!r}"]
    return []


def find_violations(repo: Path) -> list[str]:
    tracked = tracked_files(repo)
    raw = [
        f"{p}: raw sources are never tracked"
        for p in tracked
        if p.startswith(RAW_PREFIX) and Path(p).name not in ALLOWED_NAMES
    ]
    fonts = [
        p
        for p in tracked
        if p.startswith(FONTS_PREFIX) and p != FONTS_INDEX and Path(p).name not in ALLOWED_NAMES
    ]
    return sorted(raw + font_violations(repo, fonts))


def main() -> int:
    repo = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    violations = find_violations(repo)
    for v in violations:
        print(f"license-check: {v}", file=sys.stderr)
    if violations:
        return 1
    print("license-check: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
