---
title: "F-007C Native Strategic Interface Acceptance"
description: "Native strategic-interface evidence, Fleet Finder remediation, and remaining transport-limited checks"
category: qa
created: 2026-10-05
updated: 2026-10-05
tags: [qa, native, movement, fleets, bitmap, astra]
---

# F-007C Native Strategic Interface Acceptance

This evidence records the first broad native strategic-interface pass and the
focused Fleet Finder remediation. The release executable ran through a
temporary macOS application wrapper so the background-control harness could
address the raw Macroquad window. The wrapper did not replace or modify the
executable. Every run began muted and closed the game when finished.

## Initial native run

| Field | Value |
|---|---|
| Source commit | `d2036272` |
| Native executable SHA-256 | `bb657b3b01231806c9a9bb1041619a5fb6ff33051bfdf033efb4fdb16cf034de` |
| Reviewer | `codex-orchestrator`, GPT-6 Astra, medium effort |
| Window | 1280×828 logical; 1187×768 normalized captures |
| Local evidence | `.artifacts/native-checks/2026-10-05-d2036272-astra-medium/` |
| Outcome | 21 pass, 3 reported fail, 21 pending |

The main agent independently rechecked the executable hash and inspected the
menu, both faction cockpits, Fleet and System Defenses windows, the Finder, and
the cited overlap and empty-space captures.

Verified native behavior includes:

- authentic bitmap menu and distinct fresh Empire and Alliance cockpits;
- the sector fleet icon opening the Fleet window, including named ship art;
- the Fleet window Troops pane;
- populated sector quadrant controls;
- the System Defenses window, a Guerrillas regiment image, the Planetary
  Batteries tab, and Fleet/Defenses minimize and restore;
- Fleet Finder opening from the cockpit, faction tabs, ordinary keyboard
  input, query plus Enter, row plus Display, double-click, Ship Finder
  selection, Close, and Escape.

The enabled speaker artwork was a run-condition mismatch, not an audible-output
defect. The reviewer switched it to the crossed-out state and left it muted.

## Finding resolution

### F-007C-N01: covered-planet brackets were not selection evidence

The initial run reported that an empty Finder double-click selected Boordii.
That conclusion was not supported by the retained frames. Sector-window red
brackets are painted only while the native pointer hovers or clicks a planet.
The background-control cursor overlay does not show Macroquad's synthetic local
pointer position, so the brackets visible after Escape identified the pointer's
covered coordinates, not the authoritative `selected_system` value.

The deterministic browser fixture now opens an authentic sector window beneath
the Finder, seeds a different selected system, and reports every covered
planet's exact screen center. Both faction cases double-click a real planet
through empty Finder list space, close the Finder, and prove that the selected
system and open sector set are unchanged. The broad button and modeless-window
input locks explored during diagnosis were removed because the existing Finder
rectangle and egui layer ownership already enforce the contract.

### F-007C-N02: Finder opened below a Fleet window

This was a real stacking defect. Reopening Fleet Finder while a Fleet window
remained open placed the Finder below the modeless Foreground window. The
Finder now uses egui's top-level Tooltip order. A focused regression raises a
modeless Foreground window after the Finder in the same frame; changing the
Finder back to Foreground makes that regression fail. The browser matrix also
opens a Fleet window, reopens the Finder, and operates a covered right-side
Finder control for both factions.

## Final verification

| Gate | Result |
|---|---|
| Fleet Finder unit tests | 29 passed, 0 failed |
| Interface fixture tests | 34 passed, 0 failed |
| Workspace tests | 1,328 passed, 0 failed, 39 ignored |
| Stacking hand mutant | Foreground order caught; Tooltip order passes |
| Browser matrix | 18/18 across both factions |
| Browser artifact | `.artifacts/interface-parity/fleet-finder-2026-10-05T23-24-38-163Z-83071/` |
| Fixture WASM SHA-256 | `c3f50929ced3c628b1ee733c30c395f1407056a8c4f6a23ad8f5fea025a54296` |
| Release WASM SHA-256 | `0b8ca1277f6f6c3fc6fb5692c10e7f11121052436ff37e6c8be00e10304beade` |
| Browser errors | 0 console or page errors |
| Audio | muted |

Focused native runs independently confirmed the corrected Finder above an open
Fleet window. The last native run used executable SHA-256
`5cb2d2ae6afa0c648115247f4aa9278c6bb3d03a913dd7b94b7cf0ef3fbbecb6`
and retained its captures under
`.artifacts/native-checks/2026-10-05-fleet-finder-final-pass-astra-medium/`.
Its reported selection failure is retained, but reclassified as an invalid
visual assertion for the hover-only brackets described above.

## Review correction

A review of `aeafc17f` found that the `fleet-overlap` browser case passed
with the Finder put back on Foreground, on both factions. The case clicked
the Ship Finder control, which lies outside the reopened Fleet window (the
Alliance window spans x 121 to 356, the control sits at x 502), so nothing
covered it. egui keeps the older of two raised Foreground layers below, and
the Finder is the older, so wherever the two overlap the mutant puts the
Fleet window on top. The fixture now reports each Fleet window's screen rect,
and the case clicks a Finder control inside one (the Alliance tab on both
factions) and fails without one. The Foreground mutant now fails both cases
("the covered alliance_tab control responds"), and Tooltip passes. The full
matrix reran 18 of 18 with 72 of 72 HTTP 200 responses, no console or page
errors, muted: `.artifacts/interface-parity/fleet-finder-2026-10-05T23-24-38-163Z-83071/`,
fixture WASM `c3f50929ced3c628b1ee733c30c395f1407056a8c4f6a23ad8f5fea025a54296`.
The `sector-occlusion` case guards the covered-input contract but cannot fail
on the Foreground order either, since sector windows sit on egui's Middle
order below both.

## Pending native checks

These assertions remain pending rather than failed:

- right-click context-menu journeys, including Move, Confirmed Move, Create
  Fleet, and the F-019 Mission entry;
- in-canvas ship, fleet, and regiment drags;
- loaded-regiment embarkation, unloading, refusal, transit, and arrival;
- fleet joining and splitting through drag or Move;
- the Missions quadrant without an available populated native setup;
- F3 because the control transport rejects that function-key token;
- a fully typed multi-character Finder query and hierarchical Ship Finder
  expansion;
- speed-menu colors and selection.

The background-control runtime exposes secondary clicks as unsupported and its
drag route moves the macOS window, not game objects. These limitations do not
count as application failures. Browser fixture results remain useful for those
paths, but they do not close the complete F-007C native gate.

## Status

The scoped Fleet Finder stacking defect is corrected and the covered-window
input contract passes deterministic two-faction acceptance. F-007C remains open
for the transport-limited native journeys listed above.
