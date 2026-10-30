# Tooltips, stage 9: sector tracks, Build Selection, move confirmation, mission dialog

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §7a, §7f, §7g and §8.

## Sector window status tracks (CMD-11-C018)

`FUN_00459e30` gives each planet's three tracks a tip. `FUN_0045c240` and
`FUN_0045c450` then rewrite the energy and raw-material tips at run time
(`TTM_UPDATETIPTEXT`) as label + 0x1824 " " + slots in use + 0x1825 "/" +
slots:
- energy: 0x1822 "Energy Consumption";
- raw materials: 0x1823 "Raw Materials";
- support: 0x1821 "Popular Support", which keeps its text.

A0 shows Chandrila (captures 132–134; hashes in `a0-captures.sha256`):
- "Energy Consumption 8/10" over 8 white slots of 10;
- "Raw Materials 2/8" over 2 yellow slots of 8;
- "Popular Support".

Each tip sits at the pointer plus (0, 21).

In the port, `sector_window::track_tooltips` builds the three tools from the
same values that draw the tracks: 3 pixels per slot, and support across
the planet's 37 pixels. They register through
`win_tooltip::window_tool_text` while the track shows.

`tooltip.mjs` (quadrant fixture, Empire, `tooltip-result.json`) rests the
pointer on the three tracks under the player's system in uprising. All
three tips show at A0's placement, and a unit test checks the texts.

## Build Selection (CMD-11-C023)

`FUN_00437f80` / `FUN_00438620` give these TEXTSTRA tips (module 2):

| Control | String |
|---|---|
| Edit Number (the quantity box) | 0x190c |
| Items to Build | 0x1906 |
| Increase | 0x1907 |
| Decrease | 0x1908 |
| Encyclopedia | 0x190b |
| Done | 0x1909 |
| Cancel | 0x190a |

The close box has none, and a disabled Done shows none.

`build-selection.mjs` passes on Linux (`build-selection-run.txt`). With the
dialog open, it rests the pointer on each control and finds the tip at the
shared placement on all seven, and none on the close box. The VM save gave
no Build Selection A0 capture of the tips, so C023 stays partial.

## Move confirmation (CMD-11-C024) and mission dialog (CMD-11-C025)

The port registers these tips:
- **Move confirmation** (`FUN_0044f180`): Yes 0x1300, No 0x1301.
- **Mission dialog** (`FUN_0046a9c0`):
  - the mission box 0x8520 and Missions 0x8510;
  - Cancel 0x8503, Begin Mission 0x8502 and "Encylopedia" 0x8031;
  - the agent and decoy arrows 0x8505 and 0x8506.

Both stay partial. In the VM, the targeting click after a character's
Mission or Confirmed Move did not register over QMP, so neither dialog
opened (`.artifacts/a0-vm/tooltip/160..162, 181`), and no browser gate
checks these tips yet.
