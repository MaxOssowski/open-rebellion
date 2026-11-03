# The Missions window's tab tips against A0 (2026-11-03)

Surface ledger CMD-11 C021, "Missions window".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 207. "Mission at Duros" (Leia Organa's Diplomacy) was opened from the Duros mission icon. The captures are `.artifacts/a0-vm/personnel-states/22-missions-tip-agents` and `23-missions-tip-decoys`, with hashes in `a0-sha256.txt`.

- **Agents.** The pointer rested on the Agents tab at (372, 220). The tip "Agents" shows with its grey (128, 128, 128) border's top edge at y 241 and bottom edge at y 259, both at the pointer's x. Its (255, 251, 240) fill starts two pixels in, at (374, 243). The border's top is therefore at the pointer plus (0, 21), and the tip is 19 pixels high.
- **Decoys.** The pointer rested on the Decoys tab at (432, 220). The tip "Decoys" shows the same way.

## The port

`sector-quadrants.mjs` `missions` (Linux, both sides) rests the pointer on each tab. Its `tipShows` asserts exactly that placement: the border at the pointer plus (0, 21) and plus (0, 39), and the fill at plus (2, 23). Both tabs pass, in every batch since `b27`, most recently `b35`. The texts are FUN_0049f540's 0x8500 "Agents" and 0x8501 "Decoys".
