"""T3.2: generate core/mushaf-justify/src/tables.rs from the pinned oracle (rule 10: /tmp clone).

    uv run --with numpy --with freetype-py --directory data python spikes/gen_justify_tables.py

Letter sets and patterns are read from the oracle's constants and source literals and checked
against its pattern table, so none is typed by hand. Also prints run()'s step table.
"""

# ruff: noqa
import ast, sys, subprocess

ORACLE = "/tmp/mushaflab-ref/quran-madina-html/src/db/v2"
head = subprocess.run(
    ["git", "-C", ORACLE, "rev-parse", "HEAD"], capture_output=True, text=True
).stdout.strip()
assert head == "d520c5cf81a05ed92db8861c692104d4b5455c13", head
sys.path.insert(0, ORACLE)
import justify as j

src = open(f"{ORACLE}/justify.py", encoding="utf-8").read()
tree = ast.parse(src)
fn = {n.name: n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef)}


def literals(node):
    out = []
    for n in ast.walk(node):
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and not n.value.isascii():
            out.append((n.lineno, n.col_offset, n.value))
    return [v for _, _, v in sorted(out)]


kash = literals(fn["apply_kashida"])
esc = lambda s: "".join(c if c.isascii() else f"\\u{{{ord(c):04X}}}" for c in s)
# literals of apply_kashida in source order
names = [
    "QAF",
    "LAM",
    "KAF_DAL_THAL_TEH_MARBUTA",
    "HEH",
    "YEH_BEH_TEH_THEH_NOON_YEH_MAKSURA",
    "REH_ZAIN",
    "BEH_LIKE",
    "HEH",
    "MEEM",
    "BEH_LIKE",
    "HAH_LIKE",
    "MEEM",
    "HAH_LIKE",
    "FEH_QAF",
    "HAH_LIKE",
    "LAM",
    "HAH_LIKE",
    "AIN_GHAIN",
    "ALEF_DAL_LAM",
    "BEH_LIKE",
    "SEEN_SHEEN",
    "HAH_LIKE",
    "ALEF_DAL_LAM",
    "HEH_TEH_MARBUTA",
    "BEH_LIKE",
    "REH_ZAIN_NOON",
    "MEEM",
    "SEEN_SAD",
    "REH_ZAIN",
]
assert len(kash) == len(names), (len(kash), [esc(k) for k in kash])
consts = {}
for n, v in zip(names, kash):
    assert consts.get(n, v) == v, n
    consts[n] = v
alts = literals(fn["run"])
assert alts[0] == alts[2] == alts[4] and alts[1] == alts[3], [esc(a) for a in alts]
consts["ALT_BEH_KAF_NOON"], consts["ALT_YEH_SAD_SEEN_FEH"] = alts[0], alts[1]
assert alts[5] == alts[6] == alts[1] + alts[0]
# the step table of run(), to compare with the Rust port
steps = next(n for n in ast.walk(fn["run"]) if isinstance(n, ast.Assign)).value.elts
code = {alts[0]: "BKN", alts[1]: "YSS", alts[5]: "ALL"}
names_ = {1: "BEH", 2: "FINA", 3: "OTHER", 4: "KAF", 5: "SNS", 6: "SS"}
for t in steps:
    f, arg, lv = t.elts
    a = code[arg.value] if isinstance(arg, ast.Constant) else names_[getattr(j, arg.id)]
    print(f.attr, a, lv.value)
consts.update(RIGHT_NO_JOIN=j.RIGHT_NO_JOIN, FINAL_ASCENDANT=j.FINAL_ASCENDANT, R=j._R, L=j._L)
chars = {"FATHA": j.FATHA, "SHADDA": j.SHADDA, "HAMZA": "\u0621", "AYAH_END": "\u06dd"}
pats = {k: [p.pattern for p in v] for k, v in j.KASHIDA_REGEXPS.items()}
R, L, F = j._R, j._L, j.FINAL_ASCENDANT
assert pats[j.BEH] == [j._BEH]
assert pats[j.FINA_ASCENDANT] == [f"^.*([{R}][{F}])$"]
assert pats[j.OTHER] == [f".*([{R}][\u0631\u0632])", f".*([{R}](?:[\u0644]|[{L}]))"]
assert pats[j.KAF] == ["^.*([\u0643].).*$"]
assert pats[j.SECOND_NOT_SAME] == [
    j._BEH,
    f"^.*([{R}][{F}])$",
    f".*([{R}][\u0631\u0632])",
    f".*([{R}](?:[\u0644]|[{L}]))",
]
assert pats[j.SECOND_SAME] == [
    j._BEH,
    f"([{R}][{F}])$",
    f"([{R}][\u0631\u0632])",
    f"([{R}](?:[\u0644]|[{L}]))",
]
assert chars["HAMZA"] in j.RIGHT_NO_JOIN and j.BASES == set(j.DUAL_JOIN + j.RIGHT_NO_JOIN)
out = [
    "//! Letter sets and patterns of the oracle (quran-madina-html `justify.py` @ d520c5c),",
    "//! generated from its source and constants, not typed by hand. Do not edit.",
    "",
    "/// A set of characters.",
    "pub(crate) struct Set(pub &'static str);",
    "",
    "impl Set {",
    "    pub(crate) fn contains(&self, c: char) -> bool {",
    "        self.0.contains(c)",
    "    }",
    "}",
    "",
]
for k in sorted(consts):
    out.append(f'pub(crate) const {k}: Set = Set("{esc(consts[k])}");')
out.append(f'pub(crate) const BASES: Set = Set("{esc(j.DUAL_JOIN + j.RIGHT_NO_JOIN)}");')
for k, v in chars.items():
    out.append(f"pub(crate) const {k}: char = '{esc(v)}';")
out += [
    "",
    f'pub(crate) const RX_BEH: &str = "{esc(j._BEH)}";',
    f'pub(crate) const RX_KAF: &str = "{esc(pats[j.KAF][0])}";',
    "",
    "pub(crate) fn rx_fina() -> String {",
    '    format!("^.*([{}][{}])$", R.0, FINAL_ASCENDANT.0)',
    "}",
    "pub(crate) fn rx_fina_unanchored() -> String {",
    '    format!("([{}][{}])$", R.0, FINAL_ASCENDANT.0)',
    "}",
    "pub(crate) fn rx_reh() -> String {",
    '    format!(".*([{}][\\u{0631}\\u{0632}])", R.0)',
    "}",
    "pub(crate) fn rx_reh_bare() -> String {",
    '    format!("([{}][\\u{0631}\\u{0632}])", R.0)',
    "}",
    "pub(crate) fn rx_other() -> String {",
    '    format!(".*([{}](?:[\\u{0644}]|[{}]))", R.0, L.0)',
    "}",
    "pub(crate) fn rx_other_bare() -> String {",
    '    format!("([{}](?:[\\u{0644}]|[{}]))", R.0, L.0)',
    "}",
]
open("../core/mushaf-justify/src/tables.rs", "w", encoding="ascii").write("\n".join(out) + "\n")
print("ok", len(consts))
