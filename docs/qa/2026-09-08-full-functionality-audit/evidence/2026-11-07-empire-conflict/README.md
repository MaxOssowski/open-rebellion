# The Empire's Conflict messages in a new Easy Empire game (2026-11-07)

This covers EVT-03-C003 (general bombardment, read by the defender), OBJ-12-C003 (Saboteurs Strike), and CMD-08-C011.

## A0

`.artifacts/a0-vm/empire-clean/`, day 265:

| Capture | What it shows | sha256 |
|---|---|---|
| `20-day265-conflict-list` | Conflict Messages: "Saboteurs Strike at Denab" over "Orbital bombardment of Umgul" | `21308c9d09715eaf3a5909182c3608a4e5e1b4152a725d535ca7af4dd538fd63` |
| `21-saboteurs-denab-kdy` | "The following units were destroyed by saboteurs at Denab:" over "KDY-150" (a defense facility, by its class name) | `6a6bd450b3ee382fde02c7b378e655d06e0abd4e04f2779878029bbcfcd96891` |
| `22-bombardment-umgul-defender` | the Assault Summary of the Alliance's strike, opened from the index | `ce44e89df859bf9c3f67d05dccadaae2cf84a5493e5c5fc0d9a62755427a7ef2` |

**Saboteurs Strike.** The picture is STRATEGY 1014 (`0x3f6`, the Empire reader's), 0 differing pixels at (137, 85). `saboteurs_strike_message` gives the Empire's message the same picture, and `sabotage_victim` names a defense facility by its class. The unit tests cover both sides.

**The bombardment, as the defender reads it.**
- Title "Orbital bombardment of Umgul", in the attacker's red.
- Body "Alliance ships have conducted an orbital strike on the Imperial system of Umgul" (`0x7098`, `0x7099`).
- Picture `0x2b9b` (11163). Its 346 differing pixels are the texts drawn over it.
- `a_bombardment_names_the_system_in_its_title` now also builds the Alliance's strike on an Imperial system and gets A0's body.
