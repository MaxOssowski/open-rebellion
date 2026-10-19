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

Side `+0x318`/`+0x31c` are set by the strategy; see "The strategic choice
of mission" below.

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

The planner's candidate pool is its side's records (planner `+0x64`). A
record carries:
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

`FUN_004035d0` adds one argument, `extra`, and passes a record when
`extra < record.+0x2c` or the record has `0x800000`. Every call passes 0.

**Ranking (re-read 2026-09-30).** Each hit goes into a sorted list
(`FUN_0041be80`, order flag at `+0x1c`). `FUN_005f59f0` walks from the head
and inserts the new entry before the first one the comparator
`FUN_0041c250` puts it ahead of:
- order 1 (every skill query): the lower value goes first;
- order 2 (the nearness query `FUN_00403750`): the higher value goes first;
- equal values: a coin flip, `FUN_0041cd80(10) < 5`.

`FUN_0041c230` then numbers the entries 1, 2, 3… into `+0x14`. The score is
that position, not the skill.

**Picking (`FUN_004357b0`).** The selector merges its lists
(`FUN_00435790`). For each entry of the first list, `FUN_00435980` adds its
position in every other list; an entry missing from any list leaves the
first list. The highest total wins, the earliest on a tie (strict `<`), and
leaves the first list. So the best candidate has the highest skill, and a
two-query selector takes only candidates that pass both.

**The nearness query `FUN_00403750`** keys each record at a system by the
squared difference (`FUN_0041b7b0`) of the two system records' `+0x34`
halves. It joins only when the planner's `+0x38` is a system, and state 6
(`FUN_004bd0a0`) sets `+0x38` after the first cycle's selectors have run.
The per-system record's constructor builds `+0x34` as a DatId
(`FUN_004762b0`), yet a tracer read it as packed coordinates; which it holds
is unresolved.

**Team selectors** (every query `max` 10000, order 1; verified at each call
site):

| Kind | Selector | Size | Query | req flags | req caps | forbid flags | forbid caps | skill ≥ |
|------|----------|------|-------|-----------|----------|--------------|-------------|---------|
| Diplomacy | `004815a0` | 1 | `004035d0` | `0x2` | `0x1` | `0x7404051` | 0 | diplomacy 69 |
| Espionage 6 | `00480f30` | 1 | `00403460` | `0x10800002` | `0x8` | `0x7400051` | 0 | espionage 51 |
| Espionage | `004808c0` | 2 | `00403460` | `0x10800002` | `0x8` | `0x7400051` | 0 | espionage 51 |
| Recruitment | `0047db40` | 1 | `004035d0` | `0x2` | `0x400` | `0x7400051` | 0 | leadership 80 |
| Incite | `0047ff90` | 1 | `004035d0` | `0x2` | `0x100` | `0x4f400051` | 0 | leadership 49 |
| Subdue | `0047cee0` | 1 | `004035d0` | `0x2` | `0x20` | `0x7400051` | 0 | leadership 49 |
| Rescue | `0047e270` | 2 | `004035d0` | `0x2` | `0x2` | `0x4f400051` | `0x7000000` | combat 51 |
| Abduction | `00483290` | 2 | `004035d0` | `0x2` | `0x10` | `0x0f400051` | `0x7000000` | combat 51 |
| Abduction 4, Assassination | `00482ce0` | 2 | `00403460` | `0x10800002` | `+0x7c` | `0x47400051` | 0 | combat 51 |
| Sabotage | `0047d520` | 2 | `00403460` ×2 | `0x800002` | `0x4` | `0x4f400051` | 0 | combat 51 and espionage 51 |
| DS Sabotage | `00482180` | 2 | `004035d0` ×2 | `0x2` | `0x80` | `0x47400051` | 0 | combat 51 and espionage 51 |

The selectors for Espionage, Abduction 4/Assassination, and Sabotage also
join the nearness query when `+0x38` is a system. `+0x7c` is `0x40` for
Assassination and `0x10` for Abduction 4.

**Decoy selectors** all query `FUN_00403460(pool, 0x10800002, cap_bit,
0x7400051, 0, 1 espionage, 51, 10000, 1)`, plus the nearness query. AI
decoys are therefore unassigned special forces of the side with espionage
51 or more. Their caps:
- Espionage (`00480aa0`), Incite (`00480110`), Assassination/Abduction 4
  (`00482ec0`), and Sabotage (`0047d720`): `(rec.+0xbc + 2) / 2`, where `rec`
  is the per-system record of the planner's `+0x34` (`FUN_00403d30` on
  context `+0x2c`). Its `+0xbc` is the mission kind that system's posture
  recommends (`FUN_00478000` through posture slot 6, `FUN_004b7150`, shared
  by all six posture classes), or 0. The cap is 1 with no recommendation
  and 42 to 53 with one, so every eligible decoy goes.
- Rescue (`0047e3f0`), Abduction (`00483410`), and DS Sabotage
  (`00482330`): planner slot `+0x44`, `FUN_004047d0`, which returns 4.
- Diplomacy, Recruitment, Subdue, Research, Recon, and Espionage 6 take
  none (`FUN_0047f500` or `FUN_00481100`, or no selector).

`FUN_004b7150` recommends, in order: Recon `0x54` (few Recon missions and
sector bits `0x18`), Espionage `0x52`, Sabotage `0x69` (sector bit 8), and
Abduction/Assassination `0x62`/`0x63` subtype 4 (sector bits `0x40008`), each
gated by the system record's per-kind counters `+0xa0..+0xac`.

**Adding a member** (`FUN_0047b610` team, `FUN_0047b730` decoys) clears the
record's `0x800000`, so a query that requires it (`0x800002`, `0x10800002`)
no longer finds that record. A team member is therefore never a decoy.

`FUN_0047ba90` accepts a candidate only when it meets all of these:
- the side's record list (context `+0x8c`) holds it;
- when its container is a system, its record's `+0x30` bit `0x20` is clear;
- that system's record `+0x28` bit `0x2` is clear. `+0x28` is the root
  pointer of the record's `+0x24` container, so the bit is never set.

The order (`FUN_0047b360`) keeps only members with none of `0x74000f1`: a
selected member on a mission (`0x80`) or en route (`0x20`) is dropped.

The state machine advances every state unconditionally, so a planner whose
decoy list is short still emits. State 7 moves every member to the target,
and state 8 emits once the team and decoys are all there.

## The strategic choice of mission (read 2026-10-14)

The side's strategy object steps `FUN_004e5440` (driven by `FUN_004bf390`;
its cadence is not traced): state 1 `FUN_004e5530`, 2 `FUN_004e5560`, 3
`FUN_004e5710`, 4 `FUN_004e57e0`, 5 slot `+0x20`, 6 `FUN_004e5870`.

**State 3, `FUN_004e5710`** (re-read in assembly: the decompile loses the
outputs). It calls the strategy's slot `+0x18` with four outputs
`(kind, subtype, cancel_kind, cancel_subtype)`. A nonzero kind goes to side
`+0x318` and the subtype to `+0x31c`. A nonzero cancel kind picks, among
the side's planners (`+0xa8`) of that family (`+0x2c >> 24`) whose slot
`+0x14` equals the cancel subtype and whose `+0x20` lacks `0x10000000`,
the one with the least slot `+0x18`, and stores it at side `+0x314`.

**Slot `+0x18` is `FUN_004e8290` in all seven strategy classes** (vtables
`0x0065c8c0` to `0x0065c9e0`, stride `0x30`). It yields at most one request
and one cancellation per pass; the first rule whose request test holds
wins, in this order (`S` is the side controller, `F` its flag word `+4`,
`FUN_005f4960`):

| Order | Kind | Request when | Cancel when |
|---|---|---|---|
| 1 | Recruitment `0x55`/0 | `S+0x320 < 2` and `S+0xa4 > 0` | `S+0x320 > 2`, or `S+0xa4 == 0` and `S+0x320 > 0` |
| 2 | Research `0x53`/1 (ships) | `F & 0x2000000` clear and `S+0x324 < S+0x2cc` | `S+0x324 > S+0x2cc`, or the flag set and `S+0x324 > 0` |
| 3 | Research `0x53`/2 (troops) | `F & 0x4000000` clear and `S+0x328 < S+0x2d0` | likewise with `0x4000000` |
| 4 | Research `0x53`/3 (facilities) | `F & 0x8000000` clear and `S+0x32c < S+0x2c8` | likewise with `0x8000000` |
| 5 | Diplomacy `0x51`/0 | `S+0x34c < S+0x214` | `S+0x34c > S+0x214` |
| 6 | Subdue `0x57`/0 | `F & 0x100` and `S+0x330 < S+0x21c + 1` | |
| 7 | Incite `0x56`/0 | `S+0x350 < S+0x218` | |
| 8 | DS Sabotage `0x6a`/0 | strategy `+4 == 1`, `S+0x334 < 1`, `F & 4` | |
| 9 | Abduction `0x62`/0 | `S+0x338 < 1` | `S+0x338 > 1` |
| 10 | Rescue `0x61`/0 | `F & 0x200` and `S+0x340 < 1` | |
| 11 | Recon `0x54`/5 | `S+0x344 <` the strategy's count `FUN_005f50e0` | |
| 12 | Espionage `0x52`/6 | `S+0x348 < S+0x1bc -` that count | |

(The request terms are read for every row; the cancel terms of rows 6 to
12 follow the same pattern. "The other rows" below gives what each wanted
count means.)

**The live-planner counts `S+0x320..`.** `FUN_004d2580` builds the planner
of `+0x318`/`+0x31c` (`FUN_0042f640` -> `FUN_0042f830`) and, when its slot
`+0x10` returns 2, `FUN_0041b570` adds one to `S[+0x320 + 4 * slot+0x40]`;
`FUN_0042f690` (a cancellation) flags the planner `0x10000000` and
`FUN_0041b5a0` takes one off. The research planners' slot `+0x40` returns
1, 2, 3 (`FUN_00479ac0`, `FUN_0047c200`, `FUN_0047c7e0`), so `+0x324`,
`+0x328`, `+0x32c` count the live ship, troop and facility planners. Their
slot `+0x14` returns the subtype (`FUN_0040f340`, `FUN_0043c1f0`,
`FUN_00526f00`), and slot `+0x18` (`FUN_0047bc30` and siblings) sums the
team's skill in the tree (record `+0x48` for ships), so a cancellation
drops the planner with the weakest team.

**The wanted counts.** `FUN_0041bc10` tallies the side's records: each
character on the side (`+0x30` bits 2 and `0x20000000`) adds one to
`S+0x2cc` with capability `0x800` (Ship Design Research legal),
`S+0x2d0` with `0x1000` (Troop Training) and `S+0x2c8` with `0x2000`
(Facility Design). Legality runs the class creation check, so a character
counts when its skill in the tree is nonzero (`FUN_0056c820`). The AI thus
keeps one research planner per character able to research the tree.

**The Done bits.** `FUN_0042dd00` fills the controller's sub-object at
`+0x138` (vtable `0x00658c40`): `+0x140` is the highest research order of
the side's own capital ship and fighter classes (families `0x14..0x1f`,
class wrapper `+0x24` = class `+0x50`, `+0x28` the side: 1 Alliance only,
2 Empire only, 0 both, so shared classes do not count), `+0x144` the
highest of its regiment classes (`0x10..0x13`), and `+0x148` the highest of
all facility classes (`0x22..0x2b`, any side). The side's research-order
notifications (`FUN_00485f20`, `FUN_00485f00`, `FUN_00485ee0`) call
`FUN_004198a0`, `FUN_00419870`, `FUN_004197b0` with the new level; a level
equal to the target sets `F` bit `0x2000000`, `0x4000000` or `0x8000000`.
From then on the tree takes no request and its planners are cancelled.

**The target** (re-read 2026-10-14; an earlier reading here said the AI
researches where its designer stands, which was wrong). The constructors
(`FUN_0047bba0`, `FUN_0047c170`, `FUN_0047c750`) set only the record id
(`0x53000020`, `0x53000022`, `0x53000021`). The target comes from state 5,
slot `+0x50` (`FUN_0047be50` ships, `FUN_0047c430` troops, `FUN_0047ca80`
facilities), once the team selector has found a member (`+0x20` bit 1):

1. A remembered site `+0x34` whose system record still has the tree's site
   bit (`+0x2c` bit `4` ships, `8` troops, `0x10` facilities) is kept.
2. Otherwise, when `+0x38` (the target of the previous cycle) is a system:
   the nearest sector that holds such a site (`FUN_00419b80`: sector
   records with `+0x28` bit 1, 2 or 4, keyed by the squared distance from
   `+0x38`'s sector, order 2), then in it the site nearest `+0x38`
   (`FUN_0041a400` -> `FUN_00431c00`, order 2).
3. Otherwise the sector with the most sites of the tree (`FUN_00419a00`:
   sector key `0x39`, `0x3a`, `0x3b` = sector `+0x154`, `+0x158`, `+0x15c`,
   order 1), then in it the site with the highest support of the side
   (`FUN_0041a340` -> `FUN_00431830`: system record `+0x48`, which
   `FUN_00433e40` sets to `FUN_00507270`, the side's support, or 100 for an
   own unpopulated system).

The chosen site goes to `+0x34` and `+0x30`. State 6 (`FUN_004bd0a0`) then
takes it as the target when its record has `+0x30` bit 0 and lacks `+0x28`
bit `0x2`, and state 7 moves the team there before state 8 orders.

**Research sites** (`FUN_00433e40`). A system record gets `+0x2c` bit 4
(ships), 8 (troops) or `0x10` (facilities) when all of these hold:
- `+0x28 & 3 == 0`: no uprising (bit 1, system `+0x88` bit 2) and no
  blockade (bit 2, system `+0x88` bit `0x20`);
- the AI has marked the system for a yard of the area: `+0x28` bit
  `0x200000` (shipyard), `0x400000` (training facility), `0x3800000`
  (construction yards). The production planners set these when they order
  such a yard there (`FUN_004cacc0`, `FUN_004cafa0`, `FUN_004d0860`, ...),
  and the refresh keeps them (mask `0x3e00000`);
- the controller's Done bit for the tree is clear;
- a yard of the area there is active (object `+0x50` bit 0);
- `+0x90 > 0`: an active regiment stands there (the walk of families
  `0x10..0x13`, `FUN_00504c40`).

The sector record sums its systems (`FUN_00432350`): a ship site sets
sector `+0x28` bit 1 and adds to `+0x154`; troops bit 2 and `+0x158`;
facilities bit 4 and `+0x15c`.

**The planner's life.** `FUN_004bced0` ends a cycle (`*out = 1`, state 0,
then 1) after state 8 emits its order, after a failed state 6, and after
state 7 while members are still travelling. The next cycle reselects the
team (state 2 drops members that fail `FUN_0047ba40`), so a research
planner lives until it is cancelled. The order's mission reports its
result through Mission Report `0x1f0` (`FUN_00523e70` -> mission slot
`+0x234` = `FUN_00525040`), which the AI player's slot `+0x128`
(`FUN_00485f60` -> `FUN_004195f0` -> `FUN_0042f7f0`) hands to the planner
whose `+0x28` is that mission, at slot `+0x30` (`FUN_0047c890`, shared by
the three research classes):
- result 0: flag `0x80000000`;
- result 2 (failed): `+0x6c` counts failures against slot `+0x24`
  (`FUN_00480df0`, 1000); past it the planner sets `0x80` and
  `0x20000000`, else `0x40000000`;
- result 3 (succeeded): `0x40000000` and `+0x6c` cleared.
At the next state 1, `0x60000000` runs `FUN_0047b250`: while the mission
still runs, `0x40000000` asks slot `+0x34` (`FUN_0047c110` and siblings)
whether the mission's system is still a site of the tree; when it is not,
or after `0x20000000`, the planner orders `0x250` on its mission
(the abort: end code 2, see below) and returns that order.

**Cadence.** The AI player's tick is slot `+0x24` (`FUN_00485b40`, vtable
`0x0065abf0`), called in frame-loop phase 8 (`FUN_0041cf20` ->
`FUN_004361a0`), one side then the other, until both return 1. Each tick:
phase 4 refreshes the records (`FUN_00418500`, the side tallies
`FUN_0041b7e0`, `FUN_0041bc10`, `FUN_0041ba80`); phase 5 readies every
module (slot `+0x10`, `FUN_004cc990`); phase 3 runs one step of a ready
module (`FUN_004be980`: the highest priority `+0x34`, else round robin)
and counts each order it returns, except kind `0x203`, against the daily
budget `+0x428` = 10 (`FUN_00485a50`). It returns 1 once the budget is
spent or no module is ready. `FUN_00485de0` (slot `+0x54`, from
`FUN_0041e500` when the scheduler's day-boundary flag `+0x24` is set)
clears the count `+0x420` each day, since `+0x424` stays 0. The fourteen
modules (`FUN_004bea20` ids 1 to `0xe`) include the strategy (id `0xd`,
`FUN_004bf0d0`; one strategy state per step) and the mission module
(`FUN_004d2490`: state 1 builds or cancels the strategy's request, then
each step advances one planner by one state, `FUN_004d2730` and
`FUN_004d27b0`). Planners and requests therefore advance a few states per
game day, not once a week.

**The fourteen modules (read 2026-10-14).** `FUN_00485a50` adds the modules
to the scheduler list in the order `0xd`, 2, 1, 3, 6, 7, 4, 5, 8, 9, `0xa`,
`0xb`, `0xc`, `0xe`; `FUN_004bea20` builds each. All but `0xe` share the
ready slot `+0x10` (`FUN_004cc990`) and slot `+0x18` (`FUN_004dd810`); the
step is slot `+0x14`. The orders each tree builds (`FUN_004f5cd0`) and the
record bits it reads or marks identify the module; the names are first
reads.

| Id | Ctor, vtable | Step | Orders | Reads, marks | Module |
|----|--------------|------|--------|--------------|--------|
| `0xd` | `004bf0d0`, `0x65c480` | `004bf320` | Move `0x201` | strategy `FUN_004e5440`, sets the requests `+0x318`, `+0x354`/`+0x358` (`FUN_004e57e0`) | strategy: posture and sector plans (read 2026-10-15, `ai-sector-plans.md`) |
| 1 | `004da950`, `0x65c7f8` | `004dabe0` | Build `0x210`, Destination `0x214` | sector bit `0x2000` (an idle construction yard, system `+0x28` bit `0x800`); marks `0x800000`; classes `0x2c`/`0x2d` | mines and refineries |
| 2 | `004e1e50`, `0x65c838` | `004e2120` | `0x210`, `0x214` | sector bits `0xe0` (yard candidates); marks `0x1000000` | facility placement |
| 3 | `004d6b20`, `0x65c7d8` | `004d6d30` | Scrap `0x200`, `0x210`, `0x214` | sector bits `0x10000`/`0x4000` (system `[0x25]` above or below 0); marks `0x2000000` | facility balance |
| 4 | `004d2220`, `0x65c778` | `004d2460` -> `FUN_004d2490` | through its planners (`0x240`, `0x250`, `0x201`) | requests `+0x314`/`+0x318`/`+0x31c` | missions |
| 5 | `004cfb20`, `0x65c758` | `004cfed0` | troops `0x212`, Retire `0x242`, `0x210`, `0x214` | sector bit `0x40`; marks `0x400000` (`FUN_004d0860`) | troop production |
| 6 | `004dd460`, `0x65c818` | `004dd7a0` | `0x200`, `0x212`, `0x210`, `0x214` | marks `0x400000` | garrison balance |
| 7 | `004d2ef0`, `0x65c7b8` | `004d3100` | `0x200`, `0x201`, ships `0x211`, `0x210`, `0x214` | marks `0x200000` | ship balance |
| 8 | `004cf410`, `0x65c738` | `004cf600` -> `FUN_004cf630` | through its planners (`0x201`) | side `+0x204`; one planner per sector (`FUN_004762b0`, vtable `0x65a3b0`) | sector fleet planners (states read and ported 2026-10-15, `ai-sector-plans.md`; plan objects not yet) |
| 9 | `004cf0a0`, `0x65c718` | `004cf290` -> `FUN_004cf2c0` | through its planners | fleets `0x08..0x0f` | fleet planners |
| `0xa` | `004cc6b0`, `0x65c6f8` | `004cc9c0` | `0x200`, `0x211`, `0x210`, `0x214` | sector bit `0x20`; marks `0x200000`; managers `0xa0..0xaf` | shipyard placement |
| `0xb` | `004c8820`, `0x65c6d8` | `004c8b30` | `0x200`, `0x201`, `0x270`, `0x211`, `0x210`, `0x214` | marks `0x200000` (`FUN_004c8d20`, `FUN_004cacc0`) | capital ships: repair, the Death Star's return, production (read 2026-10-15, `ai-production-modules.md`) |
| `0xc` | `004c8260`, `0x65c6b8` | `004c84a0` -> `FUN_004c84d0` | through its planners (`0x201`) | requests `+0x354`/`+0x358`; planner types 1, 3, 4, 5, 6 (`FUN_0042efe0`: `0047a430`, `0047a3c0`, `0047a000`, `00479f70`, `00479a60`) | movement planners |
| `0xe` | `004d2810`, `0x65c798` | `004d29b0` | Rename `0x203` | capital ships `0x14..0x1b` with `+0x24` bit `0x4000`, up to 10 | ship names |

Runtime type codes: fleets `0x08..0x0f`, capital ships `0x14..0x1b`,
sectors `0x80..0x8f`, systems `0x90..0x97`, production managers
`0xa0..0xaf`. System record `+0x28` bits `0x200`, `0x400`, `0x800` mark an
idle shipyard, training facility and construction yard (`FUN_00433e40`);
sector `+0x24` bits `0x20`, `0x40`, `0x80` a candidate system for each
yard, `0x2000` an unmarked idle construction yard (`FUN_00432350`).

**One order a module step.** A module step that builds an order completes
the module (`+0x20` = 1) and raises its priority `+0x34` by one while its
cycle goes on: the build modules' state functions (`FUN_004db2f0`,
`FUN_004d0010`, `FUN_004d3350`, `FUN_004c8ba0`, `FUN_004dda00`, ...) and
the planner modules when the planner's cycle continues (`FUN_004d2490`
state 6, `FUN_004c84d0`, `FUN_004cf630`, `FUN_004cf2c0`). `FUN_00485b40`
zeroes `+0x34` before each step and ends the tick when a step completes
the module with an order, so the AI issues at most one order a sub-tick:
4 a day at Fast, so the day's 10 bind only from Medium. `FUN_004be980`
then takes the ready module with the highest `+0x34` above 0, scanning
from a cursor `+0x14` that moves on one module each call (`FUN_004bede0`,
the first on a tie), else the round robin `+0x10`. A module left current
(`+0x80`) when the budget runs out continues at the next tick.

**The other rows of `FUN_004e8290`.** The wanted counts come from the side
tallies of phase 4:
- Recruitment: `S+0xa4` is the count of the side's MNCHARSD classes
  (`FUN_0042dd00`, family `0x38`, at setup), so the AI keeps up to two
  recruitment planners while it has any minor characters in the data.
- Diplomacy: `S+0x214` = (systems with record `+0x2c` bit `0x40`) / 5 + 1,
  that is neutral systems and own systems whose support is below 70
  (`+0x2c` bit `0x10000000`) and not in an uprising.
- Subdue: `F & 0x100` (an own system below 70 support, sector `+0x28` bit
  `0x8000000`) and fewer planners than own systems in an uprising (`+0x2c`
  bit `0x20`, `S+0x21c`) plus one.
- Incite: `S+0x218` = (opponent systems where the side's support exceeds
  50, `+0x2c` bit `0x100`) / 6 + 1.
- DS Sabotage: strategy `+4 == 1` and `F & 4`, set by `FUN_0041ba80` for a
  fleet record with bit `0x10` (hyp: the Death Star).
- Rescue: `F & 0x200`, set by `FUN_0041bc10` when a character of the side is
  a prisoner.
- Recon: fewer planners than the side's pending system requests (`+0xd8`,
  `FUN_005f50e0`).
- Espionage: fewer planners than the sectors holding an opponent system
  (sector `+0x2c` bit 8, `S+0x1bc`) minus those requests.

**Ported (2026-10-14, save v37)** in `ai.rs`: `AIState.research_planners`
keeps each research planner (`ResearchPlanner`: held designer, site,
previous target, mission, unread Mission Report, failures).
`evaluate_research` reads the reports (`FUN_0047c890`, `FUN_0047b250`,
abort by `AIAction::AbortMission` = `MissionState::abort`, end code 2),
cancels the weakest surplus or Done planners without touching their
missions (`FUN_004e5710`, `FUN_0047aff0`), adds planners up to the able
count, holds every planner's designer out of the other queries
(`Planner::hold`), and runs each planner's cycle (`research_cycle`:
`FUN_0047ba40`, `Planner::pick`, `research_site_choice`, `FUN_004bd0a0`,
`FUN_0047b360`). Equal keys fall to coin flips (`Planner::pick_ranked`), and
every selector passes over a candidate at a blockaded system
(`FUN_0047ba90`). port: the mission's own travel stands for state 7's move
order; a tree's cancellations and additions happen within one AI
evaluation; the production AI's yard marks are kept (`AIState::yard_marks`,
save v40) but no module sets them yet, so the research site needs none (they
cover every own yard system within the first days); the distance keys read the map
position (hyp).

### The port's simplifications checked (2026-10-14)

**Holding a member.** `FUN_0047b610` writes the member's record `+0x24`
(the planner's id, planner `+0x18`), `+0x2c` (the planner's priority, slot
`+0x20`) and clears `0x800000`. `FUN_004035d0` admits an assigned record
only when the query's first argument is below that priority
(`param_1 < record +0x2c`), else only unassigned ones. The research
planners' priority is 0 (`FUN_005a5b60`) and their query passes 0, so
another research planner never takes a held designer. A planner keeps its
member across cycles while the record lacks `0x7400051`, still names it
(`+0x24`), and passes `FUN_0047ba90` (`FUN_0047ba40`); being on a mission
(`0x80`) does not release it. So each research planner keeps its own
designer for as long as it lives. The port keeps no planners: within one
evaluation `sent` has the same effect, but in a later evaluation a designer
already on research is not held, so the selector can pick him again and
the order drops him (`FUN_0047b360`).

**Cancellation does not end the mission.** `FUN_0042f690` flags the
planner `0x10000000`; at its next state 1, `FUN_0047aff0` releases its
team and decoys (`FUN_0047b870`, `FUN_0047b8e0`) and removes it
(`FUN_0041b530`). Its mission keeps running, unowned, until its own
validator ends it (for a Done tree, end `0xe` once the side's points hook
sets Done). The port's `CancelMission` ends the mission instead.

**The abort.** Order `0x250` (`FUN_00580580`, vtable `0x00669b70`) builds a
command whose execute (`FUN_0054e310`) calls the mission's slot `+0x1d4`
(`FUN_00523310`) -> `+0x1d8` (`FUN_005233a0`) -> `+0x1dc(2)`: end code 2
while the mission runs, so it ends through phase `0xb`. The Mission Report
behind the check comes from `FUN_00521880`, which notifies only on a
change of result: entering phase 8 sets 0 (`FUN_00524b70`) and phase 10
sets 2 or 3, so the planner checks its site once per research pass (every
10 to 20 days), not continuously.

**Yard marks.** The ship production module (`FUN_004c8d20`, state 7
`FUN_004cacc0`) claims a system each cycle: from a sector with a candidate
(`FUN_00419a00`, sector `+0x24` bit `0x20`), the side's system with no
mark, uprising or blockade (`FUN_00431830`: forbid `+0x28` `0x200003`,
require `+0x30` bit 1, forbid `+0x30` `0x40000000`), keyed by its
shipyard count (`+0x58`) and its free capacity (`+0x114`), and marks it
`0x200000`. No code clears the marks (the refresh keeps `0x3e00000`). The
troop and facility modules do the same with `0x400000` and
`0x800000..0x2000000`. Since the module runs every AI tick and prefers
systems with yards, every own system with a yard is marked within the
first days; the port's "any yard counts" differs only before that.

**Ties.** `FUN_005f59f0` inserts each candidate before the first entry the
comparator `FUN_0041c250` puts it ahead of; order 1 sorts ascending, 2
descending, and an equal key goes ahead on `FUN_0041cd80(10) < 5`.
`FUN_0041c230` ranks the list from 1 and `FUN_004357b0` takes the highest
rank sum, so with one list the last entry wins: the highest key for order 1
and the lowest (nearest) for order 2, with ties decided by those coin
flips. The port takes the first of equal entries.

**The remaining port simplifications (read 2026-10-14).**

- *Distance keys.* `FUN_0041b7b0` is the squared difference of two packed
  `(x, y)` shorts. A sector record's `+0x40` comes from its class record
  `+0x48` (`FUN_00431450` -> `FUN_00526560`), a system record's `+0x34`
  from its class record `+0x4c` (`FUN_00433b10` -> `FUN_00509620`); with the
  class record 0x28 bytes past the DAT record these are SECTORSD
  `x_position`/`y_position` (offset `0x20`) and SYSTEMSD
  `x_position`/`y_position` (offset `0x24`). So the keys are map
  positions, as the port reads them; this also settles the record `+0x34`
  question under "Still open".
- *Travel.* State 7 (`FUN_004bd570`) gives every free team member away from
  the target a Move order `0x201` to it (team list `+0x24`, target
  `+0x38`); a member already there sets flag 8, and state 8 orders the
  mission only once all are there. `0x201` expands to a `0x241` order for
  characters (`move-order.md`), the Move pseudo-mission (family `0x41`).
  The research mission therefore starts at its site (origin = target:
  phases 5 and 6 do nothing, and its detection runs at the AI's own
  unblockaded system). Correction (2026-10-14): Move does have a MISSNSD
  record, `0x41000001` (TEXTSTRA 11265): both sides, special forces
  `0xf0f`, characters `0x10000`, timer 0 + rand(0..=0), hidden (`+0x5c`),
  detection on, no resigning, and only an own-side target (`+0x7c`). Its
  class (vtable `0x665578`, `FUN_0056ebc0` = `0x41`) keeps the base phase
  handler `FUN_00524b70` (slot `+0x1f4`), so phase 10 rolls no one, and the
  base rules (slot `+0x1bc` = `FUN_005215e0` -> `FUN_00523670`,
  `FUN_00523450`). So the trip runs the detection manager, as any
  mission does. Ported 2026-10-14: `MissionKind::Move`, and the research planner
  orders it while its designer stands away from the target (a fleet or
  another system); a designer on the Move is busy, and the next cycle
  after it ends orders the research.
- *Cadence.* The AI player's tick (`FUN_00485b40`) runs once per frame in
  frame-loop phase 8 until it returns 1, and its phase 3 steps the current
  module until it completes (`+0x20`), then the next ready one
  (`FUN_004be980`), until no module is ready or the day's 10 orders are
  spent; phase 5 readies them all again. So every sub-tick the strategy
  module (id `0xd`: `FUN_004bf320` -> `FUN_004bf390` -> `FUN_004bf490`, the
  posture, then the six states of `FUN_004e5440`) makes one full pass, and
  the mission module (`FUN_004d2490`) builds or cancels the strategy's
  request and runs one planner through one whole cycle, the planners in
  turn (`FUN_004d2730`). A day has as many sub-ticks as the larger of the
  sides' `+0xc4` (`FUN_0051dea0`), which the speed order `0x280` sets
  (`FUN_00628ff0`, its slot `+0x20`) to the speed's rate: 600 Very Slow, 60
  Slow, 12 Medium, 4 Fast. The AI thus makes 4 to 600 strategy passes and
  planner cycles a day, capped at 10 orders. The frame loop that calls the
  tick is `FUN_0041dff0`, one phase a call, 0 to `0xd` and round again:
  phase 2 (`FUN_0041e300`) holds until the scheduler's sub-tick count
  (`+0xc`, `FUN_0041e1b0`) moves past the one stored at `+0x10`, phase 8
  (`FUN_0041cf20` -> `FUN_004361a0`) ticks the two players in turn until
  both return 1, and phase `0xd` (`FUN_0041e500`) at a day boundary
  (scheduler `+0x24`) calls `FUN_0041d050` -> `FUN_00436530`, which clears
  the order count. So the loop turns once a sub-tick.
  Ported 2026-10-14 (stage 1, save v38): `ai::passes_per_day` gives the
  speed's rate, and the simulation and the app run that many
  `AISystem::pass` a day, one side then the other, applying each pass's
  orders before the next (`AIState::begin_day` clears `orders_today`).
  Stage 2 and 3 (2026-10-14, save v39): the modules are identified above
  ("The fourteen modules"), and the port runs them in `MODULE_IDS` order
  with priorities (`module_priority`, `priority_cursor`), one order a
  step (`step_module`, `module_pending`), and the mission module one entry
  a step (`mission_next`). The port's heuristics stand in: research
  strategy for `0xd`; the research planners and the mission heuristics for
  4; production split by kind for `0xa`, `0xb`; fleets for 8; troop
  moves for `0xc`. Modules 7, 9 and `0xe` order nothing. Modules 1, 2, 3,
  5 and 6 are ported (`ai-production-modules.md`).
- *Yard marks.* As above: every cycle of the ship, troop and facility
  production modules claims one more system, so with several cycles a day
  every own yard system is marked almost at once.

**Also found.** `FUN_0047ba90` rejects a candidate standing (not en route,
record `+0x30` bit `0x20`) at a system whose record has `+0x28` bit `0x2`,
the blockade bit, for every mission kind (ported 2026-10-14).

## The AI records (re-read 2026-09-28, 2026-09-30)

`FUN_00401d20` builds a record (vtable `0x006584a0`) for one live object,
and `FUN_00402230` refreshes it. The record's `+0x3c` is the side
controller (`FUN_004178f0` stores a self-pointer at `+0x9c`), whose `+4` is
a flag word (`FUN_005f4960`).

**Kind and side flags at `+0x30`** (set by the constructor):

- `0x800000` is set, and cleared while a planner holds the object.
- `0x10000000` marks a special force (DatId family `0x3c..0x3f`).
- `0x20000000` marks a character.
- `0x2` means the object is on the record's own side; `0x4` means the other
  side. A character prisoner counts on the side that holds it.
- `0x40000000` marks the four leaders (`FUN_004024d0`: `0x30000240`,
  `0x32000242`, `0x34000280`, `0x35000281`) while the controller's bit `0x1` is
  clear; `0x400` additionally marks `0x34000280`. The bit's only writer is
  `FUN_004182a0`, from a game setting (`FUN_004fce50`), untraced.
- `0x8` marks the six main characters (`FUN_004025f0`). Any other character
  gets capability bits `0x1000000`, `0x2000000`, `0x4000000` when its class
  record may be an admiral, commander, or general (`+0xa8`, `+0xac`, `+0xb0`;
  `FUN_004ed1c0`, `FUN_004ed1e0`, `FUN_004ed200`).
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
| `0x1` | `+0x50` bit 2 clear (a character the side has not recruited) | |
| `0x40` | prisoner | `+0xac` bit 0 |
| `0x10` | injured | `+0x94` != 0 |
| `0x4000000`, `0x1000000`, `0x2000000` | officer rank 1, 2, 3 | `+0x96` |
| `0x8000000` | reserved (see below) | |
| `0x100` | Force level 1..99, not Luke or Vader | `+0x8c`, `FUN_00402cf0` |
| `0x200` | Force level 100 or more, Luke or Vader | `+0x8c`, `FUN_00402cf0` |

Bits `0x40`, `0x10` and the rank bits are set for characters only.

`0x8000000` marks the object as reserved:
- when it is legal for a Research record (capability `0x800`, `0x1000`,
  `0x2000`) and the controller's bit `0x2000000`, `0x4000000`, or
  `0x8000000` is clear. Those bits start clear and are set by
  `FUN_004198a0`, `FUN_00419870`, and `FUN_004197b0` when a callback's
  argument matches the controller's research targets at `+0x140..+0x148`;
  when that fires is untraced. The shipped Research records take every
  character and no special force.
- when it is Diplomacy-capable and its diplomacy is above 79.

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

So the AI uses the same legality rules as mission creation. The probe names
no target (context `+0xc` = 0), so only the member check below decides.

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

**The member check (re-read 2026-09-30).** `FUN_005830a0` walks the team,
decoy, and captured lists and builds a mask with `FUN_00582fb0`: a special
force ORs its class record `+0x98` (SPECFCSD's mission mask, `FUN_00503b40`)
into word 0, a character ORs `0x10000` (`FUN_004ed260`) into word 1, and each
member marks its side. `FUN_00583320`, called from `FUN_00523450`, then
requires:
- word 0 a subset of the record's `+0x48` (file `0x20`);
- word 1 a subset of `+0x4c` (file `0x24`);
- one side only (both is status `0x14`, none `0x16`), and that side's flag,
  `+0x40` (Alliance, file `0x18`) or `+0x44` (Empire, file `0x1c`).

A failure is message `0x40`/`0x01` with no end code, so it refuses creation
but never ends a running mission. No legality function reads a skill or a
rank.

| Record | `+0x48` | `+0x4c` | Alliance | Empire |
|--------|---------|---------|----------|--------|
| Diplomacy `0x51000010` | 0 | `0x10000` | 1 | 1 |
| Rescue `0x61000011` | `0x802` | `0x10000` | 1 | 1 |
| Sabotage `0x69000012` | `0x402` | `0x10000` | 1 | 1 |
| Espionage `0x52000013` | `0x208` | `0x10000` | 1 | 1 |
| Abduction `0x62000017` | `0x802` | `0x10000` | 1 | 1 |
| Subdue `0x57000080` | `0x401` | `0x10000` | 1 | 1 |
| Assassination `0x63000081` | `0x800` | `0x10000` | 0 | 1 |
| DS Sabotage `0x6a000041` | 2 | `0x10000` | 1 | 0 |
| Incite `0x56000040` | `0x401` | `0x10000` | 1 | 1 |
| Recon `0x54000015` | `0x104` | 0 | 1 | 1 |
| Recruitment `0x55000016` | 0 | `0x10000` | 1 | 1 |
| Research `0x530000(20, 21, 22)` | 0 | `0x10000` | 1 | 1 |

Diplomacy's validator (`FUN_00573ee0`) runs the common agent rules
`FUN_005868c0`. It then fails with message `0x40`/`0x33` and end code `0xf`
when the side's support at the target (`FUN_00507270`) is 100. A running
Diplomacy mission therefore repeats until the side's support there is
full.

`FUN_005868c0` fails with `0x40`/`0x24` (code -1) when the target is not a
system (`0x90..0x98`). It fails with `0x40`/`0x20` (code 6) when the
target's `+0x88` bit 2 and the record's `+0x88`/`+0x8c` do not match. It
chains to `FUN_00592600`, whose target checks run only with a target
(context `+0xc`).

## Ported (F-019 phase 5a)

`crates/rebellion-core/src/mission_planning.rs` ports the records, the
queries, the ranking, the pick, and the selectors above for the port's ten
kinds, and `MissionState::dispatch_guarded` refuses members the record does
not admit (`MissionRefusal::MembersNotAllowed`). The port departs from the
original as follows:
- port: the planners run in one cycle, so the nearness query never joins
  and the order keeps only the members standing with its first (the
  original gathers them in state 7);
- port: a plan with no team keeps no decoys, and a member one plan sends is
  not taken by the next in the same evaluation;
- port: the tie draws come from a stream seeded by the day and the side;
- hyp: the posture that sets `+0xbc` is not ported, so the four posture-cap
  kinds take one decoy;
- hyp: the controller's leader bit is read as clear, its constructor
  value, so the four leaders stay off Incite, Rescue, Sabotage,
  Assassination, and DS Sabotage teams;
- (2026-10-18) the research bits follow the side's finished trees (see
  "The research reserve" below);
- hyp: the port has no officer rank, so a character in a fleet holds one;
- port: Espionage takes the class for subtypes other than 6 and Abduction
  its default class (side `+0x31c` is untraced); the mission kind and target
  are the port's (side `+0x318`).

## Why the port's AI sends no two-character team (2026-10-18)

- In 24 campaigns of 1500 days (41,805 missions) the port's AI dispatched
  1,232 two-member teams (Sabotage, Assassination, Rescue, Espionage,
  Abduction), all of special forces. No team held two characters.
- The cause is the port's one-cycle planner (`mission_planning.rs`,
  `plan`). The selectors pick a team of two, but the order keeps only the
  members already standing with the first (`members_together`). The
  original's planner lives across ticks instead:
  - state 7 (`FUN_004bd570`) gives each member not at the target a Move
    order;
  - state 8 emits once flags 8 and `0x10` say the team and decoys are
    there.
  So two characters from different systems meet and go together.
- Consequences:
  - mechanics that need several characters on one mission never happen
    in AI play, such as learning a traitor (row 17);
  - teams are weaker than the original's.

## Persistent mission planners (ported 2026-10-18, save v80)

The user approved the save change and the AI change.

- State 6 (`FUN_004bd0a0`) picks the gathering place `+0x38`, in this
  order:
  - the previous place, while it is the side's (system record `+0x30`
    bit 1, `+0x28` bit 2 clear);
  - else the target `+0x34`, when it is the side's;
  - else the side's system that the nearness query (`FUN_0041a460`) finds
    from the target;
  - else the system the first team member stands in.

  A Move to another side's system ends at once with code 8, so the team
  never gathers at an enemy target.
- `AIState::mission_planners` holds each `MissionPlanner`:
  - kind, team, decoys and target;
  - `created`, `last_cycle` and `staging`.
- `AISystem::adopt_mission_orders` turns the heuristics' orders into
  planners (`FUN_0042f830`):
  - one already running for the same kind and target takes the new one's
    place;
  - a member another planner holds is left out (`FUN_0047b610`).
- `AISystem::mission_planner_cycle`, once per day per planner, runs these
  states:
  - state 2 drops a member the planner cannot keep (`FUN_0047ba40`), and
    an empty team ends the planner (`FUN_0047aff0`);
  - state 6 is `staging_point`;
  - state 7 gives each free member away from the place a Move order;
  - state 8 orders the mission once all stand there.
- The evaluation's `Planner` holds every planner's members, as it holds
  the research planners' designers.
- `Planner::plan` keeps both picks wherever they stand; `members_together`
  is gone.
- port:
  - the heuristics stand in for the strategy (`+0x318`, `+0x31c`);
  - (2026-10-18) states 3 and 4 refill the team and decoys each cycle
    (`Planner::refill`); a team still empty after state 3 ends the
    planner (port);
  - a planner older than `MISSION_PLANNER_DAYS` (60) ends;
  - the nearness query is read as the side's unblockaded system nearest
    the target.
- Campaigns: 24 seeds of 1500 days, before and after.
  - Dispatched missions: 41,485 before, 13,590 after.
  - Successes: 17,811 before, 14,889 after.
  - Move orders: 320 before, 23,522 after.
  - Systems held at the end are alike: Alliance 26.3 and 26.6, Empire
    19.8 and 17.0.
- Still no AI team holds two characters: the covert selectors find only
  special forces, and characters go on one-member missions (Diplomacy,
  Recruitment, Research). The gathering now lets a two-member team from
  different systems go together; 297 two-unit teams went in 8 seeds of
  800 days.

## The research reserve (2026-10-18)

- `FUN_00402230` (lines 72-93) reserves a record (`0x8000000`) for each
  research capability it has (`+0x34` bits `0x800`, `0x1000`, `0x2000`)
  whose controller bit (`0x2000000`, `0x4000000`, `0x8000000`) is clear.
- `FUN_004198a0`, `FUN_00419870` and `FUN_004197b0` set the bits once the
  side's research order reaches the controller's target (`+0x140..+0x148`,
  "The Done bits" above). That is the port's `ai_research_done`.
- The capability comes through `FUN_0054c440` and so needs a nonzero skill
  in the tree (`FUN_0056c820`). A character with no skill in any tree is
  never reserved.
- Ported: `AgentRecord::reserve` and `Planner::research_done`, which the
  AI calls with the three trees each evaluation. The port had reserved
  every character a Research record admits, for the whole game.
- Campaigns: 24 seeds of 1500 days, against the persistent-planner
  commit.
  - Teams with two characters: 0 before, 613 after.
  - Covert teams with characters: Sabotage 1,467, Incite 615, Rescue 187,
    Abduction 95.
  - Dispatched missions: 13,590 before, 12,217 after.
  - Successes: 14,889 before, 12,030 after.

## The refill (2026-10-18)

- States 3 and 4 (slots `+0x48`, `+0x4c`) run the team and decoy
  selectors on every cycle. A planner whose member was dropped in state 2
  takes another up to the class's size.
- Ported as `Planner::refill`, called after state 2 in
  `mission_planner_cycle`. The planner holds its own members first, so it
  does not pick them twice.
- Campaigns: 24 seeds of 1500 days, against the research-reserve commit.
  - Two-member teams: 2,145 of 12,217 missions before, 3,233 of 8,605
    after (38%).
  - Successes: 12,030 before, 10,757 after.
  - Failures: 8,826 before, 7,543 after.
  - Systems held at the end are alike: Alliance 579 and 557, Empire 352
    and 347, summed over seeds.

## The Diplomacy planner (read and ported 2026-10-18, save v81)

The Diplomacy planner class has vtable `0x0065a900`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_00481500` | the cancel key: the team's records' `+0x40` summed (diplomacy), 0 with `+0x20` bit `0x20` |
| `+0x30` | `FUN_0047c890` | the Mission Report, shared with the research planners |
| `+0x34` | `FUN_00481eb0` | the abort test: the target record still has `+0x2c` bit `0x40` |
| `+0x38` | `FUN_00481fe0` | release: clear the claim `0x20000000` on the record of `+0x30` |
| `+0x3c` | `FUN_00481ef0` | a better free diplomat exists (not read further) |
| `+0x40` | `FUN_004814f0` | 0xb: the count `S+0x320 + 4*0xb` = `S+0x34c` |
| `+0x48` | `FUN_004815a0` | the team selector (diplomacy 69..10000) |
| `+0x4c` | `FUN_0047f500` | the decoy selector |
| `+0x50` | `FUN_00481720` | state 5, the target |

**The want.** `FUN_0041b7e0` sums the sector records' `+0x168` (their
systems with record `+0x2c` bit `0x40`) into `S+0x214`, and
`FUN_00418500` then sets it to `/ 5 + 1` only when it is above 0. With no
candidate the side wants no Diplomacy planner. `S+0x218` (Incite) is the
sectors' `+0x178` (record `+0x2c` `0x100`) `/ 6 + 1` the same way, and
`S+0x21c` (Subdue) the sectors' `+0x164` (record `+0x2c` `0x20`).

**The target, `FUN_00481720`.** The function first clears `+0x20` bit
`0x10`.

Without a member (`+0x20` bit 1 clear), it clears the claim `0x20000000`
on the record of `+0x34` and returns 0.

With a member, it keeps `+0x34` while the record still has `+0x2c` bit
`0x40` and lacks `+0x30` bit `0x200` (an enemy fleet). Otherwise it clears
that record's claim and empties `+0x34`. It then tries these in order
until one gives a system:

1. `FUN_00419a00(0, 0x4000020, 4, 0, 0, 0, key 7)`, order 2. The pick is
   the sector with own systems and an unclaimed candidate, with the lowest
   average own support (`+0x8c`). When that sector's record (controller
   `+0x44`, at `0x4818ed`) has `+0x8c < 70`, `FUN_0041a460(sector, 0, 0x40,
   1, 0, 0x20000000, 0)`, order 2, gives a random own unclaimed candidate.
2. With `+0x38` a system: `FUN_00419b80(+0x38, 0, 0x4000020, 0x10, ...)`,
   order 2, gives the nearest sector with neutral systems. Then
   `FUN_0041a460(sector, 0, 0x40, 2, 0, 0x20000000, 0)`, order 2, gives a
   random neutral unclaimed candidate.
3. With `+0x38` a system: `FUN_0041a400(last sector, +0x38, 0, 0x40, 0, 0,
   0x20000000, 0)`, order 2, gives the candidate nearest `+0x38`.
4. Merge `FUN_00419a00(0, 0x4000020, 0x10, ..., key 3)`, the same with key
   5, and `FUN_0041a1d0(0, 0x4000020, 0x10, ...)`, all order 1. Then a
   random neutral unclaimed candidate, order 1.
5. Merge `FUN_00419a00(0, 0x4000020, 0, ..., key 3)` and
   `FUN_0041a1d0(0, 0x20, 0, ...)`, order 1. Then a random unclaimed
   candidate of any side, order 2.

A target is copied to `+0x30` and claimed (record `+0x2c |= 0x20000000`),
and the function returns 1. The refresh keeps the claim
(`FUN_00433e40`: `+0x2c & 0x60000000`), and the sector's `+0x28` bit
`0x4000000` counts only unclaimed candidates (`FUN_00432350`).

**Ported** in `ai.rs`:
- `AISystem::diplomacy_strategy` is row 5 of `FUN_004e8290` with the
  cancellation of `FUN_004e5710`.
- `diplomacy_target` is `FUN_00481720`, and `diplomacy_wanted` gives
  `S+0x214`.
- `MissionPlanner::standing` and `claimed`, and `AIState::record_claims`
  (save v81).
- `ai_records::apply_claims` keeps the claims across refreshes.
- The port's heuristic Diplomacy orders (popularity cap) and its uprising
  prevention (two diplomats a pass to own systems below 40%) are removed.

port:
- the request and cancellation reach the want within one step;
- the strategy's other rows are not yet ported, so row 5 is not ordered
  behind rows 1 to 4;
- slots `+0x30`, `+0x34` and `+0x3c` (Mission Reports, the abort and a
  better diplomat) are not ported for Diplomacy.

Campaigns: 24 seeds of 1500 days, against the F-005 commit.
- Diplomacy orders: 1,381 before, 789 after.
- Diplomacy results: 3,667 successes and 1,862 failures before, 3,610 and
  1,780 after. A Diplomacy mission repeats, so it reports many times.
- The freed diplomats research and recruit. Ship, troop and facility
  research orders: 1,247/44/134 before, 2,275/686/744 after. Recruitment:
  862 before, 1,703 after.
- Missions: 8,605 before, 11,366 after.
- Systems held at the end, summed over the seeds: Alliance 557 and 531,
  Empire 347 and 340.

## The Recruitment planner (read and ported 2026-10-18)

The Recruitment planner class has vtable `0x0065a6b8`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_0047da30` | the cancel key: the team's records' `+0x58` summed (leadership) |
| `+0x30` | `FUN_0047dad0` | the Mission Report |
| `+0x40` | `FUN_0047a470` | 0: the count `S+0x320` |
| `+0x48` | `FUN_0047db40` | the team selector (leadership 80..10000) |
| `+0x50` | `FUN_0047dcc0` | state 5, the target |

**The want.** Row 1 of `FUN_004e8290` asks while `S+0x320 < 2` and
`S+0xa4 > 0`, and cancels while `S+0x320 > 2`, or while `S+0xa4 == 0` and
`S+0x320 > 0`. `S+0xa4` is the side's MNCHARSD classes, counted once at
setup (`FUN_0042dd00`), so the side keeps two Recruitment planners all
game.

**The target, `FUN_0047dcc0`.** Without a member it empties `+0x34` and
returns 0. With one, it keeps `+0x34` while its record still has `+0x2c`
bit 2: the side's system, with no uprising or blockade, while `S+0xa4 >
0`. Otherwise it takes, the first that gives a system:

1. the system the first team member stands in. The walk goes up the
   member's containers with `FUN_004f2d10`, from the record's `+0x38`.
   It takes that system when its record has bit 2.
2. With `+0x38` a system: `FUN_00419b80(+0x38, 0, 8, 0, ...)`, order 2,
   gives the nearest sector with such a system. Then `FUN_0041a400(sector,
   +0x38, 0, 2, 0, ...)`, order 2, gives its such system nearest `+0x38`.
3. Otherwise `FUN_00419a00(0, 8, 0, 0, 0, 0, key 0x3c)`, order 1, gives
   the sector with the most such systems (`+0x160`). Then
   `FUN_0041a340(sector, 0, 2, 0, 0, 0, 0, key 0)`, order 1, gives its
   best supported such system.

It claims nothing.

**Ported** in `ai.rs`:
- `AISystem::mission_strategy` keeps rows 1 and 5 through
  `keep_planners`.
- `recruitment_target` is `FUN_0047dcc0`.
- The port's heuristic Recruitment is removed. It sent a recruiter to the
  side's most popular system for each character left in the pool.
- port: a standing Recruitment planner orders nothing while the side's
  pool is empty. Such a mission would end at once (code `0x10`,
  `FUN_0056b370`).

**Fixed with it: side `+0xa4`.** `minor_classes` counted characters whose
DatId family was `0x38..0x3b`. The loader keeps a character's record
index without its family byte, so in a real game it counted none. Then
no system record had `+0x2c` bit 2, and sector `+0x28` bit 8 and
`+0x160` stayed empty. It now counts the side's minor characters.

Campaigns: 24 seeds of 1500 days, against the Diplomacy commit.
- Recruitment orders: 1,703 before, 1,679 after. Results: 1,117 successes
  and 345 failures before, 1,224 and 409 after.
- Research orders rose: ship 2,275 to 10,312, troop 686 to 3,941,
  facility 744 to 4,161.
- Research successes rose: ship 3,041 to 4,922, troop 276 to 1,044,
  facility 420 to 1,677.
- Missions: 11,366 before, 26,519 after.
- Systems held at the end, summed over the seeds: Alliance 531 and 487,
  Empire 340 and 363.
- The research orders showed a fault that was already there. In 13 of
  the 24 seeds before this change, and 22 of 24 after, one designer was
  ordered to the same place again every few days, 35 to 394 times. Each
  of those missions ended without a result. Traced and fixed below.

## The research order's target (fixed 2026-10-18)

- `FUN_0047b360` (state 8) orders the mission on the planner's `+0x30`
  (at `0x47b3a0`).
- The research slots `+0x50` copy the chosen site `+0x34` into `+0x30`,
  as at `0x47c0cd` in `FUN_0047be50`. Diplomacy and Recruitment do the
  same.
- `+0x38` (state 6, `FUN_004bd0a0`) is only where the team gathers.
- The port's `research_cycle` ordered the research on `+0x38`. When a
  site's yard was scrapped or lost, `+0x38` kept the old system, which
  was still the side's. The research went there, and the mission's
  validator ended it the next day with code `0xc`, no research site
  (`FUN_0056c510`). The AI ordered it again on its next cycle.
- Fixed: the order goes to the site, and a cycle with no site ends in
  state 5.
- Campaigns, 24 seeds of 1500 days:
  - research orders fell from 18,414 to 602;
  - research successes rose from 7,643 to 8,300;
  - no seed repeats a designer's order more than 20 times.

## The Incite planner (read and ported 2026-10-18)

The Incite planner class has vtable `0x0065a808`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_0047fef0` | the cancel key: the team's records' `+0x58` summed (leadership) |
| `+0x30` | `FUN_0047c890` | the Mission Report, shared with Diplomacy and research |
| `+0x34` | `FUN_004806d0` | the abort test: the target record still has `+0x2c` bit `0x100` |
| `+0x40` | `FUN_0047fee0` | 0xc: the count `S+0x350` |
| `+0x48` | `FUN_0047ff90` | the team selector (leadership 49..) |
| `+0x4c` | `FUN_00480110` | the decoy selector |
| `+0x50` | `FUN_00480370` | state 5, the target |

**The want.** Row 7 asks while `S+0x350 < S+0x218` and cancels while
above. `S+0x218` is the sectors' `+0x178`, which counts systems with
record `+0x2c` bit `0x100`. Those are the opponent's systems where the
side's support is above 50. The sum becomes `/ 6 + 1` when above 0
(`FUN_00418500`).

**The target, `FUN_00480370`.** Without a member it empties `+0x34`.
With one, it keeps `+0x34` while the record has `+0x2c` `0x100` and
lacks `+0x30` `0x200`. Otherwise it takes, the first that gives a
system:

1. With `+0x38` a system: `FUN_00419b80(+0x38, 0, 0x200, 8, ...)`,
   order 2, gives the nearest sector with a candidate and opponent
   systems. Then `FUN_0041a400(sector, +0x38, 0, 0x100, 4, ...)`, order
   2, gives its candidate nearest `+0x38`.
2. Otherwise merge `FUN_00419a00(0, 0x200, 8, 0, 0, 0, key 4)` (the
   opponent systems `+0x80`) and `FUN_0041a1d0(0, 0x200, 8, ...)`, both
   order 1. Then `FUN_0041a340(sector, 0, 0x100, 4, 0, 0, 0, key 0)`,
   order 1, gives the candidate with the side's highest support.

It claims nothing.

**Ported**:
- `incite_wanted` gives `S+0x218`, and `incite_target` is
  `FUN_00480370`;
- `mission_strategy` keeps row 7;
- the port's heuristic Incite is removed. It went to the enemy system
  where the enemy was most popular. With it went `evaluate_officers`
  and the first mission group.

Campaigns: 24 seeds of 1500 days, against the research-order fix.
- Incite orders: 659 before, 404 after.
- Incite results: 300 successes and 3,691 failures before, 56 and 436
  after.
- Systems held at the end, summed over the seeds: Alliance 500 and 487,
  Empire 340 and 351.

## The Subdue planner (read and ported 2026-10-18)

The Subdue planner class has vtable `0x0065a618`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_0047ce40` | the cancel key: the team's records' `+0x58` summed (leadership) |
| `+0x34` | `FUN_0047d320` | the abort test: the target record still has `+0x2c` bit `0x20` |
| `+0x40` | `FUN_0047ce30` | 4: the count `S+0x330` |
| `+0x48` | `FUN_0047cee0` | the team selector (leadership 49..) |
| `+0x50` | `FUN_0047d060` | state 5, the target |

**The want.** Row 6 asks while controller bit `0x100` is set and
`S+0x330 < S+0x21c + 1`. It cancels while `S+0x21c + 1 < S+0x330`, or
while the bit is clear and `S+0x330 > 0`. `FUN_0041b7e0` sets bit `0x100`
for a sector record with own systems (`+0x2c` 4) and `+0x28` bit
`0x8000000`, which marks an own system below 70 support. `S+0x21c` sums
the sectors' `+0x164`: own systems in an uprising with no own fleet
(record `+0x2c` `0x20`).

**The target, `FUN_0047d060`.** Without a member it has none. With one,
it keeps `+0x34` while the record has `+0x2c` `0x20`. Otherwise:
- With `+0x38` a system: `FUN_00419b80(+0x38, 0, 0x10, 0, ...)`, order
  2, then `FUN_0041a400(sector, +0x38, 0, 0x20, 0, ...)`, order 2.
- Otherwise: `FUN_00419a00(0, 0x10, 0, 0, 0, 0, key 0x3d)`, the sector
  with the most uprisings (`+0x164`), order 1. Then `FUN_0041a340(sector,
  0, 0x20, 0, 0, 0, 0, key 0)`, its best supported revolt, order 1.

The two branches do not fall through to each other. It claims nothing.

**Ported**:
- `subdue_wanted`, `subdue_target`, and row 6 in `mission_strategy`.
- Before this, the port's AI sent no Subdue at all.
- With the last heuristic Diplomacy and Incite gone,
  `missions::record_admits_system` had no caller. It and its test are
  removed.

Campaigns: 24 seeds of 1500 days, against the Incite commit.
- Subdue orders: 0 before, 330 after (Empire 299, Alliance 31). Results:
  288 successes and 1,802 failures.
- Systems that fell to no side: 184 before, 27 after.
- Systems held at the end, summed over the seeds: Alliance 487 and 376,
  Empire 351 and 462.

## The Reconnaissance planner (read and ported 2026-10-18, save v82)

The Reconnaissance planner class has vtable `0x0065a7b0`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_0047f680` | the cancel key: the team's records' `+0x44` summed (espionage) |
| `+0x38` | `FUN_0047fe10` | release: clear the claim `0x10000000` on sector record `+0x70` |
| `+0x40` | `FUN_0047f670` | 9: the count `S+0x344` |
| `+0x48` | `FUN_0047f790` | the team selector |
| `+0x4c` | `FUN_00481100` | no decoys |
| `+0x50` | `FUN_0047f970` | state 5, the target |

**The want.** Row 11 asks with subtype 5 while `S+0x344` is below the
length of controller `+0xd8`, and cancels while above. `+0xd8` is the
list of module 8's sector planners (at `0x4e8669`), so the side keeps one
Reconnaissance planner per sector planner.

**The team, `FUN_0047f790`.** `FUN_00403460(0x800002, 0x200, 0x7400051,
0, 1, 0, 10000, 1)` gives one unassigned member of the side able for
Recon, by espionage. It must be a character or special force (`0x30..
0x3f`). With `+0x38` set, the nearness query `FUN_00403750` joins it.

**The target, `FUN_0047f970`.** It does not test for a team. It keeps
`+0x34` while the record has `+0x2c` `0x80` (not the side's). Otherwise it
releases its sector `+0x70`, then:

- While controller `+0x1c4 >= 1` (a sector record without `+0x24` bit
  `0x40000000`; that bit marks a headquarters or Coruscant):
  - the sector is near `+0x38`
    (`FUN_00419b80(+0x38, 0, 0x40, 0, 0, 0x50000000, 0x4000000)`, order
    2), or drawn (`FUN_0041a1d0` with the same masks, order 2);
  - the target is a random unexplored system not the side's
    (`FUN_0041a460(+0x70, 0, 0x80, 0, 0, 0x8000000, 0)`), order 2 near
    `+0x38`, else 1.
- Otherwise:
  - the sector masks forbid only `0x10000000` (and `+0x2c` `0x4000000`),
    the drawn sector order 1;
  - the target is the system not the side's seen longest ago
    (`FUN_0041a3a0(+0x70, 0, 0x80, 0, 0, 0, 0)`, order 1).

With a target it claims the sector (record `+0x28 |= 0x10000000`), which
the refresh keeps (`FUN_004321e0`: `+0x28 & 0x30000000`).

**Ported**:
- `recon_target`, `release_sector`, and row 11 in `mission_strategy`;
- `MissionPlanner::sector` and `AIState::sector_claims`, with
  `ai_records::apply_sector_claims` and `update_sector_state` (save v82);
- the Recon selector (`mission_planning`). Before, it was not ported, and
  the AI sent no Reconnaissance.
- The port's heuristic "reconnaissance" is removed. It sent Espionage
  missions to explored enemy systems.
- port: the nearness query does not join the selector, as for the other
  classes.

Campaigns: 24 seeds of 1500 days, against the Subdue commit.
- Reconnaissance orders: 0 before, 3,006 after (2,864 successes).
- Espionage orders: 1,194 before, 1,296 after. Results: 2,148 successes
  and 840 failures before, 2,460 and 602 after.
- Systems held at the end, summed over the seeds: Alliance 376 and 483,
  Empire 462 and 476.

## The Espionage planner of subtype 6 (read and ported 2026-10-18)

The strategy asks for Espionage with subtype 6 (`FUN_004e8290` row 12).
That class has vtable `0x0065a8a8`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_00480e10` | the cancel key: the team's records' `+0x44` summed (espionage) |
| `+0x38` | `FUN_00481420` | release: clear the claim `0x20000000` on sector record `+0x70` |
| `+0x40` | `FUN_00480e00` | 10: the count `S+0x348` |
| `+0x48` | `FUN_00480f30` | the team: one special force (`0x10800002`) able for Espionage with espionage 51.. |
| `+0x4c` | `FUN_00481100` | no decoys |
| `+0x50` | `FUN_00481110` | state 5, the target |

**The want.** Row 12 asks while `S+0x348 < S+0x1bc - |+0xd8|`, and
cancels while above. `S+0x1bc` counts the sector records with an
opponent's system (`+0x2c` 8, `FUN_0041b7e0`), and `+0xd8` is module 8's
sector planners.

**The target, `FUN_00481110`.** It does not test for a team. It keeps
`+0x34` while the record has `+0x30` 4 and `+0x2c` `0x200` (the
opponent's, explored). Otherwise it releases its sector `+0x70`. It then
picks a sector:
- near `+0x38` (`FUN_00419b80(+0x38, 0, 0x400, 8, 0, 0x20000000,
  0x4000000)`, order 2);
- or drawn (`FUN_0041a1d0` with the same masks, order 1).

The target is that sector's opponent system seen longest ago
(`FUN_0041a3a0(+0x70, 0, 0x200, 4, 0, 0, 0)`, order 1). The planner then
claims the sector (`+0x28 |= 0x20000000`).

**Ported**:
- `espionage_target`, row 12 in `mission_strategy`, and the claim bit
  `CLAIM_ESPIONAGE`;
- the Espionage selector is now subtype 6's: one member, no decoys. The
  class for other subtypes (`FUN_004808c0`, two members and a decoy) has
  no request the port makes;
- the heuristic Espionage on unexplored systems is removed. So are
  `system_strength`, which had lost its caller, and a stale doc comment.

Campaigns: 24 seeds of 1500 days, against the Reconnaissance commit.
- Espionage orders: 1,296 before, 1,840 after. Results: 2,460 successes
  and 602 failures before, 1,697 and 687 after.
- Systems held at the end, summed over the seeds: Alliance 483 and 415,
  Empire 476 and 412.

## Mission Reports of the standing planners (ported 2026-10-18, save v83)

The Diplomacy, Incite and Subdue classes share slot `+0x30`,
`FUN_0047c890`, with the research planners:
- result 2 (failed) runs `FUN_0047ae20`. It counts `+0x6c` against slot
  `+0x24`: 5 for Diplomacy and Subdue (`FUN_00404c20`), 10 for Incite
  (`FUN_004820c0`). Past it, the planner sets `+0x20` `0x80` and
  `0x20000000` (abort) and clears the count; otherwise `0x40000000`
  (check).
- result 3 (succeeded) clears the count and sets the check.
- Both run `FUN_0047b950`, which clears `0x80000000` and readies the
  mission's record.

State 1 (`FUN_0047b250`) runs only while the planner's mission `+0x28`
runs (`FUN_004f3000`, `FUN_00520b30`):
- the check asks slot `+0x34` whether the target still qualifies:
  `FUN_00481eb0` (Diplomacy, record `+0x2c` `0x40`), `FUN_004806d0`
  (Incite, `0x100`) or `FUN_0047d320` (Subdue, `0x20`). If not, the check
  turns into the abort;
- the abort builds order `0x250` on the mission (end code 2);
- then both bits clear. With no running mission they wait.

**Ported**:
- `AISystem::mission_planner_reports`, run before the planners' cycles;
- `MissionPlanner::mission`, `report`, `failures` and `pending` (save
  v83);
- `AIState::bind_mission` and `note_mission_results` now reach the
  standing planners.
- port: the `0x80` bit is not kept, and no reader of it is traced.

The other classes' report slots (ported 2026-10-19):
- Recruitment, `FUN_0047dad0` (at `0x47dad0`):
  - a success clears the count, and while the target record is the
    side's with no enemy fleet (`+0x30` 1, not `0x200`) copies `+0x34`
    into `+0x38`;
  - a failure only counts against slot `+0x24` (`FUN_00480df0`, 1000).
- Reconnaissance, `FUN_0047f720`:
  - a success marks the target record seen today (`FUN_00433dc0`:
    `+0x44` = the day, `FUN_004fcf00`) and clears the count;
  - either result empties `+0x34`.
- Espionage, `FUN_00480eb0`: the same, and it also empties `+0x38`.
- Espionage teams are special forces, so `bind_mission` now takes the
  order's first member of any kind.

Campaigns: 24 seeds of 1500 days, against the Espionage commit.
- Aborts: 216 before, 871 after.
- Diplomacy results: 3,572 successes and 1,829 failures before, 2,471 and
  1,338 after.
- Incite failures: 413 before, 203 after.
- Systems held at the end, summed over the seeds: Alliance 415 and 399,
  Empire 412 and 416.

## The Abduction planner (read and ported 2026-10-19)

The Abduction planner class has vtable `0x0065a9f8`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_004831f0` | the cancel key: the team's records' `+0x54` summed (combat) |
| `+0x40` | `FUN_004831e0` | 6: the count `S+0x338` |
| `+0x44` | `FUN_004047d0` | 4: the decoy cap |
| `+0x48` | `FUN_00483290` | the team (2, combat 51..) |
| `+0x4c` | `FUN_00483410` | the decoys (special forces). It runs even without a team |
| `+0x50` | `FUN_00483640` | state 5, the target |

**The want.** Row 9 asks while `S+0x338 < 1` and cancels while above 1, so
the side keeps one Abduction planner all game.

**The target, `FUN_00483640`.** It first counts the decoys standing free
at the gathering place `+0x38` (`FUN_004bd000`, over list `+0x44`). It
keeps its character `+0x30` while that still stands at `+0x34` (the walk
up its containers) with record `+0x30` 8 and 4 and no `0x60`.
Otherwise it lists the systems holding an opponent's main character:
- the sectors with `+0x28` `0x100`, by key `+0x174`, order 1, taken in
  turn;
- in each, the systems with `+0x2c` `0x1000` (one on the planet), by
  key `v[47]`, order 1, then those with `0x2000` (one aboard a fleet),
  order 2.

It takes the first system where the gathered decoys equal the cap or
reach half the record's `+0xbc` (its defense).

`FUN_00483d10` then picks the character. It lists the opponent's main
characters (record 8 and 4):
- on the planet, when the record has `0x1000`;
- aboard a fleet there that is not the side's and carries one (fleet
  record `+0x38` `0x200000`), when it has `0x2000`.

It takes one by `FUN_0041cd80(count)`. System bit `0x2000` comes from a
fleet record with `0x200000`, which a ship carrying a free opponent main
character sets (ship `0x40000`).

**Ported**:
- `abduction_target`, `abduction_character`, and row 9 in
  `mission_strategy`;
- the heuristic Abduction (the weakest enemy characters first) is
  removed.
- port: the candidates are taken in the world's character order.

Campaigns: 24 seeds of 1500 days, against the report-slots commit.
- Abduction orders: 185 before, 558 after. Results: 129 successes and 8
  failures before, 136 and 276 after.
- Systems held at the end, summed over the seeds: Alliance 607 and 599,
  Empire 539 and 540.

## A prisoner's side in the records (fixed 2026-10-19)

`FUN_00401d20` compares the object's side `+0x24` with the record's side
(`param_1[8]`):
- the same side sets `+0x30` 2, or 4 for a prisoner (`+0xac` bit 0);
- another side sets 4, or 2 for a prisoner.

A capture turns `+0x24` to the side opposite the character's home
(`FUN_004eec60`, `captive-escape.md`). So the two flips cancel, and a
prisoner counts on its home side, with `0x40`.

The port read `(is_alliance == side) != captive` on the home side, so it
put a prisoner on its captor's side. Fixed in `mission_planning::records`.

With the fix:
- `FUN_0041bc10`'s controller bit `0x200` (row 10, Rescue) reads "a
  character of the side is a prisoner", as `ai-mission-planning.md`
  says;
- the system bit `0x4000` marks an own prisoner held there, Rescue's
  target.

Campaigns: 24 seeds of 1500 days, against the Abduction commit. The
results barely move: Rescue orders 409 and 429, Abduction 558 and 552.
Systems held at the end, summed over the seeds: Alliance 599 and 607,
Empire 540 and 536.

## The Rescue planner (read and ported 2026-10-19)

The Rescue planner class has vtable `0x0065a710`:

| Slot | Function | Meaning |
|---|---|---|
| `+0x18` | `FUN_0047e1d0` | the cancel key: the team's records' `+0x54` summed (combat) |
| `+0x40` | `FUN_0047e1c0` | 8: the count `S+0x340` |
| `+0x44` | `FUN_004047d0` | 4: the decoy cap |
| `+0x48` | `FUN_0047e270` | the team (2, combat 51..) |
| `+0x4c` | `FUN_0047e3f0` | the decoys |
| `+0x50` | `FUN_0047e620` | state 5, the target |

**The want.** Row 10 asks while controller bit `0x200` is set and
`S+0x340 < 1`. It cancels while above 1, or while the bit is clear and
any planner remains. `FUN_0041bc10` sets the bit for a record with
`+0x30` 2, `0x20000000` and `0x40`: a character of the side who is a
prisoner, since a prisoner counts on its home side.

**The target, `FUN_0047e620`.** It reads like Abduction's, with these
differences:
- The keep test reads `(flags | 0x20) == 0` (`or eax, 0x20; jne` at
  `0x47e800`), which never holds, so the target is chosen anew each
  cycle.
- The sectors have `+0x28` `0x4000`, by key `+0x180`, order 1.
- In each sector, the systems with a prisoner on the planet (`+0x2c`
  `0x4000`) come by key `v[29]`, order 1, then those with one aboard a
  fleet (`0x8000`), by `v[29]`, order 2.
- The decoy test is Abduction's: the decoys gathered at `+0x38` fill the
  cap (4) or reach half the record's `+0xbc`.

`FUN_0047ecf0` then draws a prisoner of the side (record `0x40` and 2):
- on the planet, with `0x4000`;
- aboard a fleet there not the side's carrying one (fleet record `+0x38`
  `0x1000000`), with `0x8000`.

**Ported**:
- `rescue_target`, `rescue_character`, and row 10 in `mission_strategy`;
- the heuristic Rescue (one per captive) is removed, and with it the
  last group of mission heuristics.

Campaigns: 24 seeds of 1500 days, against the prisoner-side fix.
- Rescue orders: 429 before, 327 after. Results: 193 successes and 6
  failures before, 278 and 21 after.
- Systems held at the end, summed over the seeds: Alliance 607 and 570,
  Empire 536 and 579.

## Diplomacy's slot +0x3c (ported 2026-10-19)

State 2 (`FUN_0047ba40`) keeps a member only while its class's slot
`+0x3c` agrees. For Diplomacy that slot is `FUN_00481ef0`. It runs the
team query with priority 0, so only free records count:
`FUN_004035d0(0, 2, 1, 0x7404051, 0, 0, 0x45, 10000, 1)`. It takes the
best (`FUN_004357b0`). When that one's diplomacy (record `+0x40`)
exceeds the member's, it returns 0, and the planner lets the member go.
State 3 then takes the better one. Every other class's slot is
`FUN_0051ebb0`, which keeps.

Ported as `Planner::outranked`, asked in `mission_planner_cycle`'s state
2.

Campaigns: 24 seeds of 1500 days, against the Rescue commit.
- Diplomacy orders: 1,140 before, 1,242 after.
- Diplomacy results: 3,543 successes and 1,850 failures before, 3,729
  and 1,996 after.
- Systems held at the end, summed over the seeds: Alliance 570 and 589,
  Empire 579 and 564.

## The sector planners' mission planners (read 2026-10-19, not ported)

Sabotage and Assassination have no row in the strategy. They come from
module 8's sector planners, as do a second Recon and Espionage class.

**The requests.** Each plan kind's slot `+0x18` is the shared
`FUN_004b7150`. It reads the sector planner's counts of the mission
planners it holds (`+0xa0..+0xac`) and the sector record `+0x2c`:

| Request | When | Cancel when |
|---|---|---|
| Recon `0x54` | fewer than 2 held (`+0xa8`) and `+0x2c & 0x18` | more than 2, or none of `0x18` with any held |
| Espionage `0x52` | none held (`+0xa4`) | more than 1 |
| Sabotage `0x69` | none held (`+0xac`) and `+0x2c & 8` | more than 1, or no `8` with any held |
| `0x62` (Alliance) or `0x63` (Empire), subtype 4 | none held (`+0xa0`) and `+0x2c & 0x40008` | more than 1, or no `0x40008` with any held |

The request goes to `+0xbc`/`+0xc0`, the cancellation to `+0xc4`
(`FUN_00478000`).

**The take.** The mission module's step runs `FUN_004d2610`. It walks
the sector planners from the cursor `+0x44` to the first with a pending
request (`FUN_00476b80`).

`FUN_004d2690` then works on that planner:
- it cancels the planner named in `+0xc4` (`FUN_0042f690`);
- it builds the requested one (`FUN_0042f640` -> `FUN_0042f830`);
- it hands the new planner to the sector planner (`FUN_00476ba0`), which
  clears `+0xbc`/`+0xc0` and holds it in its list.

**The classes** (`FUN_0042f830`):

| Kind | Constructor | Vtable |
|---|---|---|
| Recon, subtype not 5 | `FUN_0047f190` | `0x0065a768` |
| Espionage, subtype not 6 | `FUN_00480730` | `0x0065a860` |
| Sabotage `0x69` | `FUN_0047d380` | `0x0065a670` |
| `0x62` subtype 4 and `0x63` | `FUN_00482b30` | `0x0065a9b0` |

Their vtables end at slot `+0x44`: `+0x40` is the team selector and
`+0x44` the decoys. Slot `+0x2c` is another state machine, `FUN_004bda60`:

- 1 reads the reports, as `FUN_004bced0` does;
- 2 (`FUN_004bdbc0`) takes the target from the owning sector planner
  `+0x70`: `FUN_00476e80(kind, &+0x34, &+0x30)` gives the plan's system
  and object for the mission's family. Without one the cycle ends (9);
- 3 (`FUN_004bdcd0`) picks the gathering place `+0x38`;
- 4 keeps the members (`FUN_0047b110`);
- 5 runs the team selector, 6 the decoys;
- 7 moves the members (`FUN_004be230`);
- 8 orders the mission (`FUN_0047b360`);
- 9 (`FUN_004be4c0`) ends the cycle.

**The plan's targets.** `FUN_00476e80` switches on the planner's record
index (`+0x2c & 0xffffff`) and calls one of the plan object's slots
(`+0x5c`) with `&+0x34` (system) and `&+0x30` (object):

| Index | Mission | Plan slot |
|---|---|---|
| `0x12` | Sabotage | `+0x30` |
| `0x13` | Espionage | `+0x28` |
| `0x15` | Reconnaissance | `+0x2c` |
| `0x17`, `0x81` | Abduction, Assassination | `+0x24` |

Any other index gives no target. The six plan kinds' slots are:

| Kind | `+0x24` | `+0x28` | `+0x2c` | `+0x30` |
|---|---|---|---|---|
| 1 | `FUN_004ba630` | `FUN_004ba830` | `FUN_004ba960` | `FUN_004bab30` |
| 2 | `FUN_004b94b0` | `FUN_004b96b0` | `FUN_004b97e0` | `FUN_004b99b0` |
| 3 | `FUN_004b3880` | `FUN_004b3bf0` | `FUN_004b3db0` | `FUN_004b3f80` |
| 4 | `FUN_004b7370` | `FUN_004b7780` | `FUN_004b7940` | `FUN_004b7b10` |
| 5 | `FUN_004b1880` | `FUN_004b1bf0` | `FUN_004b1db0` | `FUN_004b1f80` |
| 6 | `FUN_004b4e40` | `FUN_004b51b0` | `FUN_004b5370` | `FUN_004b5540` |

Together they run about 2,500 decompiled lines, not read yet.

**Port.** None of this is ported. The heuristic Sabotage (enemy
manufacturing systems) and Assassination (enemy main characters) in
`evaluate_espionage` stand in. Next:
- the sector planners' requests (the port's `SectorPlanner::missions`
  asks only Recon or Espionage today);
- `FUN_004d2610`/`FUN_004d2690`, and holding the planners;
- `FUN_00476e80` with the plan objects' targets;
- `FUN_004bdcd0` and `FUN_004be230`.

## The controller's leader bit (read and ported 2026-10-19)

`FUN_004182a0` sets the AI's controller up (case 1). It reads the game's
settings through `FUN_004fce50`, a 16-byte copy of game `+0xc0` `+0x58`
whose defaults are 0, 0, 7, 0 (`FUN_0051c740`). It sets controller `+4`
bit 0 when field `+8` is 1.

The Objectives lists (`FUN_0044b2d0`, `FUN_0044bb70`) read the same field
as the game type: `+8 == 0` shows the standard game's leader conditions
beside the VictoryConditions word (side `+0xbc`). So the bit marks a
Headquarters Only game. `FUN_00401d20` then marks no record as a leader
(`0x40000000`).

That has two effects. The selectors that forbid a leader (Incite, Rescue,
Abduction, Sabotage, `0x4f400051`/`0x0f400051`) can take one. And a
leader aboard a ship no longer adds to the ship record's `v[18]`
(`FUN_00484630`).

Ported:
- `AIState::set_game_type`, called each day by the simulation and the
  app from the campaign's setup;
- `LEADERS_UNMARKED`;
- `Planner::without_leaders`, applied in `AISystem::side_planner`.

The headless runs play the standard game, so their output is unchanged.

## Still open

- The fleet record bit `0x10` behind `F & 4`.
- The posture (`FUN_00478000`, `FUN_004b7150`) and its sector bits.
- Whether the per-system record's `+0x34` is a DatId or coordinates: it is
  the system's map position (2026-10-14, see above).

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
`FUN_004d2580`, `FUN_004d27b0`, `FUN_0054d360`, `FUN_004357b0`,
`FUN_00435790`, `FUN_00435980`, `FUN_005f59f0`, `FUN_0041c070`,
`FUN_0041c230`, `FUN_0041c250`, `FUN_0041c360`, `FUN_0041cd80`,
`FUN_0041b7b0`, `FUN_004047d0`, `FUN_00401d20`, `FUN_00402230`,
`FUN_004024d0`, `FUN_004025f0`, `FUN_00402e80`, `FUN_004ed1c0`,
`FUN_004ed1e0`, `FUN_004ed200`, `FUN_005f4960`, `FUN_00478000`,
`FUN_004b7150`, `FUN_004762b0`, `FUN_00419870`, `FUN_005830a0`,
`FUN_00582fb0`, `FUN_00583320`, `FUN_00523450`.
