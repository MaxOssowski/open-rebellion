# Quadrant icons: the defenses icon's drop and the missions icon's drop against A0

Date: 2026-11-13. Cells CMD-03-C019 (quadrant icon drag) and CMD-03-C018
(quadrant icon right-click menu). This page follows
`2026-11-11-quadrant-icon-defenses-move`, which had code and unit tests but
no A0 capture and no browser run.

## A0

The original REBEXE in the offline Windows 10 QEMU VM. It is the unsaved
Alliance game of `2026-11-13-icon-drag-move`, days 351-376, at 640x480. The
captures are under `.artifacts/a0-vm/2026-11-13-defenses-missions-drop/`
(ignored, never committed). Their `README.md` lists every PNG, and their
`SHA256SUMS` holds the hashes. The gate reads these files and checks each
hash (`A0_FILES` in `tools/interface-parity/sector-quadrants.mjs`):

| File | SHA-256 |
|---|---|
| `00-jerijador-defenses-icon-menu.png` | `f65ba930f3ab886eb69966376cb74467056c11aeb1cdd9678674b388d429980e` |
| `03-defenses-icon-mid-drag.png` | `669f219f71b7801b5b5d46f092fdfd71617954261ba8b0e09753fc409202f023` |
| `05-released-on-yavin.png` | `c2a7263457120023412405a72c675176ec1ed4b2ac8d03504689faefd4c7d286` |
| `21-jerijador-defenses-icon-menu-after.png` | `839a46138ecf8ba750a0811567fb5febf4582f021ec56f0fccfc1c984bc6a372` |
| `25-missions-icon-mid-drag.png` | `e088f02e83502bb0bcfdfd857641121c0384311a2b67c109537e9047e878684d` |
| `27-released-on-jerijador.png` | `e90b648691f86c26ea12affd677ea1601d1619cbb50846ac2b0fd092e416d40a` |
| `28-3s-after-drop.png` | `6c4af518ed4feb63bb91856340d2d52006fd043bdb919d0ce96caef55bd82d54` |

These files were read only by eye: `07`..`11` (Jerijador's lists are empty
on day 355), `29` and `30` (the Message Index has no entry for either drop),
`24`, `26` and `06`.

What the gate measures in A0. STRATEGY art is matched pixel for pixel, with
the arrow's 12 by 20 pixels left out:

- **Defenses icon.** It sits at (220, 233) and was pressed at (225, 246).
  - In `03`, the source is drawn pressed (10774, exact). The drag image is
    the normal art 10773, moved by (75, -6), exact.
  - In `05`, after the release, the icon is the normal art 10773: it stays
    on screen, unselected.
  - Menu `00` is at (225, 246), 104 by 102. Its rows are white, white,
    white, grey, white.
  - Menu `21` is at (222, 245). Its rows are grey, grey, white, grey,
    white.
- **Missions icon.** It sits at Kubindi (219, 159) and was pressed at
  (238, 170).
  - In `25`, the source is drawn pressed (10778, exact). The drag image is
    the normal art 10777, moved by (6, 30), exact.
  - In `27` and `28`, the icon is still drawn pressed (10778, exact).

## The original

The trace is in `ghidra/notes/sector-icon-menus.md` and
`ghidra/notes/sector-quadrants.md`.

- **The drop.** `FUN_00422ce0` handles window type 1.
  - A kind 8 entry (the defenses icon) goes into a `0x201` order. Its team
    is the system's regiments, squadrons, characters and special forces
    (`FUN_00512700`).
  - `FUN_00487cc0` asks nothing for an icon's Move, so no window opens.
  - A kind `0x40` entry (the missions icon) takes the members on visible
    missions. The `0x241` validator refuses each of them, and the refusal
    goes to `FUN_00487c90`.
- **The selection.** `FUN_0045d140` deselects (`FUN_0045afc0`) on every
  change notice that `FUN_0045b770` takes for the system or an object at
  it, whether or not the art changes. A0 `05` shows this: the team left,
  the shields kept the icon, and the icon is no longer selected. After the
  refused missions drop nothing changes, so the icon stays selected (A0
  `27`, `28`).

## The port before

- The drop, the drag image, the missions icon's refusal and both menus
  already matched A0. The browser run shows this.
- **One difference.** After the defenses drop, the icon stayed drawn
  selected. The port deselected an icon only when the icon hid or its art
  changed.

## The change

- `crates/rebellion-render/src/sector_window.rs`:
  - A selected icon records `system_contents` (`port:`): a key built from
    the system's fleets, regiments, squadrons, special forces, facilities
    and the characters there.
  - The selection now also lapses when that key changes. This stands in
    for the change notices the port does not have.
  - The new unit test is
    `a_selected_icon_lapses_when_its_systems_team_leaves_though_its_art_stays`.
    Without the change it failed: `[10774]` against `[10773]`. It also
    checks that a character arriving at another system keeps the selection.
  - `an_icon_that_hides_or_changes_art_is_no_longer_selected` changed one
    expectation. A second fleet that arrives is a change notice, so the
    icon now ends unselected; it used to stay selected. That old
    expectation was port behaviour, and the notes already said the original
    differs.
- `crates/rebellion-app/src/interface_test_fixture.rs`:
  - New fixture `QuadrantsFreeTeam`: scenario 117, fixture code 118,
    appended at the end. It is `Quadrants` with the player's agent at the
    primary system free, so the defenses icon's team (the agent and the
    regiment) may move, and the battery stays.
  - Test:
    `the_free_team_scenario_leaves_the_players_agent_at_the_primary_free_to_move`.
- `tools/interface-parity/sector-quadrants.mjs`: two new cases.
  - `missions-icon-drop-refused` (fixture `Quadrants`). The primary
    system's missions icon is pressed 19 by 11 from its corner, as in A0,
    moved by A0's (6, 30), and released on the player's system in uprising.
    The case checks:
    - the drag image is the normal art, exact, and the source shows its
      pressed art;
    - the log has `command=0x201 missions_icon status=refused` and no
      `status=asks`;
    - the icon stays pressed;
    - the whole screen after the drop equals the screen with the icon
      selected before the drag (0 pixels differ): no window and no message;
    - the Missions window still lists both Diplomacy missions;
    - on the Alliance side, the A0 measurements above.
  - `defenses-icon-drop-a0` (fixture `QuadrantsFreeTeam`). The icon is
    pressed at A0's (5, 13), moved by A0's (75, -6), and released on the
    system in uprising. The case checks:
    - the menu before is Move, Confirmed Move, Encyclopedia, Status (grey),
      Scrap;
    - the drag image and the pressed source;
    - the log has no `status=asks` and no `status=refused` (no
      confirmation);
    - the log has `members_move members=1 ... result=Ok`;
    - the icon stays, drawn unselected;
    - the menu after is Move grey, Confirmed Move grey, Encyclopedia white,
      Status grey, Scrap white;
    - the Defenses window counts are `[1, 0, 0, 0, 1]`;
    - on the Alliance side, both menus against A0 `00` and `21`, relative
      to each menu's corner, with the arrow's 22 by 40 pixels masked:
      - the row kinds;
      - each row's ink within a pixel, and the right edge within two (the
        accepted text-width deviation);
      - the frame ring, with 0 pixels differing;
    - also on the Alliance side, the A0 measurements above.
  - `run` now receives the page's console lines.

## Verification

- **Gate (Linux).** The gate ran on Chromium 155.0.8059.84, through a
  temporary copy with the Linux browser manifest, with `--no-build` on the
  WASM from `scripts/build-interface-test-wasm.sh`. The test WASM sha256 is
  `178a8d04a83a0e4375882e2a157a28d77e7def86f851cf3fd4f95f597cc70c67`.
  - The new cases passed on both sides in run
    `sector-quadrants-2026-11-13T22-00-40-857Z-362392`.
  - The Alliance A0 checks all matched:
    - missions drag image `different: 0`;
    - defenses drag image `different: 0`;
    - frame rings `differing: 0`;
    - ink diffs within 1, and within 2 on the right edge.
  - The full gate ran once, in run
    `sector-quadrants-2026-11-13T22-02-03-459Z-385027`: 47 of 47 results
    passed (some cases run on one side only). The A0 captures were reachable this time, so the
    existing A0 cases compared against them too.
- **Native tests.**
  - `cargo test -p rebellion-render --lib`: 1585 passed.
  - `cargo test -p rebellion-app --features interface-test-fixtures`: 191
    passed.
- **Mutants.** `cargo mutants --in-place` ran on the changed lines of
  `sector_window.rs` (the lapse and `system_contents`): 11 mutants, 11
  caught. The first run left `==` to `!=` in the character filter alive.
  The test now covers a character at another system, and that mutant is
  caught.
- **Provenance.** `cargo run -p provenance-scan -- check`: 0 uncited items.
- **Formatting.** rustfmt shows no hunks in the touched code. Both files
  still have their baseline hunks elsewhere.

## What stays open

- **The agent's departure is not in the browser check.** In the port, the
  agent's Move mission departs in phase 4 (`FUN_00556430`,
  `FUN_00556390`), which the paused fixture does not reach. So the
  Defenses window still lists the agent right after the drop. A0 shows
  Jerijador's personnel empty only on day 355, two days later. The gate
  checks the dispatched Move and the regiment's departure, not the agent's.
- **The selection rule is approximate.** `system_contents` covers objects
  that arrive and leave. It does not cover other changes at the system that
  the original also notices, such as a mission starting.
- **Audio.** Any advisor reaction to the refused missions drop
  (`FUN_00487c90` -> slot `+0xc`) was not recorded in A0.
- **Empire side.** The Empire has no A0 for these drops and ran the
  port-only checks.
