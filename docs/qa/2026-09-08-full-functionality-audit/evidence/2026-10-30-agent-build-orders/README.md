# The agent's Build orders (P34, CMD-07)

Date: 2026-10-30. Tracker item "Droidy-doradcy": the Agent menu's Build
Ships, Build Troops and Build Facilities (`0x110..0x112`), until now
disabled because module `0x17` was not ported. The full trace is in
`ghidra/notes/agent-build-orders.md`.

## The original

- **Not an automatic build.** Each item is a one-shot request, as TEXTSTRA
  advice 26 ("Giving Your Agent Build Orders") tells the player.
- **The target cursor.** The item stores the kind and starts the target
  cursor (`FUN_00439d10` → `FUN_0049e360`, `FUN_0041d5e0`). Alt+B, Alt+T
  and Alt+F do the same (`0xbc1..0xbc3`, `FUN_00422ce0`).
- **The destination.** The player clicks a system, a fleet or a capital
  ship (`FUN_0049e380`). Troops sent to a system the side does not hold,
  with no fleet of its own there, are refused at once (`{1, 0x24}`).
- **The producer.** The agent first takes the nearest sector with an own
  system that has an idle producer of the kind. In that sector it takes the
  system nearest the target (`FUN_0049e7a0`). "Idle" means: held, explored,
  populated, no uprising or blockade, a yard of the area, and the manager
  building nothing (`FUN_004ebe40`). With none, the advisor reacts
  (`{0xa0, 3}`).
- **The dialog.** The agent points the producer's Destination (`0x214`) at
  the target and opens Build Selection on it (`FUN_0049e6b0` →
  `FUN_0041d640`). Confirming the agent's own order sets bit 1, and the
  droid acknowledges it (`FUN_0043a2a0`).

## A0 (Windows 10 VM, Empire, days 186 to 208)

Captures are in `.artifacts/a0-vm/build-orders/`, not committed; the hashes
are in `a0-captures.sha256`.

- **Build Ships on Ghorman** (`05`, `08`, `09`, `11`): the target cursor
  showed, the click ended it, and no window opened. The Build items were
  enabled again afterwards. No idle shipyard was found.
- **Build Troops on Coruscant** (`15-t01` at Medium, `20-t09` at Very Slow):
  Build Selection opened on an Imperial Army Regiment with deployment 0
  days. The nearest idle facility is Coruscant's own. At Very Slow it
  opened in under a second without the day changing. So the agent does not
  wait for a day, and the port runs the request at once.

## The port

- **`agent_build.rs` (core):**
  - `BuildKind` maps the menu item to the producer's area.
  - `accept_target` is `FUN_0049e380`.
  - `find_idle_producer` is `FUN_0049e7a0` on the record bits.
  - `run` gives the producer and its destination, or the refusal.
- **`agent_menu.rs`:** the Build items are enabled with their commands
  (`FUN_00487c30`; the module is always idle in the port).
- **`targeting.rs`:** `TargetOrder::AgentBuild`.
- **`advisor_schedule.rs`:**
  - `{0xa0, 3}` and `{0xa0, 2}` have their own reactions (`FUN_004c3860`,
    `FUN_004c16a0`);
  - `{1, 0x24}` has its own reaction for a system (`FUN_004c3bb0`);
  - bit 1 is renamed `BUILD_CONFIRMED`.
- **`build_selection.rs`:** `open_for_agent`, and the confirm action says
  whether the order was the agent's.
- **The app:**
  - Alt+B/T/F and the menu start the cursor;
  - the release sets the producer's Destination and opens Build Selection,
    or plays the refusal;
  - confirming the agent's order adds `BUILD_CONFIRMED`.
- **port / hyp:**
  - The phases run at once (A0 above).
  - Equally near sectors or systems: the first in record order (hyp).
  - `FUN_004feea0` is read as "the side has a fleet there" (hyp).
  - A capital ship as the target stands for its fleet.
  - `{1, 0x27}` (another object family) cannot happen: the port's release
    gives only systems and fleets.
- **No save change.** The request ends in the frame it starts, and the
  original does not save the pending dialog either (`agent+0x180`).

## Browser (Arch Chromium, interface WASM `13c042e3…`)

The new gate `tools/interface-parity/agent-build.mjs` (fixture 55) passed
for both sides (`agent-build-result.json`). It:
1. presses Alt+T and clicks the system window;
2. checks that Build Selection opens on Akrit'tar's own training facility;
3. confirms, and checks that the regiment is queued and the agent logs the
   confirmation;
4. presses Alt+T again and checks that the agent refuses (`NoIdleProducer`).

The window is the one the object menu's Build opens. It stands where A0
shows it, (255, 87).

| Side | targeting | agent-build-selection | refused |
|---|---|---|---|
| Alliance | `874d0846…` | `5a7be69d…` | `091b2ad7…` |
| Empire | `c7d4bb88…` | `5e1646ad…` | `b297c028…` |

**Not compared in A0:** the droid's refusal and acknowledgement clips (VM
audio was not recorded). They are covered by the unit tests.

## Tests

- `cargo test --workspace`: all pass. `provenance-scan check`: ok.
- New tests:
  - `agent_build.rs`: five, for the menu kinds, the nearest-sector rule,
    passing over busy, held-away and troubled yards, the area's own yard,
    and the troops refusal;
  - `the_agents_build_refusals_have_their_own_reactions`.
- Changed expectation: `agent_menu`'s menu test now expects the Build items
  enabled with their commands (`FUN_00487900`, A0).
