# Future Jedi Discovered and Leia Uses Force (F-038)

Date: 2026-10-18.

Timer `0x389` (previous step) makes characters Force aware but posted no
message. The original sends notification `0x362` for each discovery. The
reading is in `ghidra/notes/force-awareness.md`.

## The original

Setting the discoverer `+0xa4` syncs to the side views (`FUN_004f0940`),
which send `0x362` (`CharacterForceUserDiscoveredKeyNotif`, slot `+0x3bc`,
`FUN_004f20e0`). Its builder `FUN_0048f1e0`:
- needs both objects to be characters (families `0x30..0x3b`) and shows
  only to the trainer's side (`FUN_004edc80` against the view's side);
- puts the trainer's portrait (`FUN_004c5000`, `edi`, the first object) on
  the side's background, `(side != 1) + 0x412`;
- for Leia (`0x31000241`) found: "Leia Uses Force" (TEXTSTRA `0x71d8` /
  `0x71d9`), with Leia's portrait;
- for anyone else: "Future Jedi Discovered" (`0x7178`) and the trainer's
  words about `{1:1}`, the character found. The body is `0x7179` ("We
  should consider allowing me to conduct training") when `FUN_004ed310`
  holds for the trainer (a Jedi trainer at Force GNPRTB 2581 (100) or
  more, `FUN_0053eb90`), else `0x717a` ("Once I reach the rank of Jedi
  Knight").
- `+0x28` (a per-character id `0x15..0x1b`), `+0x30` (`0x47c`/`0x47d`) and
  `+0x32` are set but not read here.

The disassembly gives the trainer as the object in `edi`. It is used for the
portrait, for the side test and for `FUN_004ed310`
(`0x48f34f..0x48f428`). The text confirms it: the speaker is the trainer.

## Port

- `story_messages::force_user_discovered` builds the message. The app posts
  one per discovery (`push_force_discoveries`), on the Mission rail
  (`FUN_0048f0e0`) for the trainer's side.
- Fixed with it:
  - The Character Healed picture looked up the portrait with the
    character's side in place of its major flag (`push_character_health`,
    previous step). An Empire major character got no portrait.
  - The Force Growth message read the Force ranking from the class
    template; it now reads the Force level `+0x8c`.
- Interface fixture scenario 55 also posts both discoveries: Vader finds
  an Empire minor character, and Luke finds Leia.

## Tests

- `story_messages.rs`:
  - `a_discovery_tells_the_trainers_side_what_it_found` (Empire background
    `0x413`, Vader's portrait, both bodies, Force 100 exactly);
  - `leia_found_uses_the_force_in_her_own_words`.
- `main.rs`:
  - `a_force_discovery_posts_future_jedi_discovered_to_the_trainers_side`;
  - `an_injury_posts_injured_and_full_healing_posts_healed` now checks
    Vader's Healed portrait. It fails with the old side-for-major lookup
    (tried).

## Browser: the messages seen

Interface fixture build (`--features interface-test-fixtures`), scenario
55, WASM
`b6ea995a1333059e017f465ddd741a59c7afeae5627fb09bceadb8fb25a81149`,
driven with `agent-browser` at 640 by 480. Click the lit Mission category,
select the row, then Display.

| Reader | Rows | Message shown | Picture |
|---|---|---|---|
| Alliance (`?fixture-code=312`) | Leia Uses Force (port `0x363`), Jabba Captures Chewbacca, Leia Uses Force (`0x362`) | "Leia Uses Force", body `0x71d9` | background `0x412` (1042), Leia's portrait `0x1841` (6209) |
| Empire (`?fixture-code=568`) | Leia Uses Force, Jabba Captures Chewbacca, Future Jedi Discovered | "Future Jedi Discovered", "I have detected that Jerjerrod has the ability to use the Force.  We should consider allowing me to conduct training in the way of the Force." | background `0x413` (1043), Vader's portrait `0x1881` (6273) |

- Each reader sees only its own side's discovery.
- Vader's body is `0x7179`, as his Force is 120.
- The screenshots show original artwork, so they are not committed. They
  are in `target/acceptance/force-discovery-messages/`, and their checksums
  are below.
- In a campaign no discovery happens (previous step), so the messages were
  not seen in live play.

```
b439d2f368ce8743c487f73f78debd59f5c33849b98c00a61518a3c6401619d6  alliance-leia-uses-force.png
71e6d73771ed987c853501a1600e43343803203e3cb373b36486c946aa0dc9d7  alliance-list.png
37801fcf08a4a2d67cf0c50dd6b7b593709b27db8adcc9f87769a33f2a3ed95b  empire-future-jedi-discovered.png
760e10d486cc947d2d342771754e2c13ddd7b3a6d48bcf609d75118f994b7556  empire-list.png
```

## Gates

- `cargo test --workspace`: 2305 passed, 0 failed.
- The simulation is unchanged: the replay equivalence check gives the same
  fingerprints (initial `v1:58473b5bf4582374`, final
  `v1:461650933b75014c`) with 0 browser errors.
- The Fleet window gate passes all 48 cases on the fixture build above.
- Release WASM SHA-256
  `4d0865053d98e3e1da249c174a1a43793680f49450aaec1ffc7168f06998690a`.
- The release build ran with `agent-browser`: 60 s at Very Slow and 60 s at
  Fast.
  - The 99th percentile is 0.7 ms and 1.5 ms, and no frame took over
    50 ms.
  - The console has 152 `[ai]` lines and 0 errors, and the 4 requests are
    HTTP 200.
  - Native GUI acceptance was not run.
- `cargo mutants --in-diff` (render and app): 9 mutants, 6 caught, 2
  unviable. The one missed is `main` as a whole, the interactive loop the
  unit tests do not run. A Force 100 case was added after the run, for the
  `>=` of the Knight test.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021) on the changed lines: one
  finding, `implicit_hasher` on the texts map. The neighbouring
  `story_message` and `story_text` take the same type, so it is kept.

Hashes of the committed files are in `SHA256SUMS`.
