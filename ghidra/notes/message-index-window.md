# Message Index window construction

This note records the original Message Index window contract recovered from the
decompiled executable. It covers the outer window, the empty index composition,
and the ten category controls. Populated rows, message selection, navigation,
clear/delete actions, Advice slowdown, and chat remain open.

## Entry and dimensions

`FUN_0042a240` handles the command-center Message Index route. It checks window
ID `0x0d`, then calls `FUN_00466350`. That constructor initializes a 470 by 331
surface and loads `encybmap.dll`; the library handle is reused by message
presentation paths and does not identify this window as the Encyclopedia.
`FUN_004665f0` is the Message Index layout routine. The corrected identity is
corroborated by `FUN_00468fb0`, which labels its index mode with TEXTSTRA
`0x8019`, and by original screenshots showing the same ten category icons.

## Index composition

`FUN_004665f0` builds its backing surface from the following STRATEGY resources.
Coordinates are relative to the Message Index window.

| Layer | Alliance | Empire | Position | Source size |
|---|---:|---:|---:|---:|
| Base shell | 10335 | 10336 | 0,0 | 470 by 331 |
| Right rail | 10820 | 10821 | 412,0 | 58 by 330 |
| Index content | 10822 | 10822 | 12,13 | 400 by 306 |

The same function also reveals two later message compositions:

- list state: resource 10823, with resource 10827 copied into a 384 by 195
  child at 22,108;
- displayed-message state: resource 10932 at 12,232 and resource 10933 at
  12,14, with the 400 by 200 message artwork occupying 12,32.

These later states are evidence for a future Message Index integration slice.

## Category controls

The index control parent begins at 22,46 and is 378 by 41. Ten child controls
use exact command IDs `0x79` through `0x82`. Right and bottom edges are
exclusive in the reconstruction, matching the existing bitmap-control input
contract.

| Command | X | Width | Alliance normal/pressed | Empire normal/pressed |
|---:|---:|---:|---:|---:|
| `0x79` | 0 | 36 | 10830 / 10831 | 10830 / 10831 |
| `0x7a` | 38 | 36 | 10832 / 10833 | 10834 / 10835 |
| `0x7b` | 76 | 36 | 10840 / 10841 | 10838 / 10839 |
| `0x7c` | 114 | 35 | 10842 / 10843 | 10844 / 10845 |
| `0x7d` | 151 | 36 | 10836 / 10837 | 10836 / 10837 |
| `0x7e` | 189 | 36 | 10852 / 10853 | 10852 / 10853 |
| `0x7f` | 227 | 34 | 10856 / 10857 | 10856 / 10857 |
| `0x80` | 263 | 36 | 10854 / 10855 | 10854 / 10855 |
| `0x81` | 301 | 35 | 10850 / 10851 | 10850 / 10851 |
| `0x82` | 338 | 37 | 10846 / 10847 | 10848 / 10849 |

`FUN_004697b0` and `FUN_00468fb0` identify the commands as All, Popular
Support, Fleet, Mission, Resource, Manufacturing, Defense, Conflict, Chat, and
Advice. Their filter masks are `0`, `0x001`, `0x080`, `0x010`, `0x004`,
`0x008`, `0x040`, `0x100`, `0x020`, and `0x200`, respectively.

## Evidence boundary

- Static source: `FUN_0042a240.c`, `FUN_00466350.c`, `FUN_004665f0.c`,
  `FUN_00468fb0.c`, and `FUN_004697b0.c` in this directory.
- Resource dimensions: the owned STRATEGY.DLL extraction under
  `data/base/ui/strategy-dll/BMP`, which remains untracked.
- Visual corroboration: the classified Message Index captures in
  `docs/qa/2026-09-10-interface-parity-audit/reference-captures/`.
- This mapping supports a source-exact, test-only empty Message Index shell. It
  does not implement production routing or close any `CMD-08` A0 cell.
