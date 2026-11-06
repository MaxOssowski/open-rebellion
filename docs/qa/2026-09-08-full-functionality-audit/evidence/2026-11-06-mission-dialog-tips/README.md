# The Create Mission dialog's tips against A0 (2026-11-06)

This covers CMD-11-C025 ("Mission dialog").

## A0

The A0 run is the Empire game in the Win10 VM: Jerjerrod's Mission on Ghorman, with the dialog at (230, 40). Each control was hovered for 2.6 s. The captures are `.artifacts/a0-vm/mission-dialog-tips/`, copied to `tooltip/401..408` (hashes in its `a0-sha256.txt`).
- **Tipped.**
  - The mission box: "Selected Mission".
  - The Missions button: "Missions".
  - The left button: "Encylopedia". TEXTSTRA `0x8031` spells it so; A0 shows "Encyclopedia" here.
  - Begin Mission and Cancel: their own tips.
- **No tip.** The target picture and the two page tabs.

## The port

`FUN_0046a9c0`'s tools are registered in `mission_dialog.rs` as `0x8520`, `0x8510`, `0x8031`, `0x8502` and `0x8503`.

## Browser (`tooltip.mjs --a0=.artifacts/a0-vm/tooltip`, Empire, Linux Chromium)

The mission fixture (code 43) opens the dialog at A0's place. The probes `md-kind` … `md-tab1` take A0's tip boxes:
- every tip's x, y (pointer + 21) and height (19) equal A0's;
- the target and the tabs show none, as in A0.

The widths differ by 2 to 4 px (port 98 against A0 94 for "Selected Mission"). That is the accepted text-width deviation.
