# Tooltips, stage 11: the battle windows and the tactical HUD

Date: 2026-10-31. Inventory: `ghidra/notes/tooltips.md` §6b–6d and §9.
The control-by-control map of §9 onto `tactical_view.rs` was made from
the decompile and checked against the original's creation calls. Every one
of the 51 controls is ported at its original place, except the
fighter-group buttons: they are 25 by 15 in the original and 25 by 17 in
the port. The tip uses 15.

## The tip in macroquad

The battle screens draw with macroquad, not egui. `tactical_view.rs`
therefore keeps its own frame of the shared tip:
- `TacticalTip` runs `win_tooltip::TipTimer` (the same 500 ms / 5 s /
  100 ms timers);
- `tactical_tool` registers a control by rect, module and string;
- `paint_tactical_tip` draws the box with `win_tooltip::tip_rect` and
  the same colours, last in `draw_tactical_view`, so it lies over
  everything.

## What registers

- **Battle Alert** (`FUN_0044f860`, TEXTSTRA): the tabs 0x1922..0x1925,
  Simulate Results 0x1926 and Take Command 0x1927. Retreat 0x194a shows only
  while it is enabled.
- **Battle Summary** (`FUN_0046f360`): the tabs 0x1922, 0x1937, 0x1938 and
  0x1939, and on a forces page the categories 0x1928..0x1931.
- **Assault Summary** (`FUN_0044c630`, egui): the rail buttons 0x1944, the
  attacker's 0x1937 or 0x1938, 0x1943 and 0x1939, and the six category
  buttons. The port's own name tooltips on the unit cells are gone, since
  the original has none.
- **Tactical HUD and panels** (TEXTTACT), through `hud_tooltip`,
  `command_tooltip` and `register_tactical_tooltips`:
  - the headers, task forces (with the runtime "Task Force N" /
    "Unassigned Task Force"), fighter groups (disabled while empty),
    navigation sets, Pause / Resume, Highlight / Dim, the camera
    controls and Game Options;
  - the battle control panel (Observe / Take Command by state; Withdraw
    only when enabled);
  - the maneuvers panel (Surround / Stand Off by the pending tactic), the
    missions panel (Recover only for a fighter group, Attack Death Star
    only when enabled), the fighter-group and capital-ship panels
    (Previous / Next by task force, hull and shield bars, the five power
    rect tools) and the withdraw confirmation.

## Browser (Arch Chromium 155, interface WASM `ca1f2c08…`)

| Gate | What it checks |
|---|---|
| `fleet-window.mjs` squadron-battle | The Battle Alert's Simulate, Take Command and four tabs all show a tip at the shared placement, both sides |
| `fleet-window.mjs` assault | The Assault Summary's four rail buttons and six categories, both sides (`fleet-window-assault-result.json`) |
| `tooltip.mjs` | The production battle fixture (code 65800 / 66056): the two headers, task force 1, Pause, both zoom controls, Game Options and the two camera memory buttons. Each tip is at the pointer plus (0, 21), above it near the bottom (440), and moved left to end at 639 near the right edge (Pause, Zoom Out, Game Options, Remember Camera) |

The tactical catalog (`run.mjs --battle --all`) passes after three gate
fixes (`battle-catalog-first-rerun.txt` is the run before the last two):
1. **Intro in battle fixtures.** The battle fixtures were not counted as
   fixtures for the intro, and the site now carries 000/001, so the intro
   played over every case. `plays_intro` now also checks the tactical
   fixture.
2. **battle-entry.** After the second Pause click, the pointer rested on
   the control and the "Resume Battle" tip showed between the two
   stability frames. The probe now moves the pointer away, after the
   release has reached the game.
3. **The trench runs.** They now play MDATA.201 / 202, which the browser
   plays since 0899c214. The site copies both, the gate expects the request
   and ends the movie with Escape, and the aborted download is not counted
   as an error.

## A0

`.artifacts/interface-parity/a0/TAC-02-C009/original.png` (Wine) shows
"Task Force 1" below the pointer on the first task force. The Windows 10
VM gave no battle capture in this session, so the battle-window and
tactical cells stay partial.
