# The Battle Alert and Battle Summary sit on the live command center

Date: 2026-11-09. Adds to TAC-01-C010 (battle entry from the Battle Alert)
and TAC-07-C001 (the Battle Summary).

## A0

- `.artifacts/a0-vm/empire-game3/battle-alert-coruscant.png` (SHA-256
  `1b2da2b6358765bf9d05f089c876eb49968b0605bca4bb846431b08f4cba3299`):
  a new Empire game in the Windows 10 VM, day 629, the Battle Alert for
  Coruscant.
- `.artifacts/interface-parity/a0/TAC-01-C006/original.png` (SHA-256
  `885a80354c0df489b793eb86124a0969673253b930f8fbfb9cf967180efdd8b2`):
  the Alliance's Battle Summary after Get Immediate Result.

Both windows are drawn over the strategy view as it stood when the battle
began: the message rail with its icons, the droid, the status bar, and
the sector window behind the Summary. Neither shows the bare command
center shell bitmap.

## The defect

The port cleared the screen and drew the command center's shell bitmap
(STRATEGY `ALLIANCE_COMMAND_CENTER_SHELL` / `EMPIRE_COMMAND_CENTER_SHELL`)
under both windows. The rail beside the Battle Alert was therefore empty.

## The change

- `rebellion-app` copies the finished strategy frame to a texture at the
  end of each galaxy frame (`TacticalState::strategy_underlay`,
  `Texture2D::grab_screen` after flushing the frame's batches).
- `draw_battle_window_backdrop` draws that copy under the Battle Alert and
  the Battle Summary when its size matches the screen. Without a copy
  (a fixture that opens a battle directly) it falls back to the shell
  bitmap, as before.

## Browser

`fleet-window.mjs` `squadron-battle` (Linux) runs a strategy game into a
battle and captures the Battle Alert. For the Empire it compares the
message rail (x 606..639, y 100..339) with A0 `battle-alert-coruscant`:

| Build | Rail pixels differing from A0 |
| --- | --- |
| b145 (`a29b7785`, shell bitmap) | 2425 |
| b146 (live underlay) | 0 |

The b145 figure comes from that batch's own capture
(`.artifacts/interface-parity/fleet-window-2026-11-08T19-39-00-737Z-587269/empire-squadron-battle/squadron-battle.png`),
so the assertion fails without the change.

## Mutants

`cargo mutants --in-diff` on `rebellion-render` gave 16 mutants: 14 missed
and 2 unviable. All of them are in `draw_battle_window_backdrop`, which
draws to the GPU and cannot run in a unit test. The browser comparison
above covers the drawing. Mutants that skip the function, drop the copy's
size or flip it, or make the size test always fail (`-` to `+` or `/`,
`<` to `==` or `>`, `&&` to `||` in some cases) draw the shell or nothing
there, which the rail comparison would catch (reasoned from the b145
capture; the browser gate was not run once per mutant). Mutants that only loosen the
test (`<` to `<=`) change nothing, because every gate viewport matches
its copy exactly.

## Batch b146

23 of 23 interface gates on Linux (Arch Chromium); workspace tests 3491
passed, interface fixtures 139 passed, provenance scan clean. Tactical
matrix `2026-11-09T00-21-31-675Z-329774`: 148 of 148.
