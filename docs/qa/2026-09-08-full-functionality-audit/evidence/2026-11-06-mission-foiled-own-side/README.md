# Mission Foiled for the mission's own side (2026-11-06)

This covers OBJ-12-C006 ("foiled"). It follows `2026-11-06-mission-report-texts/`, which posted only the opponent's "Enemy Mission Foiled". The cell stays partial, because no A0 capture of this message exists.

## The source

`FUN_00491500` handles notification `0x1f1`. For the mission's own side and end codes 3..5, the leader (`+0x7c`, resolved through `FUN_004f2d10`) reports:
- **A major character** (family `0x30..0x37`) titles it `0x7148` "{1:1} Mission Foiled" over `0x7149` "My {2:1} mission to {3:1} has been foiled by opposing forces.".
- **Any other leader** titles it `0x714a` "{2:1} Mission Foiled" over `0x714b`.
- **Substitutions.** The leader, the mission and the target, in that order.
- **Picture.** The leader's portrait (`+0x2c`) on background `0x416`/`0x417`, chosen by the reader. The character branch leaves `local_88` at 0, so the codes 3..5 switch sets `0x416`. `+0x30` is `0x480`.
- **Speaker.** The speaker map gives the advisor event.
- **No leader.** Without a resolved leader, the own side reads nothing.

## Now

- `MissionEnded.character` carries the leading character at the end.
- `story_messages::mission_foiled_texts` builds the title and body, and `message_pictures::mission_foiled` the picture.
- `push_foiled_missions` posts the report to the own side's Mission rail before the opponent's message. It uses the speaker event of the leader.

**hyp.** A killed leader no longer resolves: in the original its object is deleted, while the port keeps the record.

## Tests

- `a_foiled_mission_reports_to_its_side_through_its_leader`: first and third person, and no texts gives `None`.
- `a_foiled_mission_posts_enemy_mission_foiled_to_its_opponent` adds three cases:
  - a led code 4 gives both messages, the own one with background `0x416`;
  - a code 7 gives only the opponent's;
  - a killed leader gives only the opponent's.
- `the_side_background_carries_a_characters_portrait`: `mission_foiled` is 1046 with Han's portrait 6211 for the Alliance, and 1047 for the Empire.
- `a_mission_whose_team_is_gone_ends_with_code_five`: the end keeps the envoy as its first member.

**Mutants.** `push_foiled_missions`: 5 caught.
