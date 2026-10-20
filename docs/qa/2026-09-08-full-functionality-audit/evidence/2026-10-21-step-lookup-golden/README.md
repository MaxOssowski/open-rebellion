# The step lookup's golden change is named (F-028)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The open item

- `FUN_00595090` returns the value of the largest threshold at or below
  its argument and clamps below the first row.
- `MstbTable::step_lookup` does the same. Every table lookup uses it
  since `d6adacfe` (2026-10-14), which removed the interpolating
  `MstbTable::lookup`.
- F-028 stayed open because the golden change was not named: on that
  tree the seed-42 replay already failed for another cause (it loaded
  100 systems), so the golden was not regenerated.

## What the history shows

- `83facd86` (2026-10-14 07:43) is the first golden regenerated after
  `d6adacfe`, on the full galaxy.
- Its message names the step lookups among its causes: "the step
  lookups, AdvanceTicks and AI research gate (fb89f98a, d6adacfe,
  78c39acd, 5d163de0)".
- Every golden since was recorded with every lookup stepping.

## Measured on the current tree (`92b38a3a`)

- `step_lookup` was replaced for the run by the removed interpolation.
  Then the code was restored; nothing in this commit changes code.
- The seed-42 replay golden still passes under interpolation
  (`replay-interpolated.txt`). Its commands never make a table lookup
  fall between two rows, so the step lookup moves it by nothing. The
  golden does not guard this rule; the campaigns and the unit tests do.
- All 24 dual-AI campaigns (1500 days) diverge under interpolation,
  between day 21 and day 168 (`campaigns-interpolated.txt`).
  - Example: in seeds 8 and 15 on day 21, an Empire special force
    ordered home is still in the world under interpolation. With the
    step lookup it is already gone, so the telemetry names its stale key.
- The unit tests that fail under interpolation stand:
  - `step_lookup_takes_the_last_threshold_reached_and_clamps_below_the_first`;
  - the uprising, betrayal and escape tests that F-028 records as
    failing under interpolation.

## Result

The golden change of the step lookup is named (`83facd86`) and measured
as zero on today's golden. F-028's acceptance is met.

Hashes of the committed files are in `SHA256SUMS`.
