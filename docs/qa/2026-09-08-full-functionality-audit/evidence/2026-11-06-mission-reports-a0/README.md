# Mission Reports and Enemy Mission Foiled against A0 (2026-11-06)

This covers EVT-01-C009 ("recruitment"), EVT-01-C010 ("mission") and OBJ-12-C006 ("foiled"). It settles the port notes of the earlier mission-report evidence (`2026-11-06-mission-report-*`) against A0.

## A0

The captures are in `.artifacts/a0-vm/mission-reports/` (hashes in `a0-sha256.txt`). They come from the Win10 VM: an Expert Alliance game near day 740, then a new Empire game.

| Capture | Title | Body |
|---|---|---|
| `enemy-foiled-chandrila-day739` (Alliance) | Enemy Mission Foiled | "An enemy Reconnaissance mission to Chandrila has been foiled by our forces." |
| `empire-enemy-foiled-tralus-day129` (Empire) | Enemy Mission Foiled | "An enemy Incite Uprising mission to Tralus has been foiled by our forces." |
| `emperor-recruits-thrawn-day43` | Emperor Palpatine Recruits Thrawn | "My recruitment mission to Coruscant was successful. Thrawn is at your command." |
| `espionage-coruscant-failed-day75` (Thrawn, at home) | Espionage Mission Report | "The espionage mission to Coruscant was not successful.\nPersonnel are returning to Coruscant." |
| `diplomacy-chandrila-thrawn-day103` | Diplomacy Mission Report | "The diplomacy mission to Chandrila had no effect on our popular support on that system.  The population does not strongly support either side.\nDo you wish the mission to continue?", with the check and X buttons |
| `emperor-espionage-chandrila-day129` (sent from Coruscant) | Emperor Palpatine Mission Report | "My espionage mission to Chandrila was not successful.\nPersonnel are returning to Coruscant." |

What the captures settle:
- **Mission names.** A mission is named by its kind's name ("Reconnaissance", "Incite Uprising").
- **First and third person.** A major character reports in the first person under "{name} Mission Report" (or "{name} Recruits {recruit}"). Others report under "{Kind} Mission Report".
- **The returning line.** `0x70e6` names where the team returns: the origin Coruscant, not the target Chandrila. A report written at home names home. Espionage carries the line. The port had left it out of every kind.

## The port's fix

**`MissionResult.returning_to`.** `missions::returning_to` walks the team, as `FUN_004927c0` does, to the first member that is neither destroyed nor a captive. It takes that member's Return destination, or else the fleet or system that holds it.

**`ReportFacts.returning`.** The report appends `0x70e6` for Recon, Espionage, Rescue, Abduction, Assassination and Sabotage, the six cases whose code reaches the line.

## Browser (Linux Chromium, `message-display.mjs`)

**Fixture.** The EventMessages fixture files three things through the app's own paths:
- an Imperial Reconnaissance mission to Chandrila ended by a battle, through `push_foiled_missions`;
- the Emperor recruiting Thrawn, through `apply_mission_result`;
- the Emperor's failed Espionage at Chandrila, through `apply_mission_result`.

**Alliance.** It reads "An enemy Reconnaissance mission to Chandrila has been foiled by our forces." over 0x431. The picture differs from A0 in 0 pixels, A0's pointer excluded.

**Empire.**
- "Emperor Palpatine Mission Report" reads "My espionage mission to Chandrila was not successful.\nPersonnel are returning to Coruscant." Picture: 0 pixels differ.
- "Emperor Palpatine Recruits Thrawn" reads "My recruitment mission to Coruscant was successful. Thrawn is at your command." Picture: 0 pixels differ, A0's pointer excluded.

**Text rendering.** The title and body glyphs sit where A0's do. The port smooths them where A0 draws two tones. This is the accepted text deviation, recorded as `foiled_title_a0` and `foiled_text_a0` but not asserted.

## Tests

**Unit tests.**
- `an_espionage_report_ends_on_where_its_personnel_return_as_a0` asserts A0's exact strings.
- `a_report_names_where_its_first_free_member_returns`.
- `a_return_place_is_named_by_its_system`.

**Mutants.** `mission_report_texts`: 27 caught. `returning_to`: 4 caught, 1 unviable. `return_place_name`: 3 caught.

## Still from the decompile only

- The own-side "Mission Foiled" report.
- The Rescue, Abduction, Assassination, Sabotage, Recon, Incite, Subdue and research reports. They use the same builder and shipped texts, but none was captured.
