# GATE G1 review: Phase 3, Ḥafṣ reference renderer

Generated at the end of Phase 3 (2026-10-22, branch `claude/phase-3-render-211d7c`). Compare the
pages with the **1441H digital layout** (KFGQPC's digital pages, QUL V4, qcf4), not with a
1421H-style print: the two differ on 1,666 lines (D6 addendum, G-031).

## 1. Pages to review

All are tracked under `tests/golden/pages/` (PNG at 1000 px; the `.json` next to each pins the
draw list's sha256 and every line's fit). Render any other page with
`just render PAGE=<n> RIWAYAH=hafs` (writes `build/render/hafs/pNNN.svg/.png/.json`).

| Page | Why | Look at |
|---|---|---|
| 1 | Al-Fātiḥah, all lines short | short lines at the print's own widths (G-020), header frame |
| 2 | Start of al-Baqarah | page 2 line 5: QUL says not centred, the print is short (G-041) |
| 3 | A plain full page | letter stretching (kashida, wide finals), spacing |
| 106 | First surah header below line 1 | header, basmalah and text on one page |
| 255 | Header plus a short line inside the text | short centred line among full ones |
| 278 | 16:101 (G-015) | line 13: the pause sign over the end of its word |
| 302 | A mid page | general look |
| 507 | Strongest squeeze (0.879, line 7) | dense lines at the page-scale floor |
| 604 | Last page, three surahs | short end lines |
| 3 (edited) | T3.7 demo, `hafs-p003-edit.png` | line 2: small sīn, inserted letter, 0.5 em gap; the EDITED marker |

## 1a. Check against the 1441H print preview (2026-10-22)

The owner opened QUL's preview of the KFGQPC V4 layout (1441H print, drawn with the print's
own V4 word glyphs; `qul.tarteel.ai/resources/mushaf-layout/19`) in the session's browser, and
the ten pages were compared with it.

- **Line content:** all 121 lines of the ten pages read from the preview's page markup (line
  kind, first and last word location, number of word and ayah-number glyphs) equal our layout
  `data/layouts/uthmani-1441h.json`. No line break differs.
- **Centring:** the preview centres exactly the lines our renderer draws short and centred,
  including page 2 line 5, which QUL's database flags as not centred (G-041): the preview's own
  page markup gives it `line--center`.
- **G-015, page 278 line 13:** the preview has the pause sign over the final letter of its word,
  as our render does.
- **G-043, hizb sign:** the preview also draws it as a small star, glued to the next word (page
  507 line 12). The size matches; we leave a full word space after it, because KFGQPC stores
  it as its own item.
- **Look:** the print's surah headers are an ornamental frame with the word "surah" and the
  name, and its basmalah in header lines is a compact calligraphic form; ours are a plain frame
  with the name and the full-size basmalah (D16). The print's line pitch is about 0.100 of its
  line width (measured on page 3); ours is 0.107 (1,900 / 17,800), so a pitch of about 1,780
  would match it (D15 constant, owner's choice).

## 2. Line fit and overflow (D15)

All 604 pages, `just golden` and `cargo run --release -p mushaf-layout --example fit_report`:

- 8,820 text lines; every one is drawn within 0.1% of its target (squeezed lines included).
- 6,785 lines stretch letters (median gain 1,845 font units); widest space per line: median
  2.85x, p95 3.79x, max 5.87x a normal space.
- 209 pages are drawn with a font scale below 1; 32 are at the floor (0.90):
  30, 77, 286, 304, 350, 367, 377, 446, 453, 455, 499, 507, 530, 549, 552, 565, 566, 567, 569, 574, 575, 576, 577, 578, 579, 583, 585, 587, 589, 591, 595, 600.
- 65 lines are squeezed horizontally after spaces tightened to 60%:

| Page | Lines (squeeze) |
|---|---|
| 30 | 14 (0.991) |
| 77 | 5 (0.948), 6 (0.956), 12 (0.986) |
| 286 | 5 (0.993) |
| 304 | 3 (0.979), 6 (0.971) |
| 350 | 11 (0.984), 15 (0.969) |
| 367 | 7 (0.961), 9 (0.957) |
| 377 | 8 (0.950) |
| 446 | 3 (0.963), 4 (0.967), 13 (0.996) |
| 453 | 10 (0.976) |
| 455 | 13 (0.987) |
| 499 | 4 (0.981), 9 (0.930), 14 (0.995) |
| 507 | 5 (0.953), 6 (0.997), 7 (0.879), 10 (0.961), 14 (0.983) |
| 530 | 1 (0.997) |
| 549 | 15 (0.976) |
| 552 | 7 (0.994), 10 (0.997) |
| 565 | 1 (0.952), 2 (0.997), 11 (0.990) |
| 566 | 13 (0.990), 14 (0.995), 15 (0.938) |
| 567 | 11 (0.981) |
| 569 | 14 (0.960) |
| 574 | 3 (0.996) |
| 575 | 6 (0.999), 11 (0.965), 12 (0.975) |
| 576 | 1 (0.946), 10 (0.961), 11 (0.934) |
| 577 | 11 (0.971), 12 (0.987) |
| 578 | 15 (0.979) |
| 579 | 1 (0.998), 4 (0.957), 8 (0.948), 9 (0.961), 10 (0.956), 11 (0.959) |
| 583 | 14 (0.971) |
| 585 | 3 (0.973), 5 (0.965), 6 (0.984), 7 (0.928), 11 (0.970) |
| 587 | 15 (0.979) |
| 589 | 5 (0.991) |
| 591 | 15 (0.938) |
| 595 | 1 (0.985), 15 (0.982) |
| 600 | 14 (0.980) |

Alternatives measured (line width 17,000 or 16,500) give narrower spaces but 223 or 419
squeezed lines; see D15. The constants live in `data/render/hafs-uthmani-1441h.json`.

## 3. Text: KFGQPC 1441H vs DigitalKhatt 1421H (G-024, D14)

`docs/review/g024-dk-text-diff.html` (open it in a browser from the repository so the font
loads). 83,863 of 83,865 spine items equal DigitalKhatt's own text after the data-derived
conversion; 2 differ (2:97:5, 17:7:12), 3 differ only in word splitting (G-012), and 33 use a
lossy row (G-040, your decision: DigitalKhatt's form for now).

## 4. Shaping integrity (rule 6, T3.5a)

Every word on every page (84,428): no `.notdef`, no dotted circle (the font has none, G-042),
every mark attached by an anchor, valid cluster maps. G-015: all 16 places where a pause sign
follows a small meem or hamza below have the sign over its letter.

## 5. Open points for the owner

- D15 constants (line width, pitch, margins, page-scale floor) on the rendered pages.
- G-024 sheet, G-040 (33 lossy words), G-041 (page 2 line 5), G-043 (small standalone hizb
  sign), G-044 (free edits can create unanchored marks; Phase 7).
- Surah headers show KFGQPC's `sura_name_ar` in a plain frame; the print's ornament and the
  word "surah" are not reproduced (D16).
