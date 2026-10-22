import base64
import hashlib
import io
import json
import stat
from collections.abc import Callable
from pathlib import Path

import pytest

from pipeline.fetch import FetchError, fetch_missing, pin, verify
from pipeline.manifest import Manifest, load_manifest

PAYLOAD = b"quran text bytes"
SHA256 = hashlib.sha256(PAYLOAD).hexdigest()
SHA1_B32 = base64.b32encode(hashlib.sha1(PAYLOAD).digest()).decode()  # noqa: S324


def make_manifest(tmp_path: Path, **overrides: object) -> Manifest:
    src = {
        "id": "s1",
        "group": "g",
        "role": "primary",
        "acquire": "http",
        "url": "https://example.org/f.txt",
        "file": "g/f.txt",
        "sha256": None,
        "size": None,
        "pinned_on": None,
        "note": "",
    }
    src.update(overrides)
    path = tmp_path / "MANIFEST.json"
    path.write_text(
        json.dumps({"schema": 1, "allowed_domains": ["example.org"], "sources": [src]}),
        encoding="utf-8",
    )
    return load_manifest(path)


def opener_for(data: bytes) -> Callable[[str], io.BytesIO]:
    calls: list[str] = []

    def opener(url: str) -> io.BytesIO:
        calls.append(url)
        return io.BytesIO(data)

    opener.calls = calls  # type: ignore[attr-defined]
    return opener


@pytest.fixture
def raw(tmp_path: Path) -> Path:
    root = tmp_path / "raw"
    root.mkdir()
    return root


def test_fetch_downloads_missing_file_read_only(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path)
    assert fetch_missing(m, raw, opener_for(PAYLOAD)) == ["s1"]
    target = raw / "g/f.txt"
    assert target.read_bytes() == PAYLOAD
    assert not target.stat().st_mode & stat.S_IWUSR
    assert not list(raw.rglob("*.part"))


def test_fetch_skips_present_files(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path)
    (raw / "g").mkdir()
    (raw / "g/f.txt").write_bytes(PAYLOAD)
    opener = opener_for(b"other")
    assert fetch_missing(m, raw, opener) == []
    assert opener.calls == []  # type: ignore[attr-defined]


def test_fetch_never_downloads_manual_sources(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, acquire="manual")
    opener = opener_for(PAYLOAD)
    assert fetch_missing(m, raw, opener) == []
    assert opener.calls == []  # type: ignore[attr-defined]


def test_fetch_rejects_pinned_hash_mismatch(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, sha256=SHA256, size=len(PAYLOAD))
    with pytest.raises(FetchError, match="sha256"):
        fetch_missing(m, raw, opener_for(b"tampered"))
    assert not (raw / "g/f.txt").exists()
    assert not list(raw.rglob("*.part"))


def test_fetch_checks_archive_sha1_digest(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, expected_sha1_b32=SHA1_B32)
    assert fetch_missing(m, raw, opener_for(PAYLOAD)) == ["s1"]


def test_fetch_rejects_archive_sha1_mismatch(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, expected_sha1_b32=SHA1_B32)
    with pytest.raises(FetchError, match="sha1"):
        fetch_missing(m, raw, opener_for(b"tampered"))
    assert not (raw / "g/f.txt").exists()


def test_verify_reports_unpinned(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path)
    fetch_missing(m, raw, opener_for(PAYLOAD))
    assert [p.kind for p in verify(m, raw)] == ["unpinned"]


def test_verify_passes_when_pinned_and_matching(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, sha256=SHA256, size=len(PAYLOAD))
    fetch_missing(m, raw, opener_for(PAYLOAD))
    assert verify(m, raw) == []


def test_verify_detects_tampering(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, sha256=SHA256, size=len(PAYLOAD))
    (raw / "g").mkdir()
    (raw / "g/f.txt").write_bytes(b"quran text byteZ")
    assert [p.kind for p in verify(m, raw)] == ["sha256"]


def test_verify_detects_size_change(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, sha256=SHA256, size=len(PAYLOAD))
    (raw / "g").mkdir()
    (raw / "g/f.txt").write_bytes(PAYLOAD + b"!")
    kinds = {p.kind for p in verify(m, raw)}
    assert kinds == {"sha256", "size"}


def test_verify_reports_missing_manual_with_url_and_path(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, acquire="manual")
    problems = verify(m, raw)
    assert [p.kind for p in problems] == ["missing"]
    assert "https://example.org/f.txt" in problems[0].message
    assert str(raw / "g/f.txt") in problems[0].message


def test_pin_records_hash_size_and_date(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path)
    fetch_missing(m, raw, opener_for(PAYLOAD))
    assert pin(m, raw, today="2026-10-21") == ["s1"]
    s = m.sources[0]
    assert (s.sha256, s.size, s.pinned_on) == (SHA256, len(PAYLOAD), "2026-10-21")
    assert verify(m, raw) == []


def test_pin_is_idempotent_and_never_overwrites(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path, sha256=SHA256, size=len(PAYLOAD), pinned_on="2026-01-01")
    (raw / "g").mkdir()
    (raw / "g/f.txt").write_bytes(b"different bytes")
    assert pin(m, raw, today="2026-10-21") == []
    assert m.sources[0].sha256 == SHA256
    assert m.sources[0].pinned_on == "2026-01-01"


def test_pin_skips_missing_files(tmp_path: Path, raw: Path) -> None:
    m = make_manifest(tmp_path)
    assert pin(m, raw, today="2026-10-21") == []
    assert m.sources[0].sha256 is None
