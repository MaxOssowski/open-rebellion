# Encyclopedia window construction

This note separates the original Galactic Encyclopedia from the Message Index.
The two surfaces were conflated in P63 because both load STRATEGY resources and
the Message Index constructor also opens `encybmap.dll`. The executable and
original screenshots establish distinct routes, layouts, and control sets.

## Entry and dimensions

`FUN_00429f30` is the command-center Encyclopedia route. It checks window ID
`0x19`, then constructs `FUN_0045d400` at 470 by 330. The constructor installs
vtable `PTR_FUN_00659fa8`, loads both `encytext.dll` and `encybmap.dll`, and
resolves an initial game object or falls back to index mode.

`FUN_0045ddc0` is the Encyclopedia layout routine. It composes the faction base
from STRATEGY 10335 or 10336 and builds the separate Encyclopedia frame, index
controls, object list, and topic controls. It is not `FUN_004665f0`; that
function belongs to the Message Index.

## Recovered workflow

- Index mode uses category commands `0x6f` through `0x75`.
- `FUN_0045f100` rebuilds the object list for a selected category from the
  authoritative game-object collection and updates the selected category label.
- `FUN_0045f480` switches between index and topic modes.
- `FUN_0045fa60` resolves the selected object's EDATA bitmap, loads its text
  resource from `encytext.dll`, updates the topic title, and configures previous
  and next availability through `FUN_0045fd20`.
- `FUN_0045fe60` routes keyboard navigation; in topic mode Left and Right invoke
  commands `0x83` and `0x84`.

## Evidence boundary

- Static source: `FUN_00429f30.c`, `FUN_0045d400.c`, `FUN_0045ddc0.c`,
  `FUN_0045f100.c`, `FUN_0045f480.c`, `FUN_0045fa60.c`, `FUN_0045fd20.c`, and
  `FUN_0045fe60.c` in this directory.
- Visual corroboration: the Encyclopedia index and topic captures classified in
  `docs/qa/2026-09-10-interface-parity-audit/screenshot-ledger.md`.
- P62 proves transport for all 187 owned EDATA images. It does not prove this
  window's category ordering, entity bindings, text, navigation, geometry, or
  A0 parity.
- No `OBJ-01` cell is accepted. The next source-recovery pass must finish the
  exact STRATEGY resource and control table from `FUN_0045ddc0` before browser
  reconstruction begins.
