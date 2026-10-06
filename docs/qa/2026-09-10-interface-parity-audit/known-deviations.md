# Known Interface Deviations

This is the central register for deliberate or currently tolerated departures
from the original *Star Wars: Rebellion / Supremacy* interface. It is not a
waiver. Only rows marked **Approved extension** may remain outside the parity
denominator, and only while they cannot replace, obscure, overlap, or change an
original path.

**Temporary** and **Unapproved** rows keep every affected original cell open.
Missing work is not a deviation and belongs in the
[surface ledger](surface-ledger.json).

| ID | State | Departure | Reason and boundary | Affected cells |
|---|---|---|---|---|
| `DEV-UI-001` | Approved extension | The shuttle includes a vector-drawn, music-only mute control that the original did not have. | Player-requested convenience. It occupies a separate top-right region, has identical paint/hit bounds, preserves SFX, and is excluded as `EXT-02-C001`. | `EXT-02-C001`; does not satisfy `PRE-02` |
| `DEV-UI-002` | Approved extension | Deterministic fixture routes expose rare interface states to the browser harness. | Test-only. Fixtures cannot appear in production navigation or count as original UI. | Test infrastructure only |
| `DEV-UI-003` | Temporary | Fleet and Ship Finder lists use wheel scrolling without the original scrollbar. | Bounded implementation shortcut recorded in `ghidra/notes/fleet-finder.md`; original composition and A0 proof remain required. | `OBJ-04-C001`, `OBJ-04-C002`, `OBJ-04-C004` |
| `DEV-UI-004` | Temporary | Fleet Finder uses a top Tooltip layer, centered placement, and different keyboard ownership to remain usable above modeless windows. | Prevents occlusion in the current egui stack. It does not prove original modality, placement, or key handling. | `CMD-04-C017`, `OBJ-04-C001`, `OBJ-04-C002` |
| `DEV-UI-005` | Temporary | En-route fleets are absent from Fleet Finder because the port removes them from their origin system instead of retaining the original container relationship. | Current simulation-model constraint. The original membership and hyperspace presentation remain open. | `OBJ-04-C006`, `OBJ-05-C009`, `OBJ-05-C016` |
| `DEV-UI-006` | Unapproved | More than two sector windows can exist; additional windows reuse the two fixed columns. | No product approval or executable proof supports this. Enforce the original cap or explicitly approve a separate extension. | `CMD-04-C011` |
| `DEV-UI-007` | Temporary | Create Fleet allocates a new fleet dynamically instead of consuming an original spare-fleet object. | The port has no spare-fleet pool. The player-facing split route works, but exact identity and downstream presentation still require proof. | `OBJ-05-C011`, `OBJ-14-C003` |
| `DEV-UI-008` | Temporary | The current manufacturing panel uses a system-level combo box for delivery instead of the original per-producer Destination command and targeting cursor. | Legacy replacement UI. It must be removed from parity mode when the original Manufacturing windows land. | `OBJ-07-C015`–`OBJ-07-C019` |
| `DEV-UI-009` | Temporary | Visible Officers, Research, Jedi, Loyalty, Bombardment, Death Star, and live-ground-combat dashboards reorganize original actions. | Transitional legacy surfaces. They are required failures under `EXT-01`, not approved extensions. | `EXT-01-C001`–`EXT-01-C007` |
| `DEV-UI-011` | Approved extension | The shuttle includes a vector-drawn Fleet Registry chip and readout that choose canonical fleet names, a naming the original never offers. | Approved by the maintainer on 2026-10-06. It sits left of the music control, outside every original hotspot, holds the cockpit beneath it still while open, and defaults to the original "Fleet N" numbering (`FUN_00517760`); see `docs/mechanics/fleet-names.md`. | `EXT-02-C006`; does not satisfy `PRE-02` |
| `DEV-UI-010` | Planned extension | Enhanced widescreen will anchor original panels while extending only galaxy, system, and tactical viewports. | It may begin only after the 640×480 path is accepted. Fixed 4:3 video and other original surfaces remain pillarboxed with separate baselines. | Separate future extension; cannot satisfy any original cell |

## Change rule

Add a row before merging any intentional interface difference. Include the
decision state, reason, affected acceptance cells, and evidence. Moving a row
to **Approved extension** requires explicit maintainer approval and an excluded
`EXT-*` cell when it is visible to players. Removing a deviation requires the
same implementation and evidence updates that close its affected cells.

This register was seeded through the
[manual window cross-check](manual-window-checklists.md)
and existing `port:` notes. Faction Wars' append-only backport practice
informed the format:
[Faction Wars (TeeJS), backport template](https://github.com/TeeJS/faction-wars/blob/626b9b2bd62e7a83a9938568321c59cf64616a19/docs/BACKPORT-LOG.md#L1-L25).
The entries above are independently stated Open Rebellion decisions; no source
text or code was copied.
