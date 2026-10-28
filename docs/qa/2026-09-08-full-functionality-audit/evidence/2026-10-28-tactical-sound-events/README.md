# Tactical weapon sounds and their draws (P24)

Date: 2026-10-28. This settles the last open sound hypothesis, "the sound's
variant is drawn here". Tracing it showed that the port played the wrong
sound for each event and drew its variants at the wrong points of the
battle's single random stream.

## The original

The sound manager is `FUN_005ba8b0` / `FUN_005ba980`. It queues a request,
and `FUN_005baa60` hands it to `FUN_005bad50`, which picks the variant
from the tactical stream (`FUN_005a8a70(0, count - 1)`) at that moment.
It draws only for 2 to 4 variants; otherwise it takes the first.

`FUN_005bae60` builds the table: the counts at `+0x50` and the WAVE words
at `+0x51c` + 4 × event. Each event's first WAVE is 13000 plus the variants
of the events before it, which gives 13063 and 13065 for the jumps `0x1d`
and `0x1f`.

Only two concrete unit vtables carry the weapon slots: capital ships
(`0x0066c140`, slot `+0x20` = 0) and fighter groups (`0x0066c028`, slot
`+0x20` = 1). Both share the base class `0x0066c560`'s damage slots.

- **Fire.** The turn events `0x0d` (`FUN_005b1ea0`, laser), `0x0e`
  (`FUN_005b1f60`, turbolaser) and `0x0f` (`FUN_005b2080`, ion) ask the
  target's slots `+0xc4`, `+0xc8` and `+0xcc` for a sound:

  | Target | Laser | Turbolaser | Ion |
  |---|---|---|---|
  | Capital ship | 0 | 1 | 2 |
  | Fighter group | 3 for side 0, 4 for side 1 (`FUN_005ba0f0`, `+0x168`) | 1 | 5 (`FUN_005ba0b0`) |

  The torpedo `0x10` (`FUN_005b2c70`) asks its firer's `+0xc8`: event 1.
- **Hits.** The turn events `0x11`..`0x14` call the target's damage slot.
  Each slot sounds the shield's event when the shield holds (what is left
  is not below 0), else the hull's, before its own rolls:

  | Turn event | Slot | Function | Weapon | Shield holds | Through the shield |
  |---|---|---|---|---|---|
  | `0x11` | `+0xd8` | `FUN_005b5100` | laser | `0x0d` | `0x12` |
  | `0x12` | `+0xdc` | `FUN_005b5b60` | ion | `0x0e` | `0x13` |
  | `0x13` | `+0xe0` | `FUN_005b5f50` | turbolaser | `0x0c` | `0x11` |
  | `0x14` | `+0xd4` | `FUN_005b5750` | torpedo | `0x0c` | `0x11` |

The Death Star is one of those two unit kinds, and not a fighter group, so
it sounds as a capital ship. With no shield, its hull's event plays.

## What the port did

- It used the turn-event numbers `0x0d`..`0x14` as sound numbers. Every
  laser shot played 13033, which is a laser held by a shield, and so on.
- It drew the hit sound's variant after the slot's rolls, and drew even
  for one-variant events.

## The port now

- `TacticalAudioEvent` names the manager's events: 0..5, `0x0c`..`0x0e`,
  `0x11`..`0x13` and the jumps. `original_audio_resource_base` reads the
  WAVE table.
- `fire_for(kind, fighter target side)`, `impact_for(kind, shield holds)`
  and `draw_variant`, which draws for 2 to 4 variants only.
- `fire_weapon_shot` draws the fire sound. `queue_impact_sound` draws the
  hit sound at the slot's point:
  - before the capital hit's rolls and before the ion slot's;
  - after a fighter group's damage, whose slot rolls nothing.
- `audio.rs`, the extractor and the browser fixture carry the bank
  13000–13017, 13030–13038 and 13045–13053. `build-wasm.sh` removes the old
  names.
- port: the sound manager is taken as running (`+0x18`). With the sound
  off, the original draws nothing.

## `+0x124`

The slots' subsystem rolls run only for a unit whose `+0x124` is 0. In a
single-player battle (`+0x668` = 0), `FUN_00595f60` adds each unit through
the manager's slot `+0x60`, which sets `+0x124` = 1 when the unit's
`+0x168` differs from the manager's `+0x9ec`.

The same flag gates the jump of a withdrawing ship (`FUN_005cc300`,
`0x005cc3..`) and of a withdrawing fighter group (`FUN_005c6610`). If the
AI's units carried 1, they could never jump, and their side would wait on
them forever. The A0 captures of the AI's withdrawal from Yavin
(`.artifacts/a0-vm/2026-10-24-yavin-ai-withdrawal/`) show its ships
leaving, and the results read that the Alliance fleet withdrew. So the
AI's units carry 0, and the port rolls for both sides as before.

## Tests

- `tactical_audio_event_bank_covers_every_recovered_wave_once`;
- `a_sound_draws_its_variant_only_with_two_to_four_variants`;
- the sound expectations of:
  - `automatic_weapon_callback_*`;
  - `fighter_fore_battery_*`;
  - `capital_attack_fighters_*`;
  - the fighter-hit and Death Star test in `abstract_battle.rs`.
- The ion tests' stream helper now skips the hit sound's draw.

## Evidence

Run `2026-10-28T00-21-17-694Z-747597`, `production-tactical-audio-routing`:
36 variants routed and loaded, from event `0x00` WAVE 13000 to event `0x13`
WAVE 13053.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3009 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants (29): 25 caught, 2 unviable, 1 timeout and 1 missed.
  - The timeout is `earlier *= 1` in `original_audio_resource_base`, which
    never ends.
  - The missed one empties `configure_tactical_audio_fixture`. Its unit
    test runs only with `interface-test-fixtures`, and the browser probe
    `tactical-audio-routing` covers it.
