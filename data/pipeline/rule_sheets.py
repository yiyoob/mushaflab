"""T6.5: one HTML review sheet per rule file (ADR D38, D42).

    python -m pipeline.rule_sheets [NAME...]   # build/review/phase6/rules-<name>.html

For every rule the sheet shows:
- its register entry: id, description, status, source;
- the places it applies, each word in the Hafs form and with the rule's change, in the
  riwayah's KFGQPC font;
- every exception with its reason.
Places are found with the core's matching semantics (`rule_mining.match_set`). Sheets are not
tracked (they embed fonts); `docs/review/phase6/rules.md` (tracked) summarises them.
"""

import html
import sys
from collections import Counter
from pathlib import Path

from pipeline import kfgqpc
from pipeline.compile_db import load_yaml
from pipeline.differ import simulate_set_marks
from pipeline.review_sheets import CSS, _font_face
from pipeline.rule_mining import DATA, RULES_DIR, View, match_set
from pipeline.spine import from_cps

OUT_DIR = DATA.parent / "build" / "review" / "phase6"
SUMMARY = DATA.parent / "docs" / "review" / "phase6" / "rules.md"
SHOWN = 24


def _register() -> dict[str, dict]:
    out = {}
    for p in sorted(RULES_DIR.glob("*register.yaml")):
        for e in load_yaml(p, ascii_only=False)["rules"]:
            out[e["id"]] = e
    return out


def rule_files() -> list[Path]:
    return sorted(p for p in RULES_DIR.glob("*.yaml") if not p.name.endswith("register.yaml"))


def _changed(text: str, action: dict) -> str:
    pay = action.get("payload", {})
    if action["op"] == "replace" and "cps" in pay:
        return from_cps(pay["cps"])
    if action["op"] == "set_marks":
        return simulate_set_marks(text, pay)
    if action["op"] == "insert_after":
        return text + from_cps(pay["cluster"]["cps"])
    if action["op"] == "insert_before":
        return from_cps(pay["cluster"]["cps"]) + text
    return text  # set_attr: no change to the text (annotation channel, D40)


def _word(v: View, cid: str, action: dict | None) -> str:
    i, k = v.pos[cid]
    cs = list(v.texts[i])
    if action is not None and action.get("target") in ("t", "first_ha", None):
        cs[k] = _changed(cs[k], action)
    return "".join(cs)


def sheet(path: Path, v: View, register: dict[str, dict]) -> tuple[str, list[dict]]:
    doc = load_yaml(path)
    name = path.stem
    riwayah = doc["rules"][0]["owner"]["id"] if doc["rules"] else name
    try:
        font = _font_face("kfgqpc-riwayah", kfgqpc.font_bytes(riwayah))
    except Exception:  # a riwayah without a KFGQPC font shows Hafs's
        font = _font_face("kfgqpc-riwayah", kfgqpc.font_bytes("hafs"))
    out = [
        "<!doctype html><html><head><meta charset='utf-8'>",
        "<meta name='viewport' content='width=device-width, initial-scale=1'>",
        f"<title>Rules: {html.escape(name)}</title><style>{CSS}",
        _font_face("kfgqpc-hafs", kfgqpc.font_bytes("hafs")),
        font,
        "</style></head><body>",
        f"<h1>Rules: {html.escape(name)}</h1>",
        "<p class='muted'>Each rule with its register entry, the places it applies (Hafs form, "
        "then with the rule's change) and its exceptions. A hypothesis has no citation yet "
        "(CLAUDE.md rule 1): cite, rename, merge or reject it.</p>",
    ]
    stats = []
    for r in doc["rules"]:
        e = register.get(r["id"], {})
        places = match_set(r["pattern"], v, r.get("scope", []))
        except_ids = {x["cluster"] for x in r.get("except", [])}
        applied = [p for p in places if p not in except_ids]
        action = r["actions"][0]
        stats.append(
            {
                "id": r["id"],
                "places": len(applied),
                "exceptions": len(except_ids),
                "status": e.get("status"),
            }
        )
        src = e.get("source")
        cite = html.escape(f"{src['book']}, p. {src['page']}") if src else "none"
        out.append(f"<h2 id='{r['id']}'>{html.escape(r['id'])}</h2>")
        out.append(
            "<table>"
            f"<tr><th>Description</th><td>{html.escape(e.get('description', ''))}</td></tr>"
            f"<tr><th>Status</th><td>{html.escape(str(e.get('status')))}</td></tr>"
            f"<tr><th>Source</th><td>{cite}</td></tr>"
            f"<tr><th>Places</th><td>{len(applied)}</td></tr>"
            f"<tr><th>Option</th><td>{html.escape(str(e.get('option')))}</td></tr>"
            "</table>"
        )
        rows = [
            f"<tr><td><code>{c}</code></td><td class='h'>{html.escape(_word(v, c, None))}</td>"
            f"<td class='r'>{html.escape(_word(v, c, action))}</td></tr>"
            for c in applied[:SHOWN]
        ]
        more = (
            f"<p class='muted'>... and {len(applied) - SHOWN} more</p>"
            if len(applied) > SHOWN
            else ""
        )
        out.append(
            "<details><summary>Places</summary><table><tr><th>Cluster</th><th>Hafs</th>"
            f"<th>With the rule</th></tr>{''.join(rows)}</table>{more}</details>"
        )
        if r.get("except"):
            ex = "".join(
                f"<tr><td><code>{x['cluster']}</code></td><td class='h'>"
                f"{html.escape(_word(v, x['cluster'], None))}</td>"
                f"<td>{html.escape(x['reason'])}</td></tr>"
                for x in r["except"]
            )
            out.append(
                f"<details><summary>Exceptions ({len(r['except'])})</summary>"
                f"<table>{ex}</table></details>"
            )
    out.append("</body></html>")
    return "\n".join(out), stats


def summary(all_stats: dict[str, list[dict]]) -> str:
    lines = [
        "# Phase 6 rules: review summary (T6.5)",
        "",
        "Generated by `just review-rules`, which also writes one HTML sheet per rule file to",
        "`build/review/phase6/rules-<name>.html` (not tracked).",
        "",
        "| Rule file | Rules | Places | Exceptions | Hypothesis | Rule-derived |",
        "|---|---|---|---|---|---|",
    ]
    for name, stats in all_stats.items():
        st = Counter(s["status"] for s in stats)
        lines.append(
            f"| {name} | {len(stats)} | {sum(s['places'] for s in stats)} | "
            f"{sum(s['exceptions'] for s in stats)} | {st['hypothesis']} | {st['rule-derived']} |"
        )
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    names = set(argv[1:])
    v = View.load()
    register = _register()
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    all_stats = {}
    for p in rule_files():
        if names and p.stem not in names:
            continue
        text, stats = sheet(p, v, register)
        (OUT_DIR / f"rules-{p.stem}.html").write_text(text, encoding="utf-8")
        all_stats[p.stem] = stats
    if not names:
        SUMMARY.parent.mkdir(parents=True, exist_ok=True)
        SUMMARY.write_text(summary(all_stats), encoding="utf-8")
    print(f"rule_sheets: {len(all_stats)} sheets in {OUT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
