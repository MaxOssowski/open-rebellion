// The Planetary System Finder in the browser build
// (crates/rebellion-render/src/system_finder.rs, ghidra/notes/system-finder.md).
//
// Serves the interface-fixture site and opens the Fleet Finder fixture's
// galaxy (code 53) for each faction. F2 opens the Finder on All Systems
// with nothing chosen (FUN_00460270); a side tab chooses its first row
// (command 100, FUN_00609650); a typed name chooses the row it best begins
// and Enter opens that system's sector window and closes the Finder
// (FUN_00461000, FUN_00429ce0). The cockpit's control (0x12d) opens it too,
// and while it shows the GID control does nothing (the Finder is modal).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node system-finder.mjs [--out=DIR] [--faction=alliance|empire]
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
  `system-finder-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 52 FleetFinder.
const FLEET_FINDER = 53;
const FACTIONS = [
  { name: "alliance", byte: 1, gid: [446 + 13, 406 + 8] },
  { name: "empire", byte: 2, gid: [519 + 18, 434 + 12] },
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

const latest = (page) => page.evaluate(() => {
  const list = window.__openRebellionInterfaceSystemFinders || [];
  return list[list.length - 1] ?? null;
});

async function until(page, description, predicate) {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const value = await latest(page);
    if (value && predicate(value)) return value;
    if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
    await page.waitForTimeout(100);
  }
}

// FUN_00609650: the first row with the longest common prefix, ignoring case.
function prefixChoice(rows, text) {
  let best = null;
  rows.forEach((row, index) => {
    let common = 0;
    while (common < Math.min(text.length, row.length)
      && row[common].toLowerCase() === text[common].toLowerCase()) common += 1;
    if (best === null || common > best.common) best = { index, common };
  });
  return best?.index ?? null;
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
    await page.goto(`${base}?fixture-code=${FLEET_FINDER | (faction.byte << 8)}`);
    await until(page, "the fixture reports the closed Finder", (o) => !o.open);
    await page.locator("#glcanvas").focus();

    // F2 (0x71): All Systems, nothing chosen, every system by name.
    await page.keyboard.press("F2");
    const opened = await until(page, "F2 opens the System Finder", (o) => o.open);
    assert.equal(opened.tab, "All Systems");
    assert.equal(opened.chosen, null, "the opening chooses nothing");
    const sorted = [...opened.rows].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
    assert.deepEqual(opened.rows.map((r) => r.toLowerCase()), sorted.map((r) => r.toLowerCase()));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("all");

    // A side tab chooses its first row.
    await click(page, opened.tabs[1]);
    const rebel = await until(page, "the Rebel tab shows", (o) => o.tab === "Rebel Systems");
    assert.ok(rebel.rows.length > 0 && rebel.rows.length < opened.rows.length, JSON.stringify(rebel.rows));
    assert.equal(rebel.chosen, 0, "the tab's command chooses the first row");
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("rebel");
    // The other side tabs, for the chrome against A0 (each a first row).
    const others = [];
    for (const [index, heading, label] of [[2, "Imperial Systems", "imperial"],
      [3, "Neutral Systems", "neutral"], [4, "Unexplored Systems", "unexplored"]]) {
      await click(page, opened.tabs[index]);
      const shown = await until(page, `the ${label} tab shows`, (o) => o.tab === heading);
      assert.equal(shown.chosen, shown.rows.length > 0 ? 0 : null, `${label} chooses its first row`);
      others.push({ tab: label, rows: shown.rows.length });
      await page.mouse.move(2, 2);
      await frames(page);
      await shot(label);
    }

    // Back to All; a typed name chooses its best row; Enter opens its sector.
    await click(page, opened.tabs[0]);
    await until(page, "the All tab shows", (o) => o.tab === "All Systems");
    const text = opened.rows[Math.floor(opened.rows.length / 2)].slice(0, 2);
    await page.keyboard.type(text, { delay: 40 });
    const typed = await until(page, "the name box takes the text", (o) => o.name === text);
    assert.equal(typed.chosen, prefixChoice(typed.rows, text));
    const target = typed.rows[typed.chosen];
    await page.keyboard.press("Enter");
    const done = await until(page, "Enter opens the sector and closes the Finder", (o) => !o.open);
    assert.ok(done.sector_open.includes(target), `${target} not in ${JSON.stringify(done.sector_open)}`);
    assert.ok(log.some((line) => line.includes(`system_finder status=display system=${target}`)));
    await shot("sector");

    // The cockpit's control opens it too; while it shows, the GID control
    // does nothing; Escape closes it.
    await click(page, done.button);
    await until(page, "the cockpit control opens the Finder", (o) => o.open);
    await click(page, faction.gid);
    await frames(page, 6);
    assert.equal(log.filter((line) => line.includes("destination=gid_menu status=opened")).length, 0,
      "the GID control opened its menu under the Finder");
    await page.keyboard.down("Escape");
    await frames(page, 2);
    await page.keyboard.up("Escape");
    await until(page, "Escape closes the Finder", (o) => !o.open);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", rows: opened.rows.length, rebel: rebel.rows,
      others, typed: text, target, shots };
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
