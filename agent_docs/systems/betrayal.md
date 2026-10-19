---
title: "Betrayal"
description: "Betrayal is the mission phase only; characters never change sides"
category: "agent-docs"
created: 2026-03-15
updated: 2026-10-19
tags: [betrayal, loyalty, missions]
---

# Betrayal

The original's only betrayal is the mission phase `FUN_00589f10`, ported as
`mission_detection::betray`:
- a member betrays the mission when `draw(0..99) < 100 - loyalty`;
- the mission is then detected;
- the other members may learn the traitor (`0x361`).

Characters never change sides. The port's former `betrayal.rs` had a 50-day
defection on UPRIS1TB. It had no source and was removed on 2026-10-19
(save v88).

The base loyalty and its shifts on galaxy events (`FUN_00561910`) are in
`ghidra/notes/betrayal-and-loyalty.md`.
