# pipeline/futuh: al-Futūḥ al-Bāhira as structured text (D66)

Extracts the six volumes of *al-Futūḥ al-Bāhira fī Jamʿ al-Qirāʾāt al-ʿAshr al-Mutawātira*
(`book-al-futuh-al-bahira-vol1..6`) into JSONL: every table row with its reader label resolved
to the 20 ruwat, every Quranic word anchored to the Ḥafṣ spine, graded against the 8 KFGQPC
riwāyāt, with its letter-level colours and their meaning. Output goes to
`build/books/al-futuh-al-bahira/` only: it holds book text (rule 10).

```bash
just book-futuh --vol 1 --pages 48-55 --surah 1 --qa   # a page range, with an HTML QA page
just book-futuh --vol 2                                 # a whole volume (~2.5 min)
just book-futuh-learn                                   # re-learn Quran-font glyph corrections
just book-futuh-readers build/books/al-futuh-al-bahira/vol*.jsonl   # re-resolve reader labels
just book-futuh-review sheets build/books/al-futuh-al-bahira/vol1.jsonl high
```

Inputs: the pinned PDFs (group `books`) and Sakkal Majalla (`font-sakkal-majalla`, group
`owner-fonts`) under `data/sources/raw/`, or wherever `FUTUH_PDF_DIR` / `FUTUH_FONT` point; the
Ḥafṣ spine, the KFGQPC riwāyah zips and `data/ayah_maps/`. Nothing Quranic is typed in code.

## Why glyph level, not OCR

The PDFs are born-digital. Each glyph's ToUnicode is mostly right and in logical order (one
ligature glyph carries ḥa-mīm-nūn with their marks); ordinary extractors reverse RTL runs
character by character and scramble every multi-character glyph. So:

| module | does |
|---|---|
| `glyphs.py` | glyphs from `get_texttrace`, marks attached to their base (content-stream adjacency checked against ink extents read from the embedded fonts), RTL/bidi reordering of glyphs |
| `fontmap.py` | glyph id → text from a real font's cmap + GSUB; HarfBuzz round trip of a line |
| `page.py` | table rows from the filled cell rectangles (number / reading / explanation), segments from the thick rules, footnotes, prose repair and round-trip verdicts per line |
| `learn.py` | per-glyph corrections for the Quran fonts, learned from uncoloured text that follows the Ḥafṣ spine → `data/books/al-futuh-al-bahira/glyphfix.json` |
| `quran.py`, `anchor.py` | spine + KFGQPC riwāyāt (via `data/ayah_maps/`), word alignment, grades |
| `madd.py` | which madds a row contains (from the Ḥafṣ spine words it is aligned to) |
| `readers.py` | reader labels → ruwat (`data/books/al-futuh-al-bahira/readers.yaml`) |
| `emit.py`, `run.py` | JSONL, Shamela-style text, HTML QA page |
| `review.py` | stage D: contact sheets of flagged words, verdicts merged as `review` fields |

## JSONL records (`volN.jsonl`)

- `{"type": "page", ...}`: printed page number, prose round-trip tally, corrections, failures.
- `{"type": "row", ...}`: `n` (printed row number), `group` (segment = first row after a thick
  rule), `reading`, `readers` (the الشرح cell), `ruwat` / `ruwat_min` / `ruwat_max`,
  `readers_parse` (how the label was read, `status`: resolved / partial / contextual /
  unresolved, `madd` and its citation for madd groups), `coverage` (on a segment's first row),
  `ayahs`, `fn_refs`, `labels`, `elided`, and `words`.
- each word: `t` (as printed), `spine` (`s:a:w`), `grade`, `riwayat`, `sim`, `bbox` (PDF
  points), `decode` (`proven` or the unconfirmed glyph ids), `split` (printed in two parts), and
  `colors`: runs `{s, e, hex, class, letters, clusters}` with spine cluster ids when the word has
  the spine's letter count.
- `{"type": "footnote", ...}`, `{"type": "text", ...}`: notes and running text.

Grades: `hafs-strict` / `hafs-loose` (equal to the spine word, exactly / up to KFGQPC encoding
conventions), `riwayah-strict` / `riwayah-loose` (equal to what a named KFGQPC riwāyah prints
at that spine position), `unattested`, `unaligned`. See G-221 before reading `unattested` as an
error.

## Readers

The label head (before ":") is parsed with the book's names and terms (terms table v1 p11):
`X واندرج Y` (Y folded into X's row), `الجميع`, `الجميع عدا X`, `الجميع عدا من سيأتي` (all but
the following rows of the segment), `الباقون`, qualifiers `وقفا` (pause only) and `بخلف`.
`X واندرج من اندرج` names X only (bounds, never a guess). The madd groups
`أصحاب القصر/التوسط/الإشباع` take their members from the book's madd schools (separated madd
v1 p52, connected madd v1 p53) for the madd found in the row's spine words; members listed in
no other row of a segment that accounts for all 20 are certain.

## Status

Whole book (2026-11-20): 48,603 rows, all anchored; 345,367 words: Ḥafṣ 87.3%, another KFGQPC
riwāyah 7.6%, unattested 4.9%, unaligned 0.2%; readers 88.0% resolved, 11.0% bounded. Open
items: G-219 to G-224.
