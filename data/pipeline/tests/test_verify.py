"""T1.5: integrity verification and the page-boundary golden file."""

from pipeline import verify
from pipeline.tests.markers import needs_sources


def test_failures_are_reported_not_raised() -> None:
    def ok() -> list[str]:
        return []

    def bad() -> list[str]:
        return ["x"]

    assert verify.run_checks({"ok": ok, "bad": bad}) == {"bad": ["x"]}


@needs_sources
def test_all_integrity_checks_pass() -> None:
    assert verify.run_checks(verify.CHECKS) == {}


@needs_sources
def test_page_boundaries_match_golden() -> None:
    assert verify.page_boundaries() == verify.load_golden()
