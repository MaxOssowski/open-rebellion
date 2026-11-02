# Build Selection refuses an order over the side's maintenance (2026-11-02)

Surface ledger OBJ-08 (Build Selection): C008 "unavailable" and C013
"insufficient resources".

## A0

These are the captures from `2026-11-01-manufacturing-pages-a0`, all under
`.artifacts/a0-vm/`:

| Capture | sha256 | State |
|---|---|---|
| `manufacturing/11-build-selection.png` | `95b28744…` | The Empire, day 159. Its maintenance reads -402. Build on Coruscant's troops band shows "n/a" in all four values and a grey Confirm. The window is at (255, 87). |
| `manufacturing/12-build-selection-facilities.png` | `2305fa87…` | The same for the facilities band. |
| `manufacturing/20-alliance-overview.png` | `762f2654…` | The Alliance at Vjun, day 9. Its counters read raw 0, refined 9 and maintenance 239. |
| `manufacturing/22-alliance-build-selection.png` | `a9c49a91…` | The Alliance's Build Selection for a Construction Yard, also "n/a" with a grey Confirm. |

## The original

`FUN_00538220` validates the order (`vt+0x1c`) before it prices the times.

- **Classes.** `FUN_0051f4b0` registers the build orders 0x210, 0x211 and 0x212 with the constructors `FUN_00538cc0`, `FUN_00538d90` and `FUN_00538e60`, on vtables `0x00661148` and `0x006611a0`. They share `vt+0x1c` = `FUN_0051ff30`, which calls `vt+0x50` = `FUN_005385f0`.
- **Side object.** `FUN_005385f0` takes the producer's side object through `FUN_00506ea0`.
- **The check.** It then calls `FUN_0052e970(side, class, quantity)`:
  - cost = the class's `vt+0x24` × quantity. On the class vtable `0x0065c380` that slot is `FUN_0053b870`, the maintenance cost.
  - unallocated = side `+0x58` − `+0x5c`, the maintenance capacity less its allocated half.
  - The order fails when cost ≠ 0 and unallocated < cost. The status becomes `{0xf3, 1}`.
- **The display.** Any status other than -1 makes `FUN_00439160` write 14358 "n/a" in all four values and disable Confirm.
- **Disabled art.** `FUN_00438620` gives Confirm's states 2 and 4 STRATEGY 11620 (`FUN_00603150(2, 0x2d64)`, `(4, 0x2d64)`). That is the grey checkmark. Against A0 `11`, 2070 of its 2178 pixels match. The other 108 pixels are its magenta first-pixel key (index 174), where A0 shows the window.

## The port

- **Allocated maintenance.** `MaintenanceTotals::allocated` is side `+0x5c`, and `unallocated` is `+0x58` less it.
  - hyp: the mine and refinery rebalancers (`FUN_0052f6b0`, `FUN_0052f8f0`) have settled, so the allocation is the upkeep, bounded by the capacity.
- **The check.** `build_selection::exceeds_maintenance` is `FUN_0052e970`'s test. `estimate` takes the side's unallocated maintenance and leaves `completion` empty for a refused order, which shows "n/a" and greys Confirm.
- **Wiring.** `main.rs` passes `resource_monitors::maintenance_unallocated`, with blockaded and uprising systems left out as for the monitor.
- **Disabled Confirm.** It draws 11620, keyed on its first pixel (`bmp_cache`). Before this change it drew its rest art 10594, which A0 contradicts.
- **Fixture.** Scenario `BuildSelectionOverMaintenance` (87, code 88) is `BuildSelection` with the player's mines and refineries removed, which leaves no capacity.
- **Gate.** `build-selection.mjs` opens Build Selection from the troops band for each side and compares it with A0 `11`. It checks:
  - Confirm's 66 by 33 pixels, which must be equal;
  - the four "n/a" ink boxes, each within 2 pixels;
  - a click on Confirm, which must build nothing and leave the window open.

## Open

- **The Alliance's refusal at Vjun** (`22`..`26`) is not explained by this check. The monitor reads 239 and the Construction Yard's maintenance is 10. The later checks in `FUN_005385f0` (`FUN_005513a0`, `FUN_00552210` and `FUN_00551a20`, which try the new object at the system) are untraced.
  - hyp: Vjun had no free energy slot.
- **No VM run reached a fresh Alliance Build Selection.** On 2026-11-02, in the `test` save on day 147, C-3PO's Build Troops on Chandrila opened nothing.
- **Refined material.** The port still does not draw it down. The original refuses at Build Selection on maintenance alone.

## Tests

- **Core:**
  - `an_order_whose_maintenance_exceeds_the_sides_unallocated_is_not_valid` (`rebellion-core` `build_selection.rs`);
  - `orders_may_take_what_the_allocation_leaves_and_nothing_once_it_is_full` (`side_resources.rs`).
- **Render:**
  - `an_order_over_the_sides_maintenance_shows_na_and_greys_confirm` (`rebellion-render` `build_selection.rs`);
  - `build_selections_disabled_confirm_is_keyed_on_its_first_pixel` (`bmp_cache.rs`).
- **Changed test.** `the_window_paints_its_background_side_strip_and_controls` now expects 11620 for the Empire, which has no yard in that test. This follows A0 `11`.
- **Fixture.** `the_over_maintenance_scenario_leaves_the_side_no_capacity_and_its_yards` (`interface_test_fixture.rs`).
- **Without the change**, the core and render refusal tests fail: the order stays valid.
