# Layout edition check (2026-10-22): the owner's print vs the layout we built

The Phase 1 layout follows the "1441H" sources: QUL V4, `quran-qcf4`, and KFGQPC's own per-ayah
page data (all three agree). This compares it with printed pages.

## Printed pages examined

- The owner's photos (pages 92, 254, 278), sent at GATE G0.
- KFGQPC's own scan "Medium Medina Mushaf" (QuranFlash book `Medina2`, image = page + 3),
  pages 7, 71 and 254. Its page 254 is identical to the owner's photo.
- QuranFlash `Medina1` page 7.

| Page, lines | 1421H layout (QUL V2) | 1441H layout (what we built) | Print |
|---|---|---|---|
| 7, 11–12 | marker ٤٥ starts line 12 | marker ٤٥ ends line 11 | starts line 12 (Medina1, Medina2) |
| 71, 8–9 | line 8 ends رِضۡوَٰنَ | line 8 ends ٱللَّهِ | ends رِضۡوَٰنَ (Medina2) |
| 254, 5–6 | marker ٣٦ starts line 6 | marker ٣٦ ends line 5 | starts line 6 (owner photo, Medina2) |
| 254, 6–7 | مَا ends line 6 | مَا ends line 6 | مَا starts line 7 (owner photo, Medina2; agrees with qcf4) |
| 278, 9–10 | marker ٩٨ starts line 10 | marker ٩٨ ends line 9 | starts line 10 (owner photo) |
| 92, all | same in both | same in both | matches |

## Findings

- On every line tested, the print follows the **1421H line layout** (QUL V2). The one exception
  (254, line 6/7) is an error QUL makes in *both* its layouts; the print and qcf4 agree there.
- The 1421H and 1441H layouts differ on 1,666 lines across 375 of 604 pages: 476 boundaries
  where only an ayah marker changes line, plus word moves and 54 lines whose type changes
  (surah headers moving between pages).
- 9 of the 15 spot-check pages contain such differences (176, 187, 254, 262, 293, 378, 441,
  451, 562), mostly ayah-marker placement at a line edge.
- QUL V2 is also the line layout of DigitalKhatt Madina's own text (9,043 of 9,046 lines, S1),
  the font chosen in D2.
