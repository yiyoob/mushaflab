import subprocess
from pathlib import Path

import pytest

from pipeline.license_check import find_violations


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    git(tmp_path, "init", "-q")
    for d in ("fonts", "data/sources/raw", "data/spine"):
        (tmp_path / d).mkdir(parents=True)
        (tmp_path / d / ".gitkeep").touch()
    git(tmp_path, "add", "-A")
    return tmp_path


def stage(repo: Path, rel: str) -> None:
    (repo / rel).write_bytes(b"x")
    git(repo, "add", "-f", rel)


def test_clean_repo_has_no_violations(repo: Path) -> None:
    assert find_violations(repo) == []


def test_tracked_font_is_a_violation(repo: Path) -> None:
    stage(repo, "fonts/SomeFont.ttf")
    assert find_violations(repo) == ["fonts/SomeFont.ttf"]


def test_tracked_raw_source_is_a_violation(repo: Path) -> None:
    stage(repo, "data/sources/raw/hafs.json")
    assert find_violations(repo) == ["data/sources/raw/hafs.json"]


def test_nested_raw_source_is_a_violation(repo: Path) -> None:
    (repo / "data/sources/raw/kfgqpc").mkdir()
    stage(repo, "data/sources/raw/kfgqpc/warsh.json")
    assert find_violations(repo) == ["data/sources/raw/kfgqpc/warsh.json"]


def test_other_paths_are_not_checked(repo: Path) -> None:
    stage(repo, "data/spine/x.jsonl")
    assert find_violations(repo) == []


def test_violations_are_sorted(repo: Path) -> None:
    stage(repo, "fonts/b.ttf")
    stage(repo, "data/sources/raw/a.txt")
    assert find_violations(repo) == ["data/sources/raw/a.txt", "fonts/b.ttf"]
