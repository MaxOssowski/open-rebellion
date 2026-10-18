# Vader's Pickup at Jabba's palace (F-038)

Date: 2026-10-18.

This is plan step 4 of `ghidra/notes/jabba-palace.md`. Once Luke, Leia
and Han are held, Vader collects the palace's prisoners and Jabba
Delivers Prisoners is posted. The reading is in `jabba-palace.md` ("The
Pickup") and `final-battle.md` ("DarthPickup").

## The original

- DarthPickup (`FUN_005445d0`) pulses when all of these hold:
  - Luke, Leia and Han are captured;
  - Chewbacca is captured or destroyed (`+0x50` bit 3);
  - a prisoner is in Jabba's Palace (`FUN_004f2640(palace, 1, 2)`);
  - Vader is located, free (`FUN_005439b0`) and on no mission.

  Its hook (`FUN_0054abc0`) creates the Pickup mission `0x73000082` with
  member Vader and target the palace.
- The hit (`FUN_0056da80`):
  - sets PickupInProgress;
  - makes each palace prisoner of the mission's side on no mission join
    the mission (slot `+0x1cc`);
  - gives each joiner the member's home `+0x70`. The disassembly at
    `0x56dbba` shows `ECX` is the hit's member, Vader;
  - sets the result to 3, then clears PickupInProgress.
- At the end, `FUN_004ef260` autoroutes each prisoner: its own side is
  not the palace's, so its slot `+0x218` runs. The first candidate is its
  `+0x70` (`FUN_00554ab0`), so the prisoners leave for Vader's home,
  still captive. Vader, of the palace's side, stays.
- Jabba Delivers Prisoners (`FUN_0048b6e0`, notification `0x231`):
  TEXTSTRA `0x7200` / `0x7201` on background `0x42a`, or `0x42b` when the
  side record's `+0xb0` bit 8 is set.

## Port

- `jabba.rs`: `darth_pickup` and `JabbaStep::Pickup`, which dispatches the
  mission and aims it at the palace.
- `missions.rs`:
  - `MissionKind::Pickup`: record `0x73000082`, TEXTSTRA 11394, chance
    100, 0 days;
  - the hit's `PickupInProgress` and `PrisonerPickedUp` effects;
  - `leave_abode` autoroutes each picked-up prisoner, still captive, to
    the system the mission left from.
- `story_messages::jabba_delivers_message`. The app posts it on
  `PickupInProgress`, and the fixture posts it.
- Telemetry: the `story_jabba` step `pickup`.
- Correction to the previous stage's note: `FUN_004ef260` reads the
  character's own side. The port's held rescuer still stays in the palace;
  that is tagged `hyp`.
- No save format change. The seed-42 golden is unchanged.
- port:
  - Vader's home is the system he left from;
  - Vader's resign request when he is on another mission is not ported;
  - the message goes to both sides on `0x42a`, because the side record's
    bit is not traced.

## Tests

- `jabba.rs`: `darth_pickup_waits_for_the_four_held_and_a_free_vader`.
  It checks each condition, Chewbacca destroyed, Vader autorouting, and
  the day's step.
- `missions.rs`:
  `the_pickup_takes_the_palace_s_prisoners_to_where_vader_came_from`.
  Only the prisoner held for the Empire and on no mission is taken. The
  result is a success, the prisoner arrives captive at Vader's origin a
  day after the end, and Vader stays.
- `story_messages.rs`:
  `jabba_delivers_prisoners_reads_its_two_texts_on_its_background`.
- Shown failing without the change (`fail-without.txt`): with the hit,
  the autorouting, the check and the background reverted, all 3 tests
  fail.

## Campaigns

None reaches the palace. Han evades the hunters in all 24 seeds of the
previous stage (`2026-10-18-palace-rescue`), and this stage does not
change that path.

## Browser

- The story-message fixture (Alliance, code 312) lists "Jabba Delivers
  Prisoners", checked with `agent-browser`:
  - text: "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth
    Vader.  They are now prisoners of the Empire.";
  - background `0x42a` (1066).
- Screenshot not committed (original artwork):
  `f39a3cd0915ef0c8caf479ce18ed8cc155cfef58d120054cd3d719084b97e42a
  target/acceptance/jabba-palace/jabba-delivers.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `db5e7074f09d13496d055a7032557c65c5ae3c4f64e0cf56d53bfe060b67d660`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.5 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 3.2 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2393 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Replay equivalence passes with the unchanged golden
  (`v1:5474ae69134d1255` -> `v1:4cd75bad1e5062b8`) and 0 browser errors.
  Release WASM SHA-256
  `d537273c1c59ae2e7873828f757ff218aeb6d83bd41c5ff8540032328066c13d`.
- `cargo mutants --in-diff` (`jabba.rs`, `missions.rs`):
  - first run: 54 mutants, 1 missed. It was a redundant abode test, now
    folded into the pattern;
  - second run: 53 mutants, 48 caught, 5 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from
  the story builders' shared `implicit_hasher` and the integrator's
  no-op arm.

Hashes of the committed files are in `SHA256SUMS`.
