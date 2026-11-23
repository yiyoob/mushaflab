# corpus/

The data of the muṣḥaf, as text tables people review and sign (D67, D69). Formats:
[`docs/formats.md`](../docs/formats.md). Validate with `just corpus-check`; compile with
`just corpus-build` (→ `build/corpus.db`, never edited).

| Directory | Holds | Filled in |
| --- | --- | --- |
| `text/` | `words.tsv`: every item of the KFGQPC Ḥafṣ text | M0.4 |
| `farsh/` | khilāf places per surah, with every variant and its ruwāt | M0.6 (prototype), M2+ |
| `usul/` | one table per uṣūl category: every place it applies | M0.6 (prototype), M2+ |
| `riwayat/` | qāriʾ and riwāyah profiles: wujūh, combination tables | M0.6 (prototype), M2+ |
| `dabt/` | writing styles: how features become signs | M0.6, M1 |
| `adad/` | ayah counting systems: `kufi.tsv` (M1.3), others M2 |
| `layouts/` | muṣḥaf layouts as word ranges per line: `uthmani-1441h/` (M1.3), others M4 |

Every fact row carries `source` and `status` (rule 4). Nothing here is written from memory
(rule 1).
