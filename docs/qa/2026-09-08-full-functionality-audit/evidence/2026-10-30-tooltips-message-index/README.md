# Tooltips, stage 3: the Message Index (CMD-11-C012)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §4. The first two
stages are in `../2026-10-30-tooltips/` and `../2026-10-30-tooltips-finders/`.

## The original

`FUN_004665f0` gives every control its TEXTSTRA tip:
- the ten category controls (0x79..0x82) get their category names, from
  0x8010 "All Messages" to 0x8032 "Advice Messages";
- Close (0x28) gets 0x1954;
- Display Message (0x65) gets 0x8020. While one message shows,
  `FUN_00468fb0` renames it 0x8019 "Message Index";
- the posting toggle (0x66) gets 0x8005 "Post Messages Silently", or 0x8006
  "Post Messages with Alert" for a silent category (`FUN_004697b0` checks
  bit 2 of the category flags; the tip names what a click would do);
- Display (0x67) gets 0x8030 "Display", or 0x8031 "Encylopedia" (sic) for a
  type-4 target;
- Select All (0x90) gets 0x8008, and Delete (0x91) 0x8009 "Delete Selected
  Messages".

A disabled control shows no tip. In A0 these were disabled and had none:
Display (0x67) with no target, the 0x97 toggle, and in message mode the
greyed 0x67.

## A0 (Windows 10 VM, Empire, days 245–252)

Captures are in `.artifacts/a0-vm/tooltip/`, numbered 70–91; hashes are in
`a0-captures.sha256`.

| Captures | Mode | What they show |
|---|---|---|
| 71–80 | list | the ten categories |
| 81, 82 | list | Select All, Delete Selected Messages |
| 83–85 | list | Close, Display Message, Post Messages Silently |
| 86–89 | list, with and without a chosen row | no tip on the disabled 0x67 and 0x97 |
| 90 | message | "Message Index" on the 0x65 control |
| 91 | message | no tip on the greyed 0x67 |

## The port

`message_index::button_tooltip_string(command, mode, silent)` and
`category_tooltip_string(command)` hold these strings. The controls
register through `win_tooltip::response_tool`. It uses egui's hit test on
the control's own response, because this window's button layer does not
cover the whole window. It also requires `PtInRect`.

Disabled controls are painted from `disabled_rail_controls` and are not
interactive, so they register no tip.

## Browser (Arch Chromium 155, interface WASM `34f3ed75…`)

`tooltip.mjs --a0=.artifacts/a0-vm/tooltip` passes for both sides. On the
Empire side it loads the event-messages fixture (code 57, which has rows),
opens the Index on All (F6), and checks against A0's captures:
- the ten categories;
- Select All, Delete, Close, Display Message and the posting toggle;
- after Display Message, the "Message Index" tip (A0 90).

## Open

- **Display (0x67) when enabled.** Its tip ("Display") is unit-tested only;
  A0 had no message with a target.
- **The "Encylopedia" variant.** The port does not offer the type-4
  Display target, so the 0x8031 tip is not drawn.

So C012 stays partial.
