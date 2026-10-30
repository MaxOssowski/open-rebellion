// The agent's Build orders in the browser build
// (ghidra/notes/agent-build-orders.md).
//
// Serves the interface-fixture site and opens the Build Selection fixture
// (code 55) for each faction: the primary system holds an idle shipyard,
// training facility and construction yard, and its Manufacturing window is
// open. Alt+T (0xbc2 -> 0x111, FUN_00422ce0) starts Build Troops' target
// cursor; a click on the system window names the destination
// (FUN_0049e380). The agent takes the nearest idle training facility, the
// system's own, and opens Build Selection on it (FUN_0049e6b0). Confirm
// queues the regiment and earns the agent's acknowledgement (FUN_0043a2a0).
// A second Build Troops finds the facility busy and is refused
// (FUN_0049e7a0: {0xa0, 3}).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node agent-build.mjs [--out=DIR] [--faction=alliance|empire]
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
  `agent-build-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one, the side in the next byte
// (interface_test_fixture.rs): 54 BuildSelection.
const BUILD_SELECTION = 55;
const FACTIONS = [
  { name: "alliance", byte: 1 },
  { name: "empire", byte: 2 },
].filter(({ name }) => !args.faction || args.faction === name);
// Confirm (0x65) at (73, 224), 66 by 33 (FUN_00438620).
const CONFIRM = [73 + 33, 224 + 16];
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

async function altKey(page, key) {
  await page.keyboard.down("Alt");
  await frames(page);
  await page.keyboard.down(key);
  await frames(page);
  await page.keyboard.up(key);
  await page.keyboard.up("Alt");
  await frames(page);
}

const production = (page) => page.evaluate(() => window.__openRebellionInterfaceProduction);

async function waitFor(page, predicate, description) {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const value = await predicate();
    if (value) return value;
    if (Date.now() > deadline) throw new Error(description);
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
  const lines = (needle) => log.filter((line) => line.includes(needle));
  const troops = async () => (await production(page)).bands.find(({ area }) => area === "troops");
  try {
    await page.goto(`${base}?fixture-code=${BUILD_SELECTION | (faction.byte << 8)}`);
    await waitFor(page, async () => (await production(page))?.bands?.length === 3,
      "the fixture reports its three bands");
    const band = await troops();
    assert.equal(band.queued, 0, "the training facility starts idle");
    const inside = [band.left + band.width / 2, band.top + band.height / 2];

    // Alt+T: the target cursor, then the destination.
    await altKey(page, "KeyT");
    await waitFor(page, () => lines("[agent] command=build kind=Troops status=targeting").length === 1,
      "Alt+T starts the target cursor");
    await shot("targeting");
    await click(page, inside);
    const opened = await waitFor(page, async () => {
      const value = await production(page);
      return value.build_selection_open ? value : null;
    }, "the agent opens Build Selection");
    const openedLine = lines("[agent] build kind=Troops status=opened");
    assert.equal(openedLine.length, 1, JSON.stringify(log.slice(-6)));
    const [, producer, destination] = openedLine[0].match(/producer=(.*) destination=(.*)$/);
    assert.equal(producer, destination, "the system's own facility builds for it");
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("agent-build-selection");

    // Confirm: the regiment is queued and the agent acknowledges it.
    await click(page, [opened.build_selection_left + CONFIRM[0], opened.build_selection_top + CONFIRM[1]]);
    await waitFor(page, () => lines("[agent] build status=confirmed").length === 1,
      "Confirm reaches the agent");
    await waitFor(page, async () => (await troops()).queued === 1, "the regiment is queued");
    assert.equal((await production(page)).build_selection_open, false);

    // Again: the facility is busy now, so the agent refuses.
    await altKey(page, "KeyT");
    await waitFor(page, () => lines("[agent] command=build kind=Troops status=targeting").length === 2,
      "Alt+T starts the target cursor again");
    await click(page, inside);
    await waitFor(page, () => lines("[agent] build kind=Troops status=refused reason=NoIdleProducer").length === 1,
      "the agent refuses without an idle facility");
    await frames(page, 6);
    assert.equal((await production(page)).build_selection_open, false);
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("refused");
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", producer, agent: lines("[agent]"), shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors,
      agent: lines("[agent]") };
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
