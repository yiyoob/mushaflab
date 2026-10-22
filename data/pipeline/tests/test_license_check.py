import hashlib
import json
import subprocess
from pathlib import Path

import pytest

from pipeline.license_check import LicenseError, find_violations, redistributable_groups

LICENSES = """# Source licenses

| Group | Redistribution allowed | Basis | Terms seen |
|---|---|---|---|
| `open` | yes | owner-declared | - |
| `closed` | no | - | - |
"""
FONT = b"font bytes"


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def write_manifest(repo: Path, sources: list[dict]) -> None:
    rows = [
        {
            "id": s["id"],
            "group": s["group"],
            "role": "font-candidate",
            "acquire": "http",
            "url": "https://example.org/f",
            "file": f"{s['group']}/{s['id']}.otf",
            "sha256": s.get("sha256", hashlib.sha256(FONT).hexdigest()),
            "size": len(FONT),
            "pinned_on": "2026-10-22",
            "note": "",
        }
        for s in sources
    ]
    data = {"schema": 1, "allowed_domains": ["example.org"], "sources": rows}
    (repo / "data/sources/MANIFEST.json").write_text(json.dumps(data), encoding="utf-8")


def write_fonts(repo: Path, fonts: list[dict]) -> None:
    (repo / "fonts/FONTS.json").write_text(json.dumps({"fonts": fonts}), encoding="utf-8")
    git(repo, "add", "-f", "fonts/FONTS.json")


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    git(tmp_path, "init", "-q")
    for d in ("fonts", "data/sources/raw", "data/spine"):
        (tmp_path / d).mkdir(parents=True)
        (tmp_path / d / ".gitkeep").touch()
    (tmp_path / "data/sources/LICENSES.md").write_text(LICENSES, encoding="utf-8")
    write_manifest(
        tmp_path,
        [{"id": "open-font", "group": "open"}, {"id": "closed-font", "group": "closed"}],
    )
    git(tmp_path, "add", "-A")
    return tmp_path


def stage(repo: Path, rel: str, data: bytes = b"x") -> None:
    (repo / rel).parent.mkdir(parents=True, exist_ok=True)
    (repo / rel).write_bytes(data)
    git(repo, "add", "-f", rel)


def test_clean_repo_has_no_violations(repo: Path) -> None:
    assert find_violations(repo) == []


def test_unlisted_font_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/SomeFont.ttf")
    assert find_violations(repo) == ["fonts/SomeFont.ttf: not listed in fonts/FONTS.json"]


def test_listed_font_from_a_redistributable_group_passes(repo: Path) -> None:
    stage(repo, "fonts/open/f.otf", FONT)
    write_fonts(repo, [{"path": "fonts/open/f.otf", "source": "open-font"}])
    assert find_violations(repo) == []


def test_font_whose_group_forbids_redistribution_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/closed/f.otf", FONT)
    write_fonts(repo, [{"path": "fonts/closed/f.otf", "source": "closed-font"}])
    assert find_violations(repo) == [
        "fonts/closed/f.otf: group 'closed' does not allow redistribution (LICENSES.md)"
    ]


def test_font_that_differs_from_its_pinned_source_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/open/f.otf", b"changed bytes")
    write_fonts(repo, [{"path": "fonts/open/f.otf", "source": "open-font"}])
    assert find_violations(repo) == [
        "fonts/open/f.otf: sha256 differs from pinned source 'open-font'"
    ]


def test_font_listed_with_an_unknown_source_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/open/f.otf", FONT)
    write_fonts(repo, [{"path": "fonts/open/f.otf", "source": "nope"}])
    assert find_violations(repo) == ["fonts/open/f.otf: unknown source 'nope'"]


def test_listed_but_untracked_font_is_a_violation(repo: Path) -> None:
    write_fonts(repo, [{"path": "fonts/open/f.otf", "source": "open-font"}])
    assert find_violations(repo) == ["fonts/open/f.otf: listed in fonts/FONTS.json, not tracked"]


def test_tracked_raw_source_is_always_a_violation(repo: Path) -> None:
    stage(repo, "data/sources/raw/hafs.json")
    assert find_violations(repo) == ["data/sources/raw/hafs.json: raw sources are never tracked"]


def test_nested_raw_source_is_a_violation(repo: Path) -> None:
    stage(repo, "data/sources/raw/kfgqpc/warsh.json")
    assert find_violations(repo) == [
        "data/sources/raw/kfgqpc/warsh.json: raw sources are never tracked"
    ]


def test_other_paths_are_not_checked(repo: Path) -> None:
    stage(repo, "data/spine/x.jsonl")
    assert find_violations(repo) == []


def test_violations_are_sorted(repo: Path) -> None:
    stage(repo, "fonts/b.ttf")
    stage(repo, "data/sources/raw/a.txt")
    assert find_violations(repo) == [
        "data/sources/raw/a.txt: raw sources are never tracked",
        "fonts/b.ttf: not listed in fonts/FONTS.json",
    ]


def test_licenses_table_is_parsed_strictly(tmp_path: Path) -> None:
    path = tmp_path / "LICENSES.md"
    path.write_text(LICENSES, encoding="utf-8")
    assert redistributable_groups(path) == {"open": True, "closed": False}
    path.write_text(LICENSES.replace("| no |", "| maybe |"), encoding="utf-8")
    with pytest.raises(LicenseError):
        redistributable_groups(path)


def test_the_repository_itself_is_clean() -> None:
    assert find_violations(Path(__file__).resolve().parents[3]) == []


def derived_record(repo: Path, data: bytes, source: str = "open-font") -> None:
    rec = {
        "derived_from": {"source": source, "sha256": hashlib.sha256(FONT).hexdigest()},
        "sha256": hashlib.sha256(data).hexdigest(),
    }
    (repo / "fonts/open").mkdir(parents=True, exist_ok=True)
    (repo / "fonts/open/d.json").write_text(json.dumps(rec), encoding="utf-8")
    git(repo, "add", "-f", "fonts/open/d.json")


def test_a_derived_font_with_its_record_passes(repo: Path) -> None:
    stage(repo, "fonts/open/d.otf", b"patched")
    derived_record(repo, b"patched")
    write_fonts(
        repo,
        [{"path": "fonts/open/d.otf", "derived_from": "open-font", "record": "fonts/open/d.json"}],
    )
    assert find_violations(repo) == []


def test_a_derived_font_that_differs_from_its_record_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/open/d.otf", b"tampered")
    derived_record(repo, b"patched")
    write_fonts(
        repo,
        [{"path": "fonts/open/d.otf", "derived_from": "open-font", "record": "fonts/open/d.json"}],
    )
    assert find_violations(repo) == ["fonts/open/d.otf: sha256 differs from its build record"]


def test_a_derived_font_from_a_closed_group_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/open/d.otf", b"patched")
    derived_record(repo, b"patched", source="closed-font")
    write_fonts(
        repo,
        [
            {
                "path": "fonts/open/d.otf",
                "derived_from": "closed-font",
                "record": "fonts/open/d.json",
            }
        ],
    )
    assert find_violations(repo) == [
        "fonts/open/d.otf: group 'closed' does not allow redistribution (LICENSES.md)"
    ]


def test_a_record_for_another_source_version_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/open/d.otf", b"patched")
    derived_record(repo, b"patched")
    rec = json.loads((repo / "fonts/open/d.json").read_text())
    rec["derived_from"]["sha256"] = "0" * 64
    (repo / "fonts/open/d.json").write_text(json.dumps(rec))
    write_fonts(
        repo,
        [{"path": "fonts/open/d.otf", "derived_from": "open-font", "record": "fonts/open/d.json"}],
    )
    assert find_violations(repo) == [
        "fonts/open/d.otf: built from another version of 'open-font' than the pinned one"
    ]
