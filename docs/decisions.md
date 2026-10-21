# Architecture Decision Records

Each decision from PROJECT_PLAN §4 gets an ADR here. A decision is never made silently.
Only the owner moves an ADR from `proposed` to `accepted`.

## Template

```
## D<n> — <title>
Status: proposed | accepted | superseded by D<m>
Date: YYYY-MM-DD
Context: why the decision is needed
Options: each option with evidence (files, measurements, links)
Decision: the choice
Consequences: what follows, what it rules out
```

## Index

| ID | Title | Status |
|---|---|---|
| D1 | Line policy for non-Ḥafṣ riwāyāt | open (P0 spike, finalized P5) |
| D2 | Font per riwāyah | open (G0) |
| D3 | Justification implementation | open (G0) |
| D4 | Draw path | open (P3) |
| D5 | First mobile platform | open (G1) |
| D6 | Ḥafṣ edition: text + layout + font | open (G0) |
| D7 | Licensing and distribution | open (G0) |
| D8 | Script/layout scope | accepted |
| D9 | ID and patch model details | accepted (revisit end of P2) |
| D10 | Numeric determinism | accepted |

---

## D8 — Script/layout scope
Status: accepted
Date: 2026-10-21
Context: The owner wants a correct foundation before taking on more scripts. Indo-Pak
uses its own orthographic conventions, fonts and text sources, so it is not a re-skin of
the ʿUthmānī text.
Options:
- A: Build ʿUthmānī 15-line and Indo-Pak 15/13-line together.
- B: Stage 1 = Madinah ʿUthmānī 15-line only. Indo-Pak becomes Stage 1b, as a second spine.
Decision: B (owner, plan v2).
Consequences: `script_id` and `layout_id` are explicit keys in every table, file name
and API from the start, so Stage 1b needs no migration. Indo-Pak is never modelled as
patches on the ʿUthmānī spine.

## D9 — ID and patch model details
Status: accepted (revisit at end of P2)
Date: 2026-10-21
Context: The v1 plan left inserted-item IDs, ordering, mark-op composition, deleted
targets and spine corrections undefined.
Decision (owner, plan v2; details in PROJECT_PLAN §3):
- Spine carries `spine_version`; IDs are stable within a version, and new versions ship
  a migration map.
- Inserted items: `<anchor>+ins.<n>` with an explicit integer `seq`; never string-sorted.
- At most one patch per target per layer; mark changes merge into one `set_marks`.
- A patch whose target a lower layer deleted is an orphan: reported, never dropped.
- Ayah-end markers and waqf/section signs are spine items, patchable like any cluster.
Consequences: the differ (T4.4) must merge ops per target; the resolver needs orphan
reporting from T2.3.

## D10 — Numeric determinism
Status: accepted
Date: 2026-10-21
Context: CLAUDE.md requires byte-identical output. Floating-point results can differ
across CPUs, compilers and platforms (desktop x86/ARM, iOS, Android).
Decision: all layout and justification arithmetic in `core/` uses fixed-point integers
(font units, 26.6 where sub-unit precision is needed). Floats are allowed only at the
platform draw edge, after the core's output is final.
Consequences: the shaping backend chosen in D3 must expose integer positions (HarfBuzz
and its ports do). Golden outputs are compared exactly, not with tolerance, at the
draw-command level.
