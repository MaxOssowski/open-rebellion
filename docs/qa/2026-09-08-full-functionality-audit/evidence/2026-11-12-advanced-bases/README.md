# Advanced bases as built (2026-11-12)

Surface ledger cell: CMD-09-C010 (advanced variants). This stage covers the
part its note left open: "The advanced bases as built have no A0 or browser
check." It adds the browser check. A0 is still missing.

## The original

`FUN_00458fe0` picks a facility page's picture. For families 0x28..0x2b it
switches on `FUN_00539f50`, the class's record number in MANFACSD.DAT:

| Record | Class | Base |
|---|---|---|
| 1 | Orbital Shipyard (0x2800_0001) | 0x232e, 9006 |
| 2 | Training Facility (0x2900_0002) | 0x2336, 9014 |
| 3 | Construction Yard (0x2a00_0003) | 0x233e, 9022 |
| 4 | Advanced Shipyard (0x2800_0004) | 0x2332, 9010 |
| 5 | Advanced Training Facility (0x2900_0005) | 0x233a, 9018 |
| 6 | Advanced Construction Yard (0x2a00_0006) | 0x2342, 9026 |

A built object (`+0x50` bit 2 set, bit 4 clear) takes the base with no
offset. The side changes only the under-construction picture (+ 2 for side
1, + 1 otherwise). So both sides show the same built picture. The records
and families are read from `data/base/MANFACSD.DAT` (records 4, 5 and 6 have
families 0x28, 0x29 and 0x2a).

## The port

`Scenario::AdvancedBases` (107, fixture code 108) in
`crates/rebellion-app/src/interface_test_fixture.rs` replaces the primary
system's yards with one built advanced base of each kind, owned by the
player. The System window opens at (225, 76), as in the Build Selection
scenario. Each page's first cell then holds the advanced base. The unit test
`the_advanced_bases_scenario_shows_each_built_advanced_base_first_on_its_page`
expects `[9010]`, `[9018]` and `[9026]` for both sides. Without the
placement it fails: the Shipyards page is empty. `cargo test -p
rebellion-app` passes (160) and `cargo check --workspace` is clean.
`cargo mutants` on the new lines catches every mutant of `apply`'s branch
and `place_advanced_bases`. It misses the five mutants of `emit_production`,
which only posts to the page through the WASM import; the browser gate
covers it, since the gate waits for that report.

`tools/interface-parity/advanced-bases.mjs` opens the Shipyards, Training
Facilities and Construction Yards tabs for each side. It compares every
opaque pixel of the 67 by 35 STRATEGY bitmap at the window's (9, 79), the
same place `build-selection.mjs` checks 9006, 9014 and 9022.

Run `advanced-bases-2026-11-12T12-31-38-407Z-1339053` (Linux Chromium
155.0.8059.84, `--no-build` after `scripts/build-interface-test-wasm.sh`):
pass.

| Side | Page | Picture | Opaque pixels | Off |
|---|---|---|---|---|
| Alliance | Shipyards | 9010 | 2344 | 0 |
| Alliance | Training Facilities | 9018 | 1222 | 0 |
| Alliance | Construction Yards | 9026 | 1193 | 0 |
| Empire | Shipyards | 9010 | 2344 | 0 |
| Empire | Training Facilities | 9018 | 1222 | 0 |
| Empire | Construction Yards | 9026 | 1193 | 0 |

A scan of the six screenshots found 9010, 9018 and 9026 only on their own
pages, at screen (234, 155), and none of 9006, 9014 or 9022.

```
000e1c013c206eda7230c7d24eef7aac692f1e040d9a86c42c65154340508b76  alliance/advanced-shipyard-page.png
a7643051d2a98541af2406d7d9b70389591fb75eb89f6d7791529643c3713aa2  alliance/advanced-training-facility-page.png
c37fe596cde746016086676bb55ee9fba334284da6f67deb4a799fa4182b650e  alliance/advanced-construction-yard-page.png
1dbb8a150239524616231e70088dc2ee432f5c51b6edb47124473856e77b4caa  empire/advanced-shipyard-page.png
8729e63ad1d584b4f8bbf819927adb5a23c67fbcb81d14971171d92bc7730056  empire/advanced-training-facility-page.png
acca6254d9c40d761d8f03cbe9067cd26bf51966fcd8b33792b7ab8efcea3a30  empire/advanced-construction-yard-page.png
b43820d086161dd3f9d11b7a3ad366a1b37741816ea8c0ff29d4a0a1c1fddc51  open-rebellion-test.wasm
3f3f6876a255da49d10ec37220f6b492e5a8239b2555497f0cc84ac7e175f98c  data/runtime.orpk
c8b38a46945b2c00d1c466d20967069e6560bec50e44f41de9053f8749e584c0  STRATEGY 9010, 9018, 9026 (aggregate)
```

## A0

No A0 capture shows a built advanced base. The evidence READMEs name none,
and no file under `.artifacts/a0-vm` has "advanced" in its name. A pixel
scan then searched the main checkout's `.artifacts/a0-vm` for 9010, 9018
and 9026 (and 9022 as a control) at every position, every opaque pixel
exact. It read 2512 of the 2736 PNGs, all but the 224 opening-movie frames
in `intro/`. The scan script is not committed; it lived in the session
scratchpad.

| Capture | Picture | At (screen) | Off |
|---|---|---|---|
| `2026-10-28-nav/system-vjun-yards` | 9022 | (236, 165) | 0 of 879 |
| `manufacturing/03-tab` | 9022 | (212, 170) | 0 of 879 |
| `personnel-states/34-mon-calamari-construction-yards` | 9022 | (323, 165) | 0 of 879 |
| `pending-facility/06-tab1` | 9022 | (149, 165) | 0 of 879 |
| `personnel-states/35-xyquine-training-building` | 9018 | (107, 141) | 0 of 1222 |

The control finds every known Construction Yard capture. The one 9018 hit
is not a built base. `personnel-states/35` shows the Advanced Training
Facility under construction, 9020, which matches there over 2189 pixels
(`evidence/2026-11-03-facility-pictures-a0/`). 9018's 1222 opaque pixels lie
within 9020's, so an opaque-only match cannot tell them apart. The gate
therefore also requires each cell to differ from base + 1, + 2 and + 3.

The second run, `advanced-bases-2026-11-12T12-57-07-411Z-1619131`, adds that
check. It passes with the same six screenshots, byte for byte, and the same
WASM. The cells differ from the unbuilt pictures in:

| Picture | + 1 | + 2 | + 3 |
|---|---|---|---|
| 9010 | 711 of 2147 | 711 of 2147 | 707 of 2344 |
| 9018 | 406 of 2189 | 406 of 2189 | 1581 of 2344 |
| 9026 | 415 of 2163 | 415 of 2163 | 627 of 2344 |

The gate has an `A0_PAGES` table for such a capture. It is empty, so the
gate compares nothing with A0 now. A file listed there that is missing fails
the gate instead of being skipped.

## Still open

- No A0 capture shows a built Advanced Shipyard, Advanced Training Facility
  or Advanced Construction Yard. The built pictures rest on `FUN_00458fe0`,
  MANFACSD.DAT and the browser check. Needed: in a game late enough to
  research them, each finished base on its System window page, for at least
  one side; then add the capture and the window's place to `A0_PAGES`.
- The new gate is not in the batch list
  (`target/claude-tmp/keep/batch.sh`, the browser.json gate loop).
