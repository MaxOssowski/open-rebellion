# Native mission object targets (F-019 phases 7g and 7h)

Date: 2026-10-14. Machine: CachyOS Linux, Hyprland (Wayland with XWayland,
`DISPLAY=:1`). Driver: `cua-driver` 0.31.0 (x86_64-linux), daemon in its
standard permission mode, clicks and keys sent in the background through X11
`XSendEvent`; the hardware pointer never moved. Game:
`target/release/open-rebellion` built from `8abd30b3` and launched muted with
`scripts/launch-native.sh` (`*/binary.txt`: SHA-256, pid, window id).
Helper: `nat.sh` (canvas 640 by 480 points scaled to the 950 by 514 window).

## Save

Native builds have no fixture bridge, so `native-targets-save.rs` (run as an
ignored test, then removed from the tree) writes slot 0 from the shipped
`data/base` seed-42 campaign (large galaxy): Mon Mothma free at Davnar
(Kanchen, held by the Alliance and explored for this check), Leia Organa held
by the Empire at Bothawui (Sluis), one Imperial Orbital Shipyard and a fleet
carrying the Death Star at Bothawui, and Bothawui visible to the Alliance.
Each run loads it through the main menu's Load Game.

## Runs

Each run opens Sluis and Kanchen from the galaxy map, double-clicks the
quadrant icons to open Bothawui's target window and Davnar's Defenses
window, right-clicks Mon Mothma, chooses Mission, clicks the target, opens
the mission list, chooses its item and presses Begin Mission.

| Run | Target window | Target | Mission list | Named | After Begin |
|---|---|---|---|---|---|
| `rescue/` | Defenses, Personnel | Leia Organa, listed with the Empire's personnel (`05-bothawui-defenses.png`) | Rescue alone (`11-rescue-list.png`) | Leia Organa | Mon Mothma leaves Davnar's personnel (`13-after-begin.png`) |
| `sabotage/` | System, Shipyards page | the Orbital Shipyard | Sabotage alone (`08-sabotage-list.png`) | Orbital Shipyard | Mon Mothma leaves (`09-after-begin.png`) |
| `death-star/` | Fleet, Fleet 3 selected, ship list | the Death Star | Death Star Sabotage alone (`09-ds-list.png`) | Death Star | Mon Mothma leaves (`10-after-begin.png`) |

Every screenshot was inspected. The empty target frame is the known port
gap (no object portrait). `rescue/game.log` records the load
(`fingerprint=v1:8efc19032d46e490`) and one native note:
`asset unavailable source=rebexe-exe resource_id=1002`, the targeting cursor,
which the native data folder does not stage here; the system cursor stood in.

Not checked natively: that the dispatched mission's target is the clicked
object (the browser runs' reports show it; the native build exposes no
report), and window dragging (`mouse_drag` moved nothing, so the runs place
the two windows apart through the save instead).

Hashes are in `SHA256SUMS`.
