# Fleet Arrives and Joins Enemy, whole message against A0

Date: 2026-11-08. Closes EVT-01-C007 and EVT-01-C004.

## A0

- **Fleet Arrives at Sluis Van.** `.artifacts/a0-vm/events-day673/04-fleet-arrives-sluis-van.png`
  (SHA-256 `391fb289…6dade4`) comes from the Alliance game in the Windows 10
  VM. It reads "Fleet 1 has arrived at Sluis Van." under the Fleet rail's
  title image (`FUN_004981c0` case 1, `0x7078` / `0x7079`). Display is lit.
- **Xyquine Joins Enemy.** `.artifacts/a0-vm/joins-clean/test-day215-xyquine-joins-enemy.png`
  comes from the unedited "test" save. It reads "Popular dissent on Xyquine
  has caused that world to join the Empire." (`FUN_00499760`, `0x7039` /
  `0x703c`). The picture and title image were already compared on
  2026-11-06.

## Port

The fleet's arrival message was written inline in the movement loop. It is
now `fleet_arrival_message` in the app, and the movement loop calls it.

The event-message fixture (Alliance) files the arrival of Fleet 1 at Sluis
Van through that function (`interface_test_fixture::arrived_fleet`). The
message goes on the Fleet rail, as the app files it.

`message-display.mjs` (Linux, Alliance) checks:

- **Fleet Arrives:** opens Fleet Messages (the window's (114, 64)), reads
  the exact text, and finds the picture, title image and rail 0 pixels from
  A0 `04`.
- **Joins Enemy:** the rail (Display lit) is now compared with A0 too, and is
  0 pixels from it. The picture and title image were already 0.

These remain without A0 captures:

- Personnel Arrive, Headquarters Arrives and Units Arrive (EVT-01-C007);
- Declares Neutrality (EVT-01-C004).

## Tests

`the_event_messages_scenario_brings_fleet_1_to_sluis_van_for_the_alliance`.
