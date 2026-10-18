# The Palace rescue (F-038)

Date: 2026-10-18.

This is plan step 3 of `ghidra/notes/jabba-palace.md`. Once Han is held
in Jabba's Palace, the mission manager sends Luke, Leia and Chewbacca to
free him on Palace missions. A failed rescuer stays with Jabba. The
port's invented palace chain went in the previous stage
(`2026-10-18-bounty-hunters`).

## The original

- HanCapturedAtPalace (`FUN_005442f0`, `+0x58` bit `0x40`) holds while
  Han is:
  - captured;
  - on no mission and not adrift;
  - not autorouting;
  - located in Jabba's Palace (`0x98000481`).

  Its hook (`FUN_0054a9c0`) clears BountyHuntersActive, which cancels
  timer `0x393`.
- LukePalace, LeiaPalace and ChewbaccaPalace (`FUN_00544130`,
  `FUN_005443f0`, `FUN_005444e0`): while Han is held, each rescuer that is
  located, free and on no mission (`FUN_005439b0`, `FUN_005439e0`) pulses
  its flag. Its hook (`FUN_0054a910`, `FUN_0054aa60`, `FUN_0054ab10`)
  creates the Palace mission `0x64000044` against Han.
- The Palace class (`0x006652e8`):
  - chance (`FUN_0056e620` -> `FUN_0055c910`): espionage / GNPRTB 6154
    (3) + combat / GNPRTB 6155 (3);
  - hit (`FUN_0056e650`): result 3. The member's base combat and
    espionage rise by GNPRTB 6170 and 6169 (1). The member frees Han
    (slot `+0x210`, `FUN_004ef510`), then every other prisoner of the
    palace (`FUN_00521050`);
  - failure (`FUN_0056e870`, no success): result 2. Each member is taken
    with the mission's captor key and loses `CanEscape`.
- When a mission at an abode ends (`FUN_004ef260`), a character whose side
  `+0x24` differs from the abode's autoroutes out. A prisoner's side is
  its captor's, so a rescuer held for the palace stays there.
- The Dagobah check (`FUN_00544030`) waits while Han is held.
- Jabba Captures (`FUN_00490340` case 5) is notification `0x1e0` for a
  character whose captor is the palace. It shows TEXTSTRA `0x7156` /
  `0x7157` with the character's name, on background `0x429`.

## Port

- `jabba.rs`:
  - `han_captured_at_palace`, derived each day. Its hook clears the
    bounty flag and timer;
  - `JabbaStep::Rescue` per free rescuer, in Luke, Leia, Chewbacca order;
  - `perform` dispatches the Palace mission and aims it at the palace;
  - `held_at_palace` (case 5's captor test).
- `missions.rs`:
  - `MissionKind::Palace`: record `0x64000044`, TEXTSTRA 11332, and its
    chance;
  - the phase-10 hit, and the failure, which captures each member for the
    palace's side and bars its escape (`MissionEffect::EscapeBarred`);
  - the abode target's running check: code 6 once Han is no longer held
    there;
  - `leave_abode` also autoroutes the prisoners this mission freed, and a
    prisoner stays when its captor's side is the abode's;
  - a capture in an abode keeps the character there.
- `dagobah.rs`: the check waits while Han is held.
- `story_messages::jabba_captures_message`. The app posts it for a
  character the mission captures in the palace, and the fixture posts it
  for Chewbacca. `story_class` no longer maps the port's old `0x387`.
- Telemetry: the `story_jabba` step `rescue`.
- No save format change. The seed-42 golden is unchanged.
- port:
  - "usable" (`+0x50` bit 0) has no port field;
  - the Alliance's observation of Han's location is not ported;
  - Luke's branch on the Dagobah mission in phase 8 (slot `+0x1d8`) is
    not read;
  - no one detects a mission at an abode;
  - a freed prisoner keeps its own side, and a neutral one takes the
    rescuer's;
  - Jabba Captures goes to both sides.

## Tests

- `missions.rs`:
  - `the_palace_chance_is_espionage_and_combat_each_over_three`;
  - `a_palace_rescue_frees_han_and_every_prisoner_there`: Han, an Empire
    prisoner (who keeps its side and stays), a neutral one; not the
    killed, the free or one held elsewhere; the raises; Han and Luke
    autoroute out;
  - `a_failed_palace_rescue_leaves_the_rescuer_captive_there_for_good`;
  - `the_palace_mission_ends_once_han_is_no_longer_held_there`.
- `jabba.rs`:
  - `han_captured_at_palace_needs_him_held_there_and_settled`;
  - `only_a_prisoner_in_the_palace_is_held_by_jabba`;
  - `han_held_at_the_palace_cancels_the_hunters_and_sends_his_friends`.
- `dagobah.rs`: `luke_stays_while_han_is_held_at_jabba_s_palace`.
- `story_messages.rs`:
  `jabba_captures_names_the_failed_rescuer_on_the_palace_picture`. The
  class-map test now lists `0x387` as unmapped, because the port's event
  is gone.
- Shown failing without the change (`fail-without.txt`):
  - with the hit, the failure, the chance, the freed autorouting, the
    Dagobah gate and the rescuer steps reverted, 5 tests fail;
  - with the running check reverted, the end test fails.

## Campaigns (`playtest-palace-rescue.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.

- The hunters strike on 16 seeds, between days 310 and 594. Han evades
  every time.
- Han's effective combat less the hunters' 0 (`FUN_005349e0`) puts
  RLEVADTB near its top. So no campaign reaches the palace, and the
  rescue is verified by the tests and the fixture alone.

## Browser

- The story-message fixture (Alliance, code 312) lists "Jabba Captures
  Chewbacca", checked with `agent-browser`:
  - text: "Chewbacca was captured by Jabba while attempting to rescue Han
    Solo.";
  - background `0x429` (1065), Jabba's throne room.
- Screenshot not committed (original artwork):
  `b8df41ede62579c01936349495092835dc05e9e426b51f92f62f9ef549f0319b
  target/acceptance/jabba-palace/jabba-captures.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `4661020699cea79c3f8b3e11e64b718c0ad763fb8b29fc140d41b2e6cde63e93`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 25.0 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.2 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- Native GUI acceptance was not run for this change.

## Gates

- `cargo test --workspace`: 2390 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- The seed-42 replay equivalence check passes with the unchanged golden
  (`v1:5474ae69134d1255` -> `v1:4cd75bad1e5062b8`) and 0 browser errors.
  Release WASM SHA-256
  `8fc2804e388420877cf53bec24ab57a570a2a5bae1650fee6529823c1694d6fb`.
- `cargo mutants --in-diff` (`jabba.rs`, `missions.rs`, `dagobah.rs`,
  `mission_detection.rs`):
  - first run: 84 mutants, 7 missed. They are closed by tests of the
    freed prisoners' filter and sides, and by limiting `leave_abode` to
    the ending mission's own results;
  - last run: 83 mutants, 76 caught, 5 unviable, 2 missed:
    - `mission_detection.rs:1237` `>` to `>=` is the unchanged
      `phase > 4` test of the detection location;
    - `missions.rs:2976`, the captured character's guard in
      `leave_abode`, cannot be observed: a Palace mission has one member,
      and its failure captures every member.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from
  the `implicit_hasher` the story builders share and the fixture's
  existing `use` placement.

Hashes of the committed files are in `SHA256SUMS`.
