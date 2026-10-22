"""Load, validate and write data/sources/MANIFEST.json.

The manifest pins every third-party source by URL and SHA-256 (CLAUDE.md rules 2 and 9).
Network access is only allowed to hosts in `allowed_domains`.
"""

import json
import re
from dataclasses import asdict, dataclass, fields
from pathlib import Path, PurePosixPath
from urllib.parse import urlsplit

SCHEMA = 1
ACQUIRE_KINDS = ("http", "manual")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SHA1_B32_RE = re.compile(r"^[A-Z2-7]{32}$")


class ManifestError(ValueError):
    pass


@dataclass
class Source:
    id: str
    group: str
    role: str
    acquire: str
    url: str
    file: str
    sha256: str | None
    size: int | None
    pinned_on: str | None
    note: str
    # Archive.org CDX content digest (base32 SHA-1), when the source is an archived copy.
    expected_sha1_b32: str | None = None


@dataclass
class Manifest:
    schema: int
    allowed_domains: list[str]
    sources: list[Source]
    path: Path | None = None


SOURCE_FIELDS = {f.name for f in fields(Source)}
REQUIRED_FIELDS = SOURCE_FIELDS - {"expected_sha1_b32"}


def _check_file_path(sid: str, file: str) -> None:
    p = PurePosixPath(file)
    parts = file.split("/")
    if (
        not file
        or p.is_absolute()
        or any(part in ("", ".", "..") for part in parts)
        or "\\" in file
    ):
        raise ManifestError(f"{sid}: unsafe file path {file!r}")


def _check_url(sid: str, url: str, allowed: list[str]) -> None:
    parts = urlsplit(url)
    if parts.scheme != "https":
        raise ManifestError(f"{sid}: url must be https: {url}")
    if parts.hostname not in allowed:
        raise ManifestError(f"{sid}: host {parts.hostname!r} not in allowed_domains")


def _parse_source(raw: dict, allowed: list[str]) -> Source:
    sid = raw.get("id", "<no id>")
    unknown = set(raw) - SOURCE_FIELDS
    if unknown:
        raise ManifestError(f"{sid}: unknown field(s) {sorted(unknown)}")
    missing = REQUIRED_FIELDS - set(raw)
    if missing:
        raise ManifestError(f"{sid}: missing field(s) {sorted(missing)}")
    src = Source(**raw)
    if src.acquire not in ACQUIRE_KINDS:
        raise ManifestError(f"{sid}: acquire must be one of {ACQUIRE_KINDS}")
    _check_url(sid, src.url, allowed)
    _check_file_path(sid, src.file)
    if src.sha256 is not None and not SHA256_RE.match(src.sha256):
        raise ManifestError(f"{sid}: malformed sha256")
    if src.expected_sha1_b32 is not None and not SHA1_B32_RE.match(src.expected_sha1_b32):
        raise ManifestError(f"{sid}: malformed expected_sha1_b32")
    return src


def load_manifest(path: Path) -> Manifest:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema") != SCHEMA:
        raise ManifestError(f"unsupported schema {data.get('schema')!r}")
    allowed = data["allowed_domains"]
    sources = [_parse_source(s, allowed) for s in data["sources"]]
    for attr in ("id", "file"):
        seen: set[str] = set()
        for s in sources:
            value = getattr(s, attr)
            if value in seen:
                raise ManifestError(f"duplicate {attr} {value!r}")
            seen.add(value)
    return Manifest(schema=SCHEMA, allowed_domains=allowed, sources=sources, path=path)


def dump_manifest(manifest: Manifest, path: Path) -> None:
    sources = []
    for s in manifest.sources:
        d = asdict(s)
        if d["expected_sha1_b32"] is None:
            del d["expected_sha1_b32"]
        sources.append(d)
    data = {
        "schema": manifest.schema,
        "allowed_domains": manifest.allowed_domains,
        "sources": sources,
    }
    text = json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    path.write_text(text, encoding="utf-8")
