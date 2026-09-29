# Encyclopedia window construction

This note records the original Encyclopedia window contract recovered from the
decompiled executable. It covers the outer window, the index composition, and
the ten category controls. Catalog ordering, ENCYTEXT bindings, topic
navigation, and contextual entry are intentionally left open.

## Entry and dimensions

`FUN_00429f30` handles the command-center Encyclopedia route. It checks window
ID `0x19`, then calls `FUN_0045d400` with an outer size of `0x1d6` by `0x14a`
(470 by 330). The topic child constructor `FUN_00466350` initializes a 470 by
331 surface and loads `encybmap.dll`. The recovered STRATEGY bitmap shell is
also 470 by 331, so the one-pixel outer-window clipping question remains open
for native A0 comparison.

## Index composition

`FUN_004665f0` builds its backing surface from the following STRATEGY resources.
Coordinates are relative to the Encyclopedia window.

| Layer | Alliance | Empire | Position | Source size |
|---|---:|---:|---:|---:|
| Base shell | 10335 | 10336 | 0,0 | 470 by 331 |
| Right rail | 10820 | 10821 | 412,0 | 58 by 330 |
| Index content | 10822 | 10822 | 12,13 | 400 by 306 |

The same function also reveals two later content compositions:

- list state: resource 10823, with resource 10827 copied into a 384 by 195
  child at 22,108;
- topic state: resource 10932 at 12,232 and resource 10933 at 12,14, with the
  400 by 200 EDATA artwork occupying 12,32.

These later states are evidence for the next integration slice, not acceptance
of the current replacement Encyclopedia.

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

## Evidence boundary

- Static source: `FUN_00429f30.c`, `FUN_00466350.c`, and
  `FUN_004665f0.c` in this directory.
- Resource dimensions: the owned STRATEGY.DLL extraction under
  `data/base/ui/strategy-dll/BMP`, which remains untracked.
- Visual corroboration: the classified Encyclopedia captures in
  `docs/qa/2026-09-10-interface-parity-audit/reference-captures/`.
- This mapping supports a source-exact, test-only index shell. It does not
  identify all category semantics, prove catalog ordering, or close any
  `OBJ-01` A0 cell.
