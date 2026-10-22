# Performance (T3.6)

`just bench` (release build) on the development Mac (Apple Silicon; numbers vary by machine).
Timing uses the host clock in the bench binary only; nothing in `core/` reads a clock (rule 8).
The 16 ms/page budget is verified on a device in Phase 8.

Recorded 2026-10-22 at the Phase 3 gate:

```
load 222 ms (data, font, all outlines); resolve whole spine 66.1 ms; map lines 5.8 ms

| per page (604 pages) | median ms | p95 ms | max ms | mean ms |
|---|---|---|---|---|
| fit (shape + justify), cold cache | 5.92 | 8.87 | 139.99 | 6.41 |
| fit, one cache across pages | 2.81 | 5.33 | 8.43 | 3.08 |
| fit, warm cache (page seen before) | 0.88 | 1.29 | 1.59 | 0.91 |
| draw + SVG | 4.37 | 7.14 | 16.60 | 4.71 |

word cache after two passes: 834779 hits, 50073 misses
```

- The word cache is keyed by a word's whole shaping state (its characters and feature
  ranges), so a re-justified line reuses every trial shape it has seen. Shaping is pure, and
  `just golden` (exact draw-list hashes) runs with the cache on.
- The cold maximum is the first page (shaping plans are built once per font).
- Resolve rebuilds the whole spine on every call (G-035): fine for a page view, not for
  per-keystroke edits. Per-page or incremental resolution is T7.4's job.
