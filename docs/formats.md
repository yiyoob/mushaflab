# Corpus formats

The files under `corpus/` are the source of truth (D69). `mushaf check` validates them and
`mushaf build` compiles them into `build/corpus.db`. Status: accepted at GATE M0 (Yusuf,
2026-11-23); fields marked *(M0.6)* may still change as M2 fills farsh, uṣūl and profiles.

## Common rules

- UTF-8, LF line ends, a final newline. No Unicode normalization: Qurʾānic text is stored exactly
  as in its source (CLAUDE.md conventions).
- **TSV:** lines starting with `#` before the header are comments (provenance). Then one header
  row, then one row per fact. Cells are separated by one tab; a cell never contains a tab or a
  newline; an empty cell means "absent". Rows are sorted by the table's key, so a change is a
  small diff.
- **YAML:** only for small files written by hand (profiles, writing styles, source registry) and
  for farsh entries, which nest.
- **Ids.** Word: `surah:ayah:word` (Kūfī count, 1-based), as in `text/words.tsv`; position `0`
  is used only by a ḥizb sign that precedes an ayah's first word (as in the Ḥafṣ spine, D11). Letter:
  `surah:ayah:word:letter`. Inserted items: `<anchor>+ins.<n>`. Riwāyah, qāriʾ and category ids:
  `CLAUDE.md` and `docs/GLOSSARY.md`.

### Provenance columns (every fact row)

| Column | Values | Meaning |
| --- | --- | --- |
| `source` | `<source-id>:<locator>`, several separated by ` ; ` | where the fact is stated; `source-id` is registered in `corpus/sources.yaml` |
| `status` | `raw-derived`, `rule-derived`, `hypothesis` | how the row was obtained. `cross-checked` and `muqri-verified` never appear here: they come from approvals (rule 4, D70) |
| `support` | `cross-source`, `single-source`, empty | for book facts: both books state it, or one does (D68) |
| `note` | free text | optional |

Locators: al-Budūr `p<printed page>` (e.g. `al-budur:p65`); al-Futūḥ `v<vol>p<pdf page>r<row>`
(e.g. `al-futuh:v1p48r2`); a pinned file `<path>#<line or key>`.

## `corpus/sources.yaml`

The registry of citable sources: id, title, author, role (`truth`, `check`, `text`), and the
`MANIFEST.json` source ids that pin the files.

## `corpus/text/words.tsv`

Every item of the KFGQPC Ḥafṣ text, in order. Key: `word_id`.

| Column | Meaning |
| --- | --- |
| `word_id` | `surah:ayah:word` |
| `kind` | `word`, `ayah_end`, `hizb` |
| `text` | the KFGQPC Ḥafṣ codepoints of the item, exactly |
| `sep_before`, `sep_after` | separators as `U+XXXX` lists (space, no-break space), empty if none |
| `source` | the pinned KFGQPC file and its key |

Letters and their clusters are not stored: `mushaf-text` derives them from `text` with the
codepoint table (`data/pipeline/codepoint_classes.tsv`), and the round trip is tested (M0.4, M0.5).

## `corpus/farsh/<sss>.yaml` *(M0.6)*

One file per surah (`001.yaml` …). A list of khilāf places:

```yaml
- id: farsh.<surah>.<ayah>.<n>
  words: ["<word id>", …]
  variants:
    - ruwat: [<riwayah id>, …]
      reading: hafs                     # or a list of letter edits (below)
      when: wasl | waqf | both          # default both
      source: "<source-id>:<locator> ; …"
      support: cross-source | single-source
  combinations:                         # optional, local to these words
    - {allow: {<place or category>: <wajh>, …}, source: "…"}
  status: raw-derived
  note: …
```

A **letter edit** is one of `{set: {letter: <id>, <slot>: <value>}}`, `{insert: {after: <id>,
letter: {...}}}`, `{delete: <id>}`; slots are D72's sign and feature slots.

## `corpus/usul/<category>.tsv` *(M0.6)*

Every place a category applies. Key: (`word_id`, `letter`).

| Column | Meaning |
| --- | --- |
| `word_id`, `letter` | where the category applies (letter index within the word) |
| `category` | e.g. `badal`, `munfasil` (GLOSSARY) |
| `context` | `wasl`, `waqf`, `both` |
| `exception` | empty, or `excluded` for a listed exception |
| `source`, `status`, `support`, `note` | provenance |

## `corpus/riwayat/<id>.yaml` *(M0.6)*

A qāriʾ file (`kind: qari`) holds what his two ruwāt share; a riwāyah file names its `qari` and
overrides.

```yaml
id: warsh
kind: riwayah            # or qari
qari: nafi
tariq: shatibiyyah       # or durrah
ayah_count: <adad system id>
usul:
  <category>:
    wujuh: [<wajh>, …]
    default: <wajh>
    default_basis: book | product   # "book" needs a source stating it is preferred
    source: "…"
combinations:            # riwāyah-wide, from the books
  - {allow: {<category>: <wajh>, …}, scope: <where it applies>, source: "…"}
```

## `corpus/dabt/<style>.yaml`

A writing style: how D72 features become signs. One row per sign: codepoint, slot, the features
it marks, position rules, `source`, `status`. KFGQPC's own conventions are styles of their own
(`kfgqpc-madinah`, `kfgqpc-warsh`, …).

## `corpus/adad/<system>.tsv`

Ayah boundaries per counting system. Key: (`surah`, `ayah`). Columns: `surah`, `ayah`,
`first_word`, `last_word`, `source`, `status`.

## `corpus/layouts/<layout_id>/`

`layout.yaml` (edition, page geometry, lines per page, writing style, font, sources) and
`lines.tsv` (key `page`, `line`; columns `type`, `first_word`, `last_word`, `centered`).

## Approvals: `data/approvals/corpus.yaml`

Written only on the owner's instruction (rule 4); Claude never edits it.

```yaml
- table: corpus/usul/badal.tsv
  sha256: "<sha256 of the approved file content>"   # quoted
  status: cross-checked | muqri-verified
  by: Yusuf
  date: YYYY-MM-DD
  except: ["<row key>", …]        # rows not covered by this approval
```

An approval whose `sha256` no longer matches the file is stale: the rows fall back to their own
`status` until the table is approved again (D70).
