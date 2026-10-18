# The abodes (ABODESD), first stage (F-038, F-040)

Date: 2026-10-18.

Luke's Dagobah mission targets an abode, a place outside every system,
and the port had none (`ghidra/notes/dagobah.md`). This stage loads them
and lets a character be in one. Missions to abodes come with Dagobah.

## The original

- ABODESD holds five family-`0x98` entries. The galaxy's setup
  (`FUN_00519910`) creates each as a child of the galaxy, with a side:
  - Alliance Jail `0x98000440`, side 1;
  - Dagobah `0x98000441` and Hermitage `0x98000442`, side 3;
  - Empire Jail `0x98000480` and Jabba's Palace `0x98000481`, side 2.
- The class is its own (vtable `0x00669260`, `FUN_00577530`). It has no
  position. Missions treat it as a place like a system (`FUN_00521070`),
  and a trip to or from it takes 1 day (`FUN_00555b30`).

## Port

- `AbodeKey` and `Abode { dat_id, name, side }`, kept in `GameWorld::abodes`
  in file order.
- The loader reads ABODESD (`EntityTableFile`), naming each abode from
  TEXTSTRA. `abode_side` gives the setup's sides.
- `Character::current_abode`: the abode a character is in. Its system and
  fleet are then `None`, so no system walk finds it.
- The Status window's Attached row names a character's abode.
- The browser's legacy data list also names ABODESD.DAT. The runtime pack
  already carries every `.DAT`.
- Save v72.
- Nothing puts a character in an abode yet. Dagobah will be the first.

## Tests

- `tests/abodes.rs` (original data):
  `the_five_abodes_load_with_the_sides_the_galaxy_setup_gives_them`
  checks the five names and sides in file order, and that nobody starts
  in an abode.
- `lib.rs`: `an_abode_takes_the_side_the_galaxy_setup_gives_it`.
- `status_window.rs`: `a_character_in_an_abode_is_attached_to_it`.
- Shown failing without the change: with the loader skipped, or the
  Attached row ignoring the abode, the data test and the Status test fail.

## Campaigns

`rebellion-playtest data/base --ticks 600 --dual-ai`, seeds 1, 2, 3, 7,
13, 42 and 47, against `fd15336e`: all identical.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`f398d4a1515670008bd6b27702be1060c79fb09af90c87273d7ea235c0f909e3`).

The replay equivalence check passes with the new initial fingerprint and
0 browser errors. The fingerprint includes the abodes, so the browser
build loaded ABODESD from the runtime pack.

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.2 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.4 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2317 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v72: the world's abodes
  and the character's abode):
  - initial `v1:d077dd4c3c2b7247` -> `v1:c4aa6c7fc51080c1`;
  - final `v1:1d8dbb48d17c03de` -> `v1:0e985b5a40e2bfa8`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - release WASM SHA-256
    `3f9d87e3ae1bde0ae613468bf4a2edc969f1222378ed9853ac4b32eeee692898`.
- `cargo mutants --in-diff`:
  - the first run left the two `abode_side` arms to the data test, which
    the mutants run skips; the unit test above now catches them;
  - the loader's `load_game_data_with_options` and `convert_character`
    returning defaults stay missed: only the ignored data tests cover the
    loader.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
