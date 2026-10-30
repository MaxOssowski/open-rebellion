# Tooltips, stage 1: the shared tip, the top strip and the cockpit controls (CMD-11-C001..C007, C031)

Date: 2026-10-30. Surface ledger CMD-11, new: the port had no tooltips at
all. The inventory of every tool the original registers (about 262 in 33
windows) is in `ghidra/notes/tooltips.md`. Its cells C006..C030 stay
pending until each window's tools are ported.

## The original

`FUN_006008e0` creates one `tooltips_class32` control for the whole game,
with `TTM_SETDELAYTIME(TTDT_AUTOMATIC, 500)`: show after 500 ms, hide
after 5 s, and 100 ms from one shown tip to the next. `FUN_00422ce0`
(lines 186-244) adds four rect tools to the top strip, each with its own
`SetRect` per side:
- 0x14f Game Speed Control (5395);
- 0x14c Raw Materials Monitor (5392);
- 0x14d Refined Materials Monitor (5393);
- 0x14e Maintenance Monitor (5394).

The monitors' rectangles are wider than their text objects.

## A0 (Windows 10 VM, Empire save, day 219-221)

Captures are in `.artifacts/a0-vm/tooltip/`; hashes are in
`a0-captures.sha256`.

| Capture | Pointer | What it shows |
|---|---|---|
| `01.png` | (175, 25) | "Raw Materials Monitor", box (175, 46), 124 by 19 |
| `02-nocursor-fast.png` | (260, 25) | no tip yet, right after the move |
| `03.png` | (260, 25) | "Refined Materials Monitor", box (260, 46), 143 by 19 |
| `04-4s.png` | (175, 25) | tip still shown at 4 s |
| `05-6s5.png` | (175, 25) | tip gone at 6.5 s |
| `06-moved.png` | (185, 28) | not back after a move on the same control |
| `07-index.png`, `08-index-hover-monitor.png` | (175, 25) | Message Index open (modal): no tip on the monitor |
| `09-index-tab.png` | (203, 115) | inside the Index: "Popular Support Messages" |
| `10-index-sort.png` | (440, 148) | inside the Index: "Select All" at (440, 169) |
| `11-maintenance.png` | (400, 25) | "Maintenance Monitor" at (400, 46) |
| `12-speed.png` | (520, 25) | "Game Speed Control" at (520, 46) |
| `13-gap-244.png` | (244, 25) | no tip in the gap between the Raw and Refined rectangles |
| `20-sysfinder.png` .. `24-ency.png` | (150..472, 440) | System, Fleet, Personnel and Troop Finder, Encyclopedia: above the pointer, at y 421 |
| `25-gid.png` | (526, 440) | Galactic Information Display, moved left to end at 639 |
| `26-gamecontrols.png` | (90, 210) | Game Controls at (90, 231) |
| `27-rail-support.png`, `28-rail-chat.png` | (618, 118), (618, 318) | Popular Support Messages, Chat Messages, ending at 639 |
| `29-legend.png` | (120, 57) | nothing: a sector window covers the legend button in this save |

Each `*-base.png` is the same spot right after the move, before the tip.

The tip itself:
- fill (255, 251, 240), with a one-pixel (128, 128, 128) border;
- black text whose ink starts 4 pixels in;
- capitals 9 pixels high, starting 5 pixels down.

## The port

`crates/rebellion-render/src/win_tooltip.rs` is one shared tip. A control
registers itself while the pointer is over it:
- `tool`, for any control;
- `attach`, for an egui response;
- `chrome_tool`, for a background-painted control: half-open rectangle as
  in `PtInRect`, input enabled, and no window under the pointer.

`paint` then:
- runs `TipTimer` on the control the previous frame registered;
- draws the tip on egui's top layer at the pointer plus (0, 21), kept
  inside the 640 by 480 client.

The texts come from TEXTSTRA's string table
(`rebellion_data::ui_strings`, `win_tooltip::set_strings`).
`resource_monitors::top_bar_tooltips` registers the four top-strip tools at
`FUN_00422ce0`'s rectangles. `cockpit::cockpit_tooltips` registers the six
primary controls, Game Controls and the nine rail controls with their
`FUN_004286b0` / `FUN_00427270` strings (`control_tooltip_string`), and the
legend button attaches TEXTSTRA 0x173c. The tip stays one pixel inside the
client: it ends at x 639 at most, and goes above the pointer when it would
pass y 479. `main.rs` paints the tip at the start of the
galaxy view's frame, and registers the strip while `strategic_input_enabled`
holds, so no tip shows under a modal window.

**Accepted deviation (2026-10-30, text width):** egui draws the text about
9% wider than GDI, and the tip is sized to fit its text. So:
- "Raw Materials Monitor" is 137 wide against A0's 124;
- near the right edge, "Game Speed Control" moves left to end at 639:
  x 512 against A0's 520.

Height, top and left edge, colours, the text's first ink column and its
capital rows all match A0.

## Browser (Arch Chromium 155, interface WASM `3a950515…`)

`tools/interface-parity/tooltip.mjs --a0=.artifacts/a0-vm/tooltip` passes
for both sides (`tooltip-result.json`). For each side it checks:
- no tip at 250 ms;
- a tip at 1 s, with box x, y and height equal to A0's (Empire) or to the
  pointer plus (0, 21) (Alliance), and a width from A0's up to 12% more;
- the tip gone at 5.6 s;
- on the Empire side, Maintenance Monitor, Game Speed Control and the gap
  at x 244 against A0 11, 12 and 13, and the nine cockpit controls against
  A0 20..28 (text, left and top edge, height; where the wider tip moves
  left, that it ends at 639);
- no tip on the monitor while the System Finder (modal) is open.

## Tests

- `cargo test --workspace` passes.
- `win_tooltip` has 13 tests and `resource_monitors` has 2 new ones. They
  cover the timers, placement, painting, PtInRect edges, covered and
  disabled controls, and all eight top-strip rectangles.
- `cargo mutants` on `win_tooltip.rs` and `resource_monitors.rs`: see the
  commit message for survivors.
