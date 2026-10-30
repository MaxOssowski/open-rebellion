// The Message Index's Display (0x67) in the browser build
// (ghidra/notes/message-index-rows.md, "Title, chosen category and rail art").
//
// Serves the interface-fixture site and opens the EventMessages fixture
// (code 57) for each faction: its "Fleet Arrives" message names a system.
// The Mission rail control opens the index. Choosing rows:
// a message without a target leaves Display disabled (FUN_004697b0), the
// arrival enables it, and a click goes to its system (FUN_00467f10 ->
// FUN_00429440: port, the system's sector window) and closes the index.
// While it shows, the GID control does nothing (the index is modal).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node message-display.mjs [--out=DIR] [--faction=alliance|empire]
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
  `message-display-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one, the side in the next byte
// (interface_test_fixture.rs): 56 EventMessages.
const EVENT_MESSAGES = 57;
// Display (0x67) on the rail (FUN_004665f0): Alliance (0x1a7, 0xc9) 32 by
// 31, Empire (0x1aa, 0xcf) 44 by 41.
const FACTIONS = [
  { name: "alliance", byte: 1, display: [0x1a7 + 16, 0xc9 + 15] },
  { name: "empire", byte: 2, display: [0x1aa + 22, 0xcf + 20] },
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

const report = (page) => page.evaluate(() => window.__openRebellionInterfaceReady);

async function waitReport(page, predicate, description) {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const value = await report(page);
    if (value?.status === "story-messages" && predicate(value)) return value;
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
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  try {
    await page.goto(`${base}?fixture-code=${EVENT_MESSAGES | (faction.byte << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Mission rail opens the index");
    const row = (index) => [opened.window_left + 25 + 60, opened.window_top + 108 + 21 * index + 10];
    const display = [opened.window_left + faction.display[0], opened.window_top + faction.display[1]];
    const displays = () => log.filter((line) => line.includes("[interface] message_index display"));
    // The index is modal (A0 a0-vm/modal2/18-msg-gid.png): the GID control
    // opens nothing and the index stays.
    await click(page, faction.byte === 1 ? [459, 414] : [537, 446]);
    await frames(page, 6);
    assert.equal(log.filter((line) => line.includes("destination=gid_menu status=opened")).length, 0,
      "the GID control opened its menu under the Message Index");
    assert.ok((await report(page)).index_open, "the GID click closed the index");
    // A message without a system: Display stays disabled.
    const research = opened.rows.findIndex((text) => text.startsWith("Research Complete"));
    assert.ok(research >= 0, JSON.stringify(opened.rows));
    await click(page, row(research));
    await click(page, display);
    await page.waitForTimeout(300);
    assert.equal(displays().length, 0, "Display is disabled without a target");
    await page.mouse.move(2, 2);
    await shot("display-disabled");
    // The arrival: Display goes to its system and closes the index.
    const arrival = opened.rows.findIndex((text) => text.startsWith("Fleet Arrives"));
    assert.ok(arrival >= 0, JSON.stringify(opened.rows));
    await click(page, row(arrival));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("display-enabled");
    await click(page, display);
    const closed = await waitReport(page, (value) => !value.index_open, "Display closes the index");
    assert.equal(displays().length, 1, JSON.stringify(log.slice(-6)));
    assert.match(displays()[0], /system=Some\(/);
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("after-display");
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", display: displays(), closed: closed.mode, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors };
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
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
