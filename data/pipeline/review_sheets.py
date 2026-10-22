"""T4.8: HTML review sheets for the owner, one per riwayah (build/review/phase4/<r>.html).

    python -m pipeline.review_sheets [RIWAYAH...]     # default: all 7

Each sheet is self-contained (the pinned KFGQPC Hafs font and the riwayah's own font are
embedded) and shows what the data says, never what it should say:
  * the patch counts and the D21 note (every patch `unclassified` until rows are confirmed);
  * the plan's expected cases, labelled as hypotheses (rule 1), with what the data shows;
  * frequent patterns: identical cluster changes seen at least FREQUENT times, folded into one
    row each with their count and locations;
  * per surah: every other change in context (Hafs word, riwayah word, ops), inserted and
    deleted words and hizb signs, and the ayah boundaries that differ from Hafs.
All text shown is copied from the pinned sources. The sheets are build output, not tracked.
"""

import base64
import html
import sys
from collections import Counter, defaultdict
from pathlib import Path

from pipeline import ayah_maps, classify, emit_patches, kfgqpc
from pipeline.align import Pair
from pipeline.differ import Change
from pipeline.phase4_report import BUCKETS, bucket

OUT_DIR = Path(__file__).resolve().parents[2] / "build" / "review" / "phase4"
FREQUENT = 5
SHOWN_REFS = 40

CSS = """
:root { --fg: #1d1d1f; --bg: #fff; --muted: #6e6e73; --line: #e5e5ea; --hl: #fff4ce; }
@media (prefers-color-scheme: dark) {
  :root { --fg: #f2f2f7; --bg: #1c1c1e; --muted: #a1a1a6; --line: #3a3a3c; --hl: #4a3f12; }
}
body { font: 14px/1.45 -apple-system, system-ui, sans-serif; color: var(--fg);
  background: var(--bg); margin: 0 auto; max-width: 1200px; padding: 16px; }
h1 { font-size: 22px; } h2 { font-size: 18px; margin-top: 28px; }
table { border-collapse: collapse; width: 100%; margin: 8px 0; }
th, td { border-bottom: 1px solid var(--line); padding: 4px 8px; text-align: left;
  vertical-align: top; }
th { color: var(--muted); font-weight: 600; }
code { font: 12px ui-monospace, Menlo, monospace; }
.h { font-family: kfgqpc-hafs; font-size: 26px; direction: rtl; unicode-bidi: isolate; }
.r { font-family: kfgqpc-riwayah; font-size: 26px; direction: rtl; unicode-bidi: isolate; }
.muted { color: var(--muted); }
.hyp { background: var(--hl); padding: 8px 12px; border-radius: 6px; }
details { margin: 4px 0; } summary { cursor: pointer; }
"""


def _font_face(name: str, data: bytes) -> str:
    b64 = base64.b64encode(data).decode("ascii")
    return f"@font-face {{ font-family: {name}; src: url(data:font/ttf;base64,{b64}); }}"


def _cps(text: str) -> str:
    return " ".join(f"{ord(c):04X}" for c in text)


def _ar(text: str, cls: str) -> str:
    return f'<span class="{cls}">{html.escape(text)}</span>' if text else ""


def _change_cell(c: Change) -> str:
    old = f"{_ar(c.old, 'h')} <code>{_cps(c.old)}</code>" if c.old else "-"
    new = f"{_ar(c.new, 'r')} <code>{_cps(c.new)}</code>" if c.new else "-"
    return f"<code>{c.op} {html.escape(c.target)}</code> ({bucket(c)}): {old} &rarr; {new}"


def pattern(c: Change) -> tuple[str, str, str] | None:
    """The key a change folds under when frequent; word-level changes never fold."""
    b = bucket(c)
    return None if b in ("word", "hizb", "ayah-number") else (b, c.old, c.new)


def _word_key(c: Change) -> str:
    """The Hafs item a change belongs to (an inserted item: its riwayah ref)."""
    if c.kind == "item-insert":
        return f"ins:{c.ref}"
    return c.target if c.level == "item" else c.target.rsplit(":", 1)[0]


def _hyp_section(riwayah: str) -> list[str]:
    rows = [h for h in ayah_maps.HYPOTHESES if h[0] == riwayah]
    if not rows:
        return []
    out = [
        "<h2>Expected cases from the plan (hypotheses)</h2>",
        '<div class="hyp">These statements come from PROJECT_PLAN without a cited source. They'
        " are <b>hypotheses</b>, not tests (CLAUDE.md rule 1). The rows show what the pinned data"
        " says; the owner confirms or rejects each against a cited source.</div>",
        "<ul>",
    ]
    for line in ayah_maps.hypothesis_rows()[6:]:
        if line.startswith("| PROJECT_PLAN") and riwayah in line:
            text, data = line.strip("| ").split(" | ", 1)
            out.append(f"<li><b>{html.escape(text)}</b><br>{html.escape(data.rstrip(' |'))}</li>")
    return [*out, "</ul>"]


def sheet(riwayah: str) -> str:
    pairs, changes = emit_patches.derive(riwayah)
    hafs_word = {p.h.ref: p for p in pairs if p.h}
    riw_word = {p.r.ref: p for p in pairs if p.r}
    counts = Counter(pattern(c) for c in changes if pattern(c))
    frequent = {k for k, n in counts.items() if n >= FREQUENT}
    findings = ayah_maps.findings(riwayah)
    confirmed = classify.load_confirmed()[riwayah]
    tags = Counter((t.kind, t.rule) for t in (classify.classify(c, confirmed) for c in changes))
    head = [
        "<!doctype html><html lang=en><head><meta charset=utf-8>",
        '<meta name=viewport content="width=device-width, initial-scale=1">',
        f"<title>{riwayah} review</title><style>",
        _font_face("kfgqpc-hafs", kfgqpc.font_bytes("hafs")),
        _font_face("kfgqpc-riwayah", kfgqpc.font_bytes(riwayah)),
        CSS,
        "</style></head><body>",
        f"<h1>Phase 4 review sheet: {riwayah} (raw patches on the Hafs spine)</h1>",
        f"<p class=muted>Source {html.escape(emit_patches.source_ref(riwayah))}. Every patch is"
        " <code>raw-derived</code> and <code>single_source</code>; the text they produce is"
        " checked codepoint for codepoint by <code>just roundtrip</code>. Hafs words use the"
        " KFGQPC Hafs font, riwayah words the KFGQPC font of this riwayah.</p>",
        "<p><b>Difference kinds (D21):</b> "
        + ", ".join(f"{k} ({rule}): {n}" for (k, rule), n in sorted(tags.items()))
        + ". Only owner-confirmed rows (<code>data/approvals/difference_kinds.yaml</code>) tag a"
        " patch as anything but <code>unclassified</code>. The buckets below describe the"
        " edit, not its cause.</p>",
    ]
    by_bucket = Counter(bucket(c) for c in changes)
    head += ["<table><tr><th>Patches</th>", *(f"<th>{b}</th>" for b in BUCKETS), "</tr>"]
    head += [f"<tr><td>{len(changes)}</td>", *(f"<td>{by_bucket[b]}</td>" for b in BUCKETS)]
    head += ["</tr></table>", *_hyp_section(riwayah)]

    body = [
        f"<h2>Frequent patterns (seen {FREQUENT} times or more)</h2>",
        "<p class=muted>Identical cluster changes folded into one row. Their locations are"
        " listed; they are not repeated in the surah sections.</p>",
        "<table><tr><th>Bucket</th><th>Hafs</th><th>" + riwayah + "</th><th>Count</th>"
        "<th>Where (Hafs ids)</th></tr>",
    ]
    where: dict[tuple, list[str]] = defaultdict(list)
    for c in changes:
        k = pattern(c)
        if k in frequent:
            where[k].append(c.target)
    for k, n in sorted(((k, counts[k]) for k in frequent), key=lambda kv: (-kv[1], kv[0])):
        b, old, new = k
        refs = where[k]
        shown = ", ".join(refs[:SHOWN_REFS]) + (
            f" (+{len(refs) - SHOWN_REFS})" if len(refs) > SHOWN_REFS else ""
        )
        body.append(
            f"<tr><td>{b}</td><td>{_ar(old, 'h')} <code>{_cps(old)}</code></td>"
            f"<td>{_ar(new, 'r')} <code>{_cps(new)}</code></td><td>{n}</td>"
            f"<td><details><summary>{len(refs)} places</summary><code>{shown}</code>"
            "</details></td></tr>"
        )
    body.append("</table>")

    per_surah: dict[int, dict[str, list[Change]]] = defaultdict(lambda: defaultdict(list))
    for c in changes:
        if pattern(c) in frequent or bucket(c) == "ayah-number":
            continue
        per_surah[c.surah][_word_key(c)].append(c)
    boundaries: dict[int, list[str]] = defaultdict(list)
    for label, rows in (
        ("Hafs only", findings["only_hafs"]),
        (f"{riwayah} only", findings["only_riw"]),
    ):
        for h, r in rows:
            boundaries[int(h.split(":")[0])].append(
                f"ayah ends after Hafs {h} / {riwayah} {r}: {label}"
            )
    body.append("<h2>By surah</h2>")
    for s in sorted(set(per_surah) | set(boundaries)):
        words = per_surah.get(s, {})
        n = sum(len(v) for v in words.values())
        body.append(
            f"<details><summary>Surah {s}: {n} changes in {len(words)} words"
            f"{', ayah boundaries differ' if boundaries.get(s) else ''}</summary>"
        )
        if boundaries.get(s):
            body.append(
                "<ul>" + "".join(f"<li>{html.escape(b)}</li>" for b in boundaries[s]) + "</ul>"
            )
        body.append(f"<table><tr><th>Hafs</th><th>{riwayah}</th><th>Changes</th></tr>")
        for key, cs in words.items():
            p: Pair | None = riw_word.get(key[4:]) if key.startswith("ins:") else hafs_word.get(key)
            h = f"<code>{p.h.ref}</code> {_ar(p.h.text, 'h')}" if p and p.h else "-"
            r = f"<code>{p.r.ref}</code> {_ar(p.r.text, 'r')}" if p and p.r else "-"
            cells = "<br>".join(_change_cell(c) for c in cs)
            body.append(f"<tr><td>{h}</td><td>{r}</td><td>{cells}</td></tr>")
        body.append("</table></details>")
    return "\n".join([*head, *body, "</body></html>"]) + "\n"


def main(argv: list[str]) -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for r in argv[1:] or emit_patches.ORDER:
        path = OUT_DIR / f"{r}.html"
        path.write_text(sheet(r), encoding="utf-8")
        print(f"review sheet written: {path} ({path.stat().st_size // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
