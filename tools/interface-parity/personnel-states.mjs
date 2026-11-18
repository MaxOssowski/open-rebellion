// The Personnel Finder's character states in the browser build
// (crates/rebellion-render/src/personnel_finder.rs).
//
// Opens the PersonnelStates fixture (code 93) for each faction: one of the
// player's characters on a mission, another travelling to the secondary
// system, a third a General. F5 opens the Personnel Finder on its
// Characters rows, where the first reads "Name - Location ( On Mission ) ", the traveller names its
// destination, "Name - Destination ( Enroute ) ", a General reads "Name -
// Location ( General ) " and the rest only "Name - Location" (FUN_00465bb0;
// A0 specforces/00 and personnel-states/00: "Han Solo - Drall ( On Mission
// )", "Adar Tallon - Mon Calamari ( Enroute )", "Huoba Neva - Pzob
// ( General )", "Chewbacca - Yavin").
//
// Then, for the Alliance, the SpecForcesYavin fixture (code 114): one
// Alliance special force at Yavin, as A0 2026-11-13-gid/E1..E6 shows the
// SpecForces view (OBJ-03-C019). Typing in the name box chooses the row
// FUN_00609650 finds (FUN_00464c20, 0x408): "Q", which no row begins,
// chooses the first row; "Y" and "Ya" keep it, and nothing scrolls.
//
// Last, for each faction, the PersonnelCopied fixture (code 127): the other
// side's tab lists the player's copies of four of the other side's
// characters: one aboard a ship of a fleet only its frozen copy of a system
// holds, "Name - <ship> N"; one in no place, "Name - Location Unknown"
// (FUN_00465bb0, 0x1895; A0 2026-11-14-stale-views second run,
// 1x-personnelfinder-empire-d85 d57ed845 and d91 1b8818d8); one frozen
// en route to that system, "Name - <system> ( Enroute ) " (+0x50 bit 4,
// 0x8744); and one on a mission there, "( On Mission ) " (0x8745).
//
// Then, for each faction, the PersonnelAbode fixture (code 142) against A0
// 2026-11-17-edited-saves-6 d-01..d-04, b-07 (the Alliance's own Dagobah
// row, chosen, its menu and Display) and b-01 (the Empire's "Location
// Unknown" for Luke), OBJ-03-C011.
//
// Last, for the Alliance, the PersonnelPalace fixture (code 145) against A0
// 2026-11-18-edited-saves-7 b-19 (EVT-01-C018): four prisoners the
// Alliance's copies hold in Jabba's Palace, read "Name - Jabba's Palace".
//
// And for the Empire, the PersonnelRetired fixture (code 146) against A0
// 2026-11-18-edited-saves-9 d-03..d-08 (OBJ-03-C011): "Ozzel - Retired",
// the row chosen with the name box left empty, its menu's grey items
// ignoring clicks.
//
// And for the Empire, the PersonnelPickup fixture (code 147) against A0
// 2026-11-18-edited-saves-9 c-14 (EVT-01-C018): the Alliance tab after
// Jabba Delivers Prisoners, three prisoners "Name - Victory Destroyer 1
// ( Captured )" and Chewbacca, killed, read from a frozen copy at Yavin.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node personnel-states.mjs [--out=DIR] [--faction=alliance|empire]
//              [--only=specforces|copied|abode|palace|retired|pickup]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SITE = path.join(ROOT, ".artifacts/interface-parity/site");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `personnel-states-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 92 PersonnelStates.
const PERSONNEL_STATES = 93;
// 113 SpecForcesYavin.
const SPECFORCES_YAVIN = 114;
// 126 PersonnelCopied.
const PERSONNEL_COPIED = 127;
const FACTIONS = [
  { name: "alliance", byte: 1, encyclopedia: [62, 52] },
  { name: "empire", byte: 2, encyclopedia: [125, 52] },
].filter(({ name }) => !args.faction || args.faction === name);
// A0 encyclopedia/category-0..6-alliance-day185: each category's label,
// first eight rows and chosen row (All Databases chooses nothing, a
// category its first row).
const ENCYCLOPEDIA_A0 = [
  ["All Databases", null, ["A-wing", "Abduction", "Ackbar", "Adar Tallon", "Adega",
    "Advanced Construction Yard", "Advanced Shipyard", "Advanced Training Facility"]],
  ["System Database", 0, ["Adega", "Agrilat", "Akrit'tar", "Algarian", "Alk'lellish III",
    "Allyuen", "Anchoron", "Annaj"]],
  ["Ship Database", 0, ["A-wing", "Alliance Dreadnaught", "Alliance Escort Carrier", "Assault Frigate",
    "Assault Transport", "B-wing", "Bulk Cruiser", "Bulk Transport"]],
  ["Facilities Database", 0, ["Advanced Construction Yard", "Advanced Shipyard",
    "Advanced Training Facility", "Construction Yard", "Death Star Shield", "GenCore Level I",
    "GenCore Level II", "KDY-150"]],
  ["Missions Database", 0, ["Abduction", "Assassination", "Death Star Sabotage", "Diplomacy",
    "Espionage", "Facility Design Research", "Incite Uprising", "Jedi Training"]],
  ["Troop Database", 0, ["Alliance Army Regiment", "Alliance Fleet Regiment", "Dark Trooper Regiment",
    "Imperial Army Regiment", "Imperial Fleet Regiment", "Mon Calamari Regiment",
    "Stormtrooper Regiment", "Sullustan Regiment"]],
  ["Personnel Database", 0, ["Ackbar", "Adar Tallon", "Afyon", "Bane Nothos", "Bevel Lemelisk",
    "Bin Essada", "Borsk Fey'lya", "Bothan Spies"]],
];
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream" };

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
    try {
      const bytes = await fs.readFile(file);
      response.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

async function frames(page, count = 3) {
  await page.evaluate((n) => new Promise((resolve) => {
    let left = n;
    const step = () => (--left <= 0 ? resolve() : requestAnimationFrame(step));
    requestAnimationFrame(step);
  }), count);
}

async function click(page, [x, y]) {
  await page.mouse.move(x, y);
  await frames(page);
  await page.mouse.down();
  await frames(page);
  await page.mouse.up();
  await frames(page);
}

const latest = (page) => page.evaluate(() => window.__openRebellionInterfacePersonnelFinder ?? null);
const ready = (page) => page.evaluate(() => window.__openRebellionInterfaceReady ?? null);

async function until(page, description, predicate) {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const value = await latest(page);
    if (value && predicate(value)) return value;
    if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
    await page.waitForTimeout(100);
  }
}

async function run(browser, base, faction) {
  const directory = path.join(OUT, faction.name);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const log = [];
  const errors = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const missing = [];
  page.on("response", (response) => {
    if (response.status() >= 400) missing.push(`${response.status()} ${new URL(response.url()).pathname}`);
  });
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  try {
    const code = PERSONNEL_STATES | (faction.byte << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    const deadline = Date.now() + 30_000;
    let setup = null;
    while ((setup = await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error(`the fixture never reported: ${JSON.stringify(setup)}`);
      await page.waitForTimeout(100);
    }
    assert.equal(setup.code, code, JSON.stringify(setup));
    await frames(page, 6);
    await page.locator("#glcanvas").focus();

    // F5 opens the Personnel Finder on its Characters rows.
    await page.keyboard.press("F5");
    const shown = await until(page, "F5 opens the Personnel Finder", (o) => o.rows.length > 0);
    const busy = shown.rows.filter((row) => / - .+ \( On Mission \) $/.test(row));
    const travelling = shown.rows.filter((row) => / - .+ \( Enroute \) $/.test(row));
    const commanding = shown.rows.filter((row) => / - .+ \( General \) $/.test(row));
    // An available character has no state: "Chewbacca - Yavin" (A0 personnel-states/00).
    const available = shown.rows.filter((row) => / - [^()]+$/.test(row));
    assert.equal(busy.length, 1, JSON.stringify(shown.rows));
    assert.equal(travelling.length, 1, JSON.stringify(shown.rows));
    assert.equal(commanding.length, 1, JSON.stringify(shown.rows));
    assert.ok(available.length > 0, JSON.stringify(shown.rows));
    // With no container FUN_00465bb0 keeps "Location Unknown" alone, past
    // the state and command words (0x465cb5 -> 0x46606f).
    assert.ok(!shown.rows.some((row) => /Location Unknown \(/.test(row)), JSON.stringify(shown.rows));
    // The traveller names its destination, not where it left from.
    assert.ok(!travelling[0].includes("Location Unknown"), travelling[0]);
    // A captive aboard names the ship it rides, numbered by its class, and
    // " ( Captured ) " (A0 personnel-captured/finder-day628: "Jan Dodonna -
    // Punisher ( Captured )"; fleet2/umgul-fleet1-ships: "Corellian
    // Corvette 1"). Both sides (OBJ-12-C009; the fixture recruits up to
    // eight of the side's characters).
    // A0 2026-11-15-partial-cells (an Alliance game) reads the same words
    // and spacing: "Leia Organa - Yaga Minor ( Captured )", "Mon Mothma -
    // Plunderer ( Captured )", "Chewbacca - Ghorman ( Injured )",
    // "Orrimaarko - Killed" (r2-...-d1239 0be4236024ec, r3-...-d1258
    // e43a12aa38b1, r4-...-d795 9fbd1ee50231).
    const captured = shown.rows.filter((row) => / - .+ \( Captured \) $/.test(row));
    assert.equal(captured.length, 1, JSON.stringify(shown.rows));
    assert.match(captured[0], / - [A-Za-z][A-Za-z -]* \d+ \( Captured \) $/, captured[0]);
    assert.ok(!/ - Fleet \d+ /.test(captured[0]), captured[0]);
    // An injured free character reads " ( Injured ) " (FUN_00465bb0,
    // 0x8747 on +0x94; A0 health-2026-11-09/07-personnel-finder-wedge-
    // injured-day954: "Wedge Antilles - Averam ( Injured )").
    const injured = shown.rows.filter((row) => / - [^()]+ \( Injured \) $/.test(row));
    assert.equal(injured.length, 1, JSON.stringify(shown.rows));
    // A dead character reads "Name - Killed", with no place and no state
    // (FUN_00465bb0's destroyed branch, 0x8798; Steam guide guide-023:
    // "Ackbar - Killed"; no A0 capture yet), OBJ-12-C010.
    const killed = shown.rows.filter((row) => / - Killed$/.test(row));
    assert.equal(killed.length, 1, JSON.stringify(shown.rows));
    assert.match(killed[0], /^[A-Za-z][A-Za-z' -]* - Killed$/, killed[0]);
    // A retired character reads "Name - Retired" (destruction reason
    // 0x14..0x16, 0x8799; FUN_00534c00), and one in an abode names the abode
    // alone, with no state word (container family 0x98..0x9f), OBJ-03-C011.
    const retired = shown.rows.filter((row) => / - Retired$/.test(row));
    assert.equal(retired.length, 1, JSON.stringify(shown.rows));
    assert.match(retired[0], /^[A-Za-z][A-Za-z' -]* - Retired$/, retired[0]);
    const resting = shown.rows.filter((row) => shown.abodes.some((abode) => row.endsWith(` - ${abode}`)));
    assert.equal(resting.length, 1, JSON.stringify([shown.rows, shown.abodes]));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("characters");

    // A right-button release on the Finder opens the object menu with
    // nothing selected: Encyclopedia and Status, both grey (FUN_004ac5c0,
    // FUN_00461740; A0 personnel-states/03, 04).
    const at = [160, 220];
    await page.mouse.move(...at);
    await frames(page);
    await page.mouse.down({ button: "right" });
    await frames(page);
    await page.mouse.up({ button: "right" });
    await frames(page, 4);
    const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu ?? null);
    assert.ok(menu, "the right release opened no menu");
    assert.deepEqual(menu.row_states, [["Encyclopedia", false], ["Status", false]], JSON.stringify(menu));
    // Two rows of 20 and the frame, as A0's 7-row menu is 142 high.
    assert.equal(menu.height, 42, JSON.stringify(menu));
    await shot("row-menu");
    // Escape closes the menu and leaves the Finder open.
    await page.keyboard.press("Escape");
    await frames(page, 4);
    // Cleared after the Escape's frames: a report since then comes from a
    // menu or Finder still open.
    await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = null; window.__openRebellionInterfacePersonnelFinder = null; });
    await frames(page, 4);
    assert.equal(await page.evaluate(() => window.__openRebellionInterfaceObjectMenu), null, "the menu stayed open");
    const stays = await until(page, "the Finder stays open after the menu's Escape", (o) => o.rows.length > 0);

    // The other side's tab lists only the characters the player's side
    // sees (FUN_004f3d50 views; A0 personnel-other-side/test-day8-imperial-
    // tab-empty, an unedited Alliance game, lists no Imperial character).
    // This galaxy shows none of the other side's objects, as the Fleet
    // Finder's other tab below finds.
    const ownTab = faction.name === "alliance" ? "alliance" : "imperial";
    const otherTabName = faction.name === "alliance" ? "imperial" : "alliance";
    assert.equal(stays.tab, ownTab, JSON.stringify(stays));
    await page.mouse.move(...stays.tabs[faction.name === "alliance" ? 1 : 0]);
    await frames(page);
    await page.mouse.down();
    await frames(page);
    await page.mouse.up();
    await frames(page, 3);
    await page.mouse.move(2, 2);
    const otherCharacters = await until(page, "the other side's Personnel tab shows",
      (o) => o.tab === otherTabName);
    assert.deepEqual(otherCharacters.rows, [], JSON.stringify(otherCharacters.rows));
    await frames(page, 3);
    await shot("characters-other-side");
    if (faction.name === "alliance") {
      // The empty list (99..449, 185..353) and the side tabs against A0,
      // pixel for pixel; the header's text is the accepted deviation.
      const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/personnel-other-side/test-day8-imperial-tab-empty.png")));
      const port = PNG.sync.read(await page.screenshot());
      const differing = (x0, y0, x1, y1) => {
        let count = 0;
        for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
          const i = (y * a0.width + x) * 4;
          const j = (y * port.width + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[j + c])) count += 1;
        }
        return count;
      };
      const otherSideA0 = { list: differing(99, 185, 449, 353), tabs: differing(98, 122, 200, 163) };
      assert.deepEqual(otherSideA0, { list: 0, tabs: 0 }, JSON.stringify(otherSideA0));
    }

    // The Fleet Finder's tab of the other side, whose fleets this galaxy
    // does not show, lists nothing and has no bar (A0
    // personnel-states/07: "Imperial Fleets", empty).
    await page.keyboard.press("Escape");
    await frames(page, 4);
    await page.keyboard.press("F3");
    const fleetFinder = () => page.evaluate(() => {
      const list = window.__openRebellionInterfaceFleetFinders || [];
      return list[list.length - 1] ?? null;
    });
    const fleetUntil = async (description, predicate) => {
      const deadline = Date.now() + 15_000;
      for (;;) {
        const value = await fleetFinder();
        if (value && predicate(value)) return value;
        if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
        await page.waitForTimeout(100);
      }
    };
    const fleets = await fleetUntil("F3 opens the Fleet Finder", (o) => o.open && o.controls);
    const otherTab = faction.name === "alliance" ? 2 : 1;
    const otherName = faction.name === "alliance" ? "imperial" : "alliance";
    await page.mouse.move(...fleets.controls.tabs[otherTab]);
    await frames(page);
    await page.mouse.down();
    await frames(page);
    await page.mouse.up();
    const other = await fleetUntil("the other side's tab shows", (o) => o.tab === otherName);
    assert.deepEqual(other.rows, [], JSON.stringify(other.rows));
    await page.mouse.move(2, 2);
    await frames(page, 3);
    await shot("fleet-finder-other-side");

    // F4: the Troop Finder's other side's tab lists no regiment here either
    // (FUN_0046ea10 walks the player's side's views; A0 troop-other-side/
    // test-day220-imperial-tab-empty, an unedited Alliance game): its empty
    // list and the side tabs every pixel A0's.
    if (faction.name === "alliance") {
      await page.keyboard.press("Escape");
      await frames(page, 4);
      await page.keyboard.press("F4");
      await frames(page, 6);
      await page.mouse.move(62 + 88 + 24, 50 + 72 + 20);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.up();
      await frames(page, 3);
      await page.mouse.move(2, 2);
      await frames(page, 3);
      await shot("troop-finder-other-side");
      const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/troop-other-side/test-day220-imperial-tab-empty.png")));
      const port = PNG.sync.read(await page.screenshot());
      const differing = (x0, y0, x1, y1) => {
        let count = 0;
        for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
          const i = (y * a0.width + x) * 4;
          const j = (y * port.width + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== port.data[j + c])) count += 1;
        }
        return count;
      };
      const troopsOtherSide = { list: differing(99, 192, 449, 353), tabs: differing(98, 122, 200, 163) };
      assert.deepEqual(troopsOtherSide, { list: 0, tabs: 0 }, JSON.stringify(troopsOtherSide));
    }

    // F7: the Encyclopedia index, each category icon (0x6f..0x75 at the
    // window's (36 + 52 i, 78)) against A0's label, rows and choice.
    await page.keyboard.press("Escape");
    await frames(page, 4);
    await page.keyboard.press("F7");
    const encyclopedia = [];
    for (const [index, [label, chosen, rows]] of ENCYCLOPEDIA_A0.entries()) {
      const [ox, oy] = faction.encyclopedia;
      await page.mouse.move(ox + 36 + 52 * index + 24, oy + 78 + 20);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.up();
      await frames(page);
      await page.mouse.move(2, 2);
      const deadline = Date.now() + 10_000;
      let shownIndex;
      for (;;) {
        shownIndex = await page.evaluate(() => window.__openRebellionInterfaceEncyclopedia ?? null);
        if (shownIndex && shownIndex.command === 0x6f + index) break;
        if (Date.now() > deadline) throw new Error(`category ${index}: ${JSON.stringify(shownIndex)}`);
        await page.waitForTimeout(100);
      }
      encyclopedia.push(shownIndex);
      await shot(`encyclopedia-${index}`);
    }
    const encyclopediaDiffs = ENCYCLOPEDIA_A0.map(([label, chosen, rows], index) => {
      const shownIndex = encyclopedia[index];
      return shownIndex.label === label && shownIndex.chosen === chosen
        && JSON.stringify(shownIndex.rows) === JSON.stringify(rows) ? null
        : { index, a0: { label, chosen, rows }, port: shownIndex };
    }).filter(Boolean);
    assert.deepEqual(encyclopediaDiffs, [], JSON.stringify(encyclopediaDiffs));

    // A topic's Previous and Next (0x83 at (28, 14), 0x84 at (380, 14);
    // FUN_0045fd20) step through All Databases: A0 encyclopedia/topic-step-2..6
    // reads Abduction, Ackbar, Abduction, A-wing, then A-wing with Previous
    // grey.
    const [ex, ey] = faction.encyclopedia;
    const clickAt = async (x, y) => {
      await page.mouse.move(x, y);
      await frames(page);
      await page.mouse.down();
      await frames(page);
      await page.mouse.up();
      await frames(page);
    };
    await clickAt(ex + 36 + 24, ey + 78 + 20);
    await page.evaluate(() => { window.__openRebellionInterfaceEncyclopediaTopic = null; });
    // Two clicks a frame apart, inside egui's double-click time.
    await page.mouse.move(ex + 100, ey + 165);
    await frames(page);
    for (let i = 0; i < 2; i += 1) {
      await page.mouse.down();
      await frames(page, 1);
      await page.mouse.up();
      await frames(page, 1);
    }
    const topicSteps = [];
    const topicUntil = async (description, name) => {
      const deadline = Date.now() + 10_000;
      for (;;) {
        const value = await page.evaluate(() => window.__openRebellionInterfaceEncyclopediaTopic ?? null);
        if (value && value.topic === name) return value;
        if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
        await page.waitForTimeout(100);
      }
    };
    topicSteps.push(await topicUntil("a double click opens Abduction's topic", "Abduction"));
    // OBJ-01-C008, the topic text. The body is ENCYTEXT's text as the
    // window draws it (FUN_0045fa60 -> FUN_0041ecf0 at (17, 231), 395 by
    // 80, DT_WORDBREAK in the default font). A0 shows Abduction's five
    // lines (the Alliance's encyclopedia/topic-step-2, the Empire's
    // topic-abduction-empire-day166): the body must be exactly their
    // words, and each line's ink must sit within a pixel of A0's, so each
    // line breaks at A0's word.
    await frames(page, 4);
    const abductionShot = PNG.sync.read(await page.screenshot());
    await shot("encyclopedia-topic-abduction");
    const ABDUCTION_A0 = ["Abduction missions are used to capture a specific character. The",
      "target of this mission must be an enemy character on a specific",
      "enemy fleet or system. The success of this mission depends on the",
      "combat attributes of the characters and the SpecForces assigned to",
      "the mission team."];
    assert.equal(topicSteps[0].title, "Abduction", JSON.stringify(topicSteps[0]));
    assert.equal(topicSteps[0].body, ABDUCTION_A0.join(" "), JSON.stringify(topicSteps[0]));
    const whiteInk = (png, [x0, y0, x1, y1]) => {
      let box = null;
      for (let y = y0; y < y1; y += 1) {
        for (let x = x0; x < x1; x += 1) {
          const i = (y * png.width + x) * 4;
          if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 200) continue;
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
      }
      return box;
    };
    const inkAgainst = (a0Png, portPng, regions) => {
      const checks = {};
      for (const [name, region] of regions) {
        const [a, p] = [whiteInk(a0Png, region), whiteInk(portPng, region)];
        checks[name] = { a0: a, port: p };
        assert.ok(a && p && a.every((value, k) => Math.abs(value - p[k]) <= 1),
          `${name}: ${JSON.stringify(checks[name])}`);
      }
      return checks;
    };
    const bodyLine = (k, x0 = 17, x1 = 412) => [ex + x0, ey + 231 + 16 * k, ex + x1, ey + 247 + 16 * k];
    const abductionA0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/encyclopedia",
      faction.name === "alliance" ? "topic-step-2.png" : "topic-abduction-empire-day166.png")));
    const abductionText = inkAgainst(abductionA0, abductionShot,
      ABDUCTION_A0.map((_, k) => [`abduction-line-${k + 1}`, bodyLine(k)]));
    for (const [x, name] of [[380, "Ackbar"], [28, "Abduction"], [28, "A-wing"]]) {
      await clickAt(ex + x + 10, ey + 14 + 8);
      topicSteps.push(await topicUntil(`the arrow steps to ${name}`, name));
    }
    assert.deepEqual(topicSteps.map(({ topic, previous, next }) => [topic, previous, next]),
      [["Abduction", true, true], ["Ackbar", true, true], ["Abduction", true, true], ["A-wing", false, true]]);
    await clickAt(ex + 28 + 10, ey + 14 + 8);
    await frames(page, 4);
    const stayed = await page.evaluate(() => window.__openRebellionInterfaceEncyclopediaTopic);
    assert.equal(stayed.topic, "A-wing", "a grey Previous does nothing");
    await shot("encyclopedia-topic-first");
    // A-wing's body outgrows the 80-pixel text control (FUN_0041ecf0), so
    // its bar (FUN_0041efe0: 0x299a) stands at the control's right edge:
    // A0 encyclopedia/topic-step-6 (the Alliance) has its up and down
    // arrows, 13 by 9, at the window's (399, 231) and (399, 302), every
    // pixel; the thumb follows the line count and is not compared.
    let topicBar = null;
    const topicText = { abduction: abductionText };
    if (faction.name === "alliance") {
      const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/encyclopedia/topic-step-6.png")));
      const port = PNG.sync.read(await page.screenshot());
      const differ = (top) => {
        let count = 0;
        for (let y = ey + top; y < ey + top + 9; y += 1) {
          for (let x = ex + 399; x < ex + 412; x += 1) {
            const i = (y * 640 + x) * 4;
            if ([0, 1, 2].some((c) => port.data[i + c] !== a0.data[i + c])) count += 1;
          }
        }
        return count;
      };
      topicBar = { up: differ(231), down: differ(302) };
      assert.ok(topicBar.up === 0 && topicBar.down === 0, `the topic's bar: ${JSON.stringify(topicBar)}`);
      // A-wing's first five lines are label-tab-value rows. A0 topic-step-6
      // reads them so; each label and each value (the tab stop) inks
      // within a pixel of A0's.
      const AWING_A0 = [["Refined Material Cost:", "5"], ["Maintenance Cost:", "4"],
        ["Attack Strength (Laser Cannon):", "5"], ["Attack Strength (Ion Cannon):", "0"], ["Shield Energy:", "5"]];
      const awing = topicSteps[3];
      assert.deepEqual(awing.body.split("\n").slice(0, 5).map((line) => line.split(/\t+/)), AWING_A0,
        JSON.stringify(awing));
      topicText.awing = inkAgainst(a0, port, AWING_A0.flatMap((_, k) => [
        [`a-wing-label-${k + 1}`, bodyLine(k, 17, 224)], [`a-wing-value-${k + 1}`, bodyLine(k, 224, 398)]]));
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", busy: busy[0], travelling: travelling[0],
      commanding: commanding[0], available, rows: shown.rows,
      menu: { left: menu.left, top: menu.top, width: menu.width, height: menu.height, rows: menu.row_states },
      topicSteps: topicSteps.map(({ topic, previous, next }) => [topic, previous, next]),
      topicBar, topicText,
      encyclopedia: encyclopedia.map(({ label, chosen }) => ({ label, chosen })),
      fleet_finder_other_tab: { tab: other.tab, rows: other.rows.length, own: fleets.rows.length }, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors, missing };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

// A0 2026-11-13-gid/E1..E6 against the SpecForcesYavin fixture.
async function runSpecForces(browser, base) {
  const directory = path.join(OUT, "specforces");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/2026-11-13-gid", file)));
  // Pixels that differ in a region, outside the name and the count
  // cells' glyphs (text: the accepted deviation, compared by ink box). A
  // glyph pixel is a light one (min channel over 150) in either image; the
  // cells' red box (255, 0, 0) stays compared.
  const ROW_NAME = [100, 192, 316, 214];
  const CELLS = [318, 193, 449, 214];
  const LIST = [99, 192, 449, 353];
  const inside = ([x0, y0, x1, y1], x, y) => x >= x0 && y >= y0 && x < x1 && y < y1;
  const light = (png, i) => Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 150;
  const differing = (port, ref) => {
    let count = 0;
    for (let y = LIST[1]; y < LIST[3]; y += 1) for (let x = LIST[0]; x < LIST[2]; x += 1) {
      if (inside(ROW_NAME, x, y)) continue;
      const i = (y * ref.width + x) * 4;
      if (inside(CELLS, x, y) && (light(ref, i) || light(port, i))) continue;
      // The count's digit, drawn over its cell's box: 8 pixels from each
      // cell's left (cells 28 wide from x 318), rows 197..207.
      if (inside(CELLS, x, y) && y >= 197 && y < 208 && (x - 318) % 28 >= 8 && (x - 318) % 28 < 20) continue;
      if ([0, 1, 2].some((c) => ref.data[i + c] !== port.data[i + c])) count += 1;
    }
    return count;
  };
  // The name's ink box: white when chosen (min channel over 200); grey
  // when not (FUN_00465540: 0x787878; A0 shows (117, 114, 114), the
  // palette step), a light pixel with red and blue within 20.
  const isInk = (chosen) => (png, i) => (chosen
    ? Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 200
    : png.data[i] >= 100 && png.data[i] <= 200 && Math.abs(png.data[i] - png.data[i + 2]) <= 20);
  const ink = (png, [x0, y0, x1, y1], test) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(png, (y * png.width + x) * 4)) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  const compare = async (label, a0File, chosen) => {
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    const port = PNG.sync.read(bytes);
    const ref = await a0(a0File);
    // The row's name below the list's top line (y 192).
    const name = [100, 194, 316, 214];
    const step = { label, a0: a0File,
      sha256: createHash("sha256").update(bytes).digest("hex"),
      list_outside_text: differing(port, ref),
      name_ink: { a0: ink(ref, name, isInk(chosen)), port: ink(port, name, isInk(chosen)) },
      white_ink: { a0: ink(ref, name, isInk(true)), port: ink(port, name, isInk(true)) } };
    steps.push(step);
    assert.equal(step.list_outside_text, 0, JSON.stringify(step));
    assert.ok(step.name_ink.a0 && step.name_ink.port, JSON.stringify(step));
    // Left within a pixel, top and bottom within two: the port's
    // anti-aliased glyphs are 8 rows to A0's 10 (the accepted text
    // deviation), and so is the width.
    assert.ok(Math.abs(step.name_ink.a0[0] - step.name_ink.port[0]) <= 1
      && [1, 3].every((k) => Math.abs(step.name_ink.a0[k] - step.name_ink.port[k]) <= 2), JSON.stringify(step));
    // Not chosen: no white in the name, in A0 or the port.
    if (!chosen) assert.deepEqual(step.white_ink, { a0: null, port: null }, JSON.stringify(step));
  };
  try {
    const code = SPECFORCES_YAVIN | (1 << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the SpecForcesYavin fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("F5");
    await until(page, "F5 opens the Personnel Finder", (o) => o.tab === "alliance");
    // The fourth right button (500, 265): the SpecForces view (A0 E1).
    await click(page, [500, 265]);
    const view = await until(page, "the SpecForces view shows", (o) => o.rows.length === 1);
    assert.deepEqual([view.rows, view.chosen, view.name], [["Yavin"], null, ""], JSON.stringify(view));
    await compare("specforces-alliance", "E1-specforces-finder-alliance.png", false);
    // E2: the Empire tab lists nothing.
    await click(page, view.tabs[1]);
    const empire = await until(page, "the Empire tab shows", (o) => o.tab === "imperial");
    steps.push({ label: "specforces-empire", rows: empire.rows });
    assert.deepEqual(empire.rows, [], JSON.stringify(empire.rows));
    await click(page, view.tabs[0]);
    await until(page, "the Alliance tab shows again", (o) => o.tab === "alliance" && o.rows.length === 1);
    // E3: a click in the name box chooses nothing.
    await click(page, [320, 104]);
    const boxed = await until(page, "the name box takes the focus", (o) => o.tab === "alliance");
    assert.deepEqual([boxed.chosen, boxed.name], [null, ""], JSON.stringify(boxed));
    await compare("specforces-name-box", "E3-specforces-name-box-clicked-empty.png", false);
    // E4: "Q", which no row begins, chooses the first row.
    await page.keyboard.type("Q");
    const q = await until(page, "Q is typed", (o) => o.name === "Q");
    assert.equal(q.chosen, 0, JSON.stringify(q));
    await compare("specforces-typed-q", "E4-specforces-typed-Q.png", true);
    // E5, E6: "Y" and "Ya" keep it; one row, so nothing scrolls.
    await page.keyboard.press("Backspace");
    await page.keyboard.type("Y");
    const y = await until(page, "Y is typed", (o) => o.name === "Y");
    assert.equal(y.chosen, 0, JSON.stringify(y));
    await compare("specforces-typed-y", "E5-specforces-typed-Y.png", true);
    await page.keyboard.type("a");
    const ya = await until(page, "Ya is typed", (o) => o.name === "Ya");
    assert.equal(ya.chosen, 0, JSON.stringify(ya));
    await compare("specforces-typed-ya", "E6-specforces-typed-Ya.png", true);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "alliance-specforces", status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: "alliance-specforces", status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

async function runCopied(browser, base, faction) {
  const label = `${faction.name}-copied`;
  const directory = path.join(OUT, label);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  try {
    const code = PERSONNEL_COPIED | (faction.byte << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the PersonnelCopied fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("F5");
    const alliance = faction.name === "alliance";
    const own = await until(page, "F5 opens the Personnel Finder",
      (o) => o.tab === (alliance ? "alliance" : "imperial"));
    await click(page, own.tabs[alliance ? 1 : 0]);
    await page.mouse.move(2, 2);
    const imperial = await until(page, "the other side's tab shows",
      (o) => o.tab === (alliance ? "imperial" : "alliance"));
    await frames(page, 3);
    const bytes = await page.screenshot({ path: path.join(directory, "other-side-copies.png") });
    steps.push({ label: "other-side-copies", rows: imperial.rows,
      sha256: createHash("sha256").update(bytes).digest("hex") });
    assert.equal(imperial.rows.length, 4, JSON.stringify(imperial.rows));
    const lost = imperial.rows.filter((row) => / - Location Unknown$/.test(row));
    const enroute = imperial.rows.filter((row) => / - [^()]+ \( Enroute \) $/.test(row));
    const busy = imperial.rows.filter((row) => / - [^()]+ \( On Mission \) $/.test(row));
    // The copy aboard keeps the Admiral's word it froze with (+0x96 via
    // FUN_004f04f0; A0 2026-11-15-partial-cells r3-...-imperial-no-
    // captives-d1258 6966a4fce5ca: "Griff - Gorgon ( Admiral )").
    const aboard = imperial.rows.filter((row) => / - [^()]+ \( Admiral \) $/.test(row));
    assert.equal(lost.length, 1, JSON.stringify(imperial.rows));
    assert.equal(enroute.length, 1, JSON.stringify(imperial.rows));
    assert.equal(busy.length, 1, JSON.stringify(imperial.rows));
    assert.equal(aboard.length, 1, JSON.stringify(imperial.rows));
    // The en route and mission copies name the same copied system.
    assert.equal(enroute[0].replace(/^.* - /, "").replace(/ \(.*$/, ""),
      busy[0].replace(/^.* - /, "").replace(/ \(.*$/, ""), JSON.stringify(imperial.rows));
    // The copied ship's name, numbered by its class, with no state word
    // and not the fleet's name.
    assert.match(aboard[0], /^[A-Za-z][A-Za-z' -]* - [A-Za-z][A-Za-z -]* \d+ \( Admiral \) $/, aboard[0]);
    assert.ok(!/ - Fleet \d+ /.test(aboard[0]), aboard[0]);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: label, status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: label, status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

// OBJ-03-C011: the PersonnelAbode fixture (code 142) against A0
// 2026-11-17-edited-saves-6 (the original game, not committed; sha256
// prefixes cited). For the Alliance, d-01 (a50b31353dcd, day 368): eight
// rows, "Luke Skywalker - Dagobah" with no state word (FUN_00465bb0, an
// abode container, family 0x98..0x9f) and "Han Solo - Sluis Van
// ( Captured )" (0x8746); each row's grey ink against A0's. A click on
// Luke's row (d-02 bab3f3b8d33e) whitens it and puts "Luke Skywalker"
// alone in the name box (FUN_00464890 0xca/0x29b, FUN_00583c40). A right
// release opens Encyclopedia and Status, both grey (d-03 a74960a76c3a,
// d-04 b77c8421ac5e: a 90 by 42 menu). Display (500, 158) with the row
// chosen closes the Finder and opens nothing (b-07 99ac654bede2:
// FUN_00429440 finds no system for an abode container; FUN_00464890 case
// 0xc9 closes). For the Empire, b-01 (c8e08a5d4c6d, day 411): the Alliance
// tab names Luke "Location Unknown" (0x1895 alone), every row against A0.
const PERSONNEL_ABODE = 142;
const ABODE_A0 = ".artifacts/a0-vm/2026-11-17-edited-saves-6";
const ABODE_ROWS = {
  alliance: ["Chewbacca - Yavin", "Han Solo - Sluis Van ( Captured ) ", "Huoba Neva - Corsin",
    "Jan Dodonna - Yavin", "Leia Organa - Yavin", "Luke Skywalker - Dagobah", "Mon Mothma - Chazwa",
    "Wedge Antilles - Yavin"],
  empire: ["Chewbacca - Location Unknown", "Garm Bel Iblis - Fwillsving ( General ) ",
    "Han Solo - Fwillsving", "Jan Dodonna - Location Unknown", "Leia Organa - Location Unknown",
    "Luke Skywalker - Location Unknown", "Syub Snunb - Location Unknown",
    "Wedge Antilles - Location Unknown"],
};

async function runAbode(browser, base, faction) {
  const label = `${faction.name}-abode`;
  const directory = path.join(OUT, label);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(ROOT, ABODE_A0, file)));
  const grab = async (name) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${name}.png`) });
    steps.push({ label: name, sha256: createHash("sha256").update(bytes).digest("hex") });
    return PNG.sync.read(bytes);
  };
  // FUN_00465540's grey (A0 (117, 114, 114), the palette step of
  // 0x787878) and the chosen row's white.
  const grey = (png, i) => png.data[i] >= 100 && png.data[i] <= 200 && Math.abs(png.data[i] - png.data[i + 2]) <= 20;
  const white = (png, i) => Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 200;
  const ink = (png, [x0, y0, x1, y1], test) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(png, (y * png.width + x) * 4)) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  // Left within a pixel, top and bottom within two: the port's glyphs are
  // 8 rows to A0's 10 and narrower (the accepted text deviation), so the
  // right edge is reported, not held.
  const near = (a, p) => Boolean(a && p) && Math.abs(a[0] - p[0]) <= 1
    && Math.abs(a[1] - p[1]) <= 2 && Math.abs(a[3] - p[3]) <= 2;
  // Each row's ink, the list's rows 20 high from y 194 (A0 d-01: tops
  // 199..339) and x from 100 (the Empire's window 63 to the right).
  const rowInk = (ref, port, dx, test, skip = []) => [...Array(8).keys()]
    .filter((row) => !skip.includes(row))
    .map((row) => {
      const area = [100 + dx, 194 + 20 * row, 440 + dx, 214 + 20 * row];
      const box = { row, a0: ink(ref, area, test), port: ink(port, area, test) };
      return { ...box, near: near(box.a0, box.port) };
    });
  try {
    const code = PERSONNEL_ABODE | (faction.byte << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the PersonnelAbode fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const before = await grab("before-finder");
    await page.keyboard.press("F5");
    const alliance = faction.name === "alliance";
    const own = await until(page, "F5 opens the Personnel Finder",
      (o) => o.tab === (alliance ? "alliance" : "imperial") && o.rows.length > 0);
    if (!alliance) {
      await click(page, own.tabs[0]);
      await page.mouse.move(2, 470);
    }
    const shown = await until(page, "the Alliance tab shows", (o) => o.tab === "alliance");
    steps.push({ label: "rows", rows: shown.rows, chosen: shown.chosen, name: shown.name });
    assert.deepEqual(shown.rows, ABODE_ROWS[faction.name], JSON.stringify(shown.rows));
    assert.deepEqual([shown.chosen, shown.name], [null, ""], JSON.stringify(shown));
    await frames(page, 4);
    const listed = await grab("rows");
    if (!alliance) {
      // b-01: the Empire's window, 63 right and 2 down of the Alliance's.
      const ref = await a0("b-01-hqe393-unedited-empire-finder-alliance-tab-luke-location-unknown-d411.png");
      const rows = rowInk(ref, listed, 63, grey);
      steps.push({ label: "rows-vs-b-01", rows });
      assert.ok(rows.every(({ near: ok }) => ok), JSON.stringify(rows));
      assert.deepEqual(errors, [], errors.join("; "));
      return { faction: label, status: "pass", steps };
    }
    const d01 = await a0("d-01-hancb1-finder-luke-skywalker-dagobah-own-row-han-captured-sluis-van-d368.png");
    const rows = rowInk(d01, listed, 0, grey);
    steps.push({ label: "rows-vs-d-01", rows });
    assert.ok(rows.every(({ near: ok }) => ok), JSON.stringify(rows));

    // d-02: a click on Luke's row (the sixth) whitens it and puts his name
    // alone in the box.
    await click(page, [140, 303]);
    const chosen = await until(page, "Luke's row is chosen", (o) => o.chosen === 5);
    steps.push({ label: "chosen", chosen: chosen.chosen, name: chosen.name });
    assert.equal(chosen.name, "Luke Skywalker", JSON.stringify(chosen));
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const picked = await grab("row-chosen");
    const d02 = await a0("d-02-hancb1-finder-luke-dagobah-row-selected-d368.png");
    // A0's pointer lies over the row from x 173: its left part.
    const lukeArea = [100, 294, 172, 314];
    const luke = { a0: ink(d02, lukeArea, white), port: ink(picked, lukeArea, white),
      a0_grey: ink(d02, lukeArea, grey), port_grey: ink(picked, lukeArea, grey) };
    const boxArea = [206, 97, 445, 111];
    const nameBox = { a0: ink(d02, boxArea, white), port: ink(picked, boxArea, white) };
    const others = rowInk(d02, picked, 0, grey, [5]);
    steps.push({ label: "chosen-vs-d-02", luke, name_box: nameBox, others });
    assert.ok(near(luke.a0, luke.port), JSON.stringify(luke));
    // No grey row text: A0 has none; the port's grey, if any, is the white
    // glyphs' anti-aliased edge, inside their ink.
    assert.equal(luke.a0_grey, null, JSON.stringify(luke));
    assert.ok(!luke.port_grey || (luke.port_grey[0] >= luke.port[0] && luke.port_grey[1] >= luke.port[1]
      && luke.port_grey[2] <= luke.port[2] && luke.port_grey[3] <= luke.port[3]), JSON.stringify(luke));
    // The box is a CoolStringField in the default font 4 (FUN_005ff910,
    // FUN_00603850), drawn white over a black shadow from its left
    // (FUN_00605160, FUN_00600fd0): its ink against A0's.
    assert.ok(near(nameBox.a0, nameBox.port), JSON.stringify(nameBox));
    assert.ok(others.every(({ near: ok }) => ok), JSON.stringify(others));

    // d-03, d-04: a right release on the chosen row: Encyclopedia and
    // Status, both grey; A0's menu frame is 90 by 42 (x 170..259, y
    // 302..343).
    await page.mouse.move(172, 304);
    await frames(page);
    await page.mouse.down({ button: "right" });
    await frames(page);
    await page.mouse.up({ button: "right" });
    await frames(page, 4);
    const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu ?? null);
    assert.ok(menu, "the right release opened no menu");
    steps.push({ label: "menu", menu: { left: menu.left, top: menu.top, width: menu.width,
      height: menu.height, row_states: menu.row_states }, a0: { left: 170, top: 302, width: 90, height: 42 } });
    assert.deepEqual(menu.row_states, [["Encyclopedia", false], ["Status", false]], JSON.stringify(menu));
    assert.equal(menu.height, 42, JSON.stringify(menu));
    await page.mouse.move(2, 470);
    await frames(page, 4);
    await grab("menu");
    await page.keyboard.press("Escape");
    await frames(page, 4);
    await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = null; window.__openRebellionInterfacePersonnelFinder = null; });
    await frames(page, 4);
    const still = await until(page, "the Finder stays open after the menu", (o) => o.rows.length > 0);
    assert.equal(still.chosen, 5, JSON.stringify(still));

    // b-07: Display closes the Finder and opens nothing: the screen is
    // the one before F5.
    await click(page, [500, 158]);
    await page.evaluate(() => { window.__openRebellionInterfacePersonnelFinder = null; });
    await page.mouse.move(2, 470);
    await frames(page, 8);
    assert.equal(await latest(page), null, "the Finder stayed open after Display");
    const after = await grab("after-display");
    let changed = 0;
    for (let i = 0; i < before.data.length; i += 4) {
      if ([0, 1, 2].some((c) => before.data[i + c] !== after.data[i + c])) changed += 1;
    }
    steps.push({ label: "display-vs-b-07", changed_pixels: changed });
    assert.equal(changed, 0, `Display changed ${changed} pixels against the screen before F5`);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: label, status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: label, status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C018: the PersonnelPalace fixture (code 145) against A0
// 2026-11-18-edited-saves-7 b-19 (the original game, not committed;
// e4f53d49b065, day 31). The Alliance's copies of Chewbacca, Han, Leia and
// Luke stay in its copy of Jabba's Palace after the Pickup took them to
// Coruscant (Character_SetCaptured FUN_004ee800 syncs the Captured bit,
// then FUN_004eec60 gives them the Empire's side and FUN_004f9a60 freezes
// the copies), and an abode container reads its name alone
// (FUN_00465bb0): "Luke Skywalker - Jabba's Palace". Each row's grey ink
// against A0's; no row has an icon (the ink starts at the text's left in
// both). Display on Luke's row closes the Finder and opens nothing
// (FUN_00429440's abode branch; FUN_00464890 case 0xc9 closes).
const PERSONNEL_PALACE = 145;
const PALACE_A0 = ".artifacts/a0-vm/2026-11-18-edited-saves-7/"
  + "b-19-bntyd7-finder-alliance-luke-han-leia-chewbacca-jabbas-palace-d31.png";
const PALACE_ROWS = ["Chewbacca - Jabba's Palace", "Han Solo - Jabba's Palace", "Huoba Neva - Corsin",
  "Jan Dodonna - Yavin", "Leia Organa - Jabba's Palace", "Luke Skywalker - Jabba's Palace",
  "Mon Mothma - Chazwa", "Wedge Antilles - Yavin"];

async function runPalace(browser, base) {
  const label = "alliance-palace";
  const directory = path.join(OUT, label);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const grab = async (name) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${name}.png`) });
    steps.push({ label: name, sha256: createHash("sha256").update(bytes).digest("hex") });
    return PNG.sync.read(bytes);
  };
  // FUN_00465540's grey (A0 (117, 114, 114), the palette step of 0x787878).
  const grey = (png, i) => png.data[i] >= 100 && png.data[i] <= 200 && Math.abs(png.data[i] - png.data[i + 2]) <= 20;
  const ink = (png, [x0, y0, x1, y1], test) => {
    let box = null;
    let count = 0;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(png, (y * png.width + x) * 4)) continue;
      count += 1;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box && { box, count };
  };
  // As runAbode: left within a pixel, top and bottom within two (the
  // accepted text deviation: the port's glyphs are 8 rows to A0's 10 and
  // narrower, so the right edge is reported, not held).
  const near = (a, p) => Boolean(a && p) && Math.abs(a.box[0] - p.box[0]) <= 1
    && Math.abs(a.box[1] - p.box[1]) <= 2 && Math.abs(a.box[3] - p.box[3]) <= 2;
  try {
    const code = PERSONNEL_PALACE | (1 << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the PersonnelPalace fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const before = await grab("before-finder");
    await page.keyboard.press("F5");
    const shown = await until(page, "F5 opens the Alliance tab", (o) => o.tab === "alliance" && o.rows.length > 0);
    steps.push({ label: "rows", rows: shown.rows, chosen: shown.chosen, name: shown.name });
    assert.deepEqual(shown.rows, PALACE_ROWS, JSON.stringify(shown.rows));
    await frames(page, 4);
    const listed = await grab("rows");
    const ref = PNG.sync.read(await fs.readFile(path.join(ROOT, PALACE_A0)));
    // Rows 20 high from y 194, x from 100 (b-19 has d-01's geometry).
    const rows = [...Array(8).keys()].map((row) => {
      const area = [100, 194 + 20 * row, 440, 214 + 20 * row];
      const box = { row, text: PALACE_ROWS[row], a0: ink(ref, area, grey), port: ink(listed, area, grey) };
      return { ...box, near: near(box.a0, box.port) };
    });
    steps.push({ label: "rows-vs-b-19", rows });
    assert.ok(rows.every(({ near: ok }) => ok), JSON.stringify(rows));
    // No row icon: nothing grey between the list's left edge and the
    // text, in A0 or the port.
    const icons = [...Array(8).keys()].map((row) => {
      const area = [100, 194 + 20 * row, 102, 214 + 20 * row];
      return { row, a0: ink(ref, area, grey), port: ink(listed, area, grey) };
    });
    steps.push({ label: "row-icons-vs-b-19", icons });
    assert.ok(icons.every(({ a0, port }) => !a0 && !port), JSON.stringify(icons));
    // The row band's pixels: reported, not held (the list shows the map
    // behind it, which the fixture's galaxy does not share with A0's).
    let differing = 0;
    let total = 0;
    for (let y = 194; y < 354; y += 1) for (let x = 100; x < 440; x += 1) {
      const i = (y * 640 + x) * 4;
      total += 1;
      if ([0, 1, 2].some((c) => Math.abs(ref.data[i + c] - listed.data[i + c]) > 8)) differing += 1;
    }
    steps.push({ label: "row-band-pixels-vs-b-19", differing, total });

    // Display on Luke's row (the sixth): nothing opens.
    await click(page, [140, 303]);
    const chosen = await until(page, "Luke's row is chosen", (o) => o.chosen === 5);
    assert.equal(chosen.name, "Luke Skywalker", JSON.stringify(chosen));
    await page.mouse.move(2, 470);
    await frames(page, 4);
    await grab("row-chosen");
    await click(page, [500, 158]);
    await page.waitForTimeout(300);
    await page.evaluate(() => { window.__openRebellionInterfacePersonnelFinder = null; });
    await page.mouse.move(2, 470);
    await frames(page, 8);
    assert.equal(await latest(page), null, "the Finder stayed open after Display");
    const after = await grab("after-display");
    let changed = 0;
    for (let i = 0; i < before.data.length; i += 4) {
      if ([0, 1, 2].some((c) => before.data[i + c] !== after.data[i + c])) changed += 1;
    }
    steps.push({ label: "display-opens-nothing", changed_pixels: changed });
    assert.equal(changed, 0, `Display changed ${changed} pixels against the screen before F5`);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: label, status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: label, status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

// OBJ-03-C011: the PersonnelRetired fixture (code 146) against A0
// 2026-11-18-edited-saves-9 (an unedited Empire game, day 132; not
// committed, sha256 prefixes cited). d-03 (b9ea91b80dbb): the Imperial tab
// lists "Ozzel - Retired" sixth (FUN_00465bb0, destroyed with a Retire's
// reason, 0x8799); each row whose text the fixture shares with A0 is held
// to A0's grey ink. d-08 (2f1d872c5a2e): a chosen Ozzel row is white and
// the name box stays empty (FUN_00464890 case 0xca/0x29b copies a name
// only for an existing object, +0x50 bit 6). d-04 (5b7a43b193d5): the
// row's menu, Encyclopedia and Status both grey, 90 wide. d-05..d-07
// (d97755577429, caa8d80955c3, 7186ada10060): a click on either grey item
// opens nothing and the menu stays open (FUN_004aba60).
const PERSONNEL_RETIRED = 146;
const RETIRED_A0 = ".artifacts/a0-vm/2026-11-18-edited-saves-9";
const RETIRED_ROWS = ["Darth Vader - Yavin ( On Mission ) ", "Emperor Palpatine - Coruscant",
  "Jerjerrod - Coruscant", "Menndo - Corfai", "Needa - Coruscant", "Ozzel - Retired",
  "Piett - Coruscant", "Veers - Coruscant"];
// The rows whose text d-03 shares; Jerjerrod, Needa and Piett ride ships
// in A0 ("Victory Destroyer 1", "Galleon 1 ( Enroute )", "Galleon 2").
const RETIRED_SHARED = [0, 1, 3, 5, 7];

async function runRetired(browser, base) {
  const label = "empire-retired";
  const directory = path.join(OUT, label);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(ROOT, RETIRED_A0, file)));
  const grab = async (name) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${name}.png`) });
    steps.push({ label: name, sha256: createHash("sha256").update(bytes).digest("hex") });
    return PNG.sync.read(bytes);
  };
  // FUN_00465540's grey (A0 (117, 114, 114)) and the chosen row's white.
  const grey = (png, i) => png.data[i] >= 100 && png.data[i] <= 200 && Math.abs(png.data[i] - png.data[i + 2]) <= 20;
  const white = (png, i) => Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) > 200;
  const ink = (png, [x0, y0, x1, y1], test) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(png, (y * png.width + x) * 4)) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  // As runAbode: left within a pixel, top and bottom within two (the
  // accepted text deviation; the right edge is reported, not held).
  const near = (a, p) => Boolean(a && p) && Math.abs(a[0] - p[0]) <= 1
    && Math.abs(a[1] - p[1]) <= 2 && Math.abs(a[3] - p[3]) <= 2;
  // The Empire's list: rows 20 high from y 194, x from 163 (A0 d-03: ink
  // from x 167, tops 201..341).
  const rowArea = (row) => [163, 194 + 20 * row, 503, 214 + 20 * row];
  const menuFrames = () => page.evaluate(() => window.__openRebellionInterfaceObjectMenu ?? null);
  try {
    await page.goto(`${base}?fixture-code=${PERSONNEL_RETIRED | (2 << 8)}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the PersonnelRetired fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.mouse.move(2, 470);
    await frames(page, 4);
    await page.keyboard.press("F5");
    const shown = await until(page, "F5 opens the Imperial tab", (o) => o.tab === "imperial" && o.rows.length > 0);
    steps.push({ label: "rows", rows: shown.rows, chosen: shown.chosen, name: shown.name });
    assert.deepEqual(shown.rows, RETIRED_ROWS, JSON.stringify(shown.rows));
    assert.deepEqual([shown.chosen, shown.name], [null, ""], JSON.stringify(shown));
    await frames(page, 4);
    const listed = await grab("rows");
    const d03 = await a0("d-03-slot5-unedited-finder-imperial-ozzel-retired-vader-on-mission-d132.png");
    const rows = RETIRED_SHARED.map((row) => {
      const box = { row, text: RETIRED_ROWS[row], a0: ink(d03, rowArea(row), grey), port: ink(listed, rowArea(row), grey) };
      return { ...box, near: near(box.a0, box.port) };
    });
    steps.push({ label: "rows-vs-d-03", rows });
    assert.ok(rows.every(({ near: ok }) => ok), JSON.stringify(rows));

    // d-08: a click on Ozzel's row whitens it; the name box stays empty.
    await click(page, [200, 303]);
    await page.waitForTimeout(300);
    const chosen = await until(page, "Ozzel's row is chosen", (o) => o.chosen === 5);
    steps.push({ label: "chosen", chosen: chosen.chosen, name: chosen.name });
    assert.equal(chosen.name, "", JSON.stringify(chosen));
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const picked = await grab("row-chosen");
    const d08 = await a0("d-08-slot5-unedited-finder-ozzel-retired-row-chosen-d133.png");
    const ozzel = { a0: ink(d08, rowArea(5), white), port: ink(picked, rowArea(5), white) };
    // Inside the name box (x 269..508 below its white top edge, y 97, on
    // the Empire's window): no white ink.
    const boxArea = [269, 98, 508, 111];
    const nameBox = { a0: ink(d08, boxArea, white), port: ink(picked, boxArea, white) };
    steps.push({ label: "chosen-vs-d-08", ozzel, name_box: nameBox });
    assert.ok(near(ozzel.a0, ozzel.port), JSON.stringify(ozzel));
    assert.deepEqual([nameBox.a0, nameBox.port], [null, null], JSON.stringify(nameBox));

    // d-04: a right release on the row opens Encyclopedia and Status, both
    // grey, in a frame 90 by 42.
    await page.mouse.move(215, 305);
    await frames(page);
    await page.mouse.down({ button: "right" });
    await frames(page);
    await page.mouse.up({ button: "right" });
    await page.waitForTimeout(300);
    await frames(page, 4);
    const menu = await menuFrames();
    assert.ok(menu, "the right release opened no menu");
    steps.push({ label: "menu", menu: { left: menu.left, top: menu.top, width: menu.width,
      height: menu.height, row_states: menu.row_states } });
    assert.deepEqual(menu.row_states, [["Encyclopedia", false], ["Status", false]], JSON.stringify(menu));
    assert.deepEqual([menu.width, menu.height], [90, 42], JSON.stringify(menu));
    await page.mouse.move(2, 470);
    await frames(page, 4);
    const opened = await grab("menu");

    // d-05..d-07: clicks on the grey Encyclopedia (twice) and Status
    // choose nothing; the menu stays and the screen does not change.
    const items = [["encyclopedia", menu.top + 11], ["encyclopedia-again", menu.top + 11], ["status", menu.top + 31]];
    for (const [name, y] of items) {
      await click(page, [menu.left + 30, y]);
      await page.waitForTimeout(300);
      await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = null; });
      await page.mouse.move(2, 470);
      await frames(page, 6);
      const still = await menuFrames();
      const finder = await until(page, `the Finder stays after ${name}`, (o) => o.rows.length > 0);
      const after = await grab(`after-${name}`);
      let changed = 0;
      for (let i = 0; i < opened.data.length; i += 4) {
        if ([0, 1, 2].some((c) => opened.data[i + c] !== after.data[i + c])) changed += 1;
      }
      steps.push({ label: `click-${name}`, menu_open: Boolean(still), chosen: finder.chosen,
        name: finder.name, changed_pixels: changed });
      assert.ok(still, `the menu closed after the grey ${name}`);
      assert.deepEqual(still.row_states, menu.row_states, JSON.stringify(still));
      assert.deepEqual([finder.chosen, finder.name], [5, ""], JSON.stringify(finder));
      assert.equal(changed, 0, `the grey ${name} changed ${changed} pixels`);
    }

    // d-08: a click outside the menu closes it; Ozzel stays chosen.
    await click(page, [340, 80]);
    await page.waitForTimeout(300);
    await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = null; });
    await page.mouse.move(2, 470);
    await frames(page, 6);
    assert.equal(await menuFrames(), null, "the menu stayed open after a click outside it");
    const closed = await until(page, "the Finder stays after the menu closes", (o) => o.rows.length > 0);
    steps.push({ label: "closed", chosen: closed.chosen, name: closed.name });
    assert.deepEqual([closed.chosen, closed.name], [5, ""], JSON.stringify(closed));
    const last = await grab("menu-closed");
    let changed = 0;
    for (let i = 0; i < picked.data.length; i += 4) {
      if ([0, 1, 2].some((c) => picked.data[i + c] !== last.data[i + c])) changed += 1;
    }
    steps.push({ label: "closed-vs-chosen", changed_pixels: changed });
    assert.equal(changed, 0, `closing the menu left ${changed} pixels changed`);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: label, status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: label, status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C018: the PersonnelPickup fixture (code 147) against A0
// 2026-11-18-edited-saves-9 c-14 (88e1928e1f1a; an Empire game from an
// edited save, day 531, right after "Jabba Delivers Prisoners"; not
// committed). The Empire's Alliance tab: Han, Leia and Luke aboard the ship
// Vader joined the Pickup from, "Name - Victory Destroyer 1 ( Captured )"
// (FUN_0056da80, FUN_00465bb0, 0x8746), read live; Chewbacca, destroyed
// (FUN_005445d0 accepts him destroyed), and four others from the Empire's
// frozen copies: "Chewbacca - Yavin", "Jan Dodonna - Location Unknown"
// (0x1895), "Ma'w'shiye - Yavin ( Enroute )" (0x8744). Each row is held to
// A0's grey ink as the retired case holds d-03's; no row icon in either.
const PERSONNEL_PICKUP = 147;
const PICKUP_A0 = ".artifacts/a0-vm/2026-11-18-edited-saves-9/"
  + "c-14-e508b512-finder-alliance-han-leia-luke-victory-destroyer-1-captured-chewbacca-yavin-d531.png";
const PICKUP_ROWS = ["Chewbacca - Yavin", "Han Solo - Victory Destroyer 1 ( Captured ) ",
  "Jan Dodonna - Location Unknown", "Leia Organa - Victory Destroyer 1 ( Captured ) ",
  "Luke Skywalker - Victory Destroyer 1 ( Captured ) ", "Ma'w'shiye - Yavin ( Enroute ) ",
  "Tura Raftican - Yavin", "Wedge Antilles - Yavin"];

async function runPickup(browser, base) {
  const label = "empire-pickup";
  const directory = path.join(OUT, label);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  const steps = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const grab = async (name) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${name}.png`) });
    steps.push({ label: name, sha256: createHash("sha256").update(bytes).digest("hex") });
    return PNG.sync.read(bytes);
  };
  // FUN_00465540's grey (A0 (117, 114, 114)).
  const grey = (png, i) => png.data[i] >= 100 && png.data[i] <= 200 && Math.abs(png.data[i] - png.data[i + 2]) <= 20;
  // This case's ink, for A0 and the port alike: the grey text and the
  // strokes the port's 8-row glyphs blend into the list's blue when drawn
  // 10 rows high ("Tura Raftican"'s T bar reads (76, 82, 112) at x 167,
  // A0's grey). On c-14 it finds the same boxes as the grey test. The
  // other cases keep the grey test.
  const blended = (png, i) => png.data[i] >= 60 && png.data[i] <= 200 && png.data[i + 2] - png.data[i] <= 40;
  const ink = (png, [x0, y0, x1, y1], test) => {
    let box = null;
    for (let y = y0; y < y1; y += 1) for (let x = x0; x < x1; x += 1) {
      if (!test(png, (y * png.width + x) * 4)) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
    return box;
  };
  // As runRetired: left within a pixel, top and bottom within two (the
  // accepted text deviation; the right edge is reported, not held).
  const near = (a, p) => Boolean(a && p) && Math.abs(a[0] - p[0]) <= 1
    && Math.abs(a[1] - p[1]) <= 2 && Math.abs(a[3] - p[3]) <= 2;
  // The Empire's list: rows 20 high from y 194, x from 163.
  const rowArea = (row) => [163, 194 + 20 * row, 503, 214 + 20 * row];
  try {
    await page.goto(`${base}?fixture-code=${PERSONNEL_PICKUP | (2 << 8)}`);
    const deadline = Date.now() + 30_000;
    while ((await ready(page))?.status !== "ready") {
      if (Date.now() > deadline) throw new Error("the PersonnelPickup fixture never reported");
      await page.waitForTimeout(100);
    }
    await frames(page, 6);
    await page.locator("#glcanvas").focus();
    await page.mouse.move(2, 470);
    await frames(page, 4);
    await page.keyboard.press("F5");
    const own = await until(page, "F5 opens the Imperial tab", (o) => o.tab === "imperial");
    await click(page, own.tabs[0]);
    await page.waitForTimeout(300);
    await page.mouse.move(2, 470);
    const shown = await until(page, "the Alliance tab shows", (o) => o.tab === "alliance" && o.rows.length > 0);
    steps.push({ label: "rows", rows: shown.rows, chosen: shown.chosen, name: shown.name });
    assert.deepEqual(shown.rows, PICKUP_ROWS, JSON.stringify(shown.rows));
    await frames(page, 4);
    const listed = await grab("rows");
    const ref = PNG.sync.read(await fs.readFile(path.join(ROOT, PICKUP_A0)));
    const rows = PICKUP_ROWS.map((text, row) => {
      const box = { row, text, a0: ink(ref, rowArea(row), blended), port: ink(listed, rowArea(row), blended) };
      return { ...box, near: near(box.a0, box.port) };
    });
    steps.push({ label: "rows-vs-c-14", rows });
    assert.ok(rows.every(({ near: ok }) => ok), JSON.stringify(rows));
    // No row icon: nothing grey between the list's left edge and the text.
    const icons = PICKUP_ROWS.map((_, row) => {
      const area = [163, 194 + 20 * row, 165, 214 + 20 * row];
      return { row, a0: ink(ref, area, grey), port: ink(listed, area, grey) };
    });
    steps.push({ label: "row-icons-vs-c-14", icons });
    assert.ok(icons.every(({ a0, port }) => !a0 && !port), JSON.stringify(icons));
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: label, status: "pass", steps };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: label, status: "fail", error: String(error.stack || error), steps, errors };
  } finally {
    await context.close();
  }
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(SITE);
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const results = [];
try {
  if (args.only === "retired") {
    results.push(await runRetired(browser, base));
  } else if (args.only === "pickup") {
    results.push(await runPickup(browser, base));
  } else {
  if (args.only !== "abode" && args.only !== "palace") {
    if (args.only !== "specforces" && args.only !== "copied") {
      for (const faction of FACTIONS) results.push(await run(browser, base, faction));
    }
    if (args.only !== "copied" && (!args.faction || args.faction === "alliance")) {
      results.push(await runSpecForces(browser, base));
    }
    for (const faction of FACTIONS) results.push(await runCopied(browser, base, faction));
  }
  if (args.only !== "palace") {
    for (const faction of FACTIONS) results.push(await runAbode(browser, base, faction));
  }
  if (!args.faction || args.faction === "alliance") results.push(await runPalace(browser, base));
  if (args.only !== "palace" && (!args.faction || args.faction === "empire")) {
    results.push(await runRetired(browser, base));
    results.push(await runPickup(browser, base));
  }
  }
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion-test.wasm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, status, error, missing }) => ({ faction, status, error, missing })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
