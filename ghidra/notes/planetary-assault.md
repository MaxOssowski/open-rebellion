# Planetary Assault: the order and its resolution

This note records the Fleet menu's Planetary Assault order (`0x234`) and
how the game resolves it. Read 2026-10-16 (module 8, stage 5e). It
supersedes `ground-combat.md`'s account of how ground battles happen:
that note's `FUN_00560d50` family test reads `0x14..0x1b` as regiments,
which are capital ships (regiments are `0x10..0x13`), and the port's
`CombatSystem::resolve_ground` built on it has no counterpart here.

The manual: "In order to take over a planet, you need to conduct a
planetary assault, which means that the troops on your fleet go down to
the planet surface, fight any ground troops, and, if successful, establish
a garrison ... A battery can fire at an assaulting Fleet regiment and
destroy it. Shields protect the planet from assaults and bombardments. A
system with two shields is called fully shielded, which means you cannot
conduct an assault of that system" (Offensive play, Fig. 2.57-2.58);
"Generals improve the abilities of troops to take control of a system
through planetary assault" (Fleet menu, Characters); the Assault Summary
window reports the outcome at once (Fig. 3.66-3.67). The claims below that
the code shows match; the two-shield rule is read only in the AI's test
(`FUN_004ae240`, below), not in the order's own check.

## The order

`FUN_0051f4b0` registers `0x234` with `FUN_00581450` (0x48 bytes,
`FUN_00581340`, vtable `0x669e70`; a sub-order list at `+0x44`,
`FUN_0054ede0`), the same shape as the bombardment orders
(`bombardment-order.md`).

| Slot | Function | Role |
|---|---|---|
| `+0x0c` | `FUN_005814c0` | the kind, `0x234` |
| `+0x18` | `FUN_005814d0` | Enabled: `FUN_00553770` (a team of one side), then `FUN_00581ae0` |
| `+0x1c` | `FUN_005815f0` | Validator: each sub-order's `+0x1c` (`FUN_00553960`) |
| `+0x20` | `FUN_005816f0` | Execute: the sub-orders go out (`FUN_005539f0`) |
| `+0x4c` | `FUN_00581850` | Build: the sub-orders for the team and target (`FUN_005535b0`) |

`FUN_00581ae0(team, target)`: each member accepts the target
(`FUN_0053f150`, the members' slot `+0x2c`, not read further); then of
the team's fleets (families `8..0xf`) one active (`+0x50` bit 1) with
`+0x58` bit `0x20` and a regiment aboard one of its capital ships
(`FUN_00502db0(fleet, 1)`: families `0x14..0x1c`; `FUN_00504c40`,
`FUN_005131b0`) enables the order. With no such fleet the refusal is
`8`/`7`; with fleets but no regiment, `8`/`5`.

## The resolution

The game event class at vtable `0x6632d8` (the bombardment's sibling at
`0x663300`) runs `FUN_00563390`: it resolves the system (`+0x3c`) and
runs an assault object (`FUN_0058c0d0` over `FUN_0058b9b0`, vtable
`0x66a8f8`): `FUN_0058c1e0(system, ...)` then `FUN_0058c120`.

**`FUN_0058c1e0`.** The fleets in the system with `+0x58` bit `0x80`
(assaulting, set by the order's sub-orders) must all be one side; the
object keeps the system (`+0x10`), that side (`+0x14`) and the system's
side (`+0x18`).

**Walks.** `FUN_0058ba10` walks the assaulting fleets of the attacking
side, their capital ships (`FUN_00502e30`) and the regiments aboard each
(`FUN_00504cc0`); `FUN_0058bbd0` walks the system's regiments of its own
side. Visitors (vtables `0x66a8d8..0x66a8f0`, one slot `+4`) count
(`FUN_0058bce0`), take the `n`th (`FUN_0058bd60`), land (`FUN_0058be10`)
or fight (`FUN_0058bec0`).

The six phases run in order (`FUN_0058c120`); each returns success.

**1, `FUN_0058c2d0`, the count.** The system's `+0x88` bit `0x80`
(assaulted) is set (`FUN_0050a6d0(1)`); the attackers' regiments are
counted into `+0x1c` and `+0x24` (left), the defenders' into `+0x20` and
`+0x28` (left).

**2, `FUN_0058c3c0`, the defenses.** Each of the system's defense
facilities (`FUN_00526fd0(system, 1)`, families `0x22..0x27`; the walk's
side filter is not read) fires with chance
`attack / G0x200` percent (class `+0x5c`, DEFFACSD attack strength;
`FUN_0055ead0`: `FUN_0053e2f0(attack / 5)`, a draw of 0..99 below it). A
hit destroys a drawn attacking regiment (`FUN_0053e290(+0x1c - 1)`, the
`n`th of the walk; slot `+0xac(0xe)`), and `+0x24` falls by one; none left
stops the phase.

**3, `FUN_0058c500`, the battle.** With attackers and defenders left, each
attacking regiment in walk order fights (`FUN_0058bec0`):
- a draw `r` of `0..+0x20 - 1` (the defenders counted at the start); when
  `r` is below the defenders left (`+0x28`) the `r`th of the system's
  regiments is the opponent, else this regiment does not fight;
- `+0x2c` (the duels) rises by one;
- the leaders: the system's officer of rank 3 (a general, `FUN_00509330`)
  and the fleet's (`FUN_004fd790`), each's slot `+0x1f4` (leadership);
- `FUN_0055eb00(a, gA, d, gD)`: `x = FUN_0053e290(G0x202) + (a + gA /
  G0x201) - (d + gD / G0x201)` with `a` the attacker's class `+0x64`
  (TROOPSD attack), `d` the defender's class `+0x68` (defense); the
  attacker is lost when `x <= G0x203`, the defender when `x >= G0x204`;
- a lost defender: `+0x28` less one, the regiment destroyed (slot
  `+0xac(0xe)`); else a lost attacker: `+0x24` less one, destroyed;
- the phase stops when either side has none left.

**4, `FUN_0058c580`, the collateral damage.** Of the duels (`+0x2c`),
each with chance G`0x205` percent (`FUN_0055eb80`) destroys one of the
system's things: `FUN_0055ebc0(F, P, energy, raw)` with `F` the system's
facilities (`FUN_00539d70`, families `0x20..0x2f`) and `P` its
placeholders (`FUN_00526cf0`, `0x20..0x21`): a draw `r` of `0..(F - P) +
[energy > 0] + [raw > 0] - 1`; below `F - P` the `r`th facility (the
first placeholder skipped, hyp), equal to it an energy unit (system
`+0x5c` less one, `FUN_00509cc0`) or else a raw unit, above it a raw unit
(`+0x64`, `FUN_00509dc0`). A walk past the list ends the phase.

**5, `FUN_0058c810`, the landing.** With attackers left (`+0x24`) and no
defender (`+0x28` 0): up to `N` attacking regiments, in walk order, move
to the system (`FUN_004f6fd0`), `N` = `FUN_00559fe0(side, 0, 0, 0, 0)` =
`FUN_0055a050(0)`: `ceil((G0x1e51 - 0) / -G0x1e52)` = `ceil(60 / 10)` = 6,
the garrison a system at support 0 needs.

**6, `FUN_0058c8b0`, the end.** The system's bit `0x80` is cleared
(`FUN_0050a6d0(0)`) and every fleet of the attacking side there loses
`+0x58` bit `0x80` (`FUN_004fe4d0(0)`).

The system's control then follows its garrison and support as elsewhere
(not read here).

## The AI's test (`FUN_004ae240`)

`FUN_004ae240(fleet record, system record)` (`ai-sector-plans.md`, kind
4): the system has `+0x30` `0x20` and lacks `0x101` (not the side's and
not fully shielded: `0x100` is more than one `0x24` defense). At the
fleet's own system both thresholds are 0, else `a` = max(system `v[27]`,
controller `+0x168`) and `b` = system `v[23]`. True when the fleet's
`values[3]` is at least `a`, its `values[12]` (regiments) at least the
system's `v[52]`, and its `values[19]` is above 0 or its `values[9]` at
least `b`.

## GNPRTB

| Id | Global | Value | Use |
|---|---|---|---|
| `0x200` | `DAT_006bb788` | 5 | a defense's hit chance divisor |
| `0x201` | `DAT_006bb780` | 30 | a general's leadership divisor |
| `0x202` | `DAT_006bb78c` | 10 | the duel's draw, 0 to 10 |
| `0x203` | `DAT_006bb778` | 4 | the attacker is lost at or below it |
| `0x204` | `DAT_006bb784` | 6 | the defender is lost at or above it |
| `0x205` | `DAT_006bb77c` | 8 | a duel's collateral chance, percent |
| `0x1e51` | `DAT_006bb480` | 60 | the landing's support base |
| `0x1e52` | `DAT_006bb3a8` | -10 | its step |

`FUN_0053e2f0(p)` is a draw of 0..99 (`DAT_00661a88` = 100) below `p`;
`FUN_0053e290(n)` a draw of 0 to `n`.

## Ported (2026-10-16, module 8, stage 5f)

`crates/rebellion-core/src/assault.rs`: `resolve` runs the six phases
(the counts, the defenses, the duels, the collateral damage, the landing;
phase 6's marks are not kept), `apply` carries out the losses, the
collateral damage and the landing, `order_enabled` is the order's
enabled check, `BombardEnv::assault` draws from the tick's stream.
`AIAction::Assault` is validated by `order_enabled` and carried out by the
integrator and the app (telemetry `combat_ground`, the app's Assault
Summary line). No plan object binds the assault role yet (kind 4).

Removed (by the user's choice): the port's automatic landing on another
side's planet, its automatic ground battle (`CombatSystem::resolve_ground`
and its integrator helpers, the simulation's step 3b battle, the app's
`tactical_flow` ground routes) and the app's interactive ground-combat
screen (`rebellion-render` `ground_combat.rs`). Regiments carried to a
system land only on their side's standing, unblockaded planet
(`FUN_005073d0`); one side's regiments alone still take a planet (port).

port:
- No officer ranks: the generals' leadership terms are 0.
- A fleet's regiments are its cargo; the walk takes the fleets in the
  system's order and each one's cargo in order.
- The order's members' target test (`FUN_0053f150`) is not read; in its
  place each fleet orbits the system out of hyperspace, the system stands,
  is not the side's and has at most one shield (the manual's
  fully-shielded rule).
- hyp: phase 2 fires every defense facility of the system; phase 4's
  facilities are the system's defenses, then manufacturing, then
  production facilities, of any side, the headquarters placeholder left
  out.
- The draws come from the tick's `SplitMix64` stream (`BombardEnv`).

## Against the port (before stage 5f)

- The port has no Planetary Assault order: the player and the AI land
  regiments by Move, and the simulation (`simulation.rs`) and the app
  (`tactical_flow.rs`) run `CombatSystem::resolve_ground` whenever both
  sides have regiments at a system. The original fights on the ground
  only in this event. The port already refuses a regiment's Move onto the
  other side's populated system (`FUN_0053d430`, `RegimentUnloadRefused`).
- The port has no officer ranks, so the generals' terms would be 0.
- No duel draws a defense facility or a fleet; the space battle and the
  bombardment are separate events.

Open: the members' target test (`FUN_0053f150`, slot `+0x2c`) and where
the two-shield rule sits in it; how the sub-orders set fleet `+0x58`
`0x80` and what sets `0x20`; the Assault Summary message; the support
change the manual describes after an assault.
