"""Pilot summary and HTML coverage report (TB.6). The HTML holds book text: build/ only."""

import html
from collections import Counter, defaultdict

from pipeline.spine import from_cps

STATUS_SHORT = {"mentioned": "M", "rule-covered": "R", "not-mentioned": "-"}


def summary(p: dict, idx: dict) -> dict:
    riwayat = p["riwayat"]
    n = p["range"]["words"]
    stat = p["statuses"]
    counts = {r: Counter(stat[str(w)][r]["status"] for w in range(n)) for r in riwayat}
    flagged: dict[str, set[int]] = defaultdict(set)
    possible: dict[str, set[int]] = defaultdict(set)
    undecidable: set[int] = set()
    kinds: Counter = Counter()
    explained: Counter = Counter()
    for v in p["verdicts"]:
        for r in v["flagged"]:
            flagged[r] |= set(v["words"])
        for r in v["possible"]:
            possible[r] |= set(v["words"])
        if v["undecidable"]:
            undecidable |= set(v["words"])
        kinds.update(f["kind"] for f in v["findings"])
        explained.update(why for _pair, why in v["explained"])
    uncited = sum(
        1
        for w in range(n)
        for r in riwayat
        if stat[str(w)][r]["status"] != "not-mentioned" and not stat[str(w)][r].get("cite")
    )
    covered_h = sum(1 for w in range(n) if stat[str(w)]["hafs"]["status"] != "not-mentioned")
    return {
        "words": n,
        "counts": {r: dict(c) for r, c in counts.items()},
        "flagged": {r: sorted(flagged[r]) for r in riwayat},
        "possible": {r: sorted(possible[r] - flagged[r]) for r in riwayat},
        "undecidable_words": sorted(undecidable),
        "hafs_flag_rate": len(flagged["hafs"]) / n,
        "hafs_flag_rate_covered": len(flagged["hafs"]) / covered_h if covered_h else 0.0,
        "hafs_possible_rate": len(possible["hafs"] - flagged["hafs"]) / n,
        "finding_kinds": dict(kinds),
        "explained": dict(explained),
        "uncited_statuses": uncited,
        "placements": dict(Counter(x["status"] for v in p["placement"].values() for x in v)),
        "rules": len(idx["rules"]),
        "rule_scopes": dict(Counter(r["scope"] for r in idx["rules"])),
        "applications": len(p["applications"]),
        "corrections": len(idx["corrections"]),
        "phase6": dict(Counter(x["support"] for x in p["phase6"])),
        "awjuh": len(p["awjuh"]),
    }


def summary_lines(s: dict) -> list[str]:
    out = [f"budur pilot: {s['words']} words; placements {s['placements']}"]
    for r, c in s["counts"].items():
        out.append(
            f"  {r:12} {c}  flagged {len(s['flagged'][r])}  possible {len(s['possible'][r])}"
        )
    out.append(
        f"  hafs false-flag rate {s['hafs_flag_rate']:.4%} of words "
        f"({len(s['flagged']['hafs'])}/{s['words']}); {s['hafs_flag_rate_covered']:.4%} of "
        f"covered words; possible {s['hafs_possible_rate']:.4%}"
    )
    out.append(
        f"  undecidable words {len(s['undecidable_words'])}; findings {s['finding_kinds']}; "
        f"explained {s['explained']}"
    )
    out.append(f"  uncited mentioned/rule-covered statuses: {s['uncited_statuses']}")
    out.append(
        f"  rule index: {s['rules']} rules {s['rule_scopes']}; applications in range "
        f"{s['applications']}; corrections {s['corrections']}"
    )
    out.append(f"  phase 6 rules in range: {s['phase6']}; awjuh records {s['awjuh']}")
    return out


def _e(x: object) -> str:
    return html.escape(str(x))


def _ar(cps: str) -> str:
    return f'<span class="ar">{_e(from_cps(cps)) if cps else "&#8709;"}</span>'


def _book(text: str) -> str:
    return f"<div class='book'>{_e(text)}</div>"


def _row(cells: list[str], tag: str = "td") -> str:
    return "<tr>" + "".join(f"<{tag}>{c}</{tag}>" for c in cells) + "</tr>"


CSS = """
:root{--bg:#fbfaf7;--fg:#1d1d1b;--muted:#6b6b66;--line:#dedbd2;--m:#2f6f4f;--r:#2b5c8a;
--flag:#a8322d;--poss:#b07a12;--chip:#efece4}
@media (prefers-color-scheme:dark){:root{--bg:#161614;--fg:#ecebe6;--muted:#a3a29b;
--line:#34332f;--m:#6fbf93;--r:#7fb0e0;--flag:#ef7d75;--poss:#e5b65a;--chip:#26251f}}
body{background:var(--bg);color:var(--fg);font:14px/1.5 system-ui,sans-serif;margin:0 16px 64px}
h1{font-size:22px}h2{font-size:18px;margin-top:32px;border-bottom:1px solid var(--line)}
table{border-collapse:collapse;margin:8px 0;font-size:13px}
td,th{border:1px solid var(--line);padding:3px 6px;vertical-align:top}th{background:var(--chip)}
.ar{font-family:'Amiri',serif;font-size:20px;direction:rtl}
.book{direction:rtl;font-family:'Amiri',serif;font-size:16px;background:var(--chip);
padding:6px 10px;border-radius:6px}.M{color:var(--m);font-weight:600}
.R{color:var(--r);font-weight:600}.flag{color:var(--flag);font-weight:700}
.poss{color:var(--poss)}.muted{color:var(--muted)}code{font-size:12px}details{margin:4px 0}
.wrap{overflow-x:auto}
"""


def _summary(p: dict, s: dict) -> list[str]:
    out = ["<h2>Summary</h2><div class='wrap'><table>"]
    out.append(_row(["riwayah", "mentioned", "rule-covered", "not-mentioned", "flagged",
                     "possible"], "th"))  # fmt: skip
    for r in p["riwayat"]:
        c = s["counts"][r]
        out.append(_row([r, str(c.get("mentioned", 0)), str(c.get("rule-covered", 0)),
                         str(c.get("not-mentioned", 0)),
                         f"<span class='flag'>{len(s['flagged'][r])}</span>",
                         f"<span class='poss'>{len(s['possible'][r])}</span>"]))  # fmt: skip
    out.append("</table></div>")
    out.append(
        f"<p>Hafs false-flag rate: <b>{s['hafs_flag_rate']:.4%}</b> of words; "
        f"{s['hafs_flag_rate_covered']:.4%} of mentioned/rule-covered words; possible "
        f"{s['hafs_possible_rate']:.4%}. Undecidable words: {len(s['undecidable_words'])}. "
        f"Placements: {_e(s['placements'])}. Findings: {_e(s['finding_kinds'])}; explained: "
        f"{_e(s['explained'])}. Uncited statuses: {s['uncited_statuses']}.</p>"
    )
    return out


def _cell(p: dict, s: dict, w: dict, r: str) -> str:
    x = p["statuses"][str(w["pos"])][r]
    short = STATUS_SHORT[x["status"]]
    if x.get("unit"):
        label = f"<a class='{short}' href='#{_e(x['unit'].split('@')[0])}'>{short}</a>"
    else:
        label = f"<span class='muted'>{short}</span>"
    if w["pos"] in s["flagged"][r]:
        label += " <span class='flag'>!</span>"
    elif w["pos"] in s["possible"][r]:
        label += " <span class='poss'>?</span>"
    if w["forms"][r][1] != w["forms"]["hafs"][1]:
        label += " " + _ar(w["forms"][r][0])
    return label


def _words(p: dict, s: dict) -> list[str]:
    out = ["<h2>Words</h2><div class='wrap'><table>"]
    out.append(_row(["word", "Hafs", *p["riwayat"]], "th"))
    for w in p["words"]:
        cells = [_cell(p, s, w, r) for r in p["riwayat"]]
        out.append(_row([f"<code>{_e(w['id'])}</code>", _ar(w["forms"]["hafs"][0]), *cells]))
    out.append("</table></div>")
    return out


def _flags(p: dict, idx: dict) -> list[str]:
    rules = {r["id"]: r for r in idx["rules"]}
    by_pos = {w["pos"]: w for w in p["words"]}
    out = ["<h2>Flags</h2><p class='muted'>Each flag: the book passage next to our forms "
           "(strict, with codepoints).</p>"]  # fmt: skip
    for v in p["verdicts"]:
        if not (v["flagged"] or v["possible"] or v["undecidable"]):
            continue
        src = p["blocks"].get(v["unit"].split("@")[0]) or rules.get(v["unit"].split("@")[0])
        passage = src["text"] if src else "(no statement: the book is silent here)"
        out.append(
            f"<details open><summary><b>{_e(v['unit'])}</b> {_e(src['cite'] if src else '')}"
            f" flagged <span class='flag'>{_e(sorted(v['flagged']))}</span> possible "
            f"<span class='poss'>{_e(sorted(v['possible']))}</span> "
            f"{_e(v['undecidable'] or '')}</summary>{_book(passage)}<table>"
        )
        out.append(_row(["word", *p["riwayat"]], "th"))
        for pos in v["words"]:
            w = by_pos[pos]
            out.append(_row([f"<code>{_e(w['id'])}</code>"] + [
                f"{_ar(w['forms'][r][0])}<br><code>{_e(w['forms'][r][0])}</code>"
                for r in p["riwayat"]
            ]))  # fmt: skip
        out.append("</table><ul>")
        out += [f"<li>{_e(f['kind'])}: {_e(f['pair'])}</li>" for f in v["findings"]]
        out.append(f"</ul><p class='muted'>explained: {_e(v['explained'])}</p></details>")
    return out


def _entries(p: dict) -> list[str]:
    out = ["<h2>Book entries in range</h2>"]
    for bid, b in p["blocks"].items():
        if b["zone"] != "surahs":
            continue
        pl = [(x["status"], x["tiers"]) for x in p["placement"].get(bid, [])]
        out.append(
            f"<details id='{_e(bid)}'><summary><b>{_e(bid)}</b> {_e(b['cite'])} "
            f"{_e(b['section'])}/{_e(b['kind'])} <span class='ar'>"
            f"{_e(' + '.join(b['headwords']))}</span> {_e(pl)}</summary>{_book(b['text'])}<table>"
        )
        out.append(_row(["#", "feature", "wq", "scope", "readers", "khulf", "complement"], "th"))
        for i, c in enumerate(b["clauses"]):
            rd = c["readers"]
            comp = (
                "ambiguous"
                if c["complement_ambiguous"]
                else _e(sorted(c["complement_in_scope"] or []))
            )
            named = _e(sorted(rd["named"])) + (" all" if rd["all_readers"] else "")
            if rd["unresolved"]:
                named += f" unresolved {_e(rd['unresolved'])}"
            cells = [str(i), _e(c["feature"]), _e(c["wq"]), _e(c["scope"]), named]
            out.append(_row([*cells, _e(sorted(rd["khulf"])), comp if rd["complement"] else ""]))
        out.append("</table></details>")
    return out


def _rules(p: dict, idx: dict, s: dict) -> list[str]:
    rules = {r["id"]: r for r in idx["rules"]}
    by_pos = {w["pos"]: w["id"] for w in p["words"]}
    out = [f"<h2>Rule index (whole book)</h2><p>{s['rules']} rules: {_e(s['rule_scopes'])}. "
           f"Applications in range: {s['applications']}.</p>",
           "<h3>Applied rules (every application listed)</h3>"]  # fmt: skip
    apps = defaultdict(list)
    for a in p["applications"]:
        apps[a["rule"]].append(a)
    for rid, al in apps.items():
        r = rules[rid]
        items = "".join(
            f"<li>{_e([by_pos.get(x) for x in a['words']])} {_e(a['tier'])}"
            f"{' (own place)' if a['own_place'] else ''}"
            f"{' (stated after this place)' if a['stated_after'] else ''}</li>"
            for a in al
        )
        out.append(
            f"<details id='{_e(rid)}'><summary><b>{_e(rid)}</b> {_e(r['cite'])} {_e(r['scope'])}"
            f" <span class='ar'>{_e(' + '.join(r['headwords']))}</span> {len(al)} applications"
            f"</summary>{_book(r['text'])}<ul>{items}</ul></details>"
        )
    out.append("<h3>Rules of unclear scope (class, general): not applied, for the owner</h3>")
    for r in idx["rules"]:
        if r["scope"] in ("class", "general"):
            out.append(
                f"<details id='{_e(r['id'])}'><summary>{_e(r['id'])} {_e(r['cite'])} "
                f"{_e(r['scope'])} readers {_e(sorted(r['readers']))} {_e(r['feature'])}"
                f"</summary>{_book(r['text'])}</details>"
            )
    return out


def _phase6(p: dict) -> list[str]:
    out = ["<h2>Phase 6 rules in range</h2><div class='wrap'><table>"]
    out.append(_row(["rule", "riwayah", "places", "support", "statuses", "description"], "th"))
    for x in sorted(p["phase6"], key=lambda x: (x["support"], x["rule"])):
        out.append(_row([_e(x["rule"]), _e(x["riwayah"]), str(x["places"]), _e(x["support"]),
                         _e(x["statuses"]), _e(x["description"])]))  # fmt: skip
    out.append("</table></div>")
    return out


def _rest(p: dict, idx: dict) -> list[str]:
    out = [
        "<h2>Convention candidates</h2><p class='muted'>conventions.yaml rows (all "
        "hypothesis); the pairs explained by them are listed under each flag.</p><table>"
    ]
    out.append(_row(["cp", "action", "evidence", "note"], "th"))
    out += [_row([_e(c["cp"]), _e(c["action"]), _e(c["evidence"]), _e(c.get("note", ""))])
            for c in p["conventions"]]  # fmt: skip
    out.append("</table><h2>Awjuh and options</h2><table>")
    out.append(_row(["entry", "kind", "readers", "khulf", "wq", "restriction", "default",
                     "clause"], "th"))  # fmt: skip
    for a in p["awjuh"]:
        out.append(_row([f"<a href='#{_e(a['block'])}'>{_e(a['block'])}</a>", _e(a["kind"]),
                         _e(a["readers"]), _e(a["khulf"]), _e(a["wq"]), _e(a["restriction"]),
                         _e(a["default"]), _book(a["text"])]))  # fmt: skip
    out.append("</table><h2>Corrections on pages in range (al-Sulami, not the author)</h2><ul>")
    pages = {b["cite"] for b in p["blocks"].values()}
    for c in idx["corrections"]:
        if c["page"] is not None and any(pg.endswith(f" p. {c['page']}") for pg in pages):
            out.append(f"<li>{_e(c['id'])} (printed p. {c['page']}): {_book(c['text'])}</li>")
    out.append("</ul>")
    return out


def render(p: dict, idx: dict, s: dict) -> str:
    head = [
        "<!doctype html><html lang='en'><head><meta charset='utf-8'>",
        "<meta name='viewport' content='width=device-width,initial-scale=1'>",
        f"<title>Budur pilot</title><style>{CSS}</style></head><body>",
        "<h1>al-Budur al-Zahirah: pilot coverage</h1>",
        f"<p class='muted'>Range {_e(p['range']['first'])} to {_e(p['range']['last'])} "
        f"({s['words']} words): the book's first rub' (entry {_e(p['range']['rub_end_block'])}"
        f"); spine rub' sign right after it: {_e(p['range']['spine_rub_sign_after'])}. Book "
        "text here is from the pinned EPUB and must not leave build/.</p>",
    ]
    body = (_summary(p, s) + _words(p, s) + _flags(p, idx) + _entries(p) + _rules(p, idx, s)
            + _phase6(p) + _rest(p, idx))  # fmt: skip
    return "\n".join([*head, *body, "</body></html>"])
