"""Shared pytest markers.

Tests that read pinned sources skip locally when they are not fetched, but never in CI:
there MUSHAF_REQUIRE_SOURCES=1 makes them run (and fail loudly if sources are missing).
"""

import os

import pytest

from pipeline import sources

needs_sources = pytest.mark.skipif(
    not os.environ.get("MUSHAF_REQUIRE_SOURCES") and not sources.available(),
    reason="pinned sources not fetched (run `just data-fetch`)",
)
