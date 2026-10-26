"""Download, verify and pin the sources listed in data/sources/MANIFEST.json.

    python -m pipeline.fetch fetch    # download missing http sources, then verify
    python -m pipeline.fetch verify   # check every file against its pinned hash
    python -m pipeline.fetch pin      # record hashes for files not yet pinned

Raw files are never modified once in place (CLAUDE.md rule 2): downloads go to a
temporary file, are checked, then renamed and made read-only. A pinned hash is never
changed by this module; changing one is a manual, reviewed edit of the manifest.
"""

import base64
import datetime
import hashlib
import stat
import sys
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO

from pipeline.manifest import Manifest, Source, dump_manifest, load_manifest

DATA_DIR = Path(__file__).resolve().parent.parent
MANIFEST_PATH = DATA_DIR / "sources" / "MANIFEST.json"
RAW_ROOT = DATA_DIR / "sources" / "raw"
CHUNK = 1 << 20
USER_AGENT = "mushaflab-data-fetch/1 (+https://github.com/yiyoob/mushaflab)"

Opener = Callable[[str], BinaryIO]


class FetchError(RuntimeError):
    pass


@dataclass
class Problem:
    source_id: str
    kind: str  # missing | unpinned | sha256 | size
    message: str


def _digests(path: Path) -> tuple[str, str, int]:
    sha256 = hashlib.sha256()
    sha1 = hashlib.sha1()  # noqa: S324 - only to compare with archive.org's digest
    size = 0
    with path.open("rb") as fh:
        while chunk := fh.read(CHUNK):
            sha256.update(chunk)
            sha1.update(chunk)
            size += len(chunk)
    return sha256.hexdigest(), base64.b32encode(sha1.digest()).decode(), size


def _make_read_only(path: Path) -> None:
    mode = path.stat().st_mode
    path.chmod(mode & ~(stat.S_IWUSR | stat.S_IWGRP | stat.S_IWOTH))


def default_opener(url: str) -> BinaryIO:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})  # noqa: S310
    return urllib.request.urlopen(request, timeout=120)  # noqa: S310 - https enforced


def _download(src: Source, target: Path, opener: Opener) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    part = target.with_name(target.name + ".part")
    try:
        with opener(src.url) as response, part.open("wb") as out:
            while chunk := response.read(CHUNK):
                out.write(chunk)
        sha256, sha1_b32, size = _digests(part)
        if src.expected_sha1_b32 is not None and sha1_b32 != src.expected_sha1_b32:
            raise FetchError(
                f"{src.id}: sha1 {sha1_b32} does not match archive digest {src.expected_sha1_b32}"
            )
        if src.sha256 is not None and sha256 != src.sha256:
            raise FetchError(f"{src.id}: sha256 {sha256} does not match pinned {src.sha256}")
        if src.size is not None and size != src.size:
            raise FetchError(f"{src.id}: size {size} does not match pinned {src.size}")
        part.replace(target)
    finally:
        part.unlink(missing_ok=True)
    _make_read_only(target)


def fetch_missing(manifest: Manifest, raw_root: Path, opener: Opener = default_opener) -> list[str]:
    fetched = []
    for src in manifest.sources:
        target = raw_root / src.file
        if src.acquire != "http" or target.exists():
            continue
        print(f"fetch: {src.id} <- {src.url}", file=sys.stderr)
        _download(src, target, opener)
        fetched.append(src.id)
    return fetched


def verify(manifest: Manifest, raw_root: Path) -> list[Problem]:
    problems = []
    for src in manifest.sources:
        target = raw_root / src.file
        if not target.exists():
            if src.optional:
                continue
            if src.acquire == "manual" and not src.url:
                message = f"{src.id}: missing; place it at {target}"
            else:
                how = "download it manually from" if src.acquire == "manual" else "run fetch for"
                message = f"{src.id}: missing; {how} {src.url} into {target}"
            problems.append(Problem(src.id, "missing", message))
            continue
        if src.sha256 is None:
            problems.append(Problem(src.id, "unpinned", f"{src.id}: present but not pinned"))
            continue
        sha256, _, size = _digests(target)
        if sha256 != src.sha256:
            problems.append(
                Problem(src.id, "sha256", f"{src.id}: sha256 {sha256} != pinned {src.sha256}")
            )
        if src.size is not None and size != src.size:
            problems.append(Problem(src.id, "size", f"{src.id}: size {size} != pinned {src.size}"))
    return problems


def pin(manifest: Manifest, raw_root: Path, today: str) -> list[str]:
    pinned = []
    for src in manifest.sources:
        target = raw_root / src.file
        if src.sha256 is not None or not target.exists():
            continue
        sha256, sha1_b32, size = _digests(target)
        if src.expected_sha1_b32 is not None and sha1_b32 != src.expected_sha1_b32:
            raise FetchError(f"{src.id}: sha1 does not match archive digest; refusing to pin")
        src.sha256, src.size, src.pinned_on = sha256, size, today
        pinned.append(src.id)
    return pinned


def main(argv: list[str]) -> int:
    command = argv[1] if len(argv) > 1 else "verify"
    manifest = load_manifest(MANIFEST_PATH)
    if command == "fetch":
        fetch_missing(manifest, RAW_ROOT)
    elif command == "pin":
        ids = pin(manifest, RAW_ROOT, today=datetime.datetime.now(datetime.UTC).date().isoformat())
        dump_manifest(manifest, MANIFEST_PATH)
        for sid in ids:
            print(f"pinned: {sid}")
    elif command != "verify":
        print(f"unknown command {command!r}", file=sys.stderr)
        return 2
    problems = verify(manifest, RAW_ROOT)
    for p in problems:
        print(p.message, file=sys.stderr)
    total = len(manifest.sources)
    print(f"verify: {total - len(problems)}/{total} sources ok")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
