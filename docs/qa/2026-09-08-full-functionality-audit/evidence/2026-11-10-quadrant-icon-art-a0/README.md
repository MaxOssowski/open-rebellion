# The quadrant icons' full art matrix against A0

Date: 2026-11-10. Covers CMD-03-C016 (quadrant icon faction and state
art).

## The matrix

`FUN_0045ca80(kind, state, second)` gives each of the four quadrant
icons a pair of bitmaps per state (`ghidra/notes/sector-quadrants.md`):
- the first bitmap is the icon at rest;
- the second is the icon while selected;
- the states are side 1, side 2, and 0 or 3 (neither).

The fleet and mission kinds have no art for state 0. That makes 20
bitmaps, STRATEGY 10771..10790.

## A0

Every one of the 20 bitmaps was searched in the A0 captures. A bitmap
counts as found only when each of its opaque pixels (blue-keyed) matches
a capture exactly.

**Sources**
- All 2,635 PNGs under `.artifacts/a0-vm/`. Counts and first places for
  the selected and rarer ids are in `a0-occurrences.txt`.
- Four sector-window captures from this session's Empire games:
  Coruscant, Sluis Van and Umgul.
- Three new captures from a new Empire game in the Windows 10 VM, in
  `.artifacts/a0-vm/quadrant-icons-empire/` (SHA-256 in `SHA256SUMS`):
  - `00-coruscant-mission-icon`: Darth Vader sent on a Recruitment
    mission to Coruscant lights its Empire mission icon, 10785;
  - `01-coruscant-mission-icon-selected`: a click on that icon draws its
    selected art, 10786. Found nowhere else in A0;
  - `02-coruscant-fleet-icon-selected`: a click on Coruscant's defenses
    icon draws the Empire's selected defenses art, 10782.

| Kind | Side 1 at rest / selected | Side 2 at rest / selected | Neither at rest / selected |
| --- | --- | --- | --- |
| System (4) | 10771 / 10772 | 10779 / 10780 | 10787 / 10788 |
| Defenses (8) | 10773 / 10774 | 10781 / 10782 | 10789 / 10790 |
| Fleet (`0x10`) | 10775 / 10776 | 10783 / 10784 | — |
| Missions (`0x40`) | 10777 / 10778 | 10785 / 10786 | — |

All 20 bitmaps occur exactly in A0. So the original draws each pair
exactly as the resource holds it, keyed on pure blue.

## The port

`sector-quadrants.mjs` (Linux Chromium, both sides) checks the port's
icons against the same resources. Each check expects 0 differing pixels:
- the `icons` case covers each shown icon's rest art for both sides and
  the neutral system icon 10787;
- the `selected` case covers the selected art of each pressed icon,
  10772, 10774, 10776, 10778, 10780, 10782, 10784 and 10786.

A new check in the `icons` case presses the neutral system icon and
expects 10788. It passes on both sides, 0 of 94 opaque pixels differing:
`.artifacts/interface-parity/sector-quadrants-2026-11-10T09-18-44-531Z-211208`.

## Later: the neutral defenses pair

The neutral defenses pair 10789 and 10790 was not yet checked in the
port. A0 shows both, but no fixture placed a neutral system with
defenses in a sector window.

The Quadrant Icons fixture's neutral system now also holds one of the
player's batteries. Its defenses icon takes the system's side
(`FUN_0045ce80`), state 0, as the mine already made its system icon do.
The fixture reports the icon's rect as `neutral_defenses`.

The `icons` case then checks the icon at rest (10789) and, after a
press, selected (10790). Both sides pass: 0 of 48 and 0 of 94 opaque
pixels differ. The run is
`.artifacts/interface-parity/sector-quadrants-2026-11-10T10-22-20-690Z-334023`,
all cases on both sides.

So the port's sector windows now check all 20 bitmaps of the matrix,
and each of them occurs exactly in A0.

## Batch b171

23 of 23 interface gates on Linux, `sector-quadrants`
(`sector-quadrants-2026-11-10T09-27-42-342Z-236535`) among them with the
new check; workspace tests 3498 passed, fixture tests 139. Tactical
matrix `2026-11-10T10-08-33-741Z-303549`: 148 of 148.

## Batch b172

The first start of b172 was taken for dead and started again, so two
batches ran at once. Both were stopped and their results discarded
(`fleet-move` timed out launching Chromium). A single clean run
followed:

- 23 of 23 interface gates on Linux, `sector-quadrants`
  (`sector-quadrants-2026-11-10T10-56-45-892Z-408299`) among them with
  the neutral defenses check;
- workspace tests 3498 passed, fixture tests 139;
- tactical matrix `2026-11-10T11-36-55-055Z-464169`: 148 of 148.
