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
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node personnel-states.mjs [--out=DIR] [--faction=alliance|empire]
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
    // The Empire's fixture has only the three characters the states take.
    if (faction.name === "alliance") assert.ok(available.length > 0, JSON.stringify(shown.rows));
    // The traveller names its destination, not where it left from.
    assert.ok(!travelling[0].includes("Location Unknown"), travelling[0]);
    // A captive aboard names the ship it rides, numbered by its class, and
    // " ( Captured ) " (A0 personnel-captured/finder-day628: "Jan Dodonna -
    // Punisher ( Captured )"; fleet2/umgul-fleet1-ships: "Corellian
    // Corvette 1"). The Empire's fixture has no fourth character.
    const captured = shown.rows.filter((row) => / - .+ \( Captured \) $/.test(row));
    if (faction.name === "alliance") {
      assert.equal(captured.length, 1, JSON.stringify(shown.rows));
      assert.match(captured[0], / - [A-Za-z][A-Za-z -]* \d+ \( Captured \) $/, captured[0]);
      assert.ok(!/ - Fleet \d+ /.test(captured[0]), captured[0]);
    }
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
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", busy: busy[0], travelling: travelling[0],
      commanding: commanding[0], available, rows: shown.rows,
      menu: { left: menu.left, top: menu.top, width: menu.width, height: menu.height, rows: menu.row_states },
      topicSteps: topicSteps.map(({ topic, previous, next }) => [topic, previous, next]),
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
  for (const faction of FACTIONS) results.push(await run(browser, base, faction));
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
