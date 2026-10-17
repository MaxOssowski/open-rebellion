# Character Injured and Healed messages (F-038)

Date: 2026-10-17.

The previous step stored injuries and healed them. This step posts the
messages the original shows for them, and checks the injury's other side
effects against the port. The reading is in
`ghidra/notes/character-injury.md`.

## The original

`FUN_0048fdf0` builds notification `0x106` for a character (family
`0x30..0x3b`):
- with `+0x94` not 0: "{name} Injured" / "{name} has been injured."
  (TEXTSTRA `0x7160`/`0x7161`), picture `0x41a`;
- with `+0x94` at 0: "{name} Healed" / "{name} has been healed of all
  injuries." (`0x7162`/`0x7163`), picture `0x41b` with the portrait;
- a destroyed character gets the Killed message instead.

## Port

- `push_character_health` (app) posts the message on the Mission rail to
  the character's side (`character_audience`, as the Killed message does).
- It runs when a mission or the uprising incident injures a character, and
  when a character's healing ends.
- hyp: the builder runs on every change of `+0x94`, including each day of
  healing; the app posts only the injury and the end of healing.
- The injury hook's resign request needs no port. The port's missions
  already resign an evading member, a captured one leaves, and the uprising
  incident injures only characters outside missions (`usable_at`).
- Not ported: the command clearing (the port keeps no command `+0x96`), the
  `Damaged` bit, and death by injury for the `0x00665be0` subclass.

## Tests

- `character_health_tests::an_injury_posts_injured_and_full_healing_posts_healed`
  checks both texts and pictures.

## Gates

- `cargo test --workspace`: passes, 0 failed.
- The simulation is unchanged: the replay equivalence check gives the same
  fingerprints (initial `v1:27f93b3cb81b6605`, final
  `v1:29ab46197f1887c6`) with 0 browser errors.
- Release WASM SHA-256
  `305abf55805477931744684029b8c9a763d72950d486fd22bb6b876565dc7511`.
- The release build ran with `agent-browser`: 60 s at Very Slow and 60 s at
  Fast. The 99th percentile is 0.7 ms and 1.5 ms, and no frame took over
  50 ms. The console has 152 `[ai]` lines and 0 errors, and the 4 requests
  are HTTP 200. No injury happens in those two minutes, so the messages
  were not seen in the browser. Native GUI acceptance was not run.
- `cargo mutants --in-diff`: 8 mutants, 2 caught, 6 missed.
  - `main` and the main loop's `heal.injury == 0`: the interactive loop has
    no unit tests.
  - `apply_mission_result` (`injury > 0`, and the whole function): it needs
    an `AudioEngine`, which opens an audio device, so the app's tests do not
    call it.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
