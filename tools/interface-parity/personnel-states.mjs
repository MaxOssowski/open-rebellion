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
  { name: "alliance", byte: 1 },
  { name: "empire", byte: 2 },
].filter(({ name }) => !args.faction || args.faction === name);
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
    await until(page, "the Finder stays open after the menu's Escape", (o) => o.rows.length > 0);

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
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", busy: busy[0], travelling: travelling[0],
      commanding: commanding[0], available, rows: shown.rows,
      menu: { left: menu.left, top: menu.top, width: menu.width, height: menu.height, rows: menu.row_states },
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
