// The opening briefing in the browser build (ghidra/notes/advisor-briefing.md).
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480, and follows the advisor's console lines:
//
// - "full": the briefing plays to its end. Its map actions arrive in the
//   order of message 0x29 (C3POACT.SPT) / 0x4a (IMP22ACT.SPT), each voice
//   is the briefing DLL's, and a screenshot is taken after each map action
//   for comparison with the SquakeNet narration frames;
// - "skip": a click while the first voice plays skips to the short form
//   (0x28 / 0x49: actions 0xb and 0xd around one clip).
//
// Build first: scripts/build-wasm.sh (with the briefing DLLs staged).
// Run:         node briefing.mjs [--out=DIR] [--faction=alliance|empire] [--case=full|skip]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const WEB = path.join(ROOT, "web");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `briefing-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// The main menu's entries (galaxy-map-visual-check.mjs).
const FACTIONS = [
  { name: "alliance", menuIndex: 9, directory: "albrief-dll", shortWave: 1165 },
  { name: "empire", menuIndex: 8, directory: "embrief-dll", shortWave: 1157 },
].filter(({ name }) => !args.faction || args.faction === name);
const CASES = ["full", "skip"].filter((name) => !args.case || args.case === name);
// Message 0x29 / 0x4a: the actions (kind 2) in order (advisor-briefing.md).
const FULL_ACTIONS = [0xc, 0x1, 0xe, 0xf, 0x10, 0x11, 0x12, 0x3, 0x4, 0x5, 0x6, 0x7, 0x13, 0x14,
  0x9, 0xa, 0xb, 0xd];
// The voices, in order, of each side's sixteen clips.
const FULL_WAVES = {
  alliance: [1155, 1220, 1221, 1222, 1223, 1224, 1157, 1158, 1159, 1160, 1161, 1225, 1226, 1163,
    1164, 1168],
  empire: [1147, 1220, 1221, 1222, 1223, 1224, 1149, 1150, 1151, 1152, 1153, 1225, 1226, 1155,
    1156, 1160],
};
// The marking actions and where each mark must land. Codes from
// FUN_004c30c0 (Alliance) and FUN_004c0fc0 (Empire); the seed places Luke at
// Yavin, Mon Mothma at the headquarters, and Palpatine and Vader at
// Coruscant (crates/rebellion-data/src/seeds.rs), so their marks repeat
// those systems'.
const MARKS = {
  alliance: { 0x12: "yavin", 0x3: "headquarters", 0x4: "coruscant", 0x7: "headquarters",
    0x13: "yavin", 0x14: "coruscant" },
  empire: { 0x12: "coruscant", 0x3: "yavin", 0x7: "coruscant", 0x13: "coruscant", 0x14: "yavin" },
};
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".wav": "audio/wav", ".bmp": "image/bmp" };

function serve(root) {
  const server = http.createServer(async (req, res) => {
    const pathname = decodeURIComponent(new URL(req.url, "http://x").pathname);
    const file = path.join(root, pathname.endsWith("/") ? `${pathname}index.html` : pathname);
    if (!file.startsWith(root)) { res.writeHead(403).end(); return; }
    try {
      const body = await fs.readFile(file);
      res.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      res.end(body);
    } catch { res.writeHead(404).end(); }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

const actionPattern = /\[advisor\] clip finished kind=2 code=(0x[0-9a-f]+)/;
const markPattern = /\[advisor\] clip finished kind=2 code=(0x[0-9a-f]+) .* marked=(\S+)/;
const voicePattern = /\[advisor\] clip voice directory=(\S+) wave=(\d+) seconds=([\d.]+)/;

async function waitFor(page, log, description, predicate, timeout) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error(`${description}: last lines ${JSON.stringify(log.slice(-8))}`);
    }
    await page.waitForTimeout(100);
  }
}

async function run(browser, base, faction, name) {
  const directory = path.join(OUT, `${faction.name}-${name}`);
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
  const actions = () => log.flatMap((line) => {
    const match = line.match(actionPattern);
    return match ? [Number(match[1])] : [];
  });
  const voices = () => log.flatMap((line) => {
    const match = line.match(voicePattern);
    return match ? [{ directory: match[1], wave: Number(match[2]), seconds: Number(match[3]) }] : [];
  });
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  let marks = {};
  try {
    await page.goto(base);
    await page.waitForSelector("#main-menu-semantics[data-active=true]",
      { state: "attached", timeout: 90_000 });
    await page.waitForTimeout(1_000);
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.menuIndex);
    await waitFor(page, log, "the new game starts",
      () => log.some((line) => line.includes("[campaign] faction=")), 60_000);
    await page.mouse.move(2, 2);
    await waitFor(page, log, "the briefing's first voice starts", () => voices().length > 0, 20_000);
    await shot("first-voice");
    if (name === "skip") {
      await page.mouse.click(320, 240);
      await waitFor(page, log, "the skip is asked",
        () => log.some((line) => line.includes("[advisor] briefing skip requested")), 5_000);
      await waitFor(page, log, "the short form ends", () => actions().at(-1) === 0xd, 60_000);
      await shot("after-short-form");
      assert.deepEqual(actions(), [0xc, 0xb, 0xd], "the short form follows the skip");
      assert.deepEqual(voices().map(({ wave }) => wave), [FULL_WAVES[faction.name][0], faction.shortWave]);
    } else {
      // A screenshot after each map action, for the narration frames.
      let seen = 0;
      const deadline = Date.now() + 300_000;
      while (actions().at(-1) !== 0xd) {
        if (Date.now() > deadline) throw new Error(`the briefing ends: ${JSON.stringify(log.slice(-8))}`);
        const done = actions();
        if (done.length > seen) {
          await page.waitForTimeout(200);
          for (; seen < done.length; seen += 1) {
            await shot(`action-${String(seen + 1).padStart(2, "0")}-0x${done[seen].toString(16)}`);
          }
        }
        await page.waitForTimeout(100);
      }
      assert.deepEqual(actions(), FULL_ACTIONS, "the briefing's actions in message order");
      assert.deepEqual(voices().map(({ wave }) => wave), FULL_WAVES[faction.name]);
      // Every mark lands on a system, the same one for the same place.
      const places = {};
      for (const line of log) {
        const match = line.match(markPattern);
        const place = match && MARKS[faction.name][Number(match[1])];
        if (!place) continue;
        assert.match(match[2], /^0x[0-9a-f]+$/, `code ${match[1]} marks a system`);
        places[place] ??= match[2];
        assert.equal(match[2], places[place], `code ${match[1]} marks ${place}`);
      }
      assert.notEqual(places.coruscant, places.yavin, JSON.stringify(places));
      marks = places;
      await shot("after-briefing");
    }
    assert.ok(voices().every(({ directory: dll }) => dll === faction.directory), JSON.stringify(voices()));
    assert.ok(voices().every(({ seconds }) => seconds > 0), "every voice has its length");
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400), JSON.stringify(network.filter(({ status }) => status >= 400)));
    return { faction: faction.name, case: name, status: "pass", actions: actions(), voices: voices(), marks,
      advisor: log.filter((line) => line.startsWith("[advisor]")), shots, network };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, case: name, status: "fail", error: String(error.stack || error),
      advisor: log.filter((line) => line.startsWith("[advisor]")), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(WEB);
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const results = [];
try {
  for (const faction of FACTIONS) {
    for (const name of CASES) results.push(await run(browser, base, faction, name));
  }
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(WEB, "open-rebellion.wasm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, case: name, status, error }) => ({ faction, name, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
