# The Scrap order (F-036)

Date: 2026-10-14.

The AI's modules 1, 3, 6, 7 and `0xa` issue Scrap (`0x200`). It was
missing from the port. `ghidra/notes/scrap-order.md` traces it:

- **The order:** factory `FUN_0053dd20`, vtable `0x006619b0`. It builds
  one command per team member (vtable `0x006695a8`).
- **The check:** the object's slot `+0x5c`. For facilities, capital ships
  and regiments this is the object check `FUN_004f9860`: the class lists
  `0x200`, the object is the ordering side's, completed, not destroyed and
  not en route. A headquarters (`FUN_00526940`) refuses with `0x17`.
- **The execution:** slot `+0x98` → `+0xa0` = `FUN_00534c00`, which
  destroys the object with reason `0x14` (`FUN_004f71d0`, `FUN_004f7560`).
- **The refund:** `FUN_00530270` adds half the class's refined cost
  (`+0x48`) to the side's refined stock. It applies to destroyed reasons
  `0x14..0x16` on a completed object.

  This corrects `side-resource-monitors.md`, which read `0x14..0x16` as
  object types and the refund as applying to every destruction.

The manual (p. 86) agrees: scrapping returns the maintenance and "some of
the refined material".

## Port

- `crates/rebellion-core/src/scrap.rs`: `check` and `scrap` for
  facilities and regiments on a planet. Refusals: `0x12` unresolved, 1 not
  the side's, `0x17` headquarters.
- `AIAction::Scrap`, applied by the headless integrator (with an
  `AiAction` telemetry record `"type": "Scrap"`) and by the interactive
  loop (a message-log line).

No module issues it yet, so the simulation is unchanged.

## Gates

- `cargo test --workspace --no-fail-fast`: 1829 passed, 0 failed. New
  tests:
  - `scrap.rs`:
    - `scrapping_a_facility_removes_it_and_refunds_half_its_refined_cost`
    - `each_facility_kind_and_a_regiment_can_be_scrapped`
    - `another_side_s_object_is_refused_and_kept`
    - `a_neutral_facility_is_no_side_s_to_scrap`
    - `a_missing_object_or_a_regiment_off_the_planet_does_not_resolve`
    - `a_headquarters_object_refuses_scrap`
    - `a_class_with_no_record_refunds_nothing`
  - integrator: `an_ai_scrap_removes_its_own_facility_refunds_and_reports_it`
  - save: the v39 round trip keeps a pending Scrap.

  With the refund and the facility removal disabled, 3 `scrap.rs` tests
  and the integrator test fail.
- Seed-42 golden unchanged (`v1:33233aedba63db5c`).
  `scripts/check-replay-equivalence.py --skip-build --json`
  (`replay-equivalence.json`) passes on nine checkpoints, with 0 browser
  errors. WASM SHA-256
  `2e7ca6e4b8c785046bcadae8d249843a809e8530438fc44de6a9a762caf51e96`.
- `cargo mutants --in-diff`: 25 mutants, 21 caught, 4 unviable, none
  missed.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: `scrap.rs` is clean;
  no findings in the other changed lines.

Still open:
- Scrap of ships, squadrons, fleets and regiments aboard a fleet.
- The player's Scrap menu item, still disabled.
- The refund for reasons `0x15` and `0x16`.

Hashes are in `SHA256SUMS`.
