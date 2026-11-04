# The Missions window on the rail, and members on their way (2026-11-04)

This covers three things found while checking the Missions window:

- what a minimized Missions window keeps;
- how members in hyperspace count at their target;
- the en route mark, which bears on surface ledger cells OBJ-12 C001 ("hyperspace") and C015 ("moving-star indicator").

## A minimized Missions window keeps its state

The rail's `0x466` handler is `FUN_00428b40`, and it is the same for every window kind. It stores the window object (`param_2[9]`) in a slot and hides it with the window's vtable `+8`. It destroys a window (`FUN_005f54a0`, `FUN_00600f90`) only when a thirteenth window takes the oldest's slot.

A0 showed this for a Fleet window: Pzob's window came back with its Arrow selected (`2026-11-03-icon-windows-a0`, personnel-states/39). A Missions window is held the same way, so it comes back with its selected mission and tab.

Before this change, the port minimized a Missions window by closing it. It reopened with the first mission and the Agents tab selected.

Now `MissionsWindowState::hide` keeps the window, and `open` brings it back, as `FleetWindowState` does. `take_railed_missions` lets the sector icon's double click take a railed window back, as A0 shows for the Defenses icon (personnel-states/38).

Tests:
- The unit test `a_minimized_window_comes_back_with_its_selected_mission_and_tab` fails without the change: it comes back with `(Some(0), Agents, (0, 0))` against `(Some(1), Decoys, (20, 30))`.
- The rail test `a_railed_defenses_window_keeps_its_page_and_its_icon_takes_it_back` now also covers `take_railed_missions`.
- `sector-quadrants.mjs`'s `missions` case covers both restores. A restore from the rail slot, and one from the icon after a second minimize, each return the second mission on the Decoys tab at the same origin, on both sides. The old gate expected `selected === 0` after the restore, a fresh open; that expectation is changed on the evidence above.

## Members on their way count at their target

A mission's phase 4 starts each member's transit with `FUN_00556430`. `FUN_00556390` then moves the member into the target container `+0x74` at once (`mission-lifecycle.md`). The walks over a system's members therefore find a member still in hyperspace to that system:
- `FUN_004a1f60`, which gives the mission icon its side;
- `FUN_00553350`, which gives the icon's Abort its team;
- `FUN_0045ce80`'s count of personnel off a mission.

The port's Missions and Defenses windows already listed these members at their target. The quadrant icons read only `current_system`, so a mission whose whole team was in hyperspace lit no icon. Now `SystemContents::members_with_arrivals` adds the transits bound for the system. One bound for an abode is left out, as the Defenses window leaves it out. `transit_members` is shared with the Missions window's list.

The unit test `a_mission_whose_team_is_on_its_way_lights_its_targets_icon` covers three cases:
- the mission icon;
- the Abort team;
- an agent off any mission, on its way, lighting the Defenses icon.

## The en route mark is still

The Quadrants fixture now puts the second system's agent in hyperspace to it, on its Diplomacy mission. `sector-quadrants.mjs`'s new `missions-en-route` case ran on Linux for both sides:

1. The second system's mission icon opens its Missions window, which lists the one agent.
2. Its art is the personnel backdrop 11500, the en route mark 11501, and the GOKRES mini, keyed in that order (`FUN_0042c3b0`).
3. All 1525 compared pixels match, with the key pixels skipped.
4. 1.5 s later the same pixels still match.

The Missions window has no timer (no `SetTimer` in `FUN_0049f540`..`FUN_004a21c0`; the 500 ms timer in `FUN_004aab50` belongs to the pop-up menu). The manual's "moving starfield" (Figs. 2.35, 3.51) is the streaked bitmap 11501.

In A0, `40-mon-calamari-personnel-backdrops` (Alliance save, day 358, Mon Calamari's Defenses window, Personnel page) shows the minis on 11500. The backdrop's key pixel at its bottom left shows the window through it, (24, 41, 104), not the key's blue. The gate's keyed comparison follows that.

That capture also shows Page's mini on 11501, exact in its 699 edge pixels, while Page's Status reads "Awaiting Orders" and Move is offered. The Move ordered to Corfai had not taken effect, so this is not evidence of a character in hyperspace. Why Page carries the mark is open.

## Open

- A0 was not captured with a mission member in hyperspace. Two Mission orders from Mon Calamari (targets Vagran and Corfai) opened no dialog in the VM, so OBJ-12 C001 and C015 stay partial.

Runs (Linux Chromium):
- `sector-quadrants-2026-11-04T05-38-08-134Z-308310` (Alliance `missions`): pass.
- `sector-quadrants-2026-11-04T05-38-19-512Z-308584` (Empire `missions`): pass.
- `sector-quadrants-2026-11-04T05-54-31-646Z-333998` (Alliance `missions-en-route`): pass.
- `sector-quadrants-2026-11-04T05-54-36-691Z-334247` (Empire `missions-en-route`): pass.
- Full batch b51:
  - workspace tests 3348 and fixture tests 116 pass; provenance-scan is clean;
  - all 19 Linux gates pass, `sector-quadrants` among them with both new checks.

Mutants:
- In b51, `own_visible_missions → vec![0]` survived, because the first mission's id is 0.
- The test now gives an agent elsewhere id 0 and asserts `[1]`.
- On the rerun of `quadrant_icons.rs`, 11 mutants were caught, 8 were unviable and none survived.
