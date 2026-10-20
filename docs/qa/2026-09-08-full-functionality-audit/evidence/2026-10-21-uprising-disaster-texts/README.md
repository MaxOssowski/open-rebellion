# The uprising and disaster messages take the original texts

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- Uprising Begins / Ends (`FUN_00499460`) answers notification `0x14b`
  with the system's uprising bit.
  - Both sides' classes speak for a system a side holds; none speaks for
    a neutral one.
  - A beginning takes TEXTSTRA `0x7030` "Uprising Begins on {1:1}" and
    `0x7033` "An uprising has begun on the {1:4} controlled system
    {2:1}.". An end takes `0x7031` and `0x7032`.
  - The body's first substitution is the holder, a Side.
- The Side's text provider (`FUN_0052d920`, vtable `0x660ae8`) names a
  side from TEXTSTRA's string table:
  - kind 1: `0x4d50` / `0x4d51`, "Alliance" / "Empire";
  - kind 2: `0x4d60` / `0x4d61`, "Rebel Alliance" / "Galactic Empire";
  - kind 4: `0x4d70` / `0x4d71`, "Alliance" / "Imperial".
- Natural Disaster (`FUN_0048e940`) answers `0x154`, then `2`, for the
  holder's side only. The title is `0x7190`, and the body names what was
  lost:
  - `0x7191`: resources only;
  - `0x7192`: units only;
  - `0x7193`: both.
  Each destroyed facility (`0x302`, family `0x20`..`0x2f`,
  `FUN_0048e890`) adds a line `0x719b`. With nothing lost there is no
  message.

## What the port did before

- It posted its own lines: "Uprising at X!", "The uprising at X has
  ended" and "Natural disaster strikes X".
- The uprising lines went to the holder only.

## Port

- `rebellion_data::message_texts` adds the six side names from the
  string table (`SIDE_NAME_STRINGS`). A message text of the same id is
  kept.
- `story_messages::MessageParameter::Side` gives kinds 1, 2 and 4, and
  `side_parameter` builds one.
- `uprising_texts` and `disaster_texts` return the original title and
  body. The app files the uprising message for both sides and the
  disaster for the holder; a disaster with nothing lost posts nothing.
- port: a disaster on a neutral system keeps the port's line, off the
  Message Index.
- hyp: `resources` stands for the class's bits 2 and 4 (`+0x64`), set
  by the energy and raw material changes. Their setter is not traced.
- "Uprising incident at X" has no message class and is unchanged.

## Tests

- New, in `story_messages`:
  - `an_uprising_names_the_holder_s_side_when_it_begins_and_ends`;
  - `a_disaster_lists_what_it_destroyed_and_says_nothing_when_nothing_was_lost`.
- New, in `rebellion-data`: `the_side_names_join_the_message_texts_without_replacing_one`.
- New, on the player's data: `the_message_texts_carry_the_side_names_from_the_string_table`.
  It fails without the merge (`fail-without.txt`).
- `cargo test --workspace`: 2495 passed, 0 failed.
- `cargo mutants --in-diff`:
  - render: all caught;
  - `message_texts`: 6 of 6 caught, by the data test.

## Browser

- The `incident` case now records the disaster's title. On a held
  system it checks the original title and body.
- The disaster's pick is random. The two full-suite runs struck neutral
  systems, so four more `incident` runs were made. Two of their eight
  cases struck a held system:
  - Bothawui (Alliance): "There has been a natural disaster on Bothawui
    which has destroyed facilities and resources.  The following units
    were lost:" followed by "Mine";
  - Akrit'tar (the Alliance player's): "There has been a natural
    disaster on Akrit'tar which has destroyed resources there.";
  - each under the title "Natural Disaster on …".
- Those runs are in `incident-browser.json`, and their screenshot hashes
  are in `screenshots.sha256`. 0 errors, 4 requests, all HTTP 200.
- Not shown in the browser: the uprising's begin and end texts. The
  scenario places its revolt directly; the unit tests cover them.

## Gates (`f22`)

- The 24 dual-AI campaigns are identical; messages are not simulation
  state.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `dbc9bea9323b4cc0bac756298bdff9adda92849d8491384c21a340fcc8d84153`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `provenance-scan check`: 11 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
