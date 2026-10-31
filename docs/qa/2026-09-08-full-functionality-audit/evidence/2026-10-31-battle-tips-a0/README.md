# Battle Alert and Battle Summary tips against A0; Alt keys on Linux

Date: 2026-10-31.

## A0

The Windows 10 VM's Empire game raised a Battle Alert at Yavin when it
resumed (`.artifacts/a0-vm/tooltip/340..346`). After Simulate Results it
showed the Battle Summary (`350..358`). Hashes are in `a0-captures.sha256`.
The pointer was placed at the port's control positions for the Empire's
window origin (125, 52), and every probe hit its control.

Each tip's box was measured along its grey bottom border, which the
pointer does not cover (left, top, right inclusive):

| Control | A0 tip | Port tip (Linux) |
|---|---|---|
| Simulate Results | 338, 382, 429 | 338, 382, 433 |
| Take Command | 472, 382, 561 | 472, 382, 562 |
| Retreat | 204, 382, 246 | 204, 382, 248 |
| Alert tab Battle Summary | 549, 110, 638 | 548, 110, 638 |
| Alert tab Alliance Forces | 554, 173, 638 | 551, 173, 638 |
| Alert tab Imperial Forces | 553, 236, 638 | 551, 236, 638 |
| Alert tab System Assets | 559, 299, 638 | 554, 299, 638 |
| Summary close box | none | none |
| Summary tab Battle Summary | 549, 181, 638 | 548, 181, 638 |
| Summary tab Alliance Forces | 554, 241, 638 | 550, 241, 638 |
| Summary tab Imperial Forces | 553, 301, 638 | 551, 301, 638 |
| Summary tab Display | 575, 359, 618 | 575, 359, 619 |
| Capital Ships | 279, 152, 352 | 279, 152, 354 |
| Fighter Squadrons | 328, 152, 429 | 328, 152, 431 |
| Trooper Regiments | 377, 152, 482 | 377, 152, 484 |
| Personnel | 426, 152, 483 | 426, 152, 485 |

The texts read the same as the port's TEXTSTRA strings: 0x1926, 0x1927 and
0x194a; 0x1922..0x1925 on the alert's tabs; 0x1922, 0x1937, 0x1938 and
0x1939 on the summary's; 0x1928..0x1931 on the categories.

**Placement.** Every top is the pointer's y + 21. A tip that would cross
the screen's edge ends at column 638 (`FUN_006008e0`); every other tip
starts at the pointer's x. The port matches all of these exactly.

**Width.** The port's tips are 0 to 5 pixels wider, from the wider text,
which is the accepted text deviation.

## Gates

**`fleet-window.mjs` squadron-battle, both sides** (`fleet-window-tips-result.json`):
- The tips are measured on the alert and on the Battle Summary after
  Simulate Results.
- The close box shows no tip.
- On the Empire side each box is asserted against the A0 table above:
  top, left or right edge, and width within 15 per cent.

**`audit-batch.mjs`, both sides** (`audit-batch-result.json`):
- Alt+2..Alt+9 and then Alt+1 each select their own GID mode (0x12, 0x21,
  0x22, 0x43, 0x44, 0x65, 0x66, 0x67, 0x11).
- Alt+U turns Manage Production on and then off.
- There is no A0 capture of these keys yet.

## Alt+1..Alt+9 in A0

With the Battle Summary closed, Alt+2, Alt+3 and so on to Alt+9, then
Alt+1, were sent to the VM's galaxy view over QMP
(`.artifacts/a0-vm/alt-keys/alt-<digit>.png`, hashes above). Each key
changed the display's title, which shows between the two sector windows
in turn: Uprisings, Idle Fleets, Fleets Enroute, Idle Personnel, Active
Personnel, Idle Shipyards, Idle Training Facilities, Idle Construction
Yards, then Popular Support. The windows cover parts of the titles; the
fragments that show match these names. These are the port's
`GidMode::label` strings, which `audit-batch.mjs` logs in the same order
on Linux.

## Alt+G and Alt+U in A0

The agent's menu (right click on the droid) showed Manage Garrisons and
Manage Production unchecked (`alt-keys/menu-before.png`). After Alt+G and
Alt+U both were checked (`menu-after-g-u.png`). After the two keys again
both were unchecked (`menu-restored.png`). `audit-batch.mjs` turns both on
and off with the same keys on Linux.
