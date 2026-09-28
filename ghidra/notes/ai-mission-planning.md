# Mission teams and decoys: player dialog and AI planners

Recovered 2026-09-28 from REBEXE.EXE (read-only, no analysis). This answers
the open question in `decoy-roll.md`: who fills a mission's decoy list. Both
the player and the AI can fill it. "First read" marks a tracer's reading
that was not re-read for this note.

## Orders and the create command

Mission orders are registry-A objects. `FUN_0051f4b0` registers them with
`FUN_0051f930`, and `FUN_004f5cd0(id)` builds one. The ids are:

| Id | Constructor |
|----|-------------|
| `0x240` | `FUN_004f5250`, vtable `0x0065d118` |
| `0x241` | `FUN_004f5490`, vtable `0x0065d170` |
| `0x242` | `FUN_004f45b0` |

Slot `+0xc` returns the id (`FUN_0057c4a0`, `FUN_004f5500`, `FUN_0057db20`).

An order holds:
- the team list at `+0x2c` (slot `+0x24` sets it, `FUN_0051fb70`; slot `+0x28` gets it, `FUN_0051fb80`);
- a value at `+0x34` (slot `+0x2c`);
- `+0x44` and `+0x48`;
- the target at `+0x4c`;
- the decoy list at `+0x58`.

The order's mission slots are:

- **`+0x48`** is `FUN_004f4990`. It builds registry-B command `0x250` (`FUN_0054cd80`).
- **`+0x4c`** is `FUN_004f4a00`. It copies the order's lists and values into the command and submits it (`FUN_0054ee30`):
  - team `+0x2c` → command `+0x64`
  - decoys `+0x58` → command `+0x6c`
  - `+0x44`/`+0x48` → command `+0x50`/`+0x54`
  - side `0xf8000006` → command `+0x3c`
- **`+0x50`** is `FUN_004f4b60`, the validator. Its errors:
  - `1/0x16` when the team and decoys are both empty
  - `0x40/0x92` and `0x40/0x93` for target checks
  - `1/0x28` for a target on the wrong side

Command `0x250` executes in `FUN_0054d360`, which calls `FUN_00542b60` and
then `FUN_0054bb90(src, team, decoys, ...)`. `FUN_0054c200` then adds the
team with `as_decoy = 0` and the decoys with `as_decoy = 1`
(`decoy-roll.md`, "Who assigns decoys").

`FUN_004f5380` is the preflight. It runs `FUN_005422f0` on the order's lists
without creating anything, and the dialog code calls it (`FUN_0042a320`,
`FUN_0046a9c0`).

## The player: the mission dialog

`FUN_0046c3c0` is the dialog's message handler. Button notifications from
controls `0xc8` and `0xc9` become cases `0xca` and `0xcb`
(`param_1 = (id != 200) + 0xca`, `FUN_0046c3c0.c:86`).

- Case `0xca` moves the selected characters from the team to the decoys.
- Case `0xcb` moves them from the decoys back to the team.

Each case reads both lists (`FUN_0046c850` team, `FUN_0046c880` decoys),
writes the new decoy list to order `+0x58` with `FUN_004f43b0`
(`FUN_0046c3c0.c:158-166`), and sets the team through slot `+0x24`. The
order sits at dialog `+0x120`.

So the player picks the members and marks any of them as decoys. A new order
starts with no decoys (`FUN_00402720` builds `0x240` and leaves `+0x58`
empty).

## The AI: mission planners

Each side keeps mission requests at side `+0xd8` and planners at side
`+0xa8`. `FUN_004d2490` runs a request's state machine (first read):

1. `FUN_004d2580` builds the primary planner from the side's `+0x318`
   (mission kind) and `+0x31c` (subtype).
2. `FUN_004d2610` and `FUN_004d2690` handle secondary requests.
3. `FUN_004d27b0` ticks the planner through its slot `+0x2c`.

What sets side `+0x318` (the strategic choice of mission) is not traced.

`FUN_0042f830` builds a planner by kind and subtype. Constructor and slot
addresses are first read; the kind switch is re-read (`FUN_0042f830.c:13-198`).

| Kind | Subtype | Team selector | Decoy selector |
|------|---------|---------------|----------------|
| Diplomacy `0x51` | all | `004815a0` | `0047f500`: none, trims |
| Espionage `0x52` | 6 | `00480f30` | `00481100`: none |
| Espionage `0x52` | other | `004808c0` | `00480aa0` |
| Research `0x53` | 1, 2, 3 | `0047bcd0`, `0047c2b0`, `0047c900` | none |
| Recon `0x54` | 5 | `0047f790` | none |
| Recon `0x54` | other | `0047f320` | `0047f500`: none, trims |
| Recruitment `0x55` | all | `0047db40` | none |
| Incite `0x56` | all | `0047ff90` | `00480110` |
| Subdue `0x57` | all | `0047cee0` | none |
| Rescue `0x61` | all | `0047e270` | `0047e3f0` |
| Abduction `0x62` | not 4 | `00483290` | `00483410` |
| Abduction 4, Assassination `0x63` | all | `00482ce0` | `00482ec0` (mask from `+0x7c`) |
| Sabotage `0x69` | all | `0047d520` | `0047d720` |
| DS Sabotage `0x6a` | all | `00482180` | `00482330` |

`FUN_00481100` sets only the "decoys done" flag (`+0x20 |= 2`).
`FUN_0047f500` also trims the list (`FUN_0047b9e0`). So Diplomacy,
Recruitment, Research, Subdue, Recon, and Espionage subtype 6 send no AI
decoys. The table puts Recon subtype 5 and Research among the "none" cases
through `00481100`; that reading is first read.

### The planner state machine (`FUN_004bced0`)

The state is at `+0x1c` and the flags are at `+0x20`.

| State | Action |
|-------|--------|
| 1 | exits: flags `0x60000000` → `FUN_0047b250`; `0x10000000` → `FUN_0047aff0`; else state 2 |
| 2 | `FUN_0047b110`: drop team (`+0x3c`) and decoy (`+0x44`) members that fail `FUN_0047ba40`; `0x100` if any remain |
| 3 | slot `+0x48`: the team selector |
| 4 | slot `+0x4c`: the decoy selector |
| 5 | slot `+0x50` |
| 6 | `FUN_004bd0a0`: choose the target, flag 4 |
| 7 | `FUN_004bd570`: members not at the target get a move order; flag 8 when the team is there, `0x10` when the decoys are |
| 8 | once flags 8 and `0x10` are both set: `FUN_0047b360` emits order `0x240` |

`FUN_0047b360` builds the order:
- the team is taken from planner `+0x3c` (slot `+0x24`);
- the decoys are taken from planner `+0x44` (into order `+0x58`, `FUN_004f43b0`);
- each member is kept only when its record's `+0x30` has none of the bits `0x74000f1`, and it is then marked with `0x4000`;
- the target is taken from planner `+0x2c`;
- planner `+0x18` is copied into order `+0x44`.

The AI moves decoys to the target before it sends the order, the same as the
team.

### Selectors

The planner's candidate pool is its side's character records (planner
`+0x64`). A record carries:
- flags at `+0x30`;
- a capability mask at `+0x34`;
- a skill array at `+0x40`, indexed diplomacy 0, espionage 1, ship design 2,
  troop training 3, facility design 4, combat 5, leadership 6, loyalty 7.

`FUN_00403460(pool, req_flags, req_caps, forbid_flags, forbid_caps, skill,
min, max, order)` returns the records that meet all of these
(`FUN_00403460.c`):
- all of `req_flags` and none of `forbid_flags` in `+0x30`;
- all of `req_caps` and none of `forbid_caps` in `+0x34`;
- `min <= skill[+0x40] <= max`.

It keys each hit by that skill in a sorted container, whose order flag is
the last argument. `FUN_00403750` adds the records at one system.

Sabotage (`0047d520` team, `0047d720` decoys) runs as follows.

- **Team, up to 2** (`FUN_0047d520`):
  - First query: `FUN_00403460(pool, 0x800002, 4, 0x4f400051, 0, 5 combat,
    51, 10000, 1)`.
  - Second query: the same with skill 1 (espionage).
  - When the planner's `+0x38` is a system (`0x90..0x97`), it adds the
    candidates there (`FUN_00403750`).
  - Each candidate that passes `FUN_0047ba90` joins the team (`FUN_0047b610`)
    until the team holds 2. Then it sets flag 1.
- **Decoys, up to `n = (rec.+0xbc + 2) / 2`** (`FUN_0047d720`):
  - `rec` is the AI's record for the planner's `+0x34` (`FUN_00403d30`).
  - Query: `FUN_00403460(pool, 0x10800002, 4, 0x7400051, 0, 1 espionage,
    51, 10000, 1)`.
  - Eligible candidates join the decoys (`FUN_0047b730`) until there are
    `n`. Then it sets flag 2.
  - It trims any excess with `FUN_0047b9e0`.

The decoy siblings differ only in the capability bit they require:
- Incite `0x100`
- Espionage 8
- Rescue 2
- DS Sabotage `0x80`
- Abduction `0x10`
- Assassination planner `+0x7c`

`FUN_0047ba90` accepts a candidate only when it meets all of these:
- the side's record list (context `+0x8c`) holds it;
- when it is at a system, its record's `+0x30` bit `0x20` is clear;
- that system's record `+0x28` bit 2 is clear.

## The AI records (re-read 2026-09-28)

`FUN_00401d20` builds a record (vtable `0x006584a0`) for one live object,
and `FUN_00402230` refreshes it.

**Kind and side flags at `+0x30`** (set by the constructor):

- `0x800000` is always set.
- `0x10000000` marks a special force (DatId family `0x3c..0x3f`).
- `0x20000000` marks a character.
- `0x2` means the object is on the record's own side; `0x4` means the other side.
- Special forces also get a class bit in `+0x34` (`FUN_00402e80`): classes
  `0x3c000001`/`5` → `0x10000000`, `2`/`6` → `0x20000000`,
  `3`/`7` → `0x40000000`, `4`/`8` → `0x80000000`.

**State flags at `+0x30`**, recomputed by `FUN_00402230` after masking with
`0x7080440e`:

| Bit | Meaning | Source |
|-----|---------|--------|
| `0x80` | on a mission | role `+0x78` bit 7 |
| `0x400000` | on a mandatory mission | role bit 9 |
| `0x20` | en route | `+0x50` bit 4 |
| `0x1` | `+0x50` bit 2 clear | |
| `0x40` | prisoner | `+0xac` bit 0 |
| `0x10` | injured | `+0x94` != 0 |
| `0x4000000`, `0x1000000`, `0x2000000` | officer rank 1, 2, 3 | `+0x96` |
| `0x8000000` | reserved (see below) | |
| `0x100`, `0x200` | set from `+0x8c` via `FUN_00402cf0` (not read) | |

Bits `0x40`, `0x10` and the rank bits are set for characters only.

`0x8000000` marks the object as reserved. It is set for a research-capable
object whose side-level research flag is clear. It is also set when the
object is Diplomacy-capable and its diplomacy is at least 80.

**Skills at `+0x40`..`+0x5c`** are the eight effective skills, read through
slots `+0x1dc`..`+0x1f8`.

**Capability bits at `+0x34`** are set by `FUN_00402720`, which builds a
dummy order `0x240` holding only this object. `FUN_004f52c0` ->
`FUN_005420d0` then lists every non-hidden MISSNSD record that passes its
class's legality check (`FUN_0054c440`), and each legal record sets one bit:

| Bit | Record |
|-----|--------|
| 0 | Diplomacy `0x51000010` |
| 1 | Rescue `0x61000011` |
| 2 | Sabotage `0x69000012` |
| 3 | Espionage `0x52000013` |
| 4 | Abduction `0x62000017` |
| 5 | Subdue `0x57000080` |
| 6 | Assassination `0x63000081` |
| 7 | DS Sabotage `0x6a000041` |
| 8 | Incite `0x56000040` |
| 9 | Recon `0x54000015` |
| 10 | Recruitment `0x55000016` |
| 11 | Research `0x53000020` |
| 12 | Research `0x53000022` |
| 13 | Research `0x53000021` |

So the AI uses the same legality rules as mission creation.

### What the queries mean

- **Decoy queries** require `0x10000000`. AI decoys are therefore always
  special forces. They must also be on the own side, capable of the
  mission, not on a mission (via `0x7400051`: not `0x1`, injured, prisoner,
  mandatory, or officer), and have espionage >= 51.
- **Team queries** (Sabotage `0x800002`/`0x4f400051`) take characters or
  special forces. Beyond the decoy exclusions they also exclude reserved
  objects (`0x8000000`) and `0x40000000`. They require combat >= 51, then
  espionage >= 51.

### Legality (`FUN_0054c440`)

`FUN_0054cb70` builds a rule table with one entry per MISSNSD record. It
constructs each class and calls its slot `+0x1bc`, which returns two
functions.

- **The shared resolver** is `FUN_00592aa0` for every agent class. It
  finds the target's location.
- **The per-class validator** differs by kind:

| Kind | Validator |
|------|-----------|
| Subdue | `569b10` |
| Sabotage | `56a110` |
| Rescue | `56adb0` |
| Recruitment | `56b370` |
| Incite | `571950` |
| Espionage | `573010` |
| Diplomacy | `573ee0` |
| DS Sabotage | `5744c0` |
| Assassination | `576540` |
| Abduction | `576dd0` |

The member lists contribute a mission mask to the check (`FUN_00582fb0`).
A special force contributes its class record `+0x98`, SPECFCSD's mission
bitmask (`FUN_00503b40`). A character contributes `0x10000`
(`FUN_004ed260`). Both are checked against MISSNSD
`special_force_eligibility`/`target_flags` (inferred).

Diplomacy's validator (`FUN_00573ee0`) runs the common agent rules
`FUN_005868c0`. It then fails with message `0x40`/`0x33` and end code `0xf`
when the side's support at the target (`FUN_00507270`) is 100. A running
Diplomacy mission therefore repeats until the side's support there is
full.

`FUN_005868c0` fails with `0x40`/`0x24` (code -1) when the target is not a
system (`0x90..0x98`). It fails with `0x40`/`0x20` (code 6) when the
target's `+0x88` bit 2 and the record's `+0x88`/`+0x8c` do not match. It
chains to `FUN_00592600`, which is not read.

## Still open

- The rest of the legality chain (`FUN_00592600` and the other per-class
  validators) and `FUN_00402cf0`.
- The AI system record's `+0xbc`, which caps the decoys; its writers were
  not traced.
- The sorted container's order (`FUN_0041be80` flag, `FUN_004357b0` pop).
- The type-B planner flow (`FUN_004bd870`) and the team sizes of the other
  selectors.
- What sets side `+0x318`/`+0x31c`.

## Supporting decompiles

`FUN_0046c3c0`, `FUN_0046c850`, `FUN_0046c880`, `FUN_00402720`,
`FUN_004f4990`, `FUN_004f4a00`, `FUN_004f4b60`, `FUN_004f5380`,
`FUN_004f5cd0`, `FUN_0051f4b0`, `FUN_0051f8f0`, `FUN_0051fb70`,
`FUN_0051fb80`, `FUN_0051fbb0`, `FUN_0042f830`, `FUN_004bced0`,
`FUN_004bd0a0`, `FUN_004bd570`, `FUN_0047b110`, `FUN_0047b250`,
`FUN_0047b360`, `FUN_0047b610`, `FUN_0047b730`, `FUN_0047b9e0`,
`FUN_0047ba90`, `FUN_00481100`, `FUN_0047f500`, `FUN_0047d520`,
`FUN_0047d720`, `FUN_00480110`, `FUN_00480aa0`, `FUN_00482ec0`,
`FUN_0047e3f0`, `FUN_00482330`, `FUN_00483410`, `FUN_00403460`,
`FUN_00403750`, `FUN_00403d30`, `FUN_0041be80`, `FUN_004d2490`,
`FUN_004d2580`, `FUN_004d27b0`, `FUN_0054d360`.
