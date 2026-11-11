# HQ, Coruscant and four tactical states against A0 (2026-11-11)

This covers OBJ-13-C001, C002, C005, C006, C007, C008 and TAC-01-C004,
TAC-01-C013, TAC-03-C004, TAC-03-C017. It compares the A0 captures that
already existed. For the states with no A0 capture, it lists the captures
needed.

## A0 used

| Cell | Capture | SHA-256 |
| --- | --- | --- |
| TAC-01-C004 | `.artifacts/interface-parity/a0/TAC-01-C004/original.png` | `9fec42951416f001fb66c14d05d90a4bd5431e928dffe7d10e6e7a574a5e6739` |
| TAC-03-C004 | `.artifacts/interface-parity/a0/TAC-03-C004/original.png` | `605e464ff7aefb4cdb77892b73b6d24910293e12a923d938270f424fd1e13a1f` |
| TAC-01-C013 | `.artifacts/interface-parity/a0/TAC-01-C013/original.png` | `cd4eda7b3b4f45406eef7a8f9f2bbf95606fa39d8a04a85e23edd7a83bc77b21` |
| TAC-03-C017 | `.artifacts/interface-parity/a0/TAC-03-C017/original.png` | `65c5d82f4460df284cc965fafc5caf4523611a928d367ecb7ab4cfbf3960aa90` |
| OBJ-13-C008 | `.artifacts/a0-vm/coruscant-676/14-after-movie.png` | `c09a8f4832b492eb073ce68710912f1f6a7eb2c5be25078de3b565472b9e56ec` |

The four tactical captures come from the Windows 10 VM on 2026-10-24
(`.artifacts/interface-parity/a0/manifest.json` holds their input traces).
C004, C013 and TAC-03-C004 are the Alliance's Battle at Mon Calamari
(day 49). TAC-03-C017 is the Empire's Battle at Yavin (day 234). The
Coruscant capture is the VM's Empire game, day 679.

## TAC-01-C004: the running frame

A0 C004 is the running battle, 4 s after Pause was pressed. A0 TAC-03-C017
is also running, on the Empire's side.

`run.mjs --battle`, scenario `battle-entry`, now compares the port's
frame right after Resume (`resumed.png`) with both captures, at 640 x 480:

- The controls under the panel (x 476..639, y 262..479) match pixel for
  pixel. This takes in the running pause art 1060, the highlight
  buttons, the camera sphere and the zoom buttons. Only A0 C004's pointer
  (x 626..639, y 266..287) is skipped.
- No "Battle Paused" ink shows (red, x 17..139, y 30..49), in the port or
  in either capture.

The paused port frame differs from A0 in 366 of those pixels, so the check
fails on a paused frame. The panel's line "Attacking Victory Destroyer 1"
was compared in b126 (`2026-11-07-ship-contents-a0`). The battle view
itself holds a different battle and is not compared.

## TAC-03-C004: the destroyed ship

A0 shows a fireball where the Medium Transport died: 36 x 23 pixels at
x 165..200, y 292..314. It uses five colours: (255, 255, 115),
(255, 213, 0), (255, 170, 0), (255, 85, 0) and (201, 3, 0). The panel is
back to the emblem, as compared before (`2026-10-24-tactical-right-panel-a0`).

`run.mjs --battle`, scenario `production-detail-escort-presentation`,
holds the destroyed sequence (TACTICAL 3360..3375, `FUN_005d3e90`,
`FUN_005d41a0`) at frame 8, resource 3368. The new check finds every one
of A0's five colours in the port's fireball (43 x 23 pixels). The port's
frame adds a sixth, (255, 128, 0). Both index the battle palette (5530
plus the selector, `FUN_005d38d0`).

Not compared: A0's frame number is not known, so the fireball's shape and
size are not compared.

## TAC-01-C013: the return to strategy

A0 shows the Battle Summary on the strategy screen after the last Alliance
ship died: "The Alliance fleet is defeated." over "Imperial forces have
broken the blockade of Mon Calamari. The Alliance fleet has been
completely destroyed."

The new unit test
`an_alliance_fleet_destroyed_where_the_empire_broke_its_blockade_reads_as_a0`
(`tactical_view.rs`) builds those two lines word for word
(`FUN_0049b4d0`: 0x7003, then 0x7016 with the enemy, then 0x700b). It
passes on the current code; no change was needed.

Compared earlier: the Summary's frame (900 of 900 pixels,
`2026-11-09-battle-summary-frame-over-scene`), its headline's position
(`2026-11-09-summary-headline-and-message-arrows`) and its rail
(`2026-11-09-tactical-a0-comparisons`).

Still open: no browser case found here ends a battle by the death of the
player's last ship and then returns to the live strategy screen. The
fixture battles open the Summary over the shell bitmap.
`fleet-window.mjs` `squadron-battle` runs from strategy into a battle; it
was not changed here (another agent works in that file).

## TAC-03-C017: the target lost

A0: Task Force 1 had been "Attacking Swift". The Alliance withdrew and
Swift jumped out. The line went back to "Attack Capital Ships" over
"Tactics: Stand Off", with no target.

The new scenario `production-attack-target-lost-presentation` (tactical
fixture 38, codes 65830 and 66086) takes the target-lifecycle battle and
starts every hostile capital's jump. `FUN_005cdf50` drops a ship whose
jump has started from the target lists. The probe requires:

- every hostile capital retreating;
- the selected ship and the first player capital on order 5 (Attack
  Capital Ships) with no target;
- the panel line "Attack Capital Ships" (`FUN_005e4b40`);
- for the Empire at 640 x 480, the ink of both lines within the accepted
  text deviation of A0's (the port draws smoothed Liberation Sans, A0
  GDI's Arial):

| Line | A0 ink box | Port ink box |
| --- | --- | --- |
| Attack Capital Ships | 497, 212, 580, 220 | 497, 212, 580, 220 |
| Tactics: Stand Off | 497, 223, 572, 229 | 497, 223, 570, 229 |

The catalog keeps TAC-03-C017 on the lifecycle scenario, as a cell has one
mapping scenario. The new scenario supports it.

## OBJ-13-C008: Coruscant controlled

Compared earlier: `story-movie.mjs` checks MDATA.103's caption "The
Alliance has seized the Imperial capital system of Coruscant." against A0
`empire-clean/61` (`2026-11-07-empire-defeat-a0`).

A0 `coruscant-676/14` then shows Coruscant's Defenses window, the
Empire's player, with side 1's focused title strip 10299 (`FUN_004a9800`
by the system's side). The window is at (200, 91).

`sector-quadrants.mjs` `defenses-other-side` opens, for the Empire, the
Defenses window of a system the Alliance holds. That is the same window
state. It now compares the window's title row with A0's (local
x 2..232, y 2..17):

- outside the name's label (x 19..198), 813 pixels, 0 differ;
- the name's ink starts within a pixel of A0's (left 20, top 5, bottom
  14 in both). The names differ ("Coruscant" in A0).

The three buttons' bottom-left pixels are skipped. That pixel is each
button's green key. A0 shows black there, but other A0 captures show
green (`fleet-aboard/00..02`, one window of `status/22`) or the red strip
(`menus2/11`). The port shows green for the sector and minimize buttons
and red for the close box. What shows under a button's key is not
settled.

Not compared: the fixture's system is not Coruscant. No browser case
opens Coruscant itself after it changes hands.

## HQ cells: no new A0

No A0 capture shows the Empire's view of the Alliance headquarters. A
search of all 2723 PNGs under `.artifacts/a0-vm` for mark 904 (every
opaque pixel exact) found 75 captures. All show the Alliance cockpit but
one: the Empire's Objectives list
(`agent-menu/empire-objectives-day137`, "Destroy Rebel HQ"), which is no
map view.

The browser cases for C001, C002, C005 and C006 already exist
(`sector-quadrants.mjs` `icons`, `headquarters-menu`,
`headquarters-move`). They are unchanged.

## A0 captures still needed

The lead's Alliance game (VM, near day 212) can give C005 and C006:

1. **C005, Move.** Open the sector window that holds the HQ. Right-click
   904, choose Move, then left-click (not drag) a planet of another
   Alliance system in the same window. Capture right after the click,
   with the window still open.
2. **C005, Confirmed Move.** Right-click 904, choose Confirmed Move, click
   another Alliance system. Capture the confirmation window
   (`FUN_00487cc0`) and its text. Then confirm it.
3. **C006, en route.** Right after a move, with the window still open:
   capture where 904 stands (origin or destination). Right-click 904 and
   capture the menu (both moves grey?). Choose Status and capture the
   Defense Facility Status ("ETA Destination: Day N", "Status:
   Enroute").
4. **C006, reopened.** Close and reopen the sector window during the trip.
   Capture it.
5. **C006, arrival.** Let the days run to the ETA day. Capture the
   message rail and any message, then the HQ's Status ("Active", no ETA
   row).

An Empire game is needed for the rest:

6. **C001, hidden HQ.** Early in a new Empire game, capture each sector
   window that holds an Alliance system. None should show 904.
7. **C002, discovered HQ.** Send Espionage missions (or a fleet) to
   Alliance systems until the HQ is found. Capture the discovery message
   and the same sector window with 904, if it shows. Right-click 904 and
   capture the menu.
8. **C007, HQ destroyed.** Destroy or take the Alliance HQ's system
   (Death Star, or assault). Capture the MDATA.102 caption ("The Empire
   has destroyed the Alliance Headquarters." in the port), any message,
   and that sector window afterwards (904 gone).
9. **C008, Coruscant itself (optional).** In an Alliance game, after
   taking Coruscant, capture Coruscant's Defenses window and sector
   window for the Alliance's player.

## Other open differences

- **C006, stale mark.** A0 `headquarters-move/07` vs `/09`: an open
  sector window kept 904 over the origin until it was reopened. The port
  redraws it at once. Which notice refreshes the overlay is not traced,
  so the port is unchanged.

## Gate runs (Linux, Arch Chromium 155.0.8059.84)

All on the interface-test WASM built from this tree.

| Gate | Run | Result |
| --- | --- | --- |
| `sector-quadrants.mjs`, all cases, both sides | `sector-quadrants-2026-11-11T16-47-05-922Z-1422686` | pass |
| `run.mjs --battle --scenario=battle-entry` | `2026-11-11T16-51-20-565Z-1465902` | pass, both sides |
| `run.mjs --battle --scenario=production-detail-escort-presentation` | `2026-11-11T16-51-41-850Z-1467492` | pass, both sides |
| `run.mjs --battle --scenario=production-attack-targeting-presentation` | `2026-11-11T16-51-53-981Z-1468455` | pass, both sides |
| `run.mjs --battle --scenario=production-attack-target-lifecycle-presentation` | `2026-11-11T16-52-02-975Z-1469500` | pass, both sides |
| `run.mjs --battle --scenario=production-attack-target-lost-presentation` | `2026-11-11T16-52-11-759Z-1470108` | pass, both sides |

The single-scenario runs cover the 640 x 480 viewport only. The whole
tactical matrix was not run; the batch should run it (the catalog now has
38 scenarios).

Unit tests: `rebellion-render` 1462 passed; `rebellion-app`
`tactical_codes_are_versioned_and_do_not_accept_gid_codes` passes with
code 0x10126. Provenance scan: 0 uncited items.

Mutants (`--in-diff`, in place):

- `tactical_test_fixture.rs`: 10 mutants, 4 caught, 1 unviable, 5 missed.
  The missed ones are in `apply`, `emit_ready` and `emit_failed`, which
  run only in the WASM build. The browser scenarios above cover them: a
  skipped `apply` or `emit_ready` leaves the gate without a ready
  fixture, and the `!` / `||` mutants on the lifecycle call either drop
  the lifecycle set-up (its probe then fails) or run it twice (no
  visible change; the lost fixture runs it first anyway).
- `tactical_view.rs`: 5 mutants, all in
  `configure_attack_target_lost_fixture`, which is compiled only with
  `interface-test-fixtures`. No unit test runs it; the lost-target
  scenario's probe checks its outcome (every hostile capital retreating,
  order 5 with no target, the line).
