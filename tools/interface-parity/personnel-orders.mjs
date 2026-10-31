#!/usr/bin/env node

// The Personnel orders through their original entries, both sides: a
// character's and a special force's Move and Retire from the agent's
// right-click menu in the Defenses window, and a mission icon's Abort in
// the sector window. Each order's result is read from the app's
// `[interface]` console lines; no artwork is compared.

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";
import { launchBrowser } from "./browser-launch.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const site = path.join(root, ".artifacts/interface-parity/site");
const browserManifest = JSON.parse(fs.readFileSync(path.join(here, "browser.json"), "utf8"));
const noBuild = process.argv.includes("--no-build");
// `--only=alliance/retire` runs the named cases alone.
const only = process.argv.filter((arg) => arg.startsWith("--only=")).map((arg) => arg.slice(7));
// `--a0=DIR` (the A0 tooltip captures) compares the pause alert with
// tooltip/321-resume-box-base.png.
const a0Dir = process.argv.find((arg) => arg.startsWith("--a0="))?.slice(5);
const expectedRequests = ["/", "/data/runtime.orpk", "/gl.js", "/open-rebellion-test.wasm"];
// Fixture codes are the Scenario index plus one (interface_test_fixture.rs).
const QUADRANTS = 50;
const RECON_TARGETING = 63;
const CHARACTER_COMMAND = 69;
const CHARACTER_MOVE = 70;
// `galaxy` is the galaxy view: the Defenses window opens 5 pixels in from
// its top-right corner (mission-dialog.mjs), and the order confirmation is
// centred in it (fleet-move.mjs).
const factions = [
  { name: "alliance", byte: 1, galaxy: { x: 55, y: 40, width: 485, height: 350 } },
  { name: "empire", byte: 2, galaxy: { x: 120, y: 40, width: 480, height: 355 } },
];
// FUN_00487cc0: the confirmation, 424 by 331, and its checkmark (0x14).
const confirmation = { width: 424, height: 331 };
const checkmark = { x: 355 + 25.5, y: 244 + 17.5 };
// FUN_0042d190 lists Pause, Very Slow, Slow, Medium and Fast.
const SPEED_ROWS = { pause: 0, medium: 3 };
const runId = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
const runDir = path.join(root, ".artifacts/interface-parity", `personnel-orders-${runId}`);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function browserExecutable() {
  const candidates = [
    ...(process.env.OPEN_REBELLION_CHROME_FOR_TESTING
      ? [process.env.OPEN_REBELLION_CHROME_FOR_TESTING]
      : []),
    ...browserManifest.executable_candidates,
  ];
  const executable = candidates.find((candidate) => fs.existsSync(candidate));
  if (!executable) throw new Error(`pinned Chrome for Testing ${browserManifest.version} is missing`);
  const version = spawnSync(executable, ["--version"], { encoding: "utf8" });
  if (version.status !== 0 || !version.stdout.includes(browserManifest.version)) {
    throw new Error(`Chrome for Testing version mismatch: ${version.stdout || version.stderr}`);
  }
  return executable;
}

function mimeType(file) {
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".wasm")) return "application/wasm";
  return "application/octet-stream";
}


// The original's tooltip (win_tooltip.rs): rest the pointer on a control for
// a second; a (255, 251, 240) tip with a (128, 128, 128) border, 19 high,
// shows at the pointer plus (0, 21).
async function tipShows(page, at) {
  await page.mouse.move(at.x, at.y, { steps: 4 });
  await page.waitForTimeout(1000);
  const png = PNG.sync.read(await page.screenshot());
  const pixel = (x, y) => {
    const i = (y * png.width + x) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]].join(",");
  };
  const top = at.y + 21;
  const shown = pixel(at.x, top) === "128,128,128" && pixel(at.x, top + 18) === "128,128,128"
    && pixel(at.x + 2, top + 2) === "255,251,240";
  await page.mouse.move(2, 2, { steps: 4 });
  await page.waitForTimeout(100);
  return shown;
}

async function startServer() {
  const server = http.createServer((request, response) => {
    const pathname = new URL(request.url, "http://localhost").pathname;
    const relative = path.posix.normalize(decodeURIComponent(pathname)).replace(/^\/+/, "")
      || "index.html";
    const candidate = path.resolve(site, relative);
    if (!candidate.startsWith(`${site}${path.sep}`)) {
      response.writeHead(403).end();
      return;
    }
    fs.readFile(candidate, (error, bytes) => {
      if (error) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, {
        "content-type": mimeType(candidate),
        "content-length": bytes.length,
        "cache-control": "no-store",
      }).end(bytes);
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return server;
}

async function frames(page, count = 1) {
  for (let index = 0; index < count; index += 1) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  }
}

async function click(page, point, button = "left") {
  // egui resolves a click across frames: hover, press, then release.
  await page.mouse.move(point.x, point.y);
  await frames(page);
  await page.mouse.down({ button });
  await frames(page);
  await page.mouse.up({ button });
  await frames(page);
}

function point([x, y]) {
  return { x: Math.round(x), y: Math.round(y) };
}

async function shot(page, directory, label) {
  const bytes = await page.screenshot({ animations: "disabled" });
  fs.writeFileSync(path.join(directory, `${label}.png`), bytes);
  return { label, sha256: sha256(bytes) };
}

// The agent's cell, the personnel page's first (70 by 70 at (7, 81),
// FUN_00609ae0), centred 42 by 116 into the 235-wide Defenses window.
function agentCell(faction) {
  const windowLeft = faction.galaxy.x + faction.galaxy.width - 235 - 5;
  return { x: windowLeft + 42, y: faction.galaxy.y + 5 + 116 };
}

function confirmationCheckmark(faction) {
  const { galaxy } = faction;
  return {
    x: Math.round(galaxy.x + galaxy.width / 2 - confirmation.width / 2 + checkmark.x),
    y: Math.round(galaxy.y + galaxy.height / 2 - confirmation.height / 2 + checkmark.y),
  };
}

async function openMenu(page, at, directory, label) {
  await page.evaluate(() => { window.__openRebellionInterfaceObjectMenu = undefined; });
  await click(page, at, "right");
  await page.waitForFunction(() => window.__openRebellionInterfaceObjectMenu?.status === "object-menu",
    null, { timeout: 10_000 });
  const menu = await page.evaluate(() => window.__openRebellionInterfaceObjectMenu);
  await shot(page, directory, `${label}-menu`);
  return menu;
}

// Rows below a 2-pixel border share one height: object menu entries carry
// no icon (object_menu.rs).
async function choose(page, menu, row, name) {
  assert.notEqual(row, null, `the menu lists ${name}: ${JSON.stringify(menu)}`);
  const height = (menu.height - 2) / menu.rows;
  await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (row + 0.5) });
}

// The release over the target system's planet in its sector window
// (FUN_00422ce0 targets on the release).
async function release(page, menu) {
  await page.mouse.move(Math.round(menu.target_screen_x), Math.round(menu.target_screen_y));
  await frames(page);
  await page.mouse.down();
  await frames(page);
  await page.mouse.up();
  await frames(page);
}

// The Game Speed menu on the day readout (FUN_0042d190); the browser has
// no keypad + or -.
async function setSpeed(page, setup, row) {
  await page.evaluate(() => { window.__openRebellionInterfaceSpeedMenu = undefined; });
  await click(page, point(setup.day_readout), "right");
  await page.waitForFunction(() => window.__openRebellionInterfaceSpeedMenu?.status === "speed-menu",
    null, { timeout: 10_000 });
  const menu = await page.evaluate(() => window.__openRebellionInterfaceSpeedMenu);
  const height = (menu.height - 2) / menu.rows;
  await click(page, { x: menu.left + menu.width / 2, y: menu.top + 2 + height * (row + 0.5) });
  return menu;
}

// The Retire order (0x242): FUN_00487740 validates, FUN_00487cc0 asks,
// and the checkmark fires event 0x230, which destroys the member with
// reason 0x14 (character_retire.rs).
async function retire(page, faction, directory, log, member) {
  const menu = await openMenu(page, agentCell(faction), directory, "agent");
  assert.equal(menu.retire_enabled, true, `Retire is enabled: ${JSON.stringify(menu)}`);
  await choose(page, menu, menu.retire_row, "Retire");
  const asked = await log.logged("the Retire validates", new RegExp(`command=0x242 member=${member}\\(.*\\) result=Some\\(true\\)$`));
  await shot(page, directory, "confirmation");
  await click(page, confirmationCheckmark(faction));
  const retired = await log.logged("the checkmark retires the member",
    new RegExp(`command=0x242 destination=retire member=${member}\\(.*\\) result=Ok\\(\\(\\)\\)$`));
  return { menu, asked, retired };
}

// Each case starts from a fresh load and returns its checks.
const cases = [
  {
    name: "character-move",
    code: CHARACTER_MOVE,
    // 0x201 -> the Move mission order 0x241 (FUN_0053c810): the secondary
    // system is the player's, so FUN_004f4b60 passes.
    async run(page, faction, directory, log) {
      const menu = await openMenu(page, agentCell(faction), directory, "agent");
      assert.deepEqual([menu.move_row, menu.confirmed_move_row], [0, 1], JSON.stringify(menu));
      await choose(page, menu, menu.move_row, "Move");
      await release(page, menu);
      const moved = await log.logged("the release orders the Move", new RegExp(
        `character_move character=Some\\(".+"\\) to=Some\\("${escapeRegExp(menu.target_name)}"\\) fleet=None result=Ok\\(\\d+\\)$`));
      return { menu, moved };
    },
  },
  {
    name: "character-move-refused",
    code: CHARACTER_COMMAND,
    // FUN_004f4b60: a movement kind's target of another side is refused
    // with 1/0x28.
    async run(page, faction, directory, log) {
      const menu = await openMenu(page, agentCell(faction), directory, "agent");
      await choose(page, menu, menu.move_row, "Move");
      await release(page, menu);
      const refused = await log.logged("the enemy system refuses the Move", new RegExp(
        `character_move character=Some\\(".+"\\) to=Some\\("${escapeRegExp(menu.target_name)}"\\) fleet=None result=Err\\("1/0x28"\\)$`));
      return { menu, refused };
    },
  },
  {
    name: "special-force-move-refused",
    code: RECON_TARGETING,
    // A special force's Move is the same mission order with the unit as
    // its team; the target is the other side's.
    async run(page, faction, directory, log) {
      const menu = await openMenu(page, agentCell(faction), directory, "unit");
      assert.deepEqual([menu.move_row, menu.confirmed_move_row], [0, 1], JSON.stringify(menu));
      await choose(page, menu, menu.move_row, "Move");
      await release(page, menu);
      const refused = await log.logged("the enemy system refuses the unit's Move", new RegExp(
        `special_force_move unit=.+ to=Some\\("${escapeRegExp(menu.target_name)}"\\) fleet=None result=Err\\("1/0x28"\\)$`));
      return { menu, refused };
    },
  },
  {
    name: "character-retire",
    code: CHARACTER_MOVE,
    async run(page, faction, directory, log) {
      return retire(page, faction, directory, log, "Character");
    },
  },
  {
    name: "special-force-retire",
    code: RECON_TARGETING,
    // FUN_00533ba0: a special force's Retire check.
    async run(page, faction, directory, log) {
      return retire(page, faction, directory, log, "SpecialForce");
    },
  },
  {
    name: "abort",
    code: QUADRANTS,
    // FUN_00521600: Abort takes only running missions (phase 8). The
    // fixture's missions are ordered on day 0, so the icon's Abort is
    // disabled until a day runs them to their timers; then the
    // checkmark ends the player's mission with code 2 (FUN_005233a0).
    async run(page, faction, directory, log, setup) {
      const icon = setup.quadrants.find((entry) => entry.system_dat_id === setup.primary_dat_id
        && entry.quadrant === "missions");
      assert.ok(icon, `the setup reports the primary mission icon: ${JSON.stringify(setup)}`);
      const [left, top] = icon.rect;
      const at = { x: Math.round(left) + 24, y: Math.round(top) + 9 };
      const before = await openMenu(page, at, directory, "ordered");
      assert.notEqual(before.abort_row, null, JSON.stringify(before));
      assert.equal(before.abort_enabled, false, `an ordered mission is not yet abortable: ${JSON.stringify(before)}`);
      await page.keyboard.press("Escape");
      await frames(page, 2);
      await setSpeed(page, setup, SPEED_ROWS.medium);
      // Medium is 1.2 s a day (game_speed.rs); two days start the
      // missions, well inside Diplomacy's five-day minimum timer.
      await page.waitForTimeout(2_600);
      await setSpeed(page, setup, SPEED_ROWS.pause);
      // The alert against A0 tooltip/321-resume-box-base (an Empire game;
      // REBDLOG 10622 at (114, 152), checkmark 10625 at (290, 286)): its
      // pixels outside the message's line and A0's pointer match, and the
      // line's ink is centred within 1 pixel with the same top and bottom
      // (Class_00658950_vt3c, font 5, 0x2f0fbff).
      let alert = null;
      if (a0Dir && faction.name === "empire") {
        await page.mouse.move(2, 470);
        await frames(page, 4);
        const port = PNG.sync.read(await page.screenshot({ animations: "disabled" }));
        fs.writeFileSync(path.join(directory, "pause-alert.png"), PNG.sync.write(port));
        const a0 = PNG.sync.read(fs.readFileSync(path.join(a0Dir, "321-resume-box-base.png")));
        const rgb = (png, x, y) => png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 3).join(",");
        const inside = (x, y, [rx, ry, rw, rh]) => x >= rx && x < rx + rw && y >= ry && y < ry + rh;
        const LINE = [157, 177, 325, 87];
        const POINTER = [318, 288, 24, 32];
        let differing = 0;
        for (let y = 152; y < 328; y += 1) {
          for (let x = 114; x < 526; x += 1) {
            if (inside(x, y, LINE) || inside(x, y, POINTER)) continue;
            if (rgb(port, x, y) !== rgb(a0, x, y)) differing += 1;
          }
        }
        const ink = (png) => {
          let box = null;
          for (let y = LINE[1]; y < LINE[1] + LINE[3]; y += 1) {
            for (let x = LINE[0]; x < LINE[0] + LINE[2]; x += 1) {
              const [r, g, b] = rgb(png, x, y).split(",").map(Number);
              if (r > 200 && g > 200 && b > 180) {
                box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
              }
            }
          }
          return box;
        };
        alert = { differing, a0_ink: ink(a0), port_ink: ink(port) };
        assert.equal(differing, 0, `${differing} pause alert pixels differ from A0`);
        const [a, p] = [alert.a0_ink, alert.port_ink];
        assert.ok(Math.abs((a[0] + a[2]) / 2 - (p[0] + p[2]) / 2) <= 1 && Math.abs(a[1] - p[1]) <= 1
          && Math.abs(a[3] - p[3]) <= 1, `the message's ink ${p} vs A0 ${a}`);
      }
      // "Resume Game Play?" (FUN_00415e60) is open: its button's tip is its
      // REBDLOG label "Resume" (A0 a0-vm/tooltip/321-resume-box.png, the
      // pointer at (320, 298), the tip at (320, 319)).
      assert.ok(await tipShows(page, { x: 320, y: 298 }), "the Resume button shows no tip");
      const running = await openMenu(page, at, directory, "running");
      assert.equal(running.abort_enabled, true, `a running mission takes Abort: ${JSON.stringify(running)}`);
      await choose(page, running, running.abort_row, "Abort");
      const asked = await log.logged("the Abort validates", /command=0x250 missions=\[\d+\]$/);
      await shot(page, directory, "confirmation");
      await click(page, confirmationCheckmark(faction));
      const aborted = await log.logged("the checkmark aborts the mission",
        /command=0x250 destination=abort mission=\d+ result=true$/);
      return { before, running, asked, aborted, alert };
    },
  },
];

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function inspect(server, faction, testCase, executable) {
  const directory = path.join(runDir, `${faction.name}-${testCase.name}`);
  fs.mkdirSync(directory, { recursive: true });
  const requests = [];
  const errors = [];
  const consoleLines = [];
  const launchAttempts = [];
  let browser;
  let context;
  let page;
  let result;
  try {
    browser = await launchBrowser(chromium, {
      executablePath: executable,
      headless: true,
      args: browserManifest.launch_arguments,
      timeout: 30_000,
    }, launchAttempts);
    context = await browser.newContext({
      viewport: { width: 640, height: 480 },
      deviceScaleFactor: 1,
      locale: "en-US",
      timezoneId: "America/New_York",
      colorScheme: "dark",
      reducedMotion: "reduce",
      serviceWorkers: "block",
    });
    page = await context.newPage();
    const serverOrigin = `http://127.0.0.1:${server.address().port}`;
    page.on("response", (response) => {
      if (new URL(response.url()).origin === serverOrigin) {
        requests.push({ url: new URL(response.url()).pathname, status: response.status() });
      }
    });
    page.on("requestfailed", (request) => {
      errors.push(`requestfailed:${request.url()}:${request.failure()?.errorText}`);
    });
    page.on("pageerror", (error) => errors.push(`pageerror:${error.stack || error.message}`));
    page.on("console", (message) => {
      consoleLines.push({ type: message.type(), text: message.text() });
      if (message.type() === "error" || /\[bmp_cache\] asset unavailable/i.test(message.text())) {
        errors.push(`console:${message.type()}:${message.text()}`);
      }
    });

    const fixtureCode = testCase.code | (faction.byte << 8);
    await page.goto(`${serverOrigin}/?fixture-code=${fixtureCode}`, { waitUntil: "load", timeout: 30_000 });
    await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status, null, { timeout: 30_000 });
    const ready = await page.evaluate(() => window.__openRebellionInterfaceReady);
    assert.equal(ready.status, "ready", JSON.stringify(ready));
    assert.equal(ready.code, fixtureCode);
    let setup = null;
    if (testCase.code === QUADRANTS) {
      await page.waitForFunction(() => window.__openRebellionInterfaceQuadrantSetup, null, { timeout: 10_000 });
      setup = await page.evaluate(() => window.__openRebellionInterfaceQuadrantSetup);
      assert.equal(setup.code, fixtureCode);
    }
    await page.evaluate(() => document.fonts.ready);
    const before = await shot(page, directory, "ready");

    // Wait for the first console line matching `pattern`.
    const log = {
      async logged(description, pattern, timeout = 10_000) {
        const started = Date.now();
        while (Date.now() - started < timeout) {
          const line = consoleLines.find((entry) => pattern.test(entry.text));
          if (line) return line.text;
          await page.waitForTimeout(100);
        }
        throw new Error(`${description}: ${pattern} not logged; last lines ${JSON.stringify(consoleLines.slice(-6).map((entry) => entry.text))}`);
      },
    };
    const checks = await testCase.run(page, faction, directory, log, setup);
    await page.mouse.move(2, 2);
    await frames(page);
    const after = await shot(page, directory, "final");
    assert.deepEqual(requests.map(({ url }) => url).sort(), [...expectedRequests].sort());
    assert.ok(requests.every(({ status }) => status === 200), "startup has non-200 requests");
    assert.deepEqual(errors, [], `browser diagnostics: ${errors.join("; ")}`);
    result = {
      status: "pass",
      faction: faction.name,
      case: testCase.name,
      fixture_code: fixtureCode,
      ready,
      setup,
      checks,
      screenshots: [before, after],
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
  } catch (error) {
    result = {
      status: "fail",
      faction: faction.name,
      case: testCase.name,
      error: String(error.stack || error),
      requests,
      errors,
      console: consoleLines,
      launch_attempts: launchAttempts,
      cleanup: "pending",
    };
    if (page) await shot(page, directory, "failure").catch(() => {});
  } finally {
    if (page) await page.close().catch((error) => errors.push(`page-close:${error}`));
    if (context) await context.close().catch((error) => errors.push(`context-close:${error}`));
    if (browser) await browser.close().catch((error) => errors.push(`browser-close:${error}`));
    result.cleanup = errors.some((error) => error.includes("-close:")) ? "failed" : "closed";
    if (result.cleanup === "failed") result.status = "fail";
  }
  return result;
}

async function main() {
  if (!noBuild) {
    execFileSync("bash", [path.join(root, "scripts/build-interface-test-wasm.sh")], {
      cwd: root,
      env: { ...process.env },
      stdio: "inherit",
    });
  }
  for (const required of ["index.html", "gl.js", "open-rebellion-test.wasm", "data/runtime.orpk"]) {
    assert.ok(fs.existsSync(path.join(site, required)), `fixture site is missing ${required}`);
  }
  fs.mkdirSync(runDir, { recursive: true });
  const executable = browserExecutable();
  const server = await startServer();
  let results;
  try {
    results = [];
    for (const faction of factions) {
      for (const testCase of cases) {
        if (only.length && !only.includes(`${faction.name}/${testCase.name}`)) continue;
        results.push(await inspect(server, faction, testCase, executable));
      }
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
  const passed = results.every((result) => result.status === "pass");
  const summary = {
    schema_version: 1,
    family: "personnel-orders",
    scope: "test-only Personnel orders through their original entries: a character's Move to its side's system and its refusal at the other side's (1/0x28), a special force's Move refused there, a character's and a special force's Retire through the confirmation (0x242), and a mission icon's Abort, disabled while the mission is only ordered and confirmed once it runs (0x250), on both sides",
    status: passed ? "pass" : "fail",
    browser_version: browserManifest.version,
    browser_executable: executable,
    launch_arguments: browserManifest.launch_arguments,
    profile: "one-new-muted-process-per-faction-and-case",
    muted: browserManifest.launch_arguments.includes("--mute-audio"),
    viewport: { width: 640, height: 480, device_scale_factor: 1 },
    results,
    wasm_sha256: sha256(fs.readFileSync(path.join(site, "open-rebellion-test.wasm"))),
    runtime_pack_sha256: sha256(fs.readFileSync(path.join(site, "data/runtime.orpk"))),
  };
  fs.writeFileSync(path.join(runDir, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
  if (!passed) {
    throw new Error(JSON.stringify(results.filter((result) => result.status !== "pass")
      .map(({ faction, case: name, error }) => ({ faction, case: name, error })), null, 2));
  }
  process.stdout.write(`${JSON.stringify({ run_dir: runDir, status: summary.status, cases: results.map((r) => `${r.faction}/${r.case}`), wasm_sha256: summary.wasm_sha256 }, null, 2)}\n`);
}

await main();
