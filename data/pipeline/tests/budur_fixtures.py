"""Synthetic sentences for the al-Budur tests, from budur_sentences.yaml (no book text)."""

from pathlib import Path

import yaml

S = yaml.safe_load((Path(__file__).parent / "budur_sentences.yaml").read_text(encoding="utf-8"))
