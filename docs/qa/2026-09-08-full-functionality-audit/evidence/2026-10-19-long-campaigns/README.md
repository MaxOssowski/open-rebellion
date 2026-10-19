# Long AI-against-AI campaigns after the planner stages (F-005, F-007, F-036)

Date: 2026-10-19. This is a check of the build at commit `39a75042`, not
a code change.

`rebellion-playtest data/base --dual-ai --ticks 5000`, seeds 1 to 5
(`long-campaigns.json`). The JSONL logs are not committed.

| Seed | End | Alliance / Empire systems | Fleets (most) | Missions |
|---|---|---|---|---|
| 1 | day 4172, Empire wins (`DeathStarVictory`) | 27 / 26 (day 4000) | 12 (20) | 2,856 |
| 2 | day 5000, no winner | 23 / 23 | 12 (23) | 6,819 |
| 3 | day 5000, no winner | 51 / 20 | 19 (31) | 4,223 |
| 4 | day 4744, Alliance wins (`HqCaptured`) | 54 / 27 (day 4500) | 19 (42) | 7,078 |
| 5 | day 5000, no winner | 37 / 21 | 22 (30) | 5,431 |

The first games the port's AI has won are decided by the victory
conditions word (`FUN_005180e0`, F-005):

- Seed 1 ends with the word `0x10100031`: Luke (`0x10`) and Mon Mothma
  (`0x20`) captured, the Alliance headquarters lost (`0x1`, its system
  destroyed by the Death Star), Empire win and game over.
- Seed 4 ends with `0x11001300`: Vader (`0x1000`) and the Emperor
  (`0x2000`) captured, and Coruscant held by the Alliance (`0x100`) and
  destroyed (`0x200`), Alliance win and game over.

Both sides stay alive in every run, and the fleet count stays bounded
(at most 42). No game ended in the 24 runs of 1500 days of the earlier
stages. The captures come from the Abduction planner, ported tonight.

Hashes of the committed files are in `SHA256SUMS`.
