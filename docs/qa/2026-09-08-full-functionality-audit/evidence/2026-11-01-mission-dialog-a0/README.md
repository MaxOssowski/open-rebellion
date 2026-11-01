# Create Mission against A0 (2026-11-01)

A0: the Windows 10 VM. Alliance (save "test", Vjun; `.artifacts/a0-vm/menus2/`)
and Empire (Death Star save, Coruscant, day 582-584;
`.artifacts/a0-vm/menus3/52` to `62`). Hashes in `a0-sha256.txt`.

## What A0 shows and the port does

| Step | A0 | Port |
|---|---|---|
| Mission from a character's menu, targeting cursor 1002 | menus2/06, 08; menus3/52 | cursor 1002 pixel-identical (89/89), `mission-dialog.mjs targeting` |
| Release on a planet with no legal kind: nothing opens | menus2/09 (Adega, Basilisk) | `a_team_with_no_legal_mission_opens_no_dialog`, "nodialog" runs (C019, C024) |
| Release on the team's own cell or planet: the dialog at that system | menus2/11, 12; menus3/53 (Diplomacy at Coruscant) | `targeting` (C018) |
| Mission page chrome | menus2/11 | 0 of 47,985 pixels differ (`2026-11-01-menus2-a0`) (C001) |
| Kind drop-down | menus2/13 | `the_missions_button_opens_the_kind_list...` |
| Agents page: the team as agents, no decoys | menus3/54 | 11 of 44,237 pixels differ, small hue steps at the tabs' and frame's edges, after keying the move arrows 11117..11120 (C002, C020) |
| An agent selected, then moved to the decoys; the selection clears | menus3/55, 56 | `selected_agents_move_to_the_end_of_the_decoys_in_order` (C021) |
| Accept with decoys and no agent: the dialog closes, no mission | menus3/57, 58 | `MissionRefusal::EmptyTeam`, the advisor's reaction |
| Accept with an agent: the mission starts, the agent leaves the Personnel list | menus3/59, 60 | `begin_mission_submits_the_chosen_kind...`, targeting runs (C022) |
| Cancel: the dialog closes, the character stays | menus3/61, 62 | `mission-dialog.mjs targeting` now clicks Cancel: dialog closed, no mission, both sides (C023) |

## Change

The move arrows 11117..11120 are CoolStrobeButtons blitted keyed on
their first pixel (blue): A0 shows the starfield around each triangle.
They are now keyed, and the gate's composed expectation with them.

Run `mission-dialog-2026-11-01T14-15-00-725Z-827758` (Linux Chromium 155), test WASM `5c9ff8bd126d119968001d6da3c29d1e9a1a18a23451e315be315ca865351f4e`.
