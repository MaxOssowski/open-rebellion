# A fleet's arrival message against A0 (2026-11-06)

This covers EVT-01-C007 ("arrival").

## A0

`message-rail/fleet-arrives-averam`, a new Expert Alliance game: the Message Index shows "Fleet Arrives at Averam" over picture 1018, with the body "Fleet 3 has arrived at Averam.".

## The source

`FUN_004981c0`, the Unit Arrival class (notification 0x105 clearing):
- **Case 1.** An object of family `0x08..0x0f`, a fleet, takes TEXTSTRA `0x7078` "Fleet Arrives at {2:1}" and `0x7079` "{1:1} has arrived at {2:1}.". The first substitution is the fleet and the second the system. The picture is `0x45a` and the category `0x80`, Fleet.
- **Cases 2 and 3.** Ships and units take `0x7076`/`0x7077` and `0x707a`/`0x707b`, which the port already used (`arrival_texts`).

## Before

The port posted a fleet's arrival with its own English body, "Fleet arrived at Averam", and no title. The Message Index listed it by that body.

## Now

- **`story_messages::fleet_arrival_texts`.** It fills `0x7078` and `0x7079` with the fleet's and the system's names. The app titles the message with the first and writes the second as its body. The picture and the Fleet rail stay as they were.
- **Unit test.** `a_fleets_arrival_names_the_fleet_and_the_system` reads "Fleet Arrives at Averam" / "Fleet 3 has arrived at Averam." from the shipped templates and nothing without them. Mutants: 5 caught.
- **The b86 batch.** It ran all 21 Linux gates, and every one passed. Its in-diff mutants: 5 caught.
