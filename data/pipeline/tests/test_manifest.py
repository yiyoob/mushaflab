import json
from pathlib import Path

import pytest

from pipeline.manifest import ManifestError, dump_manifest, load_manifest


def source(**overrides: object) -> dict:
    base = {
        "id": "tanzil-uthmani",
        "group": "tanzil",
        "role": "cross-check",
        "acquire": "http",
        "url": "https://tanzil.net/x.txt",
        "file": "tanzil/uthmani.txt",
        "sha256": None,
        "size": None,
        "pinned_on": None,
        "note": "",
    }
    base.update(overrides)
    return base


def write(tmp_path: Path, sources: list[dict], domains: tuple[str, ...] = ("tanzil.net",)) -> Path:
    path = tmp_path / "MANIFEST.json"
    path.write_text(
        json.dumps({"schema": 1, "allowed_domains": list(domains), "sources": sources}),
        encoding="utf-8",
    )
    return path


def test_loads_valid_manifest(tmp_path: Path) -> None:
    m = load_manifest(write(tmp_path, [source()]))
    assert [s.id for s in m.sources] == ["tanzil-uthmani"]
    assert m.sources[0].sha256 is None


def test_rejects_host_outside_allowlist(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="not in allowed_domains"):
        load_manifest(write(tmp_path, [source(url="https://evil.example/x.txt")]))


def test_rejects_subdomain_trick(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="not in allowed_domains"):
        load_manifest(write(tmp_path, [source(url="https://tanzil.net.evil.example/x")]))


def test_rejects_plain_http(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="https"):
        load_manifest(write(tmp_path, [source(url="http://tanzil.net/x.txt")]))


@pytest.mark.parametrize("bad", ["../x.txt", "/etc/passwd", "a/../../x", "", "a//b", "a/./b"])
def test_rejects_unsafe_file_paths(tmp_path: Path, bad: str) -> None:
    with pytest.raises(ManifestError, match="file"):
        load_manifest(write(tmp_path, [source(file=bad)]))


def test_rejects_duplicate_ids(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="duplicate id"):
        load_manifest(write(tmp_path, [source(), source(file="tanzil/other.txt")]))


def test_rejects_duplicate_files(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="duplicate file"):
        load_manifest(write(tmp_path, [source(), source(id="other")]))


def test_rejects_unknown_acquire(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="acquire"):
        load_manifest(write(tmp_path, [source(acquire="ftp")]))


def test_rejects_unknown_fields(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="unknown field"):
        load_manifest(write(tmp_path, [source(sha265="typo")]))


def test_rejects_malformed_sha256(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="sha256"):
        load_manifest(write(tmp_path, [source(sha256="abc")]))


def test_round_trip_is_stable(tmp_path: Path) -> None:
    path = write(tmp_path, [source(sha256="a" * 64, size=3, pinned_on="2026-10-21")])
    m = load_manifest(path)
    dump_manifest(m, path)
    first = path.read_bytes()
    dump_manifest(load_manifest(path), path)
    assert path.read_bytes() == first
    assert first.endswith(b"\n")
