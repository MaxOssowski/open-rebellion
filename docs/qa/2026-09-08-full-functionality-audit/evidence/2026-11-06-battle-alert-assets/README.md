# The Battle Alert's System Assets against A0 (2026-11-06)

This covers EVT-02-C002 ("system assets").

## A0

The A0 run is the Empire game in the Win10 VM, day 300, with the Battle at Tralus alert; the captures are in `.artifacts/a0-vm/battle-alert-empire/`.
- **`01-system-assets`.** The planet tab lists "System Assets": GenCore Level I, Orbital Shipyard, Construction Yard, Mine, Mine. Each row shows its mini and its name, and the list's scroll bar shows at its right.
- **`02-alliance`, `03-empire`.** The two forces tabs.
- **`04-result`.** "The Imperial fleet is defeated.", after Simulate Results.

## The port

`battle_alert_rows` (`FUN_00450770(0)`) lists the defenses, then the yards, then the mines, then regiments and personnel.

`the_battle_alert_lists_a_systems_facilities_by_kind_as_a0` builds Tralus's five facilities and gets A0's list in A0's order, each row with a picture. The test passes on the existing code, so it records the agreement and changes nothing.

## Browser (`run.mjs --battle --all --scenario=production-battle-alert-entry`, Linux Chromium)

**Fixture.** The Empire's alert fixture now gives its system Tralus's five facilities, and nothing else.

**Comparison.** At 640×480 the System Assets tab is compared with A0 `01-system-assets`:
- the rows' minis (220..272, 120..320): **0 differing pixels**;
- the scene beside them (140..215, 120..330): **0 differing pixels**.

**Left out.**
- The bar: A0's list ran past its fifth row, so its thumb is shorter.
- The rail's right edge: A0 shows a sector window's dotted border behind the alert there.

The batch does not run this scenario, so it was run on its own.

## The result (later the same day)

A0 `battle-alert-empire/04-result`: after Simulate Results the Empire reads "The Imperial fleet is defeated." over "Tralus is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant." These are `FUN_0049b4d0`'s `0x7003`, `0x7006` and `0x700d`.

The unit test `the_empires_withdrawal_from_a_blockade_reads_as_a0` builds the same report with the shipped texts and gets A0's two lines word for word. This is the viewer's own withdrawal text that TAC-07-C004's note lacked an A0 for.
