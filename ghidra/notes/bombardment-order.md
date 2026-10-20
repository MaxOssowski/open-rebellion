# Planetary Bombardment: the orders and their resolution

This note records the Fleet menu's Planetary Bombardment orders (`0x220`
Target Military Facilities, `0x221` Target Civilian Facilities, `0x222`
General Bombardment, `0x223` Destroy System) and how the game resolves
them. It supersedes `bombardment.md`, whose chain (`FUN_00556430`,
`FUN_0055d8c0`) is the transit computation (F-032). Read 2026-10-16.

The manual (the Fleet menu, Planetary Bombardment): the sub-menu
"becomes available when your fleet is in orbit around an enemy or
neutral system. If you bombard a system that has defensive batteries,
they will fire at your ships in orbit ... (Exception: The planet-based
ion cannon ... robs energy from an attacking vessel, preventing it from
being able to bombard the system during the attack.) If a system has one
or more planetary shields, the attacking ships must first blast through
the shield(s). Each ship's Bombardment Modifier ... is combined and
pitted against the shield's defensive strength. Any bombardment
firepower that gets past the shield is used against troops and
facilities on the ground." Admirals "magnify the effects of ships on
bombardment" (Characters, ranks). Civilian damage lowers support in the
sector, a Death Star's destruction throughout the galaxy (Popular
support). Every claim below matches.

## The orders

`FUN_0051f4b0` registers the factories: `0x220` `FUN_00582980` (vtable
`0x66a028`), `0x221` `FUN_00582710`, `0x222` `FUN_005828b0` (`0x669fd0`),
`0x223` `FUN_005827e0` (`0x669f78`). All are 0x48 bytes over
`FUN_00581d70` (vtable `0x669ec8`, a sub-order list at `+0x44`,
`FUN_0054ede0`) and differ only in slot `+0xc`, the kind.

| Slot | Function | Role |
|---|---|---|
| `+0x18` | `FUN_00581e80` | Enabled: `FUN_00553770` (a team, every member's slot `+0x18`, all of one side, status `1`/`0x16`, `1`/`0x14`), then `FUN_00582440` |
| `+0x1c` | `FUN_00581fa0` | Validator: each sub-order's `+0x1c` (`FUN_00553960`) |
| `+0x20` | `FUN_005820a0` | Execute: each sub-order goes out to the game (`FUN_005539f0` -> `DAT_006b2b00` = `LAB_004fcff0` -> `FUN_0051d8d0` -> `FUN_0054ee30`) |
| `+0x4c` | `FUN_00582200` | Build: the sub-orders for the team and target (`FUN_005535b0`) |

`FUN_00582440`: of the team's fleets (families `8..0xf`, `+0x50` bit 1),
one with bombardment (`FUN_004fd6b0` above 0) enables the order (else
`8`/`6`); for `0x223` that fleet must also hold a Death Star
(`FUN_005003d0(fleet, 1)` and `FUN_005131b0`; else `8`/`8`).

## The resolution (`FUN_005637c0`)

The game's handler resolves the system (`+0x3c`) and runs a bombardment
object (`FUN_0058d720`, vtable `0x66a950`): `FUN_0058d800(system, order)`,
then `FUN_0058d770`.

**`FUN_0058d800`.** The kind sets the target classes:

| Kind | `+0x34` ground | `+0x38` facilities | `+0x3c` resources | `+0x40` military | `+0x44` destroy |
|---|---|---|---|---|---|
| `0x220` | 1 | | | 1 | |
| `0x221` | | 1 | | | |
| `0x222` | 1 | 1 | 1 | | |
| `0x223` | 1 | 1 | 1 | | 1 |

The fleets in the system with `+0x58` bit `0x40` (bombarding, set by the
order's sub-orders through fleet slot `+0x1d4`, `FUN_004fec00` ->
`FUN_004fe460(1)`) must all be one side; the object keeps the system
(`+0x10`), that side (`+0x14`) and the system's side (`+0x18`). The four
phases run in order; each returns success.

**Lists.** `FUN_0058ca70` walks the bombarding fleets (side `+0x14`,
`FUN_004ffef0`) and their capital ships (`FUN_00502e30`, families
`0x14..0x1b`). `FUN_0058cba0` walks the targets: with `+0x34` the
system's regiments (`FUN_00504cc0`, families `0x10..0x13`), defense
facilities (`FUN_00527050`, `0x22..0x27`) and placeholders
(`FUN_00526d70`, `0x20..0x21`), each of the system's side; with `+0x38`
its manufacturing and production facilities (`FUN_0053b760`,
`0x28..0x2f`); with `+0x3c` an energy unit (system `+0x5c` above 0) and a
raw material unit (`+0x64` above 0). Visitors (vtables `0x66a918..`)
count (`FUN_0058d1a0`, `FUN_0058d210`) or take the `n`th
(`FUN_00587470`, `FUN_0058d2f0`).

**1, `FUN_0058d990`.** The system's `+0x88` bit `0x40` (bombarded) is set
(`FUN_0050a660(1)`). With `+0x44`, a Death Star among the bombarding
ships (family `0x18..0x1b`, `FUN_0058d440`) destroys the system
(`FUN_0058ce70`: slot `+0xac` with `0x10` for the attacker's own, else
`0xf`), sets `+0x2c` and `+0x28`, and each minor character there
(families `0x38..0x3b`, `FUN_0056f3d0`) not yet hit (`+0x50` bit 5) is
hit with probability GNPRTB `0x606` (50%, `FUN_0055e9e0`): slot `+0x2f0(1,
10)`, and when `+0x50` bit 6, slot `+0xac(10)`. Without it (or another
kind) `+0x1c` counts the bombarding ships, `+0x20` the targets, and
`+0x24` is 1 for `0x220`.

**2, `FUN_0058dc10`, the batteries.** For each of the system's family
`0x22` facilities (`FUN_00527550`) a bombarding ship drawn at random
(`FUN_0053e290(+0x1c - 1)`, 0 to count - 1) takes `(general / G0x600 + 1)
* class +0x5c` (`FUN_0055e930`; G `0x600` = 40; the system's rank 3
officer, `FUN_00509330(3)`, slot `+0x1f4` leadership `+0x88`; class
`+0x5c` is DEFFACSD attack strength) into its first count (`+0x54` ->
`+0x20`); the family `0x23` facilities (`FUN_005272e0`) into its second
(`+0x24`). Family `0x22` is the KDY v-150 ion cannon (DEFFACSD 1), the
manual's exception; `0x23` the planetary batteries (2 and 5). Then each ship (`FUN_0058d540`): shields `S` = slot `+0x1e0`
less `+0x204` (its shields now), `H` its hull damage room (class hull
`+0xc0` less `+0x60` damage); the first count, then the second, is
absorbed by `S` (`FUN_0055e9a0(a, b)`: the excess of one over the other);
the second's excess falls on `H`; the shields lost go to `+0x28`; the
hull damage grows by the second's excess (`FUN_00501ae0`, capped; at the
hull the ship is destroyed, slot `+0xac(0xf)`). The first count's excess
stays: a ship with one cannot bombard (`FUN_00500f00`). No bombarding
ship left: `+0x30`, the bombardment stops.

**3, `FUN_0058df60`, the bombardment.** Unless stopped: firepower is the
sum over the bombarding fleets of `(admiral / G0x601 + 1) * bombardment`
(`FUN_0055e950`, G `0x601` = 40; the fleet's rank 2 officer,
`FUN_004fd790(2)`, leadership; `FUN_004fd6b0`: each ship's slot `+0x1dc`
and each squadron's aboard). A ship's is CAPSHPSD `bombardment_modifier`
(`+0xd4`) less its share of hull damage (`modifier * damage / hull`,
`FUN_00500f00`), 0 with a first count left (when `+0x24 & 0x30` is 0); a
squadron's FIGHTSD `+0xcc` less its share of `+0x60`. Shields are the
sum of the system's family `0x24` facilities' class `+0x60` (DEFFACSD
shield strength; side `+0x18`, `FUN_00527940`). Shots: firepower less
shields when above 0. With shots and `0x220`, collateral damage with
probability `+0x24 / (+0x20 + +0x24)` (`FUN_0053e290(+0x20 - 1 + 1) <
1`): a manufacturing or production facility (`FUN_0058d080`; the
decompile takes the first of `FUN_0053b760`, its counter never
advancing) gets the destruction roll. Then up to `shots` times: a target
drawn at random (`FUN_0053e290(+0x20 - 1)`, `FUN_0058d290`); its defense
(`FUN_0058cf80`: a facility's slot `+0x1c0` less `+0x1c4`, the class
`bombardment_defense` `+0x58`; a regiment's slot `+0x1d8` less `+0x1e4`,
TROOPSD `bombardment_defense` `+0x60`; energy G `0x604` = 9; raw G `0x605`
= 9); destroyed when the defense is below a draw of G `0x602`..G `0x603`
(1 to 10, `FUN_0055e970`, `FUN_0053e2c0`): `FUN_0058d020` -> slot
`+0xac(0x10 own / 0xf)`, a manufacturing or production facility setting
`+0x28`; an energy or raw unit lowers `+0x5c` or `+0x64` by one
(`FUN_00509cc0`, `FUN_00509dc0`) and sets `+0x28`. The targets are
counted again; none left stops.

**4, `FUN_0058e320`, the support.** With `+0x28` (civilian damage) the
attacker's support on the system moves by G `0x1e19` (-20,
`FUN_0050c9f0`: populated systems only, `+0x88` bit 1; scaled by
`DAT_006bb400` with `+0x88` bit 11, `FUN_00559be0`). With `+0x2c` (a
destroyed system) both sides' galaxy-wide support events
(`FUN_0055da90` -> `FUN_00518a70`); else with `+0x28` the sector's
(`FUN_0055d900` -> `FUN_00518ac0`), GNPRTB `DAT_006bb5f4..6d0` (not yet
read: `FUN_00518b20`, `FUN_00518be0`). The system's bombarded bit is
cleared (`FUN_0050a660(0)`), and every fleet of the side loses
`+0x58` bit `0x40` (`FUN_004fe460(0)`).

## GNPRTB

| Id | Global | Value | Use |
|---|---|---|---|
| `0x600` | `DAT_006bb768` | 40 | the general's leadership divisor (batteries) |
| `0x601` | `DAT_006bb774` | 40 | the admiral's leadership divisor (firepower) |
| `0x602` | `DAT_006bb75c` | 1 | the destruction draw's low end |
| `0x603` | `DAT_006bb764` | 10 | the destruction draw's high end |
| `0x604` | `DAT_006bb760` | 9 | an energy unit's defense |
| `0x605` | `DAT_006bb770` | 9 | a raw material unit's defense |
| `0x606` | `DAT_006bb76c` | 50 | a minor character's hit chance on destruction |
| `0x1e19` | `DAT_006bb47c` | -20 | the attacker's support after civilian damage |

## Ported (2026-10-16, save v55)

`crates/rebellion-core/src/bombardment.rs` replaces the transit misread:
`resolve` runs the four phases (the Death Star's destruction, the ion
cannons and batteries, the shots, the civilian damage), `apply` carries
out the hull damage (a ship at its hull is lost, a fleet with nothing
left disbands), the targets' destruction (the headquarters clears
`System::is_headquarters`; energy and raw materials lose a unit) and the
attacker's support (G `0x1e19`, `uprising::apply_support_change`).
`BombardEnv` holds a tick's draws and outcomes. The AI's plan objects
order it (`AIAction::Bombard`, `FUN_004b1070` -> `FUN_004b1330`); the
integrator and the app carry it out, and a Destroy System order's
destroyed system goes through the Death Star's cleanup. The automatic
bombardment after a battle is gone (simulation, app, `tactical_flow`).
The player orders it from the Fleet menu's Planetary Bombardment
sub-menu (stage 4f, `object-popup-menu.md` "Submenus"): a fleet or a
sector window's fleet icon, each order enabled by
`bombardment::order_enabled`, carried out at once. The older Bombard
panel's button still orders General Bombardment.

port:
- The draws come from a `SplitMix64` stream seeded by one roll of the
  tick (the app: its generator), not the game's one generator.
- The order's sub-orders are not ported; its validation is the enabled
  check plus: the fleets in orbit at the system, out of hyperspace, the
  system not the side's.
- hyp: the system's list holds the manufacturing facilities before the
  production ones; a ship's `+0x24 & 0x30` is 0; the port's unset packed
  word (0) reads as a shield nibble of 0; a Death Star's destruction
  leaves no battery to fire and nothing to bombard.
- Not ported: the sector's and the galaxy's support events
  (`FUN_00518b20`), the system's bombarded bit (`+0x88` bit `0x40`).

## Port notes

- The port's automatic bombardment after a space victory had no
  counterpart: the original bombards only on these orders (removed with
  the port above).
- The multipliers read the officers (2026-10-19, `character-command.md`):
  the system's general (`character_command::system_holder`) for the
  batteries, and each bombarding fleet's own admiral
  (`character_command::fleet_holder`) for its firepower. Their leadership
  is `Character::effective_skill`; without an officer the multiplier is 1.
- The port's ship keeps its current hull (`hull_current`), so `+0x60`
  damage is `hull - hull_current`; it has no shield damage (`+0x28`) or
  per-squadron damage, and squadrons are counts.

## The destroyed system's contents (read 2026-10-20)

- The system's slot `+0xac` is `FUN_005080d0`: it writes the reason
  (`FUN_004f8660` -> `FUN_004f71d0`), whose slot `+0x100` (`FUN_004fb200`
  -> `FUN_004f7560`) sets the destroyed bit and calls slot `+0x11c`,
  `FUN_00510950`: the base `FUN_004fb520` and event `0x319`.
- `FUN_004fb520` calls slot `+0xb4(1)` on each existing direct child
  (`FUN_004fcd00(this, 3)`):
  - ships, squadrons, regiments, facilities and the system class share
    `FUN_004f8710`: completed and not en route, `FUN_004f7ed0` destroys
    the child with its container's reason; otherwise, unless destroyed,
    it starts autorouting;
  - a special force (`FUN_00503e20`) is destroyed the same way;
  - a fleet's slot is `FUN_00524fb0`, `return 1`: the fleet stays, with
    its ships and the characters aboard, and one en route there keeps its
    trip;
  - a character (`FUN_004ef2e0`) on no mission goes adrift, as the reason
    (`0x10` or `0xf`) is not `0x14`.
- Event `0x319` (`Event319_Handle`, `FUN_0050d5a0`, from the fire slot
  `FUN_005650f0` for both sides) only refreshes each side's view of the
  system.
- Then `FUN_0058d990` walks the minor characters standing in the system
  (`FUN_0056f3d0`, `0x38..0x3b`, mode 3, existing): one not en route
  (`+0x50` bit 5) is hit with GNPRTB `0x606` (`FUN_0053e2f0`); a hit adds
  an injury of 1 (slot `+0x2f0`, reason 10), and a character still
  existing after it (`+0x50` bit 6, read in the disassembly at
  `0x58daeb`) is destroyed with reason 10. Every hit kills. The manual
  (p. 96, characters) says a character "may be injured ... if the system
  on which he is located is blown up by a Death Star" and may be killed.
- Ported 2026-10-20 (`death_star::cleanup_destroyed_system`,
  `bombardment::destruction_hits`): the fleets stay, the characters
  standing in the system go adrift, the hit minor characters die. Before
  this, the port removed the fleets, killed every character aboard and
  cancelled the trips there, none of it sourced. port: a captive is not
  set adrift; the characters are taken in the arena's order.

