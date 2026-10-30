# Tooltips, stage 4: the Encyclopedia (CMD-11-C013), and its layer

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §5.

## The original

`FUN_0045ddc0` gives every control a TEXTSTRA tip:

| Control | String |
|---|---|
| Close (0xfb) | 0x1954 |
| Display Topic (0x67) | 0x1860 |
| Index (0x68) | 0x1859 |
| All Databases (0x6f) | 0x1850 |
| System Database (0x70) | 0x1855 |
| Ship Database (0x71) | 0x1854 |
| Facilities Database (0x72) | 0x1852 |
| Missions Database (0x73) | 0x1851 |
| Troop Database (0x74) | 0x1856 |
| Personnel Database (0x75) | 0x1853 |
| Previous Topic (0x83) | 0x1858 |
| Next Topic (0x84) | 0x1857 |

`FUN_0045f480` disables the current mode's button with `EnableWindow`:
Index while the index shows, and Display Topic while a topic shows. A
disabled window shows no tip.

## A0 (Windows 10 VM, Empire, day 253)

Captures are in `.artifacts/a0-vm/tooltip/`, numbered 100–116; hashes are
in `a0-captures.sha256`.

| Captures | Mode | What they show |
|---|---|---|
| 101–107 | index | the seven databases |
| 108 | index | Close (the box measurement is noisy; not used) |
| 109 | index | Display Topic |
| 110 | index | no tip on the disabled Index |
| 112 | topic ("Abduction") | no tip on the disabled Display Topic |
| 113–116 | topic | Index, Previous Topic, Next Topic, Close |

## The port

`encyclopedia::encyclopedia_tooltip_string(command)` holds the strings.
`draw_original_control` registers them through
`win_tooltip::response_tool`. It registers none for the current mode's
0x67 or 0x68 button.

**Layer fix.** The tooltip gate showed that a sector window (Calaron, in
the fixture) drew over the Encyclopedia. The Encyclopedia sat on egui's
Middle order, while sector and system windows raise themselves on
Foreground. In A0 the Encyclopedia is modal and on top, so its two areas
now move to the top of Tooltip, as the Finders and the Message Index do.

## Browser (Arch Chromium 155, interface WASM `0d38e8c2…`)

`tooltip.mjs --a0=.artifacts/a0-vm/tooltip` passes for both sides. On the
Empire side it:
- opens the Encyclopedia from the cockpit control;
- checks A0 101–107, 109 and 110 in the index;
- opens Abduction (row 3, then Display Topic) and checks A0 112–116.

Every tip's left edge, top and height match A0, and both disabled buttons
show none.

Other gates:
- `status-window.mjs` passes. It opens the Encyclopedia from the Status
  window.
- `encyclopedia-index-shell.mjs` gives the same pre-existing differences
  as before (62 and 107–398 pixels; see
  `../2026-10-30-modal-windows/README.md`). The layer change did not alter
  them.
