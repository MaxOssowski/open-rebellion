---
title: "The Force"
description: "The original's Force fields and the rules that move them"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-19
tags: [force, jedi, simulation]
---

# The Force

The port keeps the original's fields:
- `ForcePotential`, `ForceAware`;
- `ForceExperience` `+0x8e` (`force_base`) and `ForceTraining` `+0x90`;
- the Force `+0x8c` = `+0x8e` + `+0x90` (`Character::force_level`);
- the discoverer `+0xa4`.

| Rule | Module | Note |
|---|---|---|
| discovery by a trainer, timer `0x389` | `force_discovery.rs` | `ghidra/notes/force-awareness.md` |
| detection across sides | `mission_detection.rs` | `ghidra/notes/decoy-roll.md` |
| growth through Jedi Training, Dagobah, escapes and the encounter | `missions.rs`, `dagobah.rs`, `mission_detection.rs` | `ghidra/notes/force-awareness.md`, `dagobah.md`, `encounter.md` |
| the Status window's ranking | `rebellion-render/status_window.rs` | `ghidra/notes/status-window.md` |

The former `jedi.rs` had tiers, XP points and flat detection. It had no
source and was removed on 2026-10-19 (`ghidra/notes/jedi-rs-replacement.md`,
save v87).
