// The advisor's events in the browser build (ghidra/notes/advisor-briefing.md,
// "The events").
//
// Serves the interface-fixture site and opens the ProductionDelivery
// fixture (code 75), which runs the clock at Fast, for each faction. A
// delivered product posts its Units Deployed and Unit Arrival messages
// (FUN_00497940, FUN_004981c0); their codes reach the player's advisor
// (FUN_0048a060 -> FUN_004c44b0 / FUN_004c22f0), which schedules an op-0x14
// message: the agent holds a frame while the second droid plays its clip
// (FUN_00411b80). The gate waits for the delivered code and the second
// droid's voice, and keeps a screenshot of the hold.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node advisor-events.mjs [--out=DIR] [--faction=alliance|empire]
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
  `advisor-events-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one, the side in the next byte
// (interface_test_fixture.rs).
const PRODUCTION_DELIVERY = 75;
// The second droid's clips are the sprite DLL's (slot 9); the agent's
// op-4 voices are too, so the gate tells them apart by the hold's records:
// ALSPRITE 10501..10531 name definitions 5001..5031 (WAVE 1501..1531),
// EMSPRITE 11501..11529 name 5501..5529 (WAVE 1801..1829).
const FACTIONS = [
  { name: "alliance", byte: 1, sprite: "alsprite-dll", waves: [1501, 1531] },
  { name: "empire", byte: 2, sprite: "emsprite-dll", waves: [1801, 1829] },
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

async function waitFor(page, log, description, predicate, timeout) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error(`${description}: last lines ${JSON.stringify(log.slice(-8))}`);
    }
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
  const network = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => network.push({ url: new URL(response.url()).pathname,
    status: response.status() }));
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  const holdVoice = (line) => {
    const match = line.match(new RegExp(`\\[advisor\\] clip voice directory=${faction.sprite} wave=(\\d+)`));
    return match && Number(match[1]) >= faction.waves[0] && Number(match[1]) <= faction.waves[1];
  };
  try {
    const code = PRODUCTION_DELIVERY | (faction.byte << 8);
    await page.goto(`${base}?fixture-code=${code}`);
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 60_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    await waitFor(page, log, "a delivered message hands the advisor its code",
      () => log.some((line) => line.includes("[advisor] reaction event code=")), 240_000);
    await waitFor(page, log, "the second droid plays the event's clip",
      () => log.some(holdVoice), 120_000);
    await page.waitForTimeout(400);
    await shot("hold");
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400),
      JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, status: "pass",
      advisor: log.filter((line) => line.startsWith("[advisor]")), shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error),
      advisor: log.filter((line) => line.startsWith("[advisor]")), errors };
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
