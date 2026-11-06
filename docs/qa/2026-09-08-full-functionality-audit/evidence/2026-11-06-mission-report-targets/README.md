# Mission Reports that name a target, from TEXTSTRA (2026-11-06)

This covers EVT-01-C009 ("recruitment") and EVT-01-C010 ("mission"), and follows `2026-11-06-mission-report-texts/`. Both cells stay partial, because no A0 capture of a report body exists.

## The source

`FUN_004927c0`, the cases by family `+0x80`. The leader `+0x74` reports in the first person when it is a major character (family `0x30..0x37`).

| Case | Kind | First person | Third person | Last substitution |
|---|---|---|---|---|
| `0x55` | Recruitment | `0x711b` "{1:1} Recruits {3:1}" / `0x7119`, failure `0x7118` / `0x711a` | none | the recruit `+0xa8` |
| `0x61` | Rescue | `0x7130`, then `0x7131` or `0x7132` | `0x7133`, then `0x7134` or `0x7135` | the target `+0x70` |
| `0x62` | Abduction | `0x70f0`, then `0x70f1`, `0x70f6` (target destroyed) or `0x70f2` | `0x70f3`, then `0x70f4`, `0x70f7` or `0x70f5` | the target |
| `0x63` | Assassination | `0x70f8`, then `0x70f9` (target destroyed), nothing (alive), or `0x70fa` | `0x70fb`, then `0x70fc`, `0x70fe` (injured) or `0x70fd` | the target |
| `0x69` | Sabotage | `0x70e0` over `0x70e3` | `0x70e1` over `0x70e2` | the target object |

Sabotage then adds `0x70e4` "The target was destroyed." or `0x70e5` "  The target was not destroyed.".

## Now

- **`MissionResult`.** It carries `target_character` and `target_object` (the mission's `+0x70`). These are runtime results only, not saved.
- **`story_messages::ReportFacts` and `mission_report_texts`.** They build the reports in the table above.
- **The app's `report_names`.** It reads the leader, the target (an object's class through `status_rows::object_name`, or a character's name) and the recruit before the effects, and sees a target killed by the effects.

**port.**
- The `0x70e6` line "Personnel are returning to {1:1}." is left out. Its place is the first member's fleet or system at the time of the report, and that has not been settled against A0.
- A failed assassination reads `0x70fa`. The `0x70ff` text of a target with `+0x50` bit 9 is not shipped.

## Tests

**Unit tests.**
- `character_mission_reports_name_their_target_and_how_it_ended`
- `a_sabotage_report_names_its_target_and_whether_it_fell`
- `a_report_names_its_leader_target_and_recruit_and_sees_the_target_killed`
- `a_result_carries_its_missions_target_character`
- `a_ship_or_squadron_target_is_named_by_its_class`

**Mutants.**
- `mission_report_texts`: 20 caught.
- `report_names`: all caught; the 7 missed are in `window_conf` and `draw_fullscreen_texture`, outside the filter.
- `object_name`: caught after the class test.

## Open

- An A0 report body.
- Recon (`0x54`), Incite Uprising (`0x56`) and research (`0x53`).
- The `0x70e6` line.
