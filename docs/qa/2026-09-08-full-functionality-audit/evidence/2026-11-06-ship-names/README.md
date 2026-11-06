# Numbered ship names and the ship a character rides (2026-11-06)

## A0

- **Fleet window.** `fleet2/umgul-fleet1-ships` lists "Corellian Corvette 1" and "Medium Transport 1". `selonia-fleet4-fighters` lists "X-wing" and "Y-wing": squadrons have no number.
- **Ship Finder.** `personnel-states/27` lists "Death Star" with no number, and "Galleon 1".
- **Personnel Finder, Empire game.** The day-4 rows, the first capture in `mission-reports/` work, read "Darth Vader - Carrack Light Cruiser 1", "Dorja - Carrack Light Cruiser 1", "Needa - Carrack Light Cruiser 2" and "Piett - Galleon 1": a character aboard names its ship.
- **Personnel Finder, captive.** `personnel-captured/finder-day628` reads "Jan Dodonna - Punisher ( Captured )".

## The source

`FUN_00517760` handles event `0x300` and also runs over every object at load through `FUN_0051b7f0`.
- **Which objects.** It numbers an object whose class answers slot `+0x48` and whose name is still its record's.
- **The number.** It takes the side's counter for that record (`FUN_005302c0`), which never goes down.
- **The name.** It writes "{record name} {N}" through the name setter `FUN_004f6e60` (`ghidra/notes/fleet-names.md`).

The Personnel Finder row (`FUN_00465bb0`) names the character's container. For a character aboard, that container is a ship.

## Before

- The port named a ship by its class alone ("Corellian Corvette").
- It named a character aboard by its fleet ("Darth Vader - Fleet 1").

## Now

**`GameWorld::number_ships`.** It gives every unnamed capital ship except the Death Star the name "{class} {N}". N comes from a counter per side and class, `FleetNames::ship_numbers`. This is a **save format change: v112 → v113**; older saves are rejected, as the save policy says.

**The Personnel Finder.** A character aboard names the ship it is seated on (`Character::ship`). One not yet seated names its fleet.

**hyp.**
- Slot `+0x48` answers for the capital ships other than the Death Star.
- The ship order of a new game is the port's seeding order, so a start-of-game ship may take a different number than in A0.

## Tests

- `number_ships_names_each_unnamed_capital_ship_by_its_sides_class_count` covers numbering per side and class, a named ship, the Death Star, and a number never given twice.
- `a_character_aboard_names_its_ship_and_one_unseated_its_fleet`.
- **Mutants.** `number_ships`: 10 caught. `character_text`: 3 caught.
- **Workspace and fixture tests** pass with the numbered names.
