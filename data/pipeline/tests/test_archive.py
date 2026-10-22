import hashlib
import json
import tarfile
from pathlib import Path

import pytest

from pipeline.archive import ArchiveError, write_archive
from pipeline.manifest import load_manifest

PAYLOAD = b"source bytes"


@pytest.fixture
def setup(tmp_path: Path) -> tuple[Path, Path, Path]:
    raw = tmp_path / "raw"
    (raw / "g").mkdir(parents=True)
    (raw / "g/f.txt").write_bytes(PAYLOAD)
    manifest = tmp_path / "MANIFEST.json"
    src = {
        "id": "s1",
        "group": "g",
        "role": "primary",
        "acquire": "http",
        "url": "https://example.org/f.txt",
        "file": "g/f.txt",
        "sha256": hashlib.sha256(PAYLOAD).hexdigest(),
        "size": len(PAYLOAD),
        "pinned_on": "2026-10-21",
        "note": "",
    }
    manifest.write_text(
        json.dumps({"schema": 1, "allowed_domains": ["example.org"], "sources": [src]}),
        encoding="utf-8",
    )
    return manifest, raw, tmp_path / "backup"


def test_archive_contains_manifest_and_raw_files(setup: tuple[Path, Path, Path]) -> None:
    manifest, raw, dest = setup
    out = write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")
    assert out.parent == dest
    assert out.name.startswith("mushaflab-sources-20261021-")
    with tarfile.open(out) as tar:
        names = sorted(tar.getnames())
        assert names == ["MANIFEST.json", "raw/g/f.txt"]
        member = tar.extractfile("raw/g/f.txt")
        assert member is not None
        assert member.read() == PAYLOAD


def test_archive_writes_matching_checksum_file(setup: tuple[Path, Path, Path]) -> None:
    manifest, raw, dest = setup
    out = write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")
    line = Path(str(out) + ".sha256").read_text(encoding="utf-8")
    assert line == f"{hashlib.sha256(out.read_bytes()).hexdigest()}  {out.name}\n"


def test_archive_refuses_when_verify_fails(setup: tuple[Path, Path, Path]) -> None:
    manifest, raw, dest = setup
    (raw / "g/f.txt").write_bytes(b"tampered")
    with pytest.raises(ArchiveError, match="verify"):
        write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")
    assert not dest.exists() or not any(dest.iterdir())


def test_archive_refuses_to_overwrite(setup: tuple[Path, Path, Path]) -> None:
    manifest, raw, dest = setup
    write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")
    with pytest.raises(ArchiveError, match="exists"):
        write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")


def test_archive_is_reproducible(setup: tuple[Path, Path, Path], tmp_path: Path) -> None:
    manifest, raw, dest = setup
    a = write_archive(load_manifest(manifest), manifest, raw, dest, today="20261021")
    b = write_archive(load_manifest(manifest), manifest, raw, tmp_path / "b", today="20261021")
    assert a.read_bytes() == b.read_bytes()
