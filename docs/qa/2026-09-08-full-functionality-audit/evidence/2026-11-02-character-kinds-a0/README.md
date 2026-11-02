# Assassination and Abduction, and the order of the kinds (2026-11-02)

Surface ledger OBJ-11: C003 "Abduction" and C004 "Assassination".

## A0

The A0 run used the Windows 10 VM and the Empire's "resource turn 14" save, days 197..202. The captures are in `.artifacts/a0-vm/kinds2/`, with hashes in `a0-sha256.txt`.

1. Veers was at Coruscant. Yavin's Defenses window was opened beside his, through the Personnel Finder's Display on Han Solo.
2. Veers' menu, then Mission, then a click on Leia Organa, opened Create Mission (`50`):
   - Assassination was shown;
   - "Target" was Leia's portrait, "Leia Organa".
3. The drop-down (`51`, `54`, scrolled by clicks on its down arrow) listed Assassination, then Abduction, and nothing more.
4. Abduction, chosen, shows in the box (`55`).
5. The icons, the window's (70, 86), rows 0..63 against GOKRES:
   - Assassination 7297 (TEXTSTRA 11393 & 0xfff, + 0x1000): 0 of 8,320 differing;
   - Abduction 7191 (11287): 0 of 8,320 differing.

## The order of the kinds

| Release | A0's box, top to bottom | MISSNSD ids |
|---|---|---|
| Darth Vader on enemy Chandrila (kinds/20, 23) | Incite Uprising, Espionage | 0x40, 0x13 |
| Darth Vader in his own cell (kinds/01, 03) | Recruitment, Espionage | 0x16, 0x13 |
| Veers on Leia (`50`..`55`) | Assassination, Abduction | 0x81, 0x17 |

A0 lists the kinds last record first, and the box shows the first of them.

- **The port's order before this change.** It listed and showed them in MISSNSD order, the order of `FUN_005422f0`'s walk.
- **`mission_dialog::box_order`.** The box's order is now the reverse of that walk. `main.rs` and the fixture apply it when they open the dialog.
- hyp: the box's fill inserts each item at its head. `FUN_005422f0`'s list has rising keys (`FUN_005f5440`). The fill itself is not traced.

## Who may undertake them

MISSNSD gives Assassination (0x81) `is_alliance` 0 and `is_empire` 1. Abduction (0x17) is open to both sides. So the Alliance lists Abduction alone, which the port already did.

## The port's case

`mission-dialog.mjs` `enemy-character` runs the new scenario `EnemyCharacterTargeting` (89, code 90): the other side's character is free at the secondary system, and the agent's Mission is released on it.

| Side | Kinds | Icon at (70, 86) |
|---|---|---|
| Empire | Assassination, Abduction | 7297 |
| Alliance | Abduction | 3095 |

## Tests

- `the_box_lists_the_kinds_last_record_first_and_shows_the_last` (`mission_dialog.rs`).
- `the_enemy_character_scenario_frees_the_target_on_the_other_side_in_its_system` (`interface_test_fixture.rs`).

## Run

`mission-dialog-2026-11-02T10-34-54-144Z-4072675` (Linux Chromium 155, test WASM `37aac526…`): pass. Every case's icon differs from its bitmap in 0 pixels.

The batch's other gates passed in `b21`. That batch's own mission-dialog run expected Assassination for the Alliance too. MISSNSD refutes that, and the gate now expects it for the Empire only.
