# Fighter squadrons on a planet (F-021, F-030, F-036)

Date: 2026-10-15.

The port kept every fighter squadron inside a fleet: a finished squadron
joined the side's fleet in its system, or started a fleet of squadrons
alone. In the original a built product joins the container its Destination
names (`FUN_0052a140`, `FUN_0052a380`, slot `+0xa8`), the system, and stands
on the planet until it is moved onto a carrier. The readers are read in
`ghidra/notes/planet-squadrons.md`. This is stage 1 of the work module 7
(the AI's squadron garrisons) needs.

## Port (save v46)

- `System::fighters`: the squadrons on the planet, per class, either side.
- A completed squadron joins its system's planet
  (`apply_build_completion_inner`).
- `crates/rebellion-core/src/squadrons.rs`: counts, a fleet's room
  (`FUN_00500b40`: living ships' `fighter_capacity` less the squadrons
  aboard), `check_move`, `move_squadrons` and `move_enabled`.
- Readers of the planet's squadrons:
  - presence (`FUN_00509710` → `FUN_00503a50`): the holder's squadrons turn
    an enemy fleet's blockade into a battle; an enemy squadron alone never
    blockades (`FUN_0050b8e0` takes the bit from fleets);
  - the withdraw percent (`FUN_0050b310`) counts them;
  - upkeep (`FUN_0052fff0`);
  - the AI's system record (`FUN_00433e40`): `v[23..28]`, `v[32]`, `+0x28`
    bit `0x80000`, and so `v[38]`, `v[53]` (squadrons against the goal; the
    note had them as regiments);
  - the mission defender walk (`FUN_00587640`), before the regiments;
  - the Defenses window's Squadrons page (`FUN_004a90d0`) and quadrant icon
    (`FUN_0045ce80`);
  - the economy's fighter posture; a Death Star's planet takes them with it.
- Move (`0x201`) of one squadron: a squadron's class lists Move, Confirmed
  Move and Scrap (`FUN_00502fe0` → `FUN_00557ce0`). From the Defenses
  window's Squadrons page or the Fleet window's Fighters tab, by the pop-up
  menu or a drag, it boards a fleet in orbit with room, lands on its planet,
  or passes to another fleet in the system. Status shows a planet squadron
  attached to its system.
- `AIAction::MoveSquadrons`, validated by `squadrons::check_move`. The
  production heuristic standing in for module `0xb` boards the side's planet
  squadrons onto its fleets in orbit up to their room, and counts planet
  squadrons toward its fighter ratio.

port:
- The room is the whole fleet's, where the original fills one ship at a
  time.
- A squadron's move to another system is refused ("squadrons cannot yet
  travel between systems").
- A planet's squadrons take no part in a battle. With the holder's
  squadrons present, an enemy fleet meets neither a blockade nor a battle.
  This is stage C, not done.
- A mission names squadrons aboard fleets only; Confirmed Move and Scrap
  stay disabled.
- Seeding still puts a seed group's squadrons into a fleet; a fleet of
  squadrons alone disbands when its last squadron lands.
- The support drift (`FUN_0050b230`) takes the planet's squadrons alone, its
  fleets' capital ships and their Death Stars; the port still counts fleets
  and all squadrons (F-037).

hyp: a squadron may not land on another side's planet (the regiment's rule,
`FUN_0053d430` `1`/`0x28`); the Galaxy display's squadron count includes
the fleets' and the planet's squadrons.

## Manual cross-check

| Claim | Trace | Port |
|---|---|---|
| "If you build a fighter squadron, the unit is stationed on the planet surface until you assign it to a carrier capital ship." (manual p. 114) | Confirmed: `FUN_0052a140`, `FUN_0052a380` | Present |
| "Fighter squadrons and troops can only be part of a fleet that has a capital ship with space for them." (p. 111) | Confirmed: `FUN_00500b40` | Present (room per fleet, port) |
| The Fleet window's Fighters tab shows the squadrons and the room (p. 112) | Confirmed: `fleet-window.md` | Present |
| The System Defenses icon shows when a system has troops or fighter squadrons (p. 26) | Confirmed: `FUN_0045ce80` | Present |
| Squadrons not in a fleet "can engage an enemy fleet when it enters your system" (p. 129); tactical mode starts at a system "where fleets or fighter squadrons are stationed" (p. 141) | Unresolved: `FUN_005442f0` → `FUN_005435f0` not read | Missing (stage C) |
| Fighters on a planet help detect enemy agents (p. 129) | Confirmed for the walk: `FUN_00587640` | Partial: the port's run flags never set the system and fighter flags together |
| The Galaxy display's star size shows how many squadrons are on the system (p. 72) | Unresolved | Partial (hyp count) |
| Commanders command a system's fighter squadrons (p. 129) | Not traced | Missing (Command is not ported) |

The manual was read on those pages before and after the change.

## Campaign (`playtest-squadrons.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: the AI
builds no fighter in 600 days, so it issues no squadron order. Its orders
by side and type are identical to module 6's run.

## Browser

### The squadron move (`fleet-window-run.json`)

`tools/interface-parity/fleet-window.mjs` with a new fixture scenario,
`SquadronLoad` (64, code 65): the `FleetLoad` world with a fighter carrier
and two of the player's squadrons on the primary planet, the Defenses window
on its Squadrons page. Two new cases, on both sides:

- `squadron-load`: Move from the squadron's pop-up menu released on the
  fleet; one squadron boards, one stays, the Fighters tab lists it
  (`3-*-squadron-page.png`, `4-*-squadron-menu.png`,
  `5-*-fighters-tab.png`);
- `squadron-unload`: the squadron dragged out of the Fighters tab onto the
  Defenses window lands on its planet (`6-*-squadron-landed.png`).

The whole family ran, 42 cases, all passing. The first, shorter run's
`alliance/squadron-load` failed on its first step (the double click did not
open the Fleet window, `failure.png` showing both squadrons listed); the
case passed on each of the three runs after it.

The run used the system Chromium 153.0.8010.52 (Arch Linux): the harness
pins Chrome for Testing 151 for macOS, which is not on this machine, so an
untracked copy with only the version check removed drove it. Test WASM
SHA-256 `c2083ad6675c90cbb5973e16e8ff2ed7b9925be745e5254298ed2f494e4a1f58`.

### Release build

`agent-browser` (headless Chromium, 640 by 480), a new Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3747 | 0.5 ms | 0.6 ms | 2.1 ms |
| Fast | 60 s | 3745 | 0.4 ms | 0.6 ms | 3.2 ms |

No frame took over 50 ms. The AI's longest day at Fast took 2 ms
(`console.txt`). The run reached the Bothawui battle window
(`2-fast.png`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2014 passed, 0 failed.
  - `squadrons.rs`: 11 tests; each behaviour test elsewhere cites its
    function, and each fails without its change (a run that disabled each
    change in turn).
  - Changed expectations, with evidence: the integrator's
    `completed_squadrons_stack_in_their_own_side_fleet` became
    `completed_squadrons_stack_on_their_planet_beside_any_fleet`
    (`FUN_0052a140`); the fleet-numbering test's second craft is a capital
    ship, since a squadron no longer starts a fleet; the object menu test
    for squadrons now expects Move, Confirmed Move and Scrap
    (`FUN_00557ce0`); the Defenses window tests now list the stocked
    squadrons (the Squadrons tab art is 10556, not the empty 10558, and the
    page counts are `[3, 1, 2, 2, 2]`); the fixture's object-menu list adds
    `SquadronLoad`.
- Seed-42 golden regenerated for save v46 (`System::fighters`):
  - initial `v1:6d35df6768400d72` -> `v1:d33088556c4445a3`;
  - final `v1:8322498b801abe81` -> `v1:9ff9c886bcd2e454`.

  The replay equivalence check (`replay-equivalence.json`) passes with 0
  browser errors. Release WASM SHA-256
  `b91aff1614c75882de50222fa717d78b07bccfeaba129d1e961a8a74df383e1e`.
- `cargo mutants --in-diff` over the changed lines of 16 files: 200
  mutants, 170 caught, 14 missed, 16 unviable. Tests were added for 11 of
  the gaps (module `0xa`'s and `0xb`'s split, the loading's bookkeeping, a
  marked system's record, an emptied entry, the refusal text,
  `move_enabled`); a retest of those functions caught all but three. The
  three left are equivalent: in `build_kept` the second operand
  `class.values[1] >= 0` is always true for a fighter record, so mutating
  the unmarked test (`& -> |`, `& -> ^`, `== -> !=`) changes nothing.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed lines. rustfmt: `squadrons.rs` is
  clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
