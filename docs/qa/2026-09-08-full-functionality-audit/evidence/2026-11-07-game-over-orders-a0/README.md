# The end of the game greys every order (A0)

Date: 2026-11-07.

## A0

The `test` game in the Windows VM, an Alliance game, on day 633:

- `.artifacts/a0-vm/pending-facility/33-pzob-ship-band-menu-day633.png`:
  the menu of Pzob's ships band. Build, Stop, Destination, Rename and
  Reserved are grey. Encyclopedia and Status are white.
- `.artifacts/a0-vm/pending-facility/36-pzob-facility-band-menu-day633.png`:
  the facilities band's menu, grey the same way.
- `.artifacts/a0-vm/pending-facility/35-pzob-facility-band-menu-day591.png`:
  the same menu on day 591, with its orders white.

On day 633 the clock no longer advanced at Medium speed
(`2026-11-04-producer-status`). The Alliance lost this game:
`alliance-defeat` shows its ending.

## The reading

`FUN_0051d990` enables an order only when the global gate `FUN_0051de80`
passes and the order's own `+0x18` accepts it. Encyclopedia (`0x100`) and
Status (`0x103`) are added after that check and do not pass the gate.

`FUN_0051de80` returns bit `0x10000000` of the game object's `+0xc0` →
`+0x74`. `FUN_005180e0` writes that word. It is the victory conditions word,
and it sets bit `0x10000000` when a side has won and the game is over
(`victory::WORD_GAME_OVER`). The port took the gate as always clear.

## Change

- `object_menu::OrderGates::orders_stopped`. While it is set,
  `object_menu_rows` greys every order and leaves Encyclopedia and Status
  as they were, and `object_submenu_rows` greys the Bombardment and Command
  rows.
- `main.rs` `order_gates` takes the victory conditions word and sets
  `orders_stopped` from bit `0x10000000`. All five menu sites pass
  `victory_state.conditions_word`.

## Tests

- `object_menu::tests::the_end_of_the_game_greys_every_order`: a ships band
  with every gate open keeps Build, Stop, Destination, Reserved,
  Encyclopedia and Status. With `orders_stopped` set, only Encyclopedia and
  Status stay enabled, and the submenus' rows are grey.
- `a_bands_stop_and_destination_belong_to_its_systems_holder` (app): the
  game-over word sets `orders_stopped`, and a word without that bit leaves
  it clear.
- cargo mutants: in `object_menu.rs` 7 caught and 2 unviable. In `main.rs`
  4 caught; the one miss, `replace main with ()`, is not covered by any
  unit test.

## Open

In the port the game-over state lasts only until the ending opens
(`pending_ending`), so there is no browser case of a menu in this state.
