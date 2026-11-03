# The Encyclopedia's seven categories against A0 (2026-11-03)

Surface ledger OBJ-01 C002..C007 (systems, ships, facilities, missions, troops, personnel).

## A0

The A0 run used the Alliance's DS_flip_allian save, day 185. F7 opens the index. The captures are in `.artifacts/a0-vm/encyclopedia/`, with hashes in `a0-sha256.txt`.

- **`category-0` .. `category-6`.** Each category icon in turn. Each shows its label ("System Database", "Ship Database", "Facilities Database", "Missions Database", "Troop Database", "Personnel Database"). With the Topic box empty, it chooses its first row in white. All Databases, as opened, chooses nothing.
- **`ex1` .. `ex3`.** A click on Adar Tallon in All Databases chooses him and writes his name in the Topic box.
  - Personnel Database keeps him chosen.
  - System Database chooses its first row, Adega. The Topic box still reads "Adar Tallon".
- **Missions Database** lists Abduction, Assassination, Death Star Sabotage, Diplomacy, Espionage, Facility Design Research, Incite Uprising and Jedi Training. It lists none of the ten internal missions.

## What the port did, and the fixes

**1. Systems missing from the index.** A world system's `DatId` is the bare SYSTEMSD record id, such as 120. The catalog's ids carry the family byte (0x90..0x97). So `set_galaxy_systems` matched nothing, and System Database was empty in the browser. Adega was also missing from All Databases.

- Fix: the filter now compares the record ids (the low 24 bits).
- Test: `the_index_lists_only_the_games_systems_once_they_are_set` has a bare id. It fails without the fix.

**2. A new category chose nothing.** The port left a new category with no row chosen.

- Fix: `choose_category_row` keeps the chosen object when the category holds it, else chooses the first row. All Databases with nothing chosen stays so. The Topic box keeps its text (`original_topic_text`), as A0's does. A row click, Previous or Next, or a new opening clears that text.
- Test: `a_category_keeps_its_object_or_chooses_its_first_row`. The 2026-10-29 expectation "a category without the object selects none" had no capture of a category change. These captures show the first row chosen, so the reconcile test's comment now points to the new rule.

**3. Hidden missions listed.** The port listed the ten internal missions: Adrift, Autorouting, Bounty, Dagobah, Move, Palace, Pickup, Return, Sabbatical and Vacation.

- Fix: `FUN_00422620` lists a mission class only when its MISSNSD record `+0x5c` (hidden) is 0 (`encyclopedia-window.md`). The catalog now records those classes in `hidden_missions`, and the window builds from `listed_entries()`. The source audit still counts all 356 entries.
- Test: `the_window_lists_every_entry_but_the_hidden_missions`.
- Mutant: `record.hidden != 0` → `==` in `load_encyclopedia_catalog` survives the unit tests, which carry no DAT files. The browser gate catches it, because Missions Database would then list only the hidden classes.

## The browser gate

`personnel-states.mjs` now opens the Encyclopedia with F7 and clicks the seven category icons, at the window's (36 + 52 i, 78). For each it compares with A0:

- the label;
- the first eight rows;
- the chosen row.

The fixture reports the index (`emit_encyclopedia`).

- **Before the fixes**, the first run failed on both sides. All Databases had "Adrift" for "Adega". System Database was empty. Missions listed Adrift, Autorouting, Bounty and Dagobah. Every category chose nothing.
- **After the fixes**, run `personnel-states-2026-11-03T00-40-58-395Z` passed on both sides: all seven categories equal A0.

## Topic: double click, Previous and Next, and the object menu (OBJ-01 C010, C011, C012)

**A0** (day 205):

- **`topic-step-2`.** A double click on Abduction shows its topic.
- **`topic-step-3..6`.** Next shows Ackbar. Previous shows Abduction, then A-wing. On A-wing, Previous is grey and does nothing.
- **`ctx2-luke`, `ctx3-luke`.** Luke's menu has Encyclopedia, which shows Luke Skywalker's topic directly.

**What the port did, and the fixes.**

1. **The double click did nothing.** `FUN_0045da70` (command 0x65, list notification 0x309) presses the Topic control 0x67. egui's own double-click flag did not fire in the browser, so the window keeps the last row click and its time (`original_last_click`), as the Finders do. A second click on the same row within the double-click time shows the topic.
   - Test: `a_double_click_on_a_row_shows_its_topic`. It fails without the change, and also checks that another row, or a late second click, only chooses.
2. **The menu's Encyclopedia opened the index, not the topic.** A world character's or system's `DatId` carries no family byte. `OriginalEncyclopediaCatalog::resolve_object` takes a bare id to the one character or system entry with that record id; `None` when two such entries share it.
   - Test: `a_bare_character_or_system_id_finds_its_entry`.

**The gates.**

- `personnel-states.mjs` double-clicks Abduction, then presses Next, Previous and Previous. It asserts the topics and the arrows' states, Abduction, Ackbar, Abduction and A-wing with Previous disabled, and that a further Previous stays on A-wing. Run `personnel-states-2026-11-03T01-47-36-244Z`: pass.
- `status-window.mjs` `force-character` chooses Encyclopedia from the agent's menu after its Status. It asserts the topic of that character, Mon Mothma. Run `status-window-2026-11-03T01-42-35-106Z-1623837`: pass.

Mutants on `encyclopedia.rs`: 39 caught, 1 unviable.
