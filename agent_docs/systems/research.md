---
title: "Research System"
description: "Three research trees per side, advanced only by research missions and the points hook"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-14
tags: [research, tech-tree, ship-design, simulation, missions]
---

# Research System

Research runs only as missions. The recovered mechanics are in
`ghidra/notes/mission-lifecycle.md`, "Research"; the rebellion2 projects,
ticks and Research panel were removed on 2026-10-14 (save v36).

## Types (`research.rs`)

| Type | Purpose |
|------|---------|
| `TechType` | Ship / Troop / Facility; `skill()` and production `area()` |
| `ResearchLevels` | One side's level per tree (side `+0x9c`, `+0xa0`, `+0xa4`) |
| `TreeProgress` | One tree's points (`+0x90`, `+0x94`, `+0x98`) and Done flag (`+0xac`, `+0xb0`, `+0xb4`) |
| `ResearchState` | Both sides' levels and progress |

## Missions (`missions.rs`)

`MissionKind::ShipDesignResearch`, `FacilityDesignResearch` and
`TroopTrainingResearch` use records `0x53000020`, `0x53000021` and
`0x53000022`.

- Validator `FUN_0056c510`, in `running_end_code`: at creation each
  character needs the tree's skill (`creation_member_ok`); while running the
  target needs a working yard of the tree's area (`research_site`, end `0xc`)
  and the side's tree must not be Done (end `0xe`).
- Phase 10 `FUN_0056cdb0`: each member rolls below its skill
  (`research_chance`); a hit adds GNPRTB 6171 + rand(0..=6172) points and
  raises the skill by GNPRTB 6158.
- `FUN_0056cf30` emits `MissionEffect::ResearchPoints`; the integrator and
  the app apply it with `ResearchState::add_points`, which runs the points
  hook and returns the levels reached.

## Points hook

`ResearchState::add_points` follows `FUN_005326f0` -> `FUN_00567ba0`: while
a next level exists (`next_level`, the least order above the current one
among the side's classes of the tree's family with a refined cost) and the
points cover its cost (`level_cost`, difficulty x systems / 100), take it
and spend the cost. With no next level the tree is Done.

## Tech Unlock Rule

A class is buildable when `class.research_order <= faction_level_for_tech_type`
(`FUN_0052e580`). Level 0 units are always available.

## AI

`mission_planning.rs` holds the planners `FUN_0047bcd0`, `FUN_0047c2b0` and
`FUN_0047c900` (one member with the tree's capability and skill 10 or more).
`AISystem::evaluate_research` follows the strategy `FUN_004e8290` with
persistent planners (`AIState.research_planners`, save v37): per tree, one
planner per character able to research it (`Planner::able`,
`FUN_0041bc10`) until the AI's Done level (`ai_research_goal`,
`FUN_0042dd00`). Surplus or Done planners are cancelled weakest first and
their missions run on (`FUN_004e5710`, `FUN_0047aff0`). Each planner holds
its designer, remembers its site and previous target
(`research_site_choice`, `FUN_0047be50`), and after each Mission Report
aborts its mission when the site is lost (`AIAction::AbortMission`, end
code 2). A research site is an own system in no uprising or blockade with
a yard of the area and a garrison (`ai_research_site`, `FUN_00433e40`). See `ghidra/notes/ai-mission-planning.md`,
"The strategic choice of mission".
