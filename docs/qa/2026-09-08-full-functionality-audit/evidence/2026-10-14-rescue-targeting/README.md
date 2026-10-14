# Rescue targeting in the browser (F-019 phase 7g)

Date: 2026-10-14. Runner: Claude Code with `agent-browser` 0.38.1 driving
headless Chromium 153.0.8010.52 (Arch Linux, `/usr/bin/chromium`), at a
640 by 480 viewport. Driver: `rescue-flow.py`.

## Scenario

Fixture scenario `RescueTargeting` (index 57, code 58; `| 1 << 8` for the
Alliance, `| 2 << 8` for the Empire) in the test-only interface WASM
(`scripts/build-interface-test-wasm.sh`). The player's agent stands free at
the primary system; the world's second character is a prisoner of the
player's side, held by the other side alone at the secondary system, which
the player sees into. Both systems' Defenses windows are open.

## Flow, for each side

1. Right-click the agent in the right-hand Defenses window: the object
   pop-up menu lists Mission enabled (`2-menu.png`).
2. Choose Mission, move over the prisoner in the left-hand window's first
   cell (`3-targeting.png`), and click.
3. The mission dialog opens with exactly one kind, Rescue, and names the
   target "Leia Organa" (`4-dialog.png`; the report's `kinds` and
   `target_character`).
4. Begin Mission: the dialog closes and a Rescue mission targeting the
   prisoner is active (`after_begin.missions`); the agent leaves its
   personnel page, which lists no member on a mission (`5-after-begin.png`).

## Results

| Side | Kinds offered | Target | Active after Begin | Requests | Console errors |
|---|---|---|---|---|---|
| Alliance | Rescue | Leia Organa | Rescue → Leia Organa | 4 of 4 HTTP 200 | 0 |
| Empire | Rescue | Leia Organa | Rescue → Leia Organa | 4 of 4 HTTP 200 | 0 |

Each side's `result.json` keeps the reports, console and request list. The
screenshots were inspected. The empty target frame in `4-dialog.png` is the
known port gap: the object's portrait (`FUN_0042c3b0(.., 1, 1)`) is not
drawn. The targeting cursor does not show in `3-targeting.png`; this run
does not check it (the mission-dialog gate does).

The production `web/open-rebellion.wasm` built alongside passes
`scripts/check-replay-equivalence.py --skip-build --json` (seed-42, all
cases, 0 browser errors).

Hashes are in `SHA256SUMS`.
