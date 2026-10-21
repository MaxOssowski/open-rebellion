# A system its support hands to the other side is reported, in the browser

Date: 2026-10-21 (overnight run). This is the browser pass for
`dfb840f2` (`evidence/2026-10-21-system-joins-message/`).

## The original

- `FUN_0050b610` gives a system with no regiment the side its support
  favours.
- When that changes the holder with no regiment side on either word,
  `FUN_00510d70` sets LoyaltyCausedCurrentControlKind.
- System Joins (`FUN_00499760`, `0x100`) then speaks. A system of the
  player's going to the other side reads TEXTSTRA `0x7039` "{1:1} Joins
  Enemy".

## The gate

- A new fixture scenario, `SupportFlip` (77, code 78), is `FleetLoad`
  with two changes:
  - the loading target's (F'tral's) support is all the other side's;
  - the clock runs Fast.
- F'tral has no regiment, so the economy's first day hands it over.
- The observation adds the player's System Joins title for the loading
  target.
- `fleet-window.mjs` `joins` waits for "F'tral Joins Enemy". It passes for
  both sides, with 0 page errors and 4 requests, all HTTP 200
  (`joins-browser.json`).
- A first run, which matched any "Joins Enemy" title, saw Talus and
  Norval II instead. The fixture's other systems of the player's have no
  regiment and low support either, and the fixture skips the setup pass,
  so they flip on day 1 too. The observation now names the target.

## Tests

- New: `the_support_flip_scenario_gives_the_target_s_support_to_the_other_side`.
  It also covers the observation's filter, for "Joins Enemy" and "Joins".
- `cargo mutants --in-diff` on the fixture:
  - the first run missed two;
  - after the test's fixes, the rerun caught 17 of 19 (2 unviable).

## Gates (`f42`)

- The Fleet window suite passes 58 cases (the 56 before, plus `joins`
  for both sides). The Status window gate passes.
- The 24 dual-AI campaigns and the seed-42 golden are unchanged; the
  release WASM is
  `dc7af1ae64c042c5609cc83ef96f3b535e22477257c8f8cf13e280c4b2fbd025`.
- The frame-time run failed once with a blank page (agent-browser) and
  passed on its rerun:
  - no frame over 50 ms;
  - 152 `[ai]` lines and 0 errors in the console (`console.txt`,
    `network.txt`).

Hashes of the committed files are in `SHA256SUMS`.
