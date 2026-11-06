# The Personnel Finder's captured row (2026-11-06)

This covers OBJ-03-C009 ("captured").

## A0

`.artifacts/a0-vm/personnel-captured/finder-day628` (the Expert Alliance game) reads "Jan Dodonna - Punisher ( Captured )". The captive's container is the Imperial ship that holds him. The state word sits in " ( " and " ) ".

## The source

`FUN_00465bb0` builds the row: the name, " - ", the container's name, then the state word `0x8747` "Captured", placed first (`personnel-finder` notes).

## Browser (Linux Chromium, `personnel-states.mjs`)

The PersonnelStates fixture adds the side's fourth character, where it has one, as a captive aboard a ship of the player's first fleet. Ships are now numbered (`2026-11-06-ship-names/`).

The Alliance Characters rows read "Han Solo - Imperial Star Destroyer 1 ( Captured )" (`characters.png`). The gate checks three things:
- exactly one Captured row;
- the row names a numbered ship;
- the row does not name a fleet.

The Empire's fixture has no fourth character.

## Tests

`a_character_aboard_names_its_ship_and_one_unseated_its_fleet` and `a_rows_state_word_sits_in_spaced_brackets_and_captured_comes_first`.
