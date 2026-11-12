# CMD-10-C010: the disabled cockpit controls before the game is ready

Date: 2026-11-13. Cell: CMD-10-C010 "disabled ignores input"
(`docs/qa/2026-09-10-interface-parity-audit/2026-10-06-surface-ledger.json`).
Sources: the typed REBEXE decompile
(`~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c`) and the
STRATEGY.DLL bitmap table read with `target/claude-tmp/keep/bmpres.py`.
"Inference" marks a claim no decompile states outright.

## Verdict

The original has the state: the galaxy view disables its cockpit controls
when it is created and enables them when the game is ready. The port cannot
reach it. It builds or restores the whole campaign inside one frame before
it draws the cockpit, so no frame exists in which the cockpit is up and the
game is not ready. Proposed status: **excluded**, under the owner's rule of
2026-11-10 (states that a trace shows absent are excluded). The ledger
status is left for the owner to change.

## The original

### Disable at creation

`FUN_00422ce0` is the galaxy view's window procedure (class vtable
`0x00658bf8`, slot `+0x14`). Its WM_CREATE branch (`param_2 == 1`) builds the
controls with `FUN_00427270` (`FUN_00422ce0.c:246`) and calls `FUN_0042d570`
right after (`:247`).

`FUN_0042d570` loads `IDC_WAIT` (`LoadCursorA(0, 0x7f02)`), stores it in
`+0x46c` and sets it. It then calls vtable slot `+0x10` on the controls in
`+0x134`, `+0x138`, `+0x13c`, `+0x140`, `+0x144`, `+0x148`, `+0x14c` and
`+0x150`, and on the legend button in `+0x248`.

For the CoolStrobeButton class (`0x0066df58`), slot `+0x10` is
`FUN_006028a0`. It calls `FUN_006030c0(2)`, which sets bit `0x2` of `+0xc4`
and invalidates the control.

`FUN_00427270` fills `+0x134..+0x14c` with the seven controls (System
Finder `0x12d` to Game Options `0x133`) and `+0x248` with the legend button
(STRATEGY 10168, class `0x00659e38`). It never fills `+0x150`, so that call
finds a null slot. The speed and left-panel buttons in `+0x154..+0x174` are
not in the list. They stay enabled.

### Enable when ready

`FUN_0042d4d0` restores the arrow cursor (`+0x464` into `+0x46c`) and calls
slot `+0xc` on the same nine slots. That slot is `FUN_00602880`, which calls
`FUN_006030f0(2)` and clears the bit.

Its only caller is `FUN_00422620` (`:271`), which refreshes the resource
readouts. `FUN_00422620` has two callers:

- `FUN_00415420`, reached from `FUN_00401030` in case 1 of the frame state
  machine `FUN_0040a050` (`FUN_0040a050.c:76`). Case 1 calls it when the
  setup sequence `FUN_0041df80` returns done. That sequence runs over several
  frames: `FUN_0051d800` advances one setup stage per frame up to `0x1a`
  (`timer-scheduler.md`), then `FUN_0041cce0` and a wait on `+0x20`.
  `FUN_00415420` acts only when the active panel `+0xc0` is a galaxy view
  (`+0x24` is 7 or 8, the type that WM_CREATE writes:
  `param_1[9] = (param_1[0x27] != 1) + 7`).
- `FUN_00440f80`, the galaxy view's own slot `+0x40` (load slot). It reads
  the saved view state and then calls `FUN_00422620`.

Inference: for a new game, the galaxy view exists while the setup sequence
runs. If the view were created after the sequence ends, `FUN_00415420` would
find no galaxy view and the controls would stay disabled, which A0 play
contradicts. For a load, the trace does not show whether a paint falls
between WM_CREATE and the load slot.

### What a disabled control looks like

`FUN_00602150`, the class constructor, stores the normal bitmap in `+0x94`
and the pressed bitmap in `+0x98`. It zeroes the disabled bitmap `+0x9c`
(`param_1[0x27] = 0`). For the seven controls, `FUN_00427270` passes only the
normal and pressed IDs and never calls `FUN_00603150(2, id)`, the setter for
`+0x9c`. It calls that setter only on the speed buttons, where slot 2 is
their selected art.

`FUN_00602d30` (slot `+0x24`, paint) maps a disabled control to state 2. It
draws `+0x9c` when that is set and `+0x94` otherwise. **A disabled cockpit
control therefore shows its normal (rest) bitmap.**

STRATEGY.DLL confirms that the controls have no third bitmap. Each control
has exactly the pair the constructor names. 10029 is unused, and the next
ID, 10030, is a 27 x 22 speed-panel bitmap.

| Control | Alliance normal / pressed | Empire normal / pressed | Disabled |
|---|---|---|---|
| System Finder `0x12d` | 10002 / 10001 | 10016 / 10015 | none (normal) |
| Fleet Finder `0x12e` | 10004 / 10003 | 10018 / 10017 | none (normal) |
| Personnel Finder `0x12f` | 10006 / 10005 | 10020 / 10019 | none (normal) |
| Troop Finder `0x130` | 10008 / 10007 | 10022 / 10021 | none (normal) |
| Encyclopedia `0x131` | 10010 / 10009 | 10024 / 10023 | none (normal) |
| Galactic Information Display `0x132` | 10012 / 10011 | 10026 / 10025 | none (normal) |
| Game Options `0x133` | 10014 / 10013 | 10028 / 10027 | none (normal) |
| GID legend `0x134` | 10168 | 10168 | none (normal) |

Because the art does not change, the only visible signs of the state are the
wait cursor and the lack of pressed art on a click.

### Input while disabled

- Mouse: `FUN_006035f0`, the press hit test, returns with `+0xf8 = 0` when
  bit `0x2` is set (`(uVar1 & 2) != 0`). The press is not captured, so no
  pressed art appears and no command is sent.
- Cursor: WM_MOUSEMOVE in `FUN_00422ce0` (`:395..:398`) sets
  `param_1[0x11b]`, the `+0x46c` cursor, which is `IDC_WAIT` until
  `FUN_0042d4d0` runs.
- Keys: the WM_KEYDOWN branch (`:281..:350`) does not test the disabled bit.
  F2 to F5 call the finders directly (`FUN_0042a000`, `FUN_0042a0c0`,
  `FUN_0042a4d0`, `FUN_0042a180`), F6 calls `FUN_0042a240` and F7 the
  Encyclopedia. F1 posts command `0x133`.
- Commands: WM_COMMAND (`:446..:495`) ignores `0x12d..0x130` and `0x133`
  while `FUN_004fcee0()` is below 2. That function reads `+0xc` of the object
  `FUN_0051ce00` returns. Inference: this is the sub-tick count, so these
  commands also wait for the clock's first two steps. The Encyclopedia
  `0x131` and GID `0x132` commands are not held.

## The port

- New game: `GameMode::GameSetup` in `crates/rebellion-app/src/main.rs`.
  The frame that takes the pending start calls
  `rebellion_data::load_game_data_with_options`, resets every system and sets
  `game_mode = GameMode::Galaxy` (`main.rs` around line 5157). The cockpit
  is first drawn on the next frame, with the game complete.
- Load: `PanelAction::LoadGame` (`main.rs` around line 9189) calls
  `rebellion_data::save::load_slot`. Natively that reads a file. In the
  browser it reads `localStorage` and decodes it synchronously
  (`crates/rebellion-data/src/save.rs`, `stored_save`). It restores the
  campaign and sets `GameMode::Galaxy` in the same frame (around line 9298).
- Asset loading ("Loading game data", "Loading UI assets") ends before the
  main menu, not under the cockpit.

No frame draws the cockpit before the campaign is ready, so the port never
needs a disabled control. Adding an artificial delay only to show the state
would add a wait that the port does not have. Because the disabled art equals
the rest art, the controls would also look the same.

## Proposal

Mark CMD-10-C010 **excluded**: the state the cell describes, cockpit
controls shown disabled before the game is ready, cannot arise in the port by
construction. If a later change makes campaign setup or loading span frames
under the cockpit, port the state then:

- normal art for the seven controls and the legend button;
- `IDC_WAIT` over the galaxy view;
- mouse presses on those eight controls ignored;
- keys F1 to F7 still handled, with commands `0x12d..0x130` and `0x133` held
  until the clock has run two steps.
